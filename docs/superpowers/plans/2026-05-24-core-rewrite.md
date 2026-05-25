# Core Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the PHP implementation of realmdex.com with a Rust + Leptos fullstack app on Cloudflare Workers + D1, matching today's site exactly plus live data updates and per-card sparklines.

**Architecture:** One Cargo workspace, two crates (`app` = Leptos UI shared by SSR + hydrate; `worker` = Cloudflare entrypoints + D1 + Cron handlers). Page renders SSR from a Worker, hydrates with WASM, refreshes data every 30s via Leptos `Resource`s calling server functions. A Cron Trigger fans out parallel polls and batch-writes to D1. A daily Cron rolls raw polls into a per-day summary table and prunes the raw table to ~30 days. Static assets (CSS, WASM, images) ship via Workers Assets. Deploy via GitHub Actions using OIDC to Cloudflare with a required-reviewer `production` environment.

**Tech Stack:** Rust (stable), Leptos (fullstack, SSR + hydrate), `worker-rs`, Cloudflare D1, Cloudflare Workers, Cloudflare Workers Assets, Cloudflare Cron Triggers, Cloudflare Rate Limiting Rules, `cargo-leptos`, `wrangler`, GitHub Actions OIDC.

**Spec:** [`docs/superpowers/specs/2026-05-24-core-rewrite-design.md`](../specs/2026-05-24-core-rewrite-design.md)

**Project conventions:** See `CLAUDE.md` — KISS, simple loops, share-don't-duplicate, minimal human-style comments.

---

## Phase 0 — Scaffolding (verify the stack actually works before building on it)

### Task 0: Verify Leptos-on-Workers integration with hello-world

> This task exists because the Leptos-on-Workers integration is the riskiest assumption in the plan. Get a literal "Hello from Leptos" rendering from `wrangler dev` before doing anything else. If the integration shape differs from what's sketched in later tasks (e.g. a different crate name, a different `wrangler.toml` shape), this is the cheap place to discover it. Adjust subsequent tasks before continuing.

**Files:**
- Create: `Cargo.toml` (workspace)
- Create: `rust-toolchain.toml`
- Create: `crates/app/Cargo.toml`
- Create: `crates/app/src/lib.rs`
- Create: `crates/worker/Cargo.toml`
- Create: `crates/worker/src/lib.rs`
- Create: `wrangler.toml`
- Create: `.gitignore`

- [ ] **Step 1: Add `.gitignore` entries for build artifacts**

```gitignore
/target
/.wrangler
/dist
/node_modules
*.wasm
.DS_Store
.env
!example.env
```

- [ ] **Step 2: Write workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/app", "crates/worker"]

[workspace.package]
edition = "2021"
version = "0.1.0"
license = "MIT"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

- [ ] **Step 3: Pin toolchain**

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
targets = ["wasm32-unknown-unknown"]
components = ["clippy", "rustfmt"]
```

- [ ] **Step 4: Write minimal `crates/app/Cargo.toml`**

```toml
[package]
name = "app"
edition.workspace = true
version.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.6", features = ["nightly"] }
leptos_meta = { version = "0.6" }
leptos_router = { version = "0.6" }
serde = { version = "1", features = ["derive"] }

[features]
default = []
hydrate = ["leptos/hydrate", "leptos_meta/hydrate", "leptos_router/hydrate"]
ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr"]
```

> Pinning Leptos to a known-working minor (0.6) for the start; the implementing agent may need to bump if the workers integration crate requires a newer version. If so, bump both together.

- [ ] **Step 5: Write `crates/app/src/lib.rs` — hello-world component**

```rust
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"Hello from Leptos"</h1>
        </main>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}
```

> If `console_error_panic_hook` / `wasm_bindgen` aren't pulled in transitively, add them as deps when this fails to compile.

- [ ] **Step 6: Write `crates/worker/Cargo.toml`**

```toml
[package]
name = "worker"
edition.workspace = true
version.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
app = { path = "../app", features = ["ssr"] }
worker = "0.3"
leptos = { version = "0.6", features = ["ssr"] }
console_error_panic_hook = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
```

- [ ] **Step 7: Write `crates/worker/src/lib.rs` — minimal SSR fetch handler**

```rust
use leptos::*;
use worker::*;

#[event(fetch)]
async fn fetch(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let html = leptos::ssr::render_to_string(app::App).to_string();
    let body = format!("<!DOCTYPE html><html><head><title>RealmDex</title></head><body>{html}</body></html>");
    Response::from_html(body)
}
```

- [ ] **Step 8: Write minimal `wrangler.toml`**

```toml
name = "realmdex"
main = "build/worker/shim.mjs"
compatibility_date = "2025-01-15"
compatibility_flags = ["nodejs_compat"]

[build]
command = "cargo install -q worker-build && worker-build --release"
```

- [ ] **Step 9: Run `wrangler dev --local` and verify "Hello from Leptos" renders**

Run: `npx wrangler dev --local`
Open: http://localhost:8787
Expected: page contains the string `Hello from Leptos`.

If this fails: read the actual error, adjust crate versions / `wrangler.toml` / build command as needed. **Do not proceed to Task 1 until this task is green.** Document any deviations in a comment at the top of `wrangler.toml`.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "scaffold: leptos hello-world on cloudflare workers"
```

---

## Phase 1 — D1 schema, types, and query helpers

### Task 1: Create D1 database + initial migration

**Files:**
- Create: `migrations/0001_init.sql`
- Modify: `wrangler.toml`

- [ ] **Step 1: Create local + production D1 databases**

```bash
npx wrangler d1 create realmdex
npx wrangler d1 create realmdex-prod
```

Capture both database IDs from the output for the next step.

- [ ] **Step 2: Wire D1 bindings into `wrangler.toml`**

Append to `wrangler.toml` (substitute the IDs from step 1):

```toml
[[d1_databases]]
binding = "DB"
database_name = "realmdex"
database_id = "<dev database id from step 1>"
migrations_dir = "migrations"

[env.production]
name = "realmdex"

[[env.production.d1_databases]]
binding = "DB"
database_name = "realmdex-prod"
database_id = "<prod database id from step 1>"
migrations_dir = "migrations"
```

