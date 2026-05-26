use crate::types::{Category, DailyUptime, ServerCardData, SparkPoint, Status};
use worker::wasm_bindgen::JsValue;
use worker::D1Database;

#[derive(serde::Deserialize)]
struct ServerRow {
    id: i64,
    name: String,
    icon_path: Option<String>,
    discord_link: Option<String>,
    host: String,
    is_wip: i64,
}

#[derive(serde::Deserialize)]
struct LastPollRow {
    server_id: i64,
    players: i64,
    online: i64,
}

#[derive(serde::Deserialize)]
struct PeakRow {
    server_id: i64,
    peak: Option<i64>,
}

#[derive(serde::Deserialize)]
struct DailyRow {
    server_id: i64,
    day: String,
    uptime_percent: f64,
}

#[derive(serde::Deserialize)]
struct SparkRow {
    server_id: i64,
    t_unix: i64,
    players: i64,
}

#[derive(serde::Deserialize)]
struct AdminRow {
    id: i64,
    name: String,
    icon_path: Option<String>,
    discord_link: Option<String>,
    host: String,
    category: String,
    is_wip: i64,
    polled: i64,
    created_at: String,
}

fn admin_row_to_type(r: AdminRow) -> crate::types::AdminServerRow {
    crate::types::AdminServerRow {
        id: r.id,
        name: r.name,
        icon_path: r.icon_path.unwrap_or_default(),
        discord_link: r.discord_link.unwrap_or_default(),
        host: r.host,
        category: if r.category == "realm-like" { Category::RealmLike } else { Category::Pserver },
        is_wip: r.is_wip == 1,
        polled: r.polled == 1,
        created_at: r.created_at,
    }
}

pub async fn list_servers_in_category(
    db: &D1Database,
    category: Category,
) -> worker::Result<Vec<ServerCardData>> {
    let cat = category.as_db_str();

    let servers: Vec<ServerRow> = db
        .prepare("SELECT id, name, icon_path, discord_link, host, is_wip FROM servers WHERE category = ?1 ORDER BY id")
        .bind(&[JsValue::from_str(cat)])?
        .all()
        .await?
        .results()?;

    if servers.is_empty() {
        return Ok(Vec::new());
    }

    let last_polls: Vec<LastPollRow> = db
        .prepare(
            "SELECT server_id, players, online FROM server_polls p
             WHERE id = (SELECT MAX(id) FROM server_polls WHERE server_id = p.server_id)
               AND server_id IN (SELECT id FROM servers WHERE category = ?1)",
        )
        .bind(&[JsValue::from_str(cat)])?
        .all()
        .await?
        .results()?;

    let peaks: Vec<PeakRow> = db
        .prepare(
            "SELECT server_id, MAX(players) AS peak FROM server_polls
             WHERE time >= datetime('now', '-1 day')
               AND server_id IN (SELECT id FROM servers WHERE category = ?1)
             GROUP BY server_id",
        )
        .bind(&[JsValue::from_str(cat)])?
        .all()
        .await?
        .results()?;

    let daily: Vec<DailyRow> = db
        .prepare(
            "SELECT server_id, day, ROUND(100.0 * up_checks / total_checks, 2) AS uptime_percent
             FROM server_polls_daily
             WHERE day >= date('now', '-14 days')
               AND server_id IN (SELECT id FROM servers WHERE category = ?1)
             ORDER BY server_id, day DESC",
        )
        .bind(&[JsValue::from_str(cat)])?
        .all()
        .await?
        .results()?;

    let sparks: Vec<SparkRow> = db
        .prepare(
            "SELECT server_id, CAST(strftime('%s', time) AS INTEGER) AS t_unix, players
             FROM server_polls
             WHERE time >= datetime('now', '-1 hour')
               AND server_id IN (SELECT id FROM servers WHERE category = ?1)
             ORDER BY server_id, time",
        )
        .bind(&[JsValue::from_str(cat)])?
        .all()
        .await?
        .results()?;

    let mut out = Vec::with_capacity(servers.len());
    for s in servers {
        let last = last_polls.iter().find(|r| r.server_id == s.id);
        let peak = peaks
            .iter()
            .find(|r| r.server_id == s.id)
            .and_then(|r| r.peak)
            .unwrap_or(0) as i32;
        let online = last.map(|r| r.online == 1).unwrap_or(false);
        let status = if s.is_wip == 1 {
            Status::Wip
        } else if online {
            Status::Online
        } else {
            Status::Offline
        };

        let uptime_14d = daily
            .iter()
            .filter(|r| r.server_id == s.id)
            .map(|r| DailyUptime {
                day: r.day.clone(),
                uptime_percent: r.uptime_percent as f32,
            })
            .collect();

        let sparkline = sparks
            .iter()
            .filter(|r| r.server_id == s.id)
            .map(|r| SparkPoint {
                t_unix: r.t_unix,
                players: r.players as i32,
            })
            .collect();

        let secure = s.host.starts_with("https://");

        out.push(ServerCardData {
            id: s.id,
            name: s.name,
            icon_path: s.icon_path.unwrap_or_default(),
            link: s.discord_link.unwrap_or_default(),
            status,
            current_players: last.map(|r| r.players as i32).unwrap_or(0),
            peak_24h: peak,
            uptime_14d,
            sparkline,
            secure,
        });
    }
    Ok(out)
}

