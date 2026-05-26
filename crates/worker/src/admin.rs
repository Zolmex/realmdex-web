use app::components::admin::{edit::{EditPage, EditPageProps}, list::{ListPage, ListPageProps}};
use app::db;
use app::types::ServerInput;
use app::validation::validate_server_input;
use leptos::prelude::Owner;
use leptos::tachys::view::RenderHtml;
use worker::*;

use crate::security;

pub async fn handle(req: &mut Request, env: &Env, path: &str, email: &str) -> Result<Response> {
    if path.starts_with("/api/admin/") {
        return handle_api(req, env, path, email).await;
    }
    handle_page(req, env, path, email).await
}

async fn handle_api(req: &mut Request, env: &Env, path: &str, email: &str) -> Result<Response> {
    let db = env.d1("DB")?;

    if path == "/api/admin/servers" && req.method() == Method::Post {
        let input: ServerInput = match req.json().await {
            Ok(b) => b,
            Err(e) => return Ok(security::add_cors(Response::error(format!("bad json: {e}"), 400)?)),
        };
        if let Err(e) = validate_server_input(&input) {
            return Ok(security::add_cors(Response::error(e.0, 422)?));
        }
        console_log!("admin: {email} creating server '{}'", input.name);
        let server = db::create_server(&db, &input).await?;
        return Ok(security::add_cors(Response::from_json(&server)?));
    }

    let server_id = extract_server_id(path);

    if let Some(id) = server_id {
        if req.method() == Method::Put {
            let input: ServerInput = match req.json().await {
                Ok(b) => b,
                Err(e) => return Ok(security::add_cors(Response::error(format!("bad json: {e}"), 400)?)),
            };
            if let Err(e) = validate_server_input(&input) {
                return Ok(security::add_cors(Response::error(e.0, 422)?));
            }
            console_log!("admin: {email} updating server {id}");
            return match db::update_server(&db, id, &input).await? {
                Some(server) => Ok(security::add_cors(Response::from_json(&server)?)),
                None => Ok(security::add_cors(Response::error("not found", 404)?)),
            };
        }

        if req.method() == Method::Delete {
            console_log!("admin: {email} deleting server {id}");
            return if db::delete_server(&db, id).await? {
                Ok(security::add_cors(Response::from_json(&serde_json::json!({"deleted": true}))?))
            } else {
                Ok(security::add_cors(Response::error("not found", 404)?))
            };
        }
    }

    Ok(security::add_cors(Response::error("not found", 404)?))
}

fn extract_server_id(path: &str) -> Option<i64> {
    path.strip_prefix("/api/admin/servers/")
        .and_then(|s| s.parse().ok())
}

async fn handle_page(_req: &mut Request, env: &Env, path: &str, email: &str) -> Result<Response> {
    let db = env.d1("DB")?;

    let (title, content, json) = if path == "/admin" || path == "/admin/" {
        let servers = db::list_servers_admin(&db).await.unwrap_or_default();
        let json = serde_json::to_string(&servers).unwrap_or_default().replace("</", "<\\/");
        let owner = Owner::new();
        let html = owner.with(|| {
            ListPage(ListPageProps {
                email: email.to_string(),
                servers,
            })
            .to_html()
        });
        ("Admin - Server List".to_string(), html, json)
    } else if let Some(id) = path.strip_prefix("/admin/edit/").and_then(|s| s.parse::<i64>().ok()) {
        let server = db::get_server_admin(&db, id).await.unwrap_or(None);
        match server {
            Some(s) => {
                let title = format!("Admin - Edit {}", s.name);
                let json = serde_json::to_string(&s).unwrap_or_default().replace("</", "<\\/");
                let owner = Owner::new();
                let html = owner.with(|| {
                    EditPage(EditPageProps {
                        email: email.to_string(),
                        server: s,
                    })
                    .to_html()
                });
                (title, html, json)
            }
            None => return Ok(Response::error("server not found", 404)?),
        }
    } else {
        return Ok(Response::error("not found", 404)?);
    };

    let html = format!(
        "<!DOCTYPE html><html lang=\"en\"><head>\
            <meta charset=\"utf-8\">\
            <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
            <title>{title}</title>\
            <link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">\
            <link rel=\"stylesheet\" href=\"/styles/index.css\">\
        </head><body>{content}\
        <script id=\"admin-data\" type=\"application/json\">{json}</script>\
        <script>{controller}</script>\
        </body></html>",
        controller = super::ADMIN_CONTROLLER
    );
    Response::from_html(html)
}