- [ ] **Step 3: Write `migrations/0001_init.sql`**

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE servers (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    icon_path     TEXT,
    discord_link  TEXT,
    host          TEXT NOT NULL,
    category      TEXT NOT NULL DEFAULT 'pserver',
    is_wip        INTEGER NOT NULL DEFAULT 0,
    polled        INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE server_polls (
    id         INTEGER PRIMARY KEY,
    server_id  INTEGER NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    online     INTEGER NOT NULL CHECK (online IN (0,1)),
    players    INTEGER NOT NULL DEFAULT 0,
    time       TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_server_polls_server_time ON server_polls(server_id, time);

CREATE TABLE server_polls_daily (
    server_id     INTEGER NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    day           TEXT NOT NULL,
    total_checks  INTEGER NOT NULL,
    up_checks     INTEGER NOT NULL,
    peak_players  INTEGER NOT NULL,
    PRIMARY KEY (server_id, day)
);
```

- [ ] **Step 4: Apply migration locally**

Run: `npx wrangler d1 migrations apply realmdex --local`
Expected: "🚣 Executed 1 command in X.Xms"

- [ ] **Step 5: Verify tables**

Run: `npx wrangler d1 execute realmdex --local --command "SELECT name FROM sqlite_master WHERE type='table';"`
Expected output includes `servers`, `server_polls`, `server_polls_daily`.

- [ ] **Step 6: Commit**

```bash
git add migrations/0001_init.sql wrangler.toml
git commit -m "db: initial d1 schema with rollup table and polled flag"
```

---

### Task 2: Dev seed data

**Files:**
- Create: `migrations/0002_seed_dev.sql`

> Dev-only seed. The production deploy step will skip this by convention — we run migrations with `--env production` and this file's name signals dev-only. (Wrangler doesn't have a built-in concept of "dev-only migrations"; the implementing agent should add the file to a `.wranglerignore`-style check during deploy, or apply migrations selectively. See Task 22 for the production migration strategy.)

- [ ] **Step 1: Write seed file (port enough from `system/seed.sql` to render the current page)**

```sql
INSERT INTO servers (id, name, icon_path, discord_link, host, category, is_wip, polled) VALUES
  (1, 'Valor',     '/content/images/valor.webp',     'https://discord.gg/valormc',     'http://localhost:9001/v', 'pserver', 0, 1),
  (2, 'Pserver A', '/content/images/placeholder.webp','https://discord.gg/a',          'http://localhost:9001/a', 'pserver', 0, 1),
  (3, 'WIP One',   '/content/images/placeholder.webp','https://discord.gg/w',          '',                        'pserver', 1, 0),
  (4, 'Realmlike', '/content/images/placeholder.webp','https://example.com',           'http://localhost:9001/r', 'realm-like', 0, 1);

INSERT INTO server_polls (server_id, online, players, time)
SELECT 1, 1, 42, datetime('now', '-' || (n * 60) || ' seconds') FROM (
  WITH RECURSIVE c(n) AS (SELECT 0 UNION ALL SELECT n+1 FROM c WHERE n < 60) SELECT n FROM c
);

INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
SELECT 1, date('now', '-' || n || ' days'), 1440, CASE WHEN n IN (3,7) THEN 1000 ELSE 1440 END, 50
FROM (WITH RECURSIVE c(n) AS (SELECT 0 UNION ALL SELECT n+1 FROM c WHERE n < 14) SELECT n FROM c);
```

- [ ] **Step 2: Apply locally**

Run: `npx wrangler d1 migrations apply realmdex --local`
Expected: applies `0002_seed_dev.sql`.

- [ ] **Step 3: Verify**

Run: `npx wrangler d1 execute realmdex --local --command "SELECT COUNT(*) FROM servers;"`
Expected: 4.

- [ ] **Step 4: Commit**

```bash
git add migrations/0002_seed_dev.sql
git commit -m "db: dev seed data"
```

---

### Task 3: Shared types

**Files:**
- Create: `crates/app/src/types.rs`
- Modify: `crates/app/src/lib.rs` (add `pub mod types;`)

- [ ] **Step 1: Write `crates/app/src/types.rs`**

```rust
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
```

- [ ] **Step 2: Expose module from `lib.rs`**

Add at the top of `crates/app/src/lib.rs`:

```rust
pub mod types;
```

- [ ] **Step 3: Build**

Run: `cargo build -p app`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/types.rs crates/app/src/lib.rs
git commit -m "app: shared types for cards, uptime, sparkline"
```

---

### Task 4: D1 query helpers (worker crate)

**Files:**
- Create: `crates/worker/src/db.rs`
- Modify: `crates/worker/src/lib.rs` (add `mod db;`)

> KISS: thin, typed wrappers around D1's query API. No ORM. No traits. Plain `async fn`s that take the binding by `&D1Database` and return concrete types from `app::types`.

- [ ] **Step 1: Write `crates/worker/src/db.rs` — `list_servers_in_category`**

```rust
use app::types::{Category, DailyUptime, ServerCardData, SparkPoint, Status};
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
        .bind(&[cat.into()])?
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
        .bind(&[cat.into()])?
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
        .bind(&[cat.into()])?
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
        .bind(&[cat.into()])?
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
        .bind(&[cat.into()])?
        .all()
        .await?
        .results()?;

    let mut out = Vec::with_capacity(servers.len());
    for s in servers {
        let last = last_polls.iter().find(|r| r.server_id == s.id);
        let peak = peaks.iter().find(|r| r.server_id == s.id).and_then(|r| r.peak).unwrap_or(0) as i32;
        let online = last.map(|r| r.online == 1).unwrap_or(false);
        let status = if s.is_wip == 1 { Status::Wip } else if online { Status::Online } else { Status::Offline };

        let uptime_14d = daily
            .iter()
            .filter(|r| r.server_id == s.id)
            .map(|r| DailyUptime { day: r.day.clone(), uptime_percent: r.uptime_percent as f32 })
            .collect();

        let sparkline = sparks
            .iter()
            .filter(|r| r.server_id == s.id)
            .map(|r| SparkPoint { t_unix: r.t_unix, players: r.players as i32 })
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
```

- [ ] **Step 2: Hook module into `crates/worker/src/lib.rs`**

Add near the top:

```rust
mod db;
```

- [ ] **Step 3: Build**

Run: `cargo build -p worker --target wasm32-unknown-unknown`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add crates/worker/src/db.rs crates/worker/src/lib.rs
git commit -m "worker: db helpers for list_servers_in_category"
```

---

### Task 5: Unit test the uptime color util (pure Rust, no Workers)

**Files:**
- Create: `crates/app/src/uptime.rs`
- Modify: `crates/app/src/lib.rs` (add `pub mod uptime;`)

- [ ] **Step 1: Write failing tests in `crates/app/src/uptime.rs`**

```rust
pub fn uptime_color(percent: f32) -> String {
    // unimplemented; tests should fail
    let _ = percent;
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_pure_red() {
        assert_eq!(uptime_color(0.0), "rgb(255, 0, 0)");
    }

    #[test]
    fn fifty_is_pure_yellow() {
        assert_eq!(uptime_color(50.0), "rgb(255, 255, 0)");
    }

    #[test]
    fn seventy_five_is_pure_yellow_green_transition() {
        // at 75 the second branch ends; ratio = 1; b = 255 -> rgb(255, 255, 255) per the PHP logic.
        // but the third branch starts at >=75 with greenIntensity = 0 -> rgb(255, 255, 0).
        // Our port chooses the >=75 branch, matching PHP.
        assert_eq!(uptime_color(75.0), "rgb(255, 255, 0)");
    }

    #[test]
    fn hundred_is_pure_green() {
        assert_eq!(uptime_color(100.0), "rgb(0, 255, 0)");
    }

    #[test]
    fn twenty_five_red_to_yellow() {
        assert_eq!(uptime_color(25.0), "rgb(255, 127, 0)");
    }
}
```

- [ ] **Step 2: Run tests, confirm they fail**

Run: `cargo test -p app uptime::tests`
Expected: 5 failures.

- [ ] **Step 3: Implement to match the PHP port**

Replace the placeholder body:

```rust
pub fn uptime_color(percent: f32) -> String {
    if percent >= 75.0 {
        let green_intensity = (((percent - 75.0) / 25.0) * 255.0).floor() as i32;
        format!("rgb({}, 255, 0)", 255 - green_intensity)
    } else if percent >= 50.0 {
        let ratio = (percent - 50.0) / 25.0;
        format!("rgb(255, 255, {})", (ratio * 255.0).floor() as i32)
    } else if percent > 0.0 {
        let ratio = percent / 50.0;
        format!("rgb(255, {}, 0)", (ratio * 255.0).floor() as i32)
    } else {
        "rgb(255, 0, 0)".to_string()
    }
}
```

- [ ] **Step 4: Run tests, confirm pass**

Run: `cargo test -p app uptime::tests`
Expected: 5 passing.

- [ ] **Step 5: Expose module**

Add to `crates/app/src/lib.rs`:

```rust
pub mod uptime;
```

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/uptime.rs crates/app/src/lib.rs
git commit -m "app: uptime color util ported from php"
```

---

## Phase 2 — Server functions

### Task 6: `list_servers` server function

**Files:**
- Create: `crates/app/src/server_fns.rs`
- Modify: `crates/app/src/lib.rs` (add `pub mod server_fns;`)

- [ ] **Step 1: Write `crates/app/src/server_fns.rs`**

```rust
use leptos::*;
use crate::types::{Category, ServerCardData};

#[server(ListServers, "/api")]
pub async fn list_servers(category: Category) -> Result<Vec<ServerCardData>, ServerFnError> {
    use leptos::use_context;
    let env = use_context::<worker::Env>()
        .ok_or_else(|| ServerFnError::ServerError("no env in ssr context".into()))?;
    let db = env.d1("DB").map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    let servers = worker_db::list_servers_in_category(&db, category)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    Ok(servers)
}

#[cfg(feature = "ssr")]
use worker as worker_db_;
#[cfg(feature = "ssr")]
mod worker_db {
    pub use worker::*;
    pub use worker::D1Database;
    // re-export the worker crate's db helpers; the `worker` crate exposes db.rs under its own name.
    pub use worker_lib::db::list_servers_in_category;
}
```

> **Note for the implementing engineer:** The worker-crate's `db.rs` module isn't directly importable from `app` because `app` is the dep and `worker` depends on `app`. The cleanest fix is to **move `db.rs` into the `app` crate behind `#[cfg(feature = "ssr")]`**, since it's pure SSR-only code that talks to D1 (which exists only in the Workers runtime). Restructure as follows in this step:
>
> - Move `crates/worker/src/db.rs` → `crates/app/src/db.rs`, gated behind `#[cfg(feature = "ssr")]`.
> - In `crates/app/src/lib.rs`, add `#[cfg(feature = "ssr")] pub mod db;`.
> - In `crates/app/Cargo.toml`, add `worker = { version = "0.3", optional = true }` and update the `ssr` feature: `ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr", "dep:worker"]`.
> - In `crates/worker/Cargo.toml`, drop the now-unused direct `worker` re-export; keep the dep but `app` already brings it in via `features = ["ssr"]`.
> - The server fn body then becomes:
>
> ```rust
> #[server(ListServers, "/api")]
> pub async fn list_servers(category: Category) -> Result<Vec<ServerCardData>, ServerFnError> {
>     #[cfg(feature = "ssr")]
>     {
>         let env = leptos::use_context::<worker::Env>()
>             .ok_or_else(|| ServerFnError::ServerError("no env".into()))?;
>         let db = env.d1("DB").map_err(|e| ServerFnError::ServerError(e.to_string()))?;
>         return crate::db::list_servers_in_category(&db, category)
>             .await
>             .map_err(|e| ServerFnError::ServerError(e.to_string()));
>     }
>     #[cfg(not(feature = "ssr"))]
>     { let _ = category; unreachable!("server fn body runs ssr-only") }
> }
> ```

- [ ] **Step 2: Apply the restructure above**

Move the file:

```bash
git mv crates/worker/src/db.rs crates/app/src/db.rs
```

Update `crates/app/src/lib.rs`:

```rust
pub mod types;
pub mod uptime;
pub mod server_fns;

#[cfg(feature = "ssr")]
pub mod db;
```

Update `crates/app/Cargo.toml` `[dependencies]`:

```toml
worker = { version = "0.3", optional = true }
```

And the `ssr` feature line:

```toml
ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr", "dep:worker"]
```

Update `crates/worker/src/lib.rs` to drop `mod db;`.

- [ ] **Step 3: Make the worker fetch handler put `env` into Leptos context**

Modify `crates/worker/src/lib.rs`:

```rust
use leptos::*;
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let env_clone = env.clone();
    let html = leptos::ssr::render_to_string_with_context(
        move || provide_context(env_clone.clone()),
        app::App,
    ).to_string();
    let body = format!(
        "<!DOCTYPE html><html lang=\"en\"><head>\
            <meta charset=\"utf-8\">\
            <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
            <title>RealmDex</title>\
        </head><body>{html}</body></html>"
    );
    let _ = req;
    Response::from_html(body)
}
```

> If `render_to_string_with_context` doesn't exist in the pinned Leptos version, the equivalent is `leptos::ssr::render_to_string` wrapped in a `provide_context` call inside the component tree; adapt as needed.

- [ ] **Step 4: Build**

Run: `cargo build -p worker --target wasm32-unknown-unknown`
Expected: clean.

- [ ] **Step 5: Smoke-test via `wrangler dev`**

Run: `npx wrangler dev --local`
Open: `http://localhost:8787/api/list_servers` with a POST sending `{"category":"pserver"}` as JSON.
Expected: 200 with a JSON array of `ServerCardData` (initially containing the dev seed rows).

```bash
curl -X POST http://localhost:8787/api/list_servers \
  -H "Content-Type: application/json" \
  -d '{"category":"pserver"}'
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "app+worker: list_servers server fn"
```

---

### Task 7: `server_sparkline` server function

> Even though `list_servers` already returns a sparkline, the standalone `server_sparkline` exists for future high-resolution use. KISS — implement it as a thin wrapper around a focused D1 query.

**Files:**
- Modify: `crates/app/src/server_fns.rs`
- Modify: `crates/app/src/db.rs`

- [ ] **Step 1: Add `sparkline_for_server` helper in `crates/app/src/db.rs`**

```rust
pub async fn sparkline_for_server(
    db: &worker::D1Database,
    server_id: i64,
) -> worker::Result<Vec<SparkPoint>> {
    #[derive(serde::Deserialize)]
    struct Row { t_unix: i64, players: i64 }

    let rows: Vec<Row> = db
        .prepare(
            "SELECT CAST(strftime('%s', time) AS INTEGER) AS t_unix, players
             FROM server_polls
             WHERE server_id = ?1 AND time >= datetime('now', '-1 hour')
             ORDER BY time",
        )
        .bind(&[server_id.into()])?
        .all()
        .await?
        .results()?;
    Ok(rows.into_iter().map(|r| SparkPoint { t_unix: r.t_unix, players: r.players as i32 }).collect())
}
```

(Make sure `SparkPoint` is in scope at the top: `use crate::types::SparkPoint;`.)

- [ ] **Step 2: Add the server fn to `crates/app/src/server_fns.rs`**

```rust
use crate::types::SparkPoint;

#[server(ServerSparkline, "/api")]
pub async fn server_sparkline(server_id: i64) -> Result<Vec<SparkPoint>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let env = leptos::use_context::<worker::Env>()
            .ok_or_else(|| ServerFnError::ServerError("no env".into()))?;
        let db = env.d1("DB").map_err(|e| ServerFnError::ServerError(e.to_string()))?;
        return crate::db::sparkline_for_server(&db, server_id)
            .await
            .map_err(|e| ServerFnError::ServerError(e.to_string()));
    }
    #[cfg(not(feature = "ssr"))]
    { let _ = server_id; unreachable!("server fn body runs ssr-only") }
}
```

- [ ] **Step 3: Build + smoke**

Run: `cargo build -p worker --target wasm32-unknown-unknown` then `npx wrangler dev --local`
Then: `curl -X POST http://localhost:8787/api/server_sparkline -H "Content-Type: application/json" -d '{"server_id":1}'`
Expected: 200 + JSON array of `{t_unix, players}` points.

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/db.rs crates/app/src/server_fns.rs
git commit -m "app: server_sparkline server fn"
```

---

## Phase 3 — Components

### Task 8: Sparkline SVG component (pure, unit-testable)

**Files:**
- Create: `crates/app/src/components/mod.rs`
- Create: `crates/app/src/components/sparkline.rs`
- Modify: `crates/app/src/lib.rs` (add `pub mod components;`)

- [ ] **Step 1: Write failing test for the path-string helper**

Create `crates/app/src/components/sparkline.rs`:

```rust
use leptos::*;
use crate::types::SparkPoint;

pub fn build_path(points: &[SparkPoint], width: f32, height: f32) -> String {
    let _ = (points, width, height);
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_points_yields_empty_path() {
        assert_eq!(build_path(&[], 100.0, 20.0), "");
    }

    #[test]
    fn single_point_renders_horizontal_segment() {
        let p = vec![SparkPoint { t_unix: 0, players: 10 }];
        let s = build_path(&p, 100.0, 20.0);
        assert!(s.starts_with("M 0"));
        assert!(s.contains(" L 100"));
    }

    #[test]
    fn flat_line_uses_midpoint() {
        let p = vec![
            SparkPoint { t_unix: 0, players: 10 },
            SparkPoint { t_unix: 60, players: 10 },
        ];
        let s = build_path(&p, 100.0, 20.0);
        assert!(s.contains("10"));
    }
}
```

- [ ] **Step 2: Run, confirm fails**

Run: `cargo test -p app components::sparkline::tests`
Expected: fails.

- [ ] **Step 3: Implement `build_path`**

```rust
pub fn build_path(points: &[SparkPoint], width: f32, height: f32) -> String {
    if points.is_empty() {
        return String::new();
    }
    if points.len() == 1 {
        let y = height / 2.0;
        return format!("M 0 {y:.1} L {width:.1} {y:.1}");
    }
    let min = points.iter().map(|p| p.players).min().unwrap_or(0) as f32;
    let max = points.iter().map(|p| p.players).max().unwrap_or(0) as f32;
    let range = (max - min).max(1.0);
    let n = points.len() as f32;

    let mut out = String::new();
    for (i, p) in points.iter().enumerate() {
        let x = (i as f32 / (n - 1.0)) * width;
        let y = height - ((p.players as f32 - min) / range) * height;
        if i == 0 {
            out.push_str(&format!("M {x:.1} {y:.1}"));
        } else {
            out.push_str(&format!(" L {x:.1} {y:.1}"));
        }
    }
    out
}
```

- [ ] **Step 4: Run, confirm pass**

Run: `cargo test -p app components::sparkline::tests`
Expected: 3 passing.

- [ ] **Step 5: Add the Leptos component**

Append to `crates/app/src/components/sparkline.rs`:

```rust
#[component]
pub fn Sparkline(#[prop()] points: Signal<Vec<SparkPoint>>) -> impl IntoView {
    let width = 120.0_f32;
    let height = 24.0_f32;
    let path = move || build_path(&points.get(), width, height);
    view! {
        <svg class="sparkline" width=width height=height viewBox=format!("0 0 {width} {height}")>
            <path d=path fill="none" stroke="currentColor" stroke-width="1.5"/>
        </svg>
    }
}
```

- [ ] **Step 6: Write the components module file**

Create `crates/app/src/components/mod.rs`:

```rust
pub mod sparkline;
```

- [ ] **Step 7: Expose from `lib.rs`**

Add to `crates/app/src/lib.rs`:

```rust
pub mod components;
```

- [ ] **Step 8: Commit**

```bash
git add crates/app/src/components crates/app/src/lib.rs
git commit -m "app: sparkline svg component + path tests"
```

---

### Task 9: SiteHeader component

**Files:**
- Create: `crates/app/src/components/site_header.rs`
- Modify: `crates/app/src/components/mod.rs`

- [ ] **Step 1: Write component**

```rust
use leptos::*;

#[component]
pub fn SiteHeader() -> impl IntoView {
    view! {
        <header class="site-header" role="banner">
            <div id="h-realmdex-icon">
                <img src="/content/images/logo.webp" alt="RealmDex logo"/>
                <p>"RealmDex"</p>
            </div>
        </header>
    }
}
```

- [ ] **Step 2: Expose from `components/mod.rs`**

```rust
pub mod sparkline;
pub mod site_header;
```

- [ ] **Step 3: Build**

Run: `cargo build -p app`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/components
git commit -m "app: SiteHeader component"
```

---

### Task 10: ServerCard component (card + uptime grid)

**Files:**
- Create: `crates/app/src/components/server_card.rs`
- Modify: `crates/app/src/components/mod.rs`

- [ ] **Step 1: Write component**

```rust
use leptos::*;
use crate::types::{ServerCardData, Status};
use crate::uptime::uptime_color;
use super::sparkline::Sparkline;

#[component]
pub fn ServerCard(#[prop()] data: ServerCardData) -> impl IntoView {
    let status_class = match data.status {
        Status::Online => "online",
        Status::Offline => "offline",
        Status::Wip => "wip",
    };
    let status_text = match data.status {
        Status::Online => "Online",
        Status::Offline => "Offline",
        Status::Wip => "WIP",
    };
    let is_wip = matches!(data.status, Status::Wip);
    let link_text = if data.link.contains("discord") { "Join Discord" } else { "Visit Homepage" };

    let week = data.uptime_14d.iter().take(7).cloned().collect::<Vec<_>>();
    let two_week = data.uptime_14d.clone();

    let spark_points = create_rw_signal(data.sparkline.clone());

    view! {
        <div class="server-card" data-server-id=data.id>
            <div class="card-header">
                <img src=data.icon_path.clone() alt=data.name.clone() class="server-icon" data-discord=data.link.clone()/>
                <div class="server-info">
                    <h3 class="server-name">{data.name.clone()}</h3>
                    <a href=data.link.clone() class="server-discord" target="_blank" rel="noopener noreferrer">{link_text}</a>
                </div>
                <div class="status-container">
                    <div class=format!("status-indicator {status_class}") title=status_text></div>
                    <span class=format!("status-text {status_class}")>{status_text}</span>
                </div>
            </div>

            <div class="card-stats">
                <div class="stat-row">
                    <span class="stat-label">"Players"</span>
                    <span class="stat-value">{if is_wip { "-".to_string() } else { data.current_players.to_string() }}</span>
                </div>
                <div class="stat-row">
                    <span class="stat-label">"24h Peak"</span>
                    <span class="stat-value">{if is_wip { "-".to_string() } else { data.peak_24h.to_string() }}</span>
                </div>
            </div>

            { (!is_wip).then(|| view! {
                <div class="sparkline-wrapper">
                    <Sparkline points=spark_points.into()/>
                </div>
                <div class="uptime-section">
                    <div class="uptime-wrapper">
                        <div class="uptime-labels">
                            <div class="uptime-label uptime-label-week">"Uptime (Past Week)"</div>
                            <div class="uptime-label uptime-label-2week">"Uptime (Past 2 Weeks)"</div>
                        </div>
                        <div class="uptime-grids">
                            <div class="uptime-week">{uptime_grid(&week)}</div>
                            <div class="uptime-2week">{uptime_grid(&two_week)}</div>
                        </div>
                    </div>
                </div>
            }) }
        </div>
    }
}

fn uptime_grid(days: &[crate::types::DailyUptime]) -> impl IntoView {
    let cells: Vec<_> = days.iter().enumerate().map(|(i, d)| {
        let color = uptime_color(d.uptime_percent);
        view! {
            <div class="uptime-day"
                 style=format!("background-color: {color}")
                 data-uptime=d.uptime_percent
                 data-day=i + 1></div>
        }
    }).collect();
    view! { <div class="uptime-grid">{cells}</div> }
}
```

- [ ] **Step 2: Expose from `components/mod.rs`**

```rust
pub mod sparkline;
pub mod site_header;
pub mod server_card;
```

- [ ] **Step 3: Build**

Run: `cargo build -p app`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/app/src/components
git commit -m "app: ServerCard component with uptime grid + sparkline"
```

---

### Task 11: ServerGrid component (tabs, sort, live resource)

**Files:**
- Create: `crates/app/src/components/server_grid.rs`
- Modify: `crates/app/src/components/mod.rs`

- [ ] **Step 1: Write component**

```rust
use leptos::*;
use crate::types::{Category, ServerCardData, Status};
use crate::server_fns::list_servers;
use super::server_card::ServerCard;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    PlayersDesc,
    PlayersAsc,
    UptimeDesc,
    Random,
}

fn avg_uptime(c: &ServerCardData) -> f32 {
    if c.uptime_14d.is_empty() { 0.0 } else {
        let total: f32 = c.uptime_14d.iter().map(|d| d.uptime_percent).sum();
        total / c.uptime_14d.len() as f32
    }
}

fn partition_and_sort(mut data: Vec<ServerCardData>, sort: Sort) -> (Vec<ServerCardData>, Vec<ServerCardData>, Vec<ServerCardData>) {
    let mut online = Vec::new();
    let mut offline = Vec::new();
    let mut wip = Vec::new();
    for s in data.drain(..) {
        match s.status {
            Status::Online => online.push(s),
            Status::Offline => offline.push(s),
            Status::Wip => wip.push(s),
        }
    }
    match sort {
        Sort::PlayersDesc => online.sort_by_key(|s| -s.current_players),
        Sort::PlayersAsc => online.sort_by_key(|s| s.current_players),
        Sort::UptimeDesc => online.sort_by(|a, b| avg_uptime(b).partial_cmp(&avg_uptime(a)).unwrap_or(std::cmp::Ordering::Equal)),
        Sort::Random => {
            // deterministic-enough shuffle without a heavy rng dep
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            online.sort_by_key(|s| {
                let mut h = DefaultHasher::new();
                (s.id, leptos::window().performance().map(|p| p.now() as u64).unwrap_or(0)).hash(&mut h);
                h.finish()
            });
        }
    }
    (online, offline, wip)
}

#[component]
pub fn ServerGrid() -> impl IntoView {
    let category = create_rw_signal(Category::Pserver);
    let sort = create_rw_signal(Sort::PlayersDesc);

    // 30s refresh interval; first load is SSR-driven via Resource.
    let servers = create_resource(
        move || category.get(),
        |cat| async move { list_servers(cat).await.unwrap_or_default() },
    );

    // Re-fetch every 30s on the client.
    #[cfg(feature = "hydrate")]
    {
        use leptos::leptos_dom::helpers::set_interval;
        use std::time::Duration;
        let s = servers.clone();
        set_interval(move || { s.refetch(); }, Duration::from_secs(30));
    }

    view! {
        <main class="server-grid-container" role="main">
            <div class="category-tabs">
                <button class=move || if category.get() == Category::Pserver { "category-tab active" } else { "category-tab" }
                        on:click=move |_| category.set(Category::Pserver)>"Private Servers"</button>
                <button class=move || if category.get() == Category::RealmLike { "category-tab active" } else { "category-tab" }
                        on:click=move |_| category.set(Category::RealmLike)>"Realm-Likes"</button>
            </div>
            <div class="controls-bar">
                <label for="sort-select" class="sort-label">"Sort by:"</label>
                <select id="sort-select" class="sort-select"
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            sort.set(match v.as_str() {
                                "players-asc" => Sort::PlayersAsc,
                                "uptime-desc" => Sort::UptimeDesc,
                                "random" => Sort::Random,
                                _ => Sort::PlayersDesc,
                            });
                        }>
                    <option value="players-desc">"Players (High to Low)"</option>
                    <option value="players-asc">"Players (Low to High)"</option>
                    <option value="uptime-desc">"Uptime (High to Low)"</option>
                    <option value="random">"Random"</option>
                </select>
            </div>

            <Suspense fallback=move || view! { <div class="loading">"Loading servers…"</div> }>
                {move || servers.get().map(|data| {
                    let (online, offline, wip) = partition_and_sort(data, sort.get());
                    view! {
                        <div class="server-grid" data-category=category.get().as_db_str()>
                            {online.into_iter().map(|s| view! { <ServerCard data=s/> }).collect_view()}
                            { (!offline.is_empty()).then(|| view! {
                                <div class="wip-divider offline-divider"><span>"Offline"</span></div>
                                {offline.into_iter().map(|s| view! { <ServerCard data=s/> }).collect_view()}
                            }) }
                            { (!wip.is_empty()).then(|| view! {
                                <div class="wip-divider"><span>"Work in Progress"</span></div>
                                {wip.into_iter().map(|s| view! { <ServerCard data=s/> }).collect_view()}
                            }) }
                        </div>
                    }
                })}
            </Suspense>
        </main>
    }
}
```

- [ ] **Step 2: Expose from `components/mod.rs`**

```rust
pub mod sparkline;
pub mod site_header;
pub mod server_card;
pub mod server_grid;
```

- [ ] **Step 3: Update `<App/>` in `crates/app/src/lib.rs`**

Replace the hello-world `App`:

```rust
use crate::components::{site_header::SiteHeader, server_grid::ServerGrid};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <SiteHeader/>
        <ServerGrid/>
    }
}
```

- [ ] **Step 4: Smoke-test in browser**

Run: `npx wrangler dev --local`
Open: `http://localhost:8787`
Expected: header + category tabs + sort dropdown + server cards from the dev seed render. Switching tabs swaps category. Sort changes order.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "app: ServerGrid with category tabs, sort, live resource"
```

---

## Phase 4 — Styles + assets

### Task 12: Port SCSS and wire `cargo-leptos`

**Files:**
- Move: `styles/index.scss` → `crates/app/style/index.scss`
- Move: `content/` → `public/content/`
- Move: `favicon.ico` → `public/favicon.ico`
- Modify: `wrangler.toml`
- Modify: `crates/app/Cargo.toml` (add `[package.metadata.leptos]` section)

- [ ] **Step 1: Move files**

```bash
mkdir -p crates/app/style public
git mv styles/index.scss crates/app/style/index.scss
git mv content public/content
git mv favicon.ico public/favicon.ico
```

- [ ] **Step 2: Configure `cargo-leptos` in `crates/app/Cargo.toml`**

Append:

```toml
[package.metadata.leptos]
output-name = "realmdex"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "style/index.scss"
assets-dir = "public"
site-addr = "127.0.0.1:8787"
bin-features = []
lib-features = ["hydrate"]
```

- [ ] **Step 3: Wire Workers Assets in `wrangler.toml`**

Append:

```toml
[assets]
directory = "target/site"
binding = "ASSETS"
```

- [ ] **Step 4: Update the worker's HTML shell to link the right paths**

In `crates/worker/src/lib.rs`, the body should be:

```rust
let body = format!(
    "<!DOCTYPE html><html lang=\"en\"><head>\
        <meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
        <title>RealmDex - RotMG Private Server Stats & Uptime</title>\
        <meta name=\"description\" content=\"Track server status, player counts, and uptime for RotMG private servers and Realm-Like games.\">\
        <link rel=\"icon\" type=\"image/x-icon\" href=\"/favicon.ico\">\
        <link rel=\"stylesheet\" href=\"/pkg/realmdex.css\">\
    </head><body>{html}\
    <script type=\"module\">import init from '/pkg/realmdex.js'; init('/pkg/realmdex_bg.wasm').then(m => m.hydrate());</script>\
    </body></html>"
);
```

- [ ] **Step 5: Build with `cargo-leptos`**

Install if missing: `cargo install cargo-leptos`
Run: `cargo leptos build --release`
Expected: `target/site/pkg/realmdex.{js,wasm,css}` exists, `target/site/favicon.ico` exists, `target/site/content/images/*` exists.

- [ ] **Step 6: Smoke**

Run: `npx wrangler dev --local`
Open: `http://localhost:8787`
Expected: styled page (RotMG wallpaper background, card layout). Open DevTools and confirm `/pkg/realmdex.js` and `/pkg/realmdex.css` load with 200s.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "build: wire cargo-leptos for css + wasm + assets"
```

---

### Task 13: Remove the legacy PHP/Docker/SCSS files

> Only after Task 12's smoke test shows the new page rendering with the original look.

**Files:**
- Delete: `index.php`, `Dockerfile`, `docker-compose.yml`, `docker-compose.prod.yml`, `entrypoint.sh`, `update.sh`, `system/`, `scripts/index.js`, `styles/`, `package.json` and `package-lock.json` if present, `data/uptime.db` (keep a copy in `legacy/` for the migration script).

- [ ] **Step 1: Stash the legacy DB**

```bash
mkdir -p legacy
git mv data/uptime.db legacy/uptime.db
```

- [ ] **Step 2: Remove PHP/Docker/legacy build files**

```bash
git rm index.php Dockerfile docker-compose.yml docker-compose.prod.yml entrypoint.sh update.sh
git rm -r system scripts styles
```

- [ ] **Step 3: Build + smoke**

Run: `cargo leptos build --release && npx wrangler dev --local`
Open: `http://localhost:8787`
Expected: identical to Task 12 step 6.

- [ ] **Step 4: Commit**

```bash
git commit -m "remove: php, docker, legacy build artifacts"
```

---

## Phase 5 — Poller + rollup

### Task 14: Cron-driven poller

**Files:**
- Create: `crates/worker/src/poller.rs`
- Modify: `crates/worker/src/lib.rs` (`mod poller;` + `scheduled` handler dispatch)
- Modify: `wrangler.toml` (cron triggers)

- [ ] **Step 1: Add cron triggers to `wrangler.toml`**

```toml
[triggers]
crons = [
  "*/1 * * * *",   # poller every minute
  "30 3 * * *",    # rollup daily at 03:30 UTC
]
```

- [ ] **Step 2: Write `crates/worker/src/poller.rs`**

```rust
use futures::future::join_all;
use worker::*;

