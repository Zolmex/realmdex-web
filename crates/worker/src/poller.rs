use futures::future::join_all;
use worker::js_sys;
use worker::wasm_bindgen::{JsCast, JsValue};
use worker::wasm_bindgen_futures;
use worker::*;

const TIMEOUT_MS: u32 = 10_000;

#[derive(serde::Deserialize)]
struct Row {
    id: i64,
    host: String,
}

pub async fn run(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    let servers: Vec<Row> = db
        .prepare("SELECT id, host FROM servers WHERE polled = 1")
        .all()
        .await?
        .results()?;

    if servers.is_empty() {
        return Ok(());
    }

    let outcomes: Vec<(i64, i64, i64)> =
        join_all(servers.into_iter().map(|s| poll_one(s.id, s.host))).await;

    // D1 caps at 100 bind params per prepared statement
    const MAX_ROWS: usize = 33;
    for chunk in outcomes.chunks(MAX_ROWS) {
        let mut sql = String::from("INSERT INTO server_polls (server_id, online, players) VALUES ");
        let mut binds: Vec<JsValue> = Vec::with_capacity(chunk.len() * 3);
        for (i, (id, online, players)) in chunk.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            let base = i * 3;
            sql.push_str(&format!("(?{}, ?{}, ?{})", base + 1, base + 2, base + 3));
            binds.push(JsValue::from_f64(*id as f64));
            binds.push(JsValue::from_f64(*online as f64));
            binds.push(JsValue::from_f64(*players as f64));
        }
        db.prepare(&sql).bind(&binds)?.run().await?;
    }
    Ok(())
}

async fn poll_one(id: i64, host: String) -> (i64, i64, i64) {
    if host.is_empty() {
        return (id, 0, 0);
    }
    let mut req = match Request::new(&host, Method::Get) {
        Ok(r) => r,
        Err(e) => {
            console_log!("[poll id={id}] bad request for {host}: {e:?}");
            return (id, 0, 0);
        }
    };
    let _ = req.headers_mut().map(|h| h.set("User-Agent", "realmdex/1.0"));
    let fetch_fut = async move { Fetch::Request(req).send().await };
    let mut resp = match wait_with_timeout(fetch_fut, TIMEOUT_MS).await {
        Some(Ok(r)) => r,
        Some(Err(e)) => {
            console_log!("[poll id={id}] fetch error for {host}: {e:?}");
            return (id, 0, 0);
        }
        None => {
            console_log!("[poll id={id}] timeout for {host}");
            return (id, 0, 0);
        }
    };
    if resp.status_code() != 200 {
        let body = resp.text().await.unwrap_or_default();
        console_log!("[poll id={id}] {host} returned status {} body={}", resp.status_code(), &body[..body.len().min(200)]);
        return (id, 0, 0);
    }
    let body = match resp.text().await {
        Ok(s) => s,
        Err(e) => {
            console_log!("[poll id={id}] body read error for {host}: {e:?}");
            return (id, 0, 0);
        }
    };
    let players: i64 = body.trim().parse().unwrap_or(0);
    (id, 1, players)
}

async fn wait_with_timeout<F, T>(fut: F, ms: u32) -> Option<T>
where
    F: std::future::Future<Output = T>,
{
    use futures::future::{select, Either};
    use futures::FutureExt;

    let timer = timeout_future(ms).fuse();
    futures::pin_mut!(fut, timer);
    match select(fut, timer).await {
        Either::Left((v, _)) => Some(v),
        Either::Right(_) => None,
    }
}

async fn timeout_future(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let set_timeout = js_sys::Reflect::get(&js_sys::global(), &"setTimeout".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>();
        let _ = set_timeout.call2(&JsValue::NULL, &resolve, &JsValue::from(ms));
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
