use app::db;
use app::types::ServerInput;
use app::validation::validate_server_input;
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

    let (title, content) = if path == "/admin" || path == "/admin/" {
        let servers = db::list_servers_admin(&db).await.unwrap_or_default();
        let json = serde_json::to_string(&servers).unwrap_or_default().replace("</", "<\\/");
        ("Admin - Server List".to_string(), render_list_page(email, &json))
    } else if let Some(id) = path.strip_prefix("/admin/edit/").and_then(|s| s.parse::<i64>().ok()) {
        let server = db::get_server_admin(&db, id).await.unwrap_or(None);
        match server {
            Some(s) => {
                let json = serde_json::to_string(&s).unwrap_or_default().replace("</", "<\\/");
                (format!("Admin - Edit {}", s.name), render_edit_page(email, &json))
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
        </head><body>{content}</body></html>"
    );
    Response::from_html(html)
}

fn render_list_page(email: &str, servers_json: &str) -> String {
    format!(
        "<div class=\"admin-container\">\
            <div class=\"admin-header\">\
                <h1>RealmDex Admin</h1>\
                <span class=\"admin-user\">{email}</span>\
                <a href=\"/\" class=\"admin-back\">Back to site</a>\
            </div>\
            <div id=\"admin-flash\"></div>\
            <details class=\"admin-add-form\">\
                <summary>Add New Server</summary>\
                <form id=\"add-form\" class=\"admin-form\">\
                    <label>Name <input name=\"name\" required maxlength=\"100\"/></label>\
                    <label>Host URL <input name=\"host\" required placeholder=\"https://...\"/></label>\
                    <label>Category\
                        <select name=\"category\">\
                            <option value=\"pserver\">Private Server</option>\
                            <option value=\"realm-like\">Realm-Like</option>\
                        </select>\
                    </label>\
                    <label>Icon Path <input name=\"icon_path\" maxlength=\"500\" placeholder=\"/content/images/...\"/></label>\
                    <label>Discord Link <input name=\"discord_link\" maxlength=\"500\" placeholder=\"https://discord.gg/...\"/></label>\
                    <label><input type=\"checkbox\" name=\"is_wip\"/> Work in Progress</label>\
                    <label><input type=\"checkbox\" name=\"polled\" checked/> Polled</label>\
                    <button type=\"submit\">Create Server</button>\
                </form>\
            </details>\
            <table class=\"admin-table\">\
                <thead><tr>\
                    <th>ID</th><th>Name</th><th>Category</th><th>Host</th><th>Polled</th><th>WIP</th><th>Actions</th>\
                </tr></thead>\
                <tbody id=\"server-rows\"></tbody>\
            </table>\
            <script id=\"admin-data\" type=\"application/json\">{servers_json}</script>\
            <script>{controller}</script>\
        </div>",
        controller = super::ADMIN_CONTROLLER
    )
}

fn render_edit_page(email: &str, server_json: &str) -> String {
    format!(
        "<div class=\"admin-container\">\
            <div class=\"admin-header\">\
                <h1>RealmDex Admin — Edit Server</h1>\
                <span class=\"admin-user\">{email}</span>\
                <a href=\"/admin\" class=\"admin-back\">Back to list</a>\
            </div>\
            <div id=\"admin-flash\"></div>\
            <form id=\"edit-form\" class=\"admin-form\">\
                <input type=\"hidden\" name=\"id\" id=\"edit-id\"/>\
                <label>Name <input name=\"name\" required maxlength=\"100\"/></label>\
                <label>Host URL <input name=\"host\" required/></label>\
                <label>Category\
                    <select name=\"category\">\
                        <option value=\"pserver\">Private Server</option>\
                        <option value=\"realm-like\">Realm-Like</option>\
                    </select>\
                </label>\
                <label>Icon Path <input name=\"icon_path\" maxlength=\"500\"/></label>\
                <label>Discord Link <input name=\"discord_link\" maxlength=\"500\"/></label>\
                <label><input type=\"checkbox\" name=\"is_wip\"/> Work in Progress</label>\
                <label><input type=\"checkbox\" name=\"polled\"/> Polled</label>\
                <button type=\"submit\">Save Changes</button>\
            </form>\
            <script id=\"admin-data\" type=\"application/json\">{server_json}</script>\
            <script>{controller}</script>\
        </div>",
        controller = super::ADMIN_CONTROLLER
    )
}