const TIMEOUT_MS: u32 = 10_000;

#[derive(serde::Deserialize)]
struct Row { id: i64, host: String }

pub async fn run(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;
    let servers: Vec<Row> = db
        .prepare("SELECT id, host FROM servers WHERE polled = 1")
        .all().await?.results()?;

    let futures = servers.into_iter().map(|s| poll_one(s.id, s.host));
    let outcomes: Vec<(i64, i64, i64)> = join_all(futures).await;

    if outcomes.is_empty() { return Ok(()); }

    let mut sql = String::from("INSERT INTO server_polls (server_id, online, players) VALUES ");
    let mut binds: Vec<JsValue> = Vec::with_capacity(outcomes.len() * 3);
    for (i, (id, online, players)) in outcomes.iter().enumerate() {
        if i > 0 { sql.push(','); }
        let base = i * 3;
        sql.push_str(&format!("(?{}, ?{}, ?{})", base + 1, base + 2, base + 3));
        binds.push((*id).into());
        binds.push((*online).into());
        binds.push((*players).into());
    }
    db.prepare(&sql).bind(&binds)?.run().await?;
    Ok(())
}

async fn poll_one(id: i64, host: String) -> (i64, i64, i64) {
    if host.is_empty() {
        return (id, 0, 0);
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Get);
    // worker-rs supports passing an AbortSignal to fetch; fall back to a manual race if not.
    let req = match Request::new_with_init(&host, &init) {
        Ok(r) => r,
        Err(_) => return (id, 0, 0),
    };
    let fetch_fut = Fetch::Request(req).send();
    let timeout_fut = async {
        Delay::from(std::time::Duration::from_millis(TIMEOUT_MS as u64)).await;
        Err::<Response, Error>(Error::RustError("timeout".into()))
    };
    let mut resp = match futures::future::select(Box::pin(fetch_fut), Box::pin(timeout_fut)).await {
        futures::future::Either::Left((Ok(r), _)) => r,
        _ => return (id, 0, 0),
    };
    if resp.status_code() != 200 { return (id, 0, 0); }
    let body = match resp.text().await { Ok(s) => s, Err(_) => return (id, 0, 0) };
    let players: i64 = body.trim().parse().unwrap_or(0);
    (id, 1, players)
}
```

> `Delay` may not be exposed by `worker-rs`; if not, use `worker::js_sys::Promise` + `setTimeout` via `wasm_bindgen_futures`, or pull in `gloo_timers`. The implementing engineer picks whichever the pinned `worker` version offers and adjusts.

- [ ] **Step 3: Dispatch `scheduled` events in `crates/worker/src/lib.rs`**

Add alongside the `fetch` handler:

```rust
mod poller;