pub async fn sparkline_for_server(
    db: &D1Database,
    server_id: i64,
) -> worker::Result<Vec<SparkPoint>> {
    #[derive(serde::Deserialize)]
    struct Row {
        t_unix: i64,
        players: i64,
    }

    let rows: Vec<Row> = db
        .prepare(
            "SELECT CAST(strftime('%s', time) AS INTEGER) AS t_unix, players
             FROM server_polls
             WHERE server_id = ?1 AND time >= datetime('now', '-1 hour')
             ORDER BY time",
        )
        .bind(&[JsValue::from_f64(server_id as f64)])?
        .all()
        .await?
        .results()?;
    Ok(rows
        .into_iter()
        .map(|r| SparkPoint {
            t_unix: r.t_unix,
            players: r.players as i32,
        })
        .collect())
}

pub async fn list_servers_admin(db: &D1Database) -> worker::Result<Vec<crate::types::AdminServerRow>> {
    let rows: Vec<AdminRow> = db
        .prepare("SELECT id, name, icon_path, discord_link, host, category, is_wip, polled, created_at FROM servers ORDER BY id")
        .all()
        .await?
        .results()?;
    Ok(rows.into_iter().map(admin_row_to_type).collect())
}

pub async fn get_server_admin(db: &D1Database, id: i64) -> worker::Result<Option<crate::types::AdminServerRow>> {
    let rows: Vec<AdminRow> = db
        .prepare("SELECT id, name, icon_path, discord_link, host, category, is_wip, polled, created_at FROM servers WHERE id = ?1")
        .bind(&[JsValue::from_f64(id as f64)])?
        .all()
        .await?
        .results()?;
    Ok(rows.into_iter().next().map(admin_row_to_type))
}

fn bool_js(b: bool) -> JsValue {
    JsValue::from_f64(if b { 1.0 } else { 0.0 })
}

fn server_input_binds(input: &crate::types::ServerInput) -> Vec<JsValue> {
    vec![
        JsValue::from_str(&input.name),
        JsValue::from_str(input.icon_path.as_deref().unwrap_or("")),
        JsValue::from_str(input.discord_link.as_deref().unwrap_or("")),
        JsValue::from_str(&input.host),
        JsValue::from_str(input.category.as_db_str()),
        bool_js(input.is_wip),
        bool_js(input.polled),
    ]
}

pub async fn create_server(db: &D1Database, input: &crate::types::ServerInput) -> worker::Result<crate::types::AdminServerRow> {
    db.prepare(
        "INSERT INTO servers (name, icon_path, discord_link, host, category, is_wip, polled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )
    .bind(&server_input_binds(input))?
    .run()
    .await?;

    let rows: Vec<AdminRow> = db
        .prepare("SELECT id, name, icon_path, discord_link, host, category, is_wip, polled, created_at FROM servers WHERE id = last_insert_rowid()")
        .all()
        .await?
        .results()?;
    rows.into_iter().next().map(admin_row_to_type)
        .ok_or_else(|| worker::Error::RustError("insert succeeded but row not found".into()))
}

pub async fn update_server(db: &D1Database, id: i64, input: &crate::types::ServerInput) -> worker::Result<Option<crate::types::AdminServerRow>> {
    let mut binds = server_input_binds(input);
    binds.push(JsValue::from_f64(id as f64));
    db.prepare(
        "UPDATE servers SET name = ?1, icon_path = ?2, discord_link = ?3, host = ?4, category = ?5, is_wip = ?6, polled = ?7
         WHERE id = ?8"
    )
    .bind(&binds)?
    .run()
    .await?;

    get_server_admin(db, id).await
}

pub async fn delete_server(db: &D1Database, id: i64) -> worker::Result<bool> {
    // D1 doesn't enforce PRAGMA foreign_keys per-connection, so CASCADE won't fire
    let id_bind = [JsValue::from_f64(id as f64)];
    db.prepare("DELETE FROM server_polls WHERE server_id = ?1")
        .bind(&id_bind)?.run().await?;
    db.prepare("DELETE FROM server_polls_daily WHERE server_id = ?1")
        .bind(&id_bind)?.run().await?;
    db.prepare("DELETE FROM servers WHERE id = ?1")
        .bind(&id_bind)?.run().await?;
    Ok(true)
}
