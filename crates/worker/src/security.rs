use worker::*;

const ALLOWED_ORIGIN: &str = "https://realmdex.com";

pub fn guard_api(req: &Request) -> std::result::Result<(), Response> {
    let path = req.path();
    if !path.starts_with("/api/") {
        return Ok(());
    }

    // Allow requests served by `wrangler dev --local` (host is localhost / 127.0.0.1).
    let host = req.headers().get("Host").ok().flatten().unwrap_or_default();
    if host.starts_with("localhost") || host.starts_with("127.0.0.1") {
        return Ok(());
    }

    let origin = req.headers().get("Origin").ok().flatten();
    let referer = req.headers().get("Referer").ok().flatten();
    let ok_origin = origin.as_deref() == Some(ALLOWED_ORIGIN);
    let ok_referer = referer
        .as_deref()
        .map(|r| r.starts_with(&format!("{ALLOWED_ORIGIN}/")))
        .unwrap_or(false);
    if ok_origin || ok_referer {
        return Ok(());
    }

    let mut resp = Response::error("forbidden", 403).unwrap();
    let _ = resp.headers_mut().set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    Err(resp)
}

pub fn add_cors(mut resp: Response) -> Response {
    let _ = resp.headers_mut().set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    let _ = resp.headers_mut().set("Vary", "Origin");
    resp
}

// turnstile hook left for later; no-op stub.
#[allow(dead_code)]
pub async fn verify_turnstile(_token: &str) -> bool { true }