#[event(scheduled)]
async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_error_panic_hook::set_once();
    let cron = event.cron();
    let res = if cron.starts_with("*/1") {
        poller::run(&env).await
    } else {
        // rollup added in next task
        Ok(())
    };
    if let Err(e) = res {
        console_log!("scheduled error: {e}");
    }
}
```

- [ ] **Step 4: Test the poller with a local mock target**

In a separate terminal, run a 1-liner mock:
```bash
python3 -c "import http.server, socketserver; \
class H(http.server.BaseHTTPRequestHandler): \
    def do_GET(self): self.send_response(200); self.end_headers(); self.wfile.write(b'42')\n; \
socketserver.TCPServer(('127.0.0.1', 9001), H).serve_forever()"
```

(Or any local server returning `200 OK` with body `42`.)

Trigger the cron once:
Run: `npx wrangler dev --local --test-scheduled`
In a third terminal: `curl "http://localhost:8787/__scheduled?cron=*/1+*+*+*+*"`
Expected: 200; then `npx wrangler d1 execute realmdex --local --command "SELECT COUNT(*) FROM server_polls WHERE time > datetime('now','-1 minute');"` returns ≥1.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "worker: cron-driven poller with fan-out and batch insert"
```

---

### Task 15: Daily rollup + prune

**Files:**
- Create: `crates/worker/src/rollup.rs`
- Modify: `crates/worker/src/lib.rs`

