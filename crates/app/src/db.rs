// worker-rs 0.8 API: D1PreparedStatement::bind takes &[JsValue]; all() returns
// D1Result (not Result<D1Result>) which then yields .results::<T>()?.
use crate::types::{Category, DailyUptime, ServerCardData, SparkPoint, Status};
use worker::wasm_bindgen::JsValue;
use worker::D1Database;

#[derive(serde::Deserialize)]
struct ServerRow {
    id: i64,
    name: String,
    icon_path: Option<String>,
    discord_link: Option<String>,
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

pub async fn list_servers_in_category(
    db: &D1Database,
    category: Category,
) -> worker::Result<Vec<ServerCardData>> {
    let cat = category.as_db_str();

    let servers: Vec<ServerRow> = db
        .prepare("SELECT id, name, icon_path, discord_link, is_wip FROM servers WHERE category = ?1 ORDER BY id")
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
