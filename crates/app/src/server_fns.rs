// Note: We intentionally skip the Leptos `#[server]` macro here. server_fn 0.8
// requires server fn futures to be `Send`, but worker-rs 0.8 D1 calls use
// `JsFuture`, which is `!Send`. Once we add a workers-compatible Leptos
// integration (or move to a server-fn flavor that allows `LocalBoxFuture`), we
// can restore the macro. For now, the worker fetch handler calls these plain
// async functions directly.

use crate::types::{Category, ServerCardData, SparkPoint};
use leptos::prelude::expect_context;

pub async fn list_servers(category: Category) -> worker::Result<Vec<ServerCardData>> {
    let env = expect_context::<worker::Env>();
    let db = env.d1("DB")?;
    crate::db::list_servers_in_category(&db, category).await
}

pub async fn server_sparkline(server_id: i64) -> worker::Result<Vec<SparkPoint>> {
    let env = expect_context::<worker::Env>();
    let db = env.d1("DB")?;
    crate::db::sparkline_for_server(&db, server_id).await
}