- [ ] **Step 1: Write `crates/worker/src/rollup.rs`**

```rust
use worker::*;

pub async fn run(env: &Env) -> Result<()> {
    let db = env.d1("DB")?;

    // 1) aggregate yesterday into the daily table (idempotent on conflict)
    db.prepare(
        "INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
         SELECT server_id,
                date(time) AS day,
                COUNT(*) AS total_checks,
                SUM(online) AS up_checks,
                MAX(players) AS peak_players
         FROM server_polls
         WHERE date(time) = date('now', '-1 day')
         GROUP BY server_id, date(time)
         ON CONFLICT(server_id, day) DO UPDATE SET
           total_checks = excluded.total_checks,
           up_checks    = excluded.up_checks,
           peak_players = excluded.peak_players"
    ).run().await?;

    // 2) prune raw rows older than 30 days
    db.prepare("DELETE FROM server_polls WHERE time < datetime('now', '-30 days')")
        .run().await?;

    Ok(())
}
```

- [ ] **Step 2: Wire into the `scheduled` dispatch**

In `crates/worker/src/lib.rs`, update the dispatch:

```rust
mod rollup;

// inside scheduled():
let res = if cron.starts_with("*/1") {
    poller::run(&env).await
} else if cron.starts_with("30 3") {
    rollup::run(&env).await
} else {
    Ok(())
};
```

