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

async fn parse_and_validate(req: &mut Request) -> std::result::Result<ServerInput, Response> {
    let input: ServerInput = req.json().await.map_err(|e| {
        console_log!("bad json: {e}");
        security::add_cors(Response::error("invalid request", 400).unwrap())
    })?;
    validate_server_input(&input)
        .map_err(|e| security::add_cors(Response::error(e.0, 422).unwrap()))?;
    Ok(input)
}

async fn handle_api(req: &mut Request, env: &Env, path: &str, email: &str) -> Result<Response> {
    let db = env.d1("DB")?;

    if path == "/api/admin/servers" && req.method() == Method::Post {
        let input = match parse_and_validate(req).await {
            Ok(i) => i,
            Err(resp) => return Ok(resp),
        };
        console_log!("admin: {email} creating server '{}'", input.name);
        let server = db::create_server(&db, &input).await?;
        return Ok(security::add_cors(Response::from_json(&server)?));
    }

    let server_id = extract_server_id(path);

    if let Some(id) = server_id {
        if req.method() == Method::Put {
            let input = match parse_and_validate(req).await {
                Ok(i) => i,
                Err(resp) => return Ok(resp),
            };
            console_log!("admin: {email} updating server {id}");
            return match db::update_server(&db, id, &input).await? {
                Some(server) => Ok(security::add_cors(Response::from_json(&server)?)),
                None => Ok(security::add_cors(Response::error("not found", 404)?)),
            };
        }

        if req.method() == Method::Delete {
            console_log!("admin: {email} deleting server {id}");
            db::delete_server(&db, id).await?;
            return Ok(security::add_cors(Response::from_json(&serde_json::json!({"deleted": true}))?));
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
        let json = super::safe_json(&servers);
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
                let json = super::safe_json(&s);
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
            None => return Response::error("server not found", 404),
        }
    } else {
        return Response::error("not found", 404);
    };

    let html = super::html_shell(&title, "", &content, "admin-data", &json, super::ADMIN_CONTROLLER);
    Ok(security::add_cors(Response::from_html(html)?))
}
