use worker::js_sys;
use worker::wasm_bindgen::JsValue;
use worker::wasm_bindgen_futures::JsFuture;
use worker::*;

const ALLOWED_ORIGIN: &str = "https://realmdex.com";
const ALLOWED_ORIGIN_SLASH: &str = "https://realmdex.com/";

#[derive(serde::Deserialize)]
struct JwtHeader {
    kid: String,
}

#[derive(serde::Deserialize)]
struct JwtClaims {
    email: Option<String>,
    aud: serde_json::Value,
    exp: u64,
}

#[derive(serde::Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

#[derive(serde::Deserialize)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
}

pub fn guard_api(req: &Request) -> std::result::Result<(), Response> {
    let path = req.path();
    if !path.starts_with("/api/") {
        return Ok(());
    }

    let host = req.headers().get("Host").ok().flatten().unwrap_or_default();
    if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        return Ok(());
    }

    let origin = req.headers().get("Origin").ok().flatten();
    let referer = req.headers().get("Referer").ok().flatten();
    let ok_origin = origin.as_deref() == Some(ALLOWED_ORIGIN);
    let ok_referer = referer
        .as_deref()
        .map(|r| r.starts_with(ALLOWED_ORIGIN_SLASH))
        .unwrap_or(false);
    if ok_origin || ok_referer {
        return Ok(());
    }

    let mut resp = Response::error("forbidden", 403).unwrap();
    let _ = resp.headers_mut().set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    Err(resp)
}

pub fn add_cors(mut resp: Response) -> Response {
    let h = resp.headers_mut();
    let _ = h.set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    let _ = h.set("Vary", "Origin");
    let _ = h.set("X-Content-Type-Options", "nosniff");
    let _ = h.set("X-Frame-Options", "DENY");
    let _ = h.set("Referrer-Policy", "strict-origin-when-cross-origin");
    resp
}

fn get_cookie(req: &Request, name: &str) -> Option<String> {
    let header = req.headers().get("Cookie").ok().flatten()?;
    for pair in header.split(';') {
        let pair = pair.trim();
        if let Some(val) = pair.strip_prefix(name) {
            if let Some(val) = val.strip_prefix('=') {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn base64url_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    engine
        .decode(input)
        .map_err(|e| worker::Error::RustError(format!("base64: {e}")))
}

pub async fn guard_admin(req: &Request, env: &Env) -> std::result::Result<String, Response> {
    let host = req.headers().get("Host").ok().flatten().unwrap_or_default();
    // safe: Cloudflare sets Host from the request URL; can't be spoofed in production
    if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        return Ok("dev@localhost".into());
    }

    let token = get_cookie(req, "CF_Authorization")
        .ok_or_else(|| Response::error("unauthorized: no token", 401).unwrap())?;

    let email = verify_access_jwt(&token, env).await.map_err(|e| {
        console_log!("admin auth failed: {e}");
        Response::error("forbidden", 403).unwrap()
    })?;

    Ok(email)
}

async fn verify_access_jwt(token: &str, env: &Env) -> Result<String> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err("malformed jwt".into());
    }

    let header_bytes = base64url_decode(parts[0])?;
    let header: JwtHeader = serde_json::from_slice(&header_bytes)
        .map_err(|e| worker::Error::RustError(format!("jwt header: {e}")))?;

    let payload_bytes = base64url_decode(parts[1])?;
    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| worker::Error::RustError(format!("jwt payload: {e}")))?;

    let now = (js_sys::Date::now() / 1000.0) as u64;
    if claims.exp < now {
        return Err("jwt expired".into());
    }

    let expected_aud = env
        .secret("CF_ACCESS_AUD")
        .map_err(|_| worker::Error::RustError("CF_ACCESS_AUD secret not set".into()))?
        .to_string();
    let aud_ok = match &claims.aud {
        serde_json::Value::String(s) => s == &expected_aud,
        serde_json::Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some(&expected_aud)),
        _ => false,
    };
    if !aud_ok {
        return Err("jwt aud mismatch".into());
    }

    let team_domain = env
        .secret("CF_ACCESS_TEAM_DOMAIN")
        .map_err(|_| worker::Error::RustError("CF_ACCESS_TEAM_DOMAIN secret not set".into()))?
        .to_string();
    let certs_url = format!("https://{team_domain}.cloudflareaccess.com/cdn-cgi/access/certs");

    let certs_req = Request::new(&certs_url, Method::Get)
        .map_err(|e| worker::Error::RustError(format!("certs request: {e}")))?;
    let mut certs_resp = Fetch::Request(certs_req).send().await?;
    let jwks: JwksResponse = certs_resp.json().await?;

    let key = jwks
        .keys
        .iter()
        .find(|k| k.kid == header.kid)
        .ok_or_else(|| worker::Error::RustError("no matching kid in jwks".into()))?;

    verify_rs256(parts[0], parts[1], parts[2], &key.n, &key.e).await?;

    claims
        .email
        .ok_or_else(|| worker::Error::RustError("jwt missing email claim".into()))
}