- [ ] **Step 3: Smoke**

Manually trigger:
```bash
curl "http://localhost:8787/__scheduled?cron=30+3+*+*+*"
```
Then verify the daily table has rows for yesterday's date.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "worker: daily rollup + 30d prune"
```

---

## Phase 6 — Security middleware

### Task 16: Origin/Referer guard on server-fn requests

**Files:**
- Create: `crates/worker/src/security.rs`
- Modify: `crates/worker/src/lib.rs`

- [ ] **Step 1: Write `crates/worker/src/security.rs`**

```rust
use worker::*;

const ALLOWED_ORIGIN: &str = "https://realmdex.com";

pub fn guard_api(req: &Request) -> std::result::Result<(), Response> {
    // server-fn endpoints live under /api/
    let path = req.path();
    if !path.starts_with("/api/") {
        return Ok(());
    }
    let dev = req.headers().get("Host").ok().flatten()
        .map(|h| h.starts_with("localhost") || h.starts_with("127.0.0.1"))
        .unwrap_or(false);
    if dev {
        return Ok(());
    }
    let origin = req.headers().get("Origin").ok().flatten();
    let referer = req.headers().get("Referer").ok().flatten();
    let ok_origin = origin.as_deref().map(|o| o == ALLOWED_ORIGIN).unwrap_or(false);
    let ok_referer = referer.as_deref().map(|r| r.starts_with(&format!("{ALLOWED_ORIGIN}/"))).unwrap_or(false);
    if ok_origin || ok_referer {
        return Ok(());
    }
    let mut resp = Response::error("forbidden", 403).unwrap();
    let _ = resp.headers_mut().set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    Err(resp)
}

