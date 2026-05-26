use futures::future::join_all;
use worker::wasm_bindgen::JsValue;
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

    let mut sql = String::from("INSERT INTO server_polls (server_id, online, players) VALUES ");
    let mut binds: Vec<JsValue> = Vec::with_capacity(outcomes.len() * 3);
    for (i, (id, online, players)) in outcomes.iter().enumerate() {
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
    Ok(())
}

async fn poll_one(id: i64, host: String) -> (i64, i64, i64) {
    if host.is_empty() {
        return (id, 0, 0);
    }
    let req = match Request::new(&host, Method::Get) {
        Ok(r) => r,
        Err(_) => return (id, 0, 0),
    };
    let fetch_fut = async move { Fetch::Request(req).send().await };
    let mut resp = match wait_with_timeout(fetch_fut, TIMEOUT_MS).await {
        Some(Ok(r)) => r,
        _ => return (id, 0, 0),
    };
    if resp.status_code() != 200 {
        return (id, 0, 0);
    }
    let body = match resp.text().await {
        Ok(s) => s,
        Err(_) => return (id, 0, 0),
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

    let timer = gloo_timers::future::TimeoutFuture::new(ms).fuse();
    futures::pin_mut!(fut, timer);
    match select(fut, timer).await {
        Either::Left((v, _)) => Some(v),
        Either::Right(_) => None,
    }
}
