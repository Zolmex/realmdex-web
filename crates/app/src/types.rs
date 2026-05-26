use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Pserver,
    RealmLike,
}

impl Category {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Category::Pserver => "pserver",
            Category::RealmLike => "realm-like",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    Wip,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DailyUptime {
    pub day: String,         // YYYY-MM-DD UTC
    pub uptime_percent: f32, // 0..=100
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SparkPoint {
    pub t_unix: i64, // seconds
    pub players: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerCardData {
    pub id: i64,
    pub name: String,
    pub icon_path: String,
    pub link: String,        // discord_link, may be a homepage URL
    pub status: Status,
    pub current_players: i32,
    pub peak_24h: i32,
    pub uptime_14d: Vec<DailyUptime>, // newest first; up to 14 entries
    pub sparkline: Vec<SparkPoint>,   // ~60 points covering the last hour
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InitialServers {
    pub pserver: Vec<ServerCardData>,
    pub realm_like: Vec<ServerCardData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdminServerRow {
    pub id: i64,
    pub name: String,
    pub icon_path: String,
    pub discord_link: String,
    pub host: String,
    pub category: Category,
    pub is_wip: bool,
    pub polled: bool,
    pub created_at: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerInput {
    pub name: String,
    pub host: String,
    pub category: Category,
    #[serde(default)]
    pub icon_path: Option<String>,
    #[serde(default)]
    pub discord_link: Option<String>,
    #[serde(default)]
    pub is_wip: bool,
    #[serde(default = "default_true")]
    pub polled: bool,
}

fn default_true() -> bool { true }