pub fn add_cors(mut resp: Response) -> Response {
    let _ = resp.headers_mut().set("Access-Control-Allow-Origin", ALLOWED_ORIGIN);
    let _ = resp.headers_mut().set("Vary", "Origin");
    resp
}

// turnstile hook left for later; no-op stub.
#[allow(dead_code)]
pub async fn verify_turnstile(_token: &str) -> bool { true }
```

- [ ] **Step 2: Wire into `fetch` handler**

In `crates/worker/src/lib.rs`:

```rust
mod security;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    if let Err(deny) = security::guard_api(&req) {
        return Ok(deny);
    }
    // ... existing SSR code ...
    let resp = Response::from_html(body)?;
    Ok(security::add_cors(resp))
}
```

- [ ] **Step 3: Test the guard**

Run: `npx wrangler dev --local`
Then:
```bash
# Should pass (dev host)
curl -X POST http://localhost:8787/api/list_servers -H "Content-Type: application/json" -d '{"category":"pserver"}' -i
# Should be blocked once deployed: simulate by forcing Host header
curl -X POST http://localhost:8787/api/list_servers -H "Host: realmdex.com" -H "Content-Type: application/json" -d '{"category":"pserver"}' -i
# Expected: 403 forbidden
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "worker: origin/referer guard on /api/*"
```

---

### Task 17: Cloudflare Rate Limiting Rule (setup doc)

> Cloudflare Rate Limiting Rules are configured in the dashboard, not in `wrangler.toml`. The implementing engineer needs to set this up once per environment.

**Files:**
- Create: `docs/setup/rate-limiting.md`

- [ ] **Step 1: Write the setup doc**

```markdown
# Cloudflare Rate Limiting Setup

Configured once per environment via the Cloudflare dashboard. Not in source.

## Production rule

- Zone: `realmdex.com`
- Path matches: `/api/*`
- Threshold: 60 requests per minute per IP
- Action: Block for 1 minute
- Response: 429

## Add via dashboard

1. Cloudflare dashboard → Security → WAF → Rate limiting rules
2. Create rule with the values above.
3. Confirm with `curl` from an external IP — 61st request in a minute should 429.
```

- [ ] **Step 2: Reference it from `README.md`** — add a "Deployment setup" link to this file.

- [ ] **Step 3: Commit**

```bash
git add docs/setup/rate-limiting.md README.md
git commit -m "docs: rate limiting setup steps"
```

---

## Phase 7 — Deploy pipeline

### Task 18: CI workflow (no secrets)

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write workflow (pin actions by SHA — placeholders below; the implementing engineer resolves to real SHAs at PR time)**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<SHA>   # pin to v4 SHA
      - uses: dtolnay/rust-toolchain@<SHA>   # pin to stable SHA
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@<SHA>
      - name: Format check
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Unit tests
        run: cargo test -p app --lib
      - name: Install cargo-leptos
        run: cargo install cargo-leptos --locked
      - name: Build
        run: cargo leptos build --release
      - name: Migration dry-run (local sqlite)
        run: |
          for f in migrations/000[01]_*.sql; do
            sqlite3 ":memory:" < "$f"
          done
```

- [ ] **Step 2: Resolve SHAs**

For each `<SHA>`, look up the commit SHA of the latest stable tag and substitute it in. Document the tag → SHA mapping in a comment at the top of the file.

- [ ] **Step 3: Push branch, confirm CI runs green on PR**

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: build, fmt, clippy, tests, migration dry-run"
```

---

### Task 19: Cloudflare OIDC setup (one-time, documented)

**Files:**
- Create: `docs/setup/cloudflare-oidc.md`

- [ ] **Step 1: Write the setup doc**

```markdown
# Cloudflare OIDC for GitHub Actions

One-time setup. Replaces a long-lived `CLOUDFLARE_API_TOKEN` repo secret with short-lived OIDC-issued credentials.

## Steps

1. In Cloudflare dashboard → My Profile → API Tokens → Create Token, choose "Custom Token" with:
   - Account → Workers Scripts: Edit
   - Account → D1: Edit
   - Account → Workers KV Storage: Edit (only if KV added later)
   - Zone Resources: Include → Specific Zone → `realmdex.com`
   - **Save the token**, but do NOT add it to GitHub. Instead, use it to create the OIDC trust below.
2. Use the Cloudflare API to register a GitHub OIDC trust:
   - Issuer: `https://token.actions.githubusercontent.com`
   - Audience: `cloudflare-workers-deploy`
   - Subject filter: `repo:<owner>/<repo>:environment:production`
3. In the GitHub repo Settings → Secrets and variables → Actions → Variables, add:
   - `CLOUDFLARE_ACCOUNT_ID` (the account id from the dashboard URL)
4. In Settings → Environments, create `production`:
   - Required reviewers: yourself (and any co-maintainer)
   - Wait timer: 0
   - Branches: limit to `main`

## Verify

A subsequent `deploy.yml` run on a push to `main` should pause for approval, then complete the OIDC exchange and run `wrangler deploy` without any long-lived secret.

## If OIDC isn't viable yet

Fallback: add a scoped `CLOUDFLARE_API_TOKEN` repo secret (Workers + D1 scopes only). Document the scope in this file. Rotate every 90 days.
```

- [ ] **Step 2: Commit**

```bash
git add docs/setup/cloudflare-oidc.md
git commit -m "docs: cloudflare oidc setup"
```

---

### Task 20: Deploy workflow

**Files:**
- Create: `.github/workflows/deploy.yml`

- [ ] **Step 1: Write workflow**

```yaml
name: Deploy

on:
  push:
    branches: [main]

permissions:
  contents: read
  id-token: write   # required for OIDC

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment: production
    steps:
      - uses: actions/checkout@<SHA>
      - uses: dtolnay/rust-toolchain@<SHA>
        with:
          toolchain: stable
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@<SHA>
      - name: Install cargo-leptos
        run: cargo install cargo-leptos --locked
      - name: Build
        run: cargo leptos build --release
      - name: Cloudflare OIDC exchange
        id: cf-auth
        uses: cloudflare/wrangler-action@<SHA>
        with:
          accountId: ${{ vars.CLOUDFLARE_ACCOUNT_ID }}
          # OIDC flow — no apiToken field; the action uses id-token: write
          command: "--version"
      - name: Apply production migrations
        run: npx wrangler d1 migrations apply realmdex-prod --env production --remote
      - name: Deploy worker
        run: npx wrangler deploy --env production