async fn verify_rs256(
    header_b64: &str,
    payload_b64: &str,
    sig_b64: &str,
    n: &str,
    e: &str,
) -> Result<()> {
    let sig = base64url_decode(sig_b64)?;
    let message = format!("{header_b64}.{payload_b64}");

    let crypto = js_sys::Reflect::get(&js_sys::global(), &"crypto".into())
        .map_err(|_| worker::Error::RustError("no crypto global".into()))?;
    let subtle = js_sys::Reflect::get(&crypto, &"subtle".into())
        .map_err(|_| worker::Error::RustError("no subtle crypto".into()))?;

    let jwk = js_sys::Object::new();
    js_sys::Reflect::set(&jwk, &"kty".into(), &"RSA".into()).unwrap();
    js_sys::Reflect::set(&jwk, &"n".into(), &JsValue::from_str(n)).unwrap();
    js_sys::Reflect::set(&jwk, &"e".into(), &JsValue::from_str(e)).unwrap();

    let algo = js_sys::Object::new();
    js_sys::Reflect::set(&algo, &"name".into(), &"RSASSA-PKCS1-v1_5".into()).unwrap();
    js_sys::Reflect::set(&algo, &"hash".into(), &"SHA-256".into()).unwrap();

    let import_fn: js_sys::Function = js_sys::Reflect::get(&subtle, &"importKey".into())
        .map_err(|_| worker::Error::RustError("no importKey".into()))?
        .into();
    let import_args = js_sys::Array::new();
    import_args.push(&"jwk".into());
    import_args.push(&jwk.into());
    import_args.push(&algo.clone().into());
    import_args.push(&false.into());
    import_args.push(&js_sys::Array::of1(&"verify".into()).into());
    let import_promise: js_sys::Promise = import_fn
        .apply(&subtle, &import_args)
        .map_err(|e| worker::Error::RustError(format!("importKey: {e:?}")))?
        .into();

    let crypto_key = JsFuture::from(import_promise)
        .await
        .map_err(|e| worker::Error::RustError(format!("importKey await: {e:?}")))?;

    let sig_array = js_sys::Uint8Array::from(sig.as_slice());
    let msg_array = js_sys::Uint8Array::from(message.as_bytes());

    let verify_fn: js_sys::Function = js_sys::Reflect::get(&subtle, &"verify".into())
        .map_err(|_| worker::Error::RustError("no verify".into()))?
        .into();
    let verify_args =
        js_sys::Array::of4(&algo.into(), &crypto_key, &sig_array.into(), &msg_array.into());
    let verify_promise: js_sys::Promise = verify_fn
        .apply(&subtle, &verify_args)
        .map_err(|e| worker::Error::RustError(format!("verify: {e:?}")))?
        .into();

    let result = JsFuture::from(verify_promise)
        .await
        .map_err(|e| worker::Error::RustError(format!("verify await: {e:?}")))?;

    if result.as_bool() == Some(true) {
        Ok(())
    } else {
        Err("jwt signature invalid".into())
    }
}
