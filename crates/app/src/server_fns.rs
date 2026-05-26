// leptos #[server] requires Send futures; worker-rs D1 is !Send.
// hand-dispatch from the worker fetch handler instead.
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