```

> The Cloudflare `wrangler-action` OIDC support depends on its current version. The implementing engineer verifies the exact field names against the action's README at the SHA being pinned and adjusts if the API has shifted.

- [ ] **Step 2: Resolve action SHAs as in Task 18.**

- [ ] **Step 3: Configure branch protection on `main`**

Via GitHub UI (Settings → Branches → Branch protection rules):
- Require pull request reviews before merging (1 approval).
- Require status checks to pass: `CI / build`.
- Require branches to be up to date.
- Disallow force pushes.
- Do not allow bypass for administrators.

- [ ] **Step 4: Document the setup in `README.md`**

Add a "Deployment" section with links to `docs/setup/cloudflare-oidc.md` and `docs/setup/rate-limiting.md`, plus the branch-protection bullets.

- [ ] **Step 5: First real deploy**

Merge to `main`. The deploy job should pause at the `production` environment for your approval. Approve. Confirm:
- The job completes.
- `https://realmdex.com` serves the new app.
- D1 migrations applied on prod (`npx wrangler d1 execute realmdex-prod --remote --command "SELECT name FROM sqlite_master WHERE type='table';"`).

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/deploy.yml README.md
git commit -m "ci: deploy via oidc to cloudflare with production environment gate"
```

---

## Phase 8 — Migration from legacy DB

### Task 21: Import script for the existing `uptime.db`

**Files:**
- Create: `scripts/import_legacy_db.sh`

- [ ] **Step 1: Write the script**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Imports legacy SQLite data (legacy/uptime.db) into local D1.
# After running, manually inspect the result; if happy, dump it as a one-shot
# migration to apply against production.

SRC="legacy/uptime.db"
[ -f "$SRC" ] || { echo "missing $SRC"; exit 1; }

# Dump only the tables we care about, then transform on the fly.
sqlite3 "$SRC" .schema | grep -E "^CREATE TABLE (servers|server_polls)" > /tmp/legacy_schema.sql

# Export rows
sqlite3 "$SRC" "SELECT 'INSERT INTO servers (id,name,icon_path,discord_link,host,category,is_wip,polled,created_at) VALUES (' || id || ',' || quote(name) || ',' || quote(icon_path) || ',' || quote(discord_link) || ',' || quote(host) || ',' || quote(coalesce(category,'pserver')) || ',' || coalesce(is_wip,0) || ',' || (CASE coalesce(is_wip,0) WHEN 1 THEN 0 ELSE 1 END) || ',CURRENT_TIMESTAMP);' FROM servers;" > /tmp/import_servers.sql
sqlite3 "$SRC" "SELECT 'INSERT INTO server_polls (server_id,online,players,time) VALUES (' || server_id || ',' || online || ',' || coalesce(players,0) || ',' || quote(time) || ');' FROM server_polls;" > /tmp/import_polls.sql

# Apply to local D1
npx wrangler d1 execute realmdex --local --file /tmp/import_servers.sql
npx wrangler d1 execute realmdex --local --file /tmp/import_polls.sql

echo "Done. Inspect local DB before promoting."
```

- [ ] **Step 2: Make executable + run**

```bash
chmod +x scripts/import_legacy_db.sh
./scripts/import_legacy_db.sh
```

Verify with:
```bash
npx wrangler d1 execute realmdex --local --command "SELECT COUNT(*) FROM servers; SELECT COUNT(*) FROM server_polls;"
```

- [ ] **Step 3: Smoke**

Run: `npx wrangler dev --local`
Open: `http://localhost:8787`
Expected: real production server list renders, uptime grids look plausible.

- [ ] **Step 4: Commit**

```bash
git add scripts/import_legacy_db.sh
git commit -m "scripts: import legacy uptime.db into local d1"
```

---

### Task 22: Production data migration (one-shot)

**Files:**
- Create: `migrations/0003_backfill_from_php.sql` (generated, gitignored if large)

> Run **once**, immediately before flipping DNS to the new Worker, while the PHP poller is paused. Generated locally from the live legacy DB, applied to production D1 manually.

- [ ] **Step 1: Pause the PHP cron** on the legacy host (so polls don't fork between two systems).

- [ ] **Step 2: Pull the latest legacy DB**

```bash
scp <legacy-host>:/var/www/html/data/uptime.db legacy/uptime.db.live
```

- [ ] **Step 3: Generate a migration file**

Adapt `scripts/import_legacy_db.sh` to write its output to `migrations/0003_backfill_from_php.sql` instead of applying it. Then back-fill the daily table:

```sql
INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
SELECT server_id, date(time), COUNT(*), SUM(online), MAX(players)
FROM server_polls
WHERE date(time) < date('now')
GROUP BY server_id, date(time)
ON CONFLICT(server_id, day) DO NOTHING;

DELETE FROM server_polls WHERE time < datetime('now', '-30 days');
```

- [ ] **Step 4: Apply to production D1**

```bash
npx wrangler d1 execute realmdex-prod --remote --file migrations/0003_backfill_from_php.sql
```

- [ ] **Step 5: Flip DNS**

Point `realmdex.com` at the Worker route. Confirm the live site serves real data from D1.

- [ ] **Step 6: Decommission the legacy host** after 24h of clean operation.

- [ ] **Step 7: Commit (without the .live DB file)**

```bash
echo "legacy/uptime.db.live" >> .gitignore
git add migrations/0003_backfill_from_php.sql .gitignore
git commit -m "migrate: one-shot backfill from legacy uptime.db"
```

---

## Phase 9 — Wrap-up

### Task 23: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace the "Development" section**

New content:

```markdown
## Development

### Prerequisites

- Rust (stable, install via [rustup](https://rustup.rs))
- `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- `cargo-leptos` (`cargo install cargo-leptos --locked`)
- Node 20+ (for `wrangler`)
- `wrangler` CLI (`npx wrangler` works without a global install)

### Running locally

```bash
npx wrangler d1 migrations apply realmdex --local
cargo leptos watch
npx wrangler dev --local
```

Open http://localhost:8787.

### Tests

```bash
cargo test --workspace
```

### Resetting local data

```bash
rm -rf .wrangler
npx wrangler d1 migrations apply realmdex --local
```

## Deployment

See:
- `docs/setup/cloudflare-oidc.md` for the GitHub Actions → Cloudflare auth setup.
- `docs/setup/rate-limiting.md` for edge rate limiting.

Branch protection on `main`:
- Require PR review (1 approval)
- Require status checks: `CI / build`
- No force push, no admin bypass

The `production` environment in GitHub requires reviewer approval; every deploy pauses for a human click.
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: update readme for rust/cloudflare stack"
```

---

### Task 24: End-to-end smoke test against production

- [ ] **Step 1:** Visit `https://realmdex.com` after deploy.
- [ ] **Step 2:** Confirm header, cards, tabs, sort, and uptime grids match the legacy site.
- [ ] **Step 3:** Open DevTools → Network. Confirm `/pkg/realmdex.js`, `/pkg/realmdex.css`, `/pkg/realmdex_bg.wasm` load with 200s.
- [ ] **Step 4:** Confirm `/api/list_servers` returns minimal JSON (no `host` field).
- [ ] **Step 5:** Confirm cards update without a page reload after ~30s (force a player count change via the poller mock if testing in staging).
- [ ] **Step 6:** Confirm a cross-origin `curl` to `/api/list_servers` with a wrong `Origin` returns 403.
- [ ] **Step 7:** Confirm 60+ requests/min from a single IP to `/api/*` returns 429.

If all green: the core rewrite is done.

---

## Notes for the executing engineer

- **`worker-rs` and Leptos versions are pins, not law.** If a newer version is required to make the integration work, bump both Leptos and `worker-rs` in lockstep and verify Task 0 still passes.
- **The `Delay` / timeout primitive in Task 14** is the most likely place for an API mismatch with the pinned `worker-rs`. The implementing engineer picks whatever timeout primitive exists in the version they pinned.
- **OIDC vs token fallback** — if the Cloudflare OIDC setup hits snags during Task 19, fall back to a scoped API token in a GitHub repo secret and proceed. Document the fallback in `docs/setup/cloudflare-oidc.md` and create a follow-up issue to revisit OIDC.
- **Action SHAs:** all third-party actions in Tasks 18 and 20 must be pinned to commit SHAs, not tags. Resolve at PR time.
- **Code style:** see `CLAUDE.md`. KISS, simple loops, minimal human comments, share-don't-duplicate.
- **Observability follow-up:** the spec calls for the `tracing` crate with a `WorkersLayer` and a Worker Analytics Engine binding. The plan uses plain `console_log!` everywhere as a baseline. Swap to `tracing` + Analytics Engine once the rest of the rewrite is solid — small mechanical change, no architectural impact.
