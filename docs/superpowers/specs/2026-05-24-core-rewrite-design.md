# Core Rewrite Design — RealmDex on Rust + Cloudflare

**Status:** Draft for review
**Date:** 2026-05-24
**Scope:** Sub-project 1 of 4. Replaces the existing PHP site with a Rust/Leptos app running on Cloudflare Workers + D1. Same look, parity with current features, plus live data updates and per-card sparklines. Out of scope: ingestion API, admin dashboard, full CI/CD platform — each is its own follow-up spec.

## Goals

- Replace the PHP app at `index.php` + `system/poll.php` with a Rust implementation running entirely on Cloudflare's edge.
- Maintain visual + functional parity with today's site: server cards, category tabs (Private Servers / Realm-Likes), sort dropdown, 7-day and 14-day uptime grids, online/offline/WIP states.
- Add modest live-update behavior and a per-card last-hour sparkline so the experience feels alive without adding scope creep.
- Keep the deploy story safe for a public repository: external forks must not be able to trigger deploys; deploys must require human approval; no long-lived deploy credentials in repo secrets.
- Leave clean seams for the follow-up sub-projects (ingestion API, admin dashboard).

## Non-goals

- Per-PR preview deploys, staging environments, automated rollback, bot-driven dependency updates (deferred to the CI/CD sub-project).
- Per-minute history beyond 30 days (rolled up to daily).
- E2E browser tests.
- Theming, dark mode, accessibility audits, mobile-specific polish (none of these are present today; not introducing them now).
- Cloudflare Turnstile (decided against for the core rewrite; hook left for later if abuse appears).

## Stack

- **Rust + Leptos fullstack** running on Cloudflare Workers via `worker-rs` + the appropriate Leptos-on-Workers integration.
- **D1** for storage (SQLite under the hood; current schema ports almost as-is).
- **Workers Assets** for the WASM hydrate bundle, CSS, images, favicon.
- **Cron Triggers** for the poller (every 60s) and the rollup job (daily at ~03:00 UTC).
- **Cloudflare Rate Limiting Rules** for per-IP/per-path quotas on the public API surface.

Rationale for Leptos: the user explicitly asked for "experimental / fun" and chose Leptos over the leaner `axum + maud + HTMX` option after seeing both. The reactive primitives make the live-update + sparkline features cheap to implement and natural to extend later in the admin dashboard.

## Architecture

```
                ┌───────────────────────┐
   Visitor ───▶ │  Worker (SSR)         │ ──▶ D1 (read)
                │  - Leptos server fns  │
                │  - /api/servers JSON  │
                │  - serves HTML +      │
                │    /pkg/ wasm hydrate │
                └───────────┬───────────┘
                            │
                            ▼
                  Workers Assets (CSS, images, /pkg/)

                ┌───────────────────────┐
  Cron (1m) ──▶ │  Worker (poller)      │ ──▶ Fan-out fetches to each
                │  - reads servers      │     server's host endpoint
                │  - writes polls       │     (parallel, per-server timeout)
                └───────────┬───────────┘
                            ▼
                          D1 (write)

                ┌───────────────────────┐
  Cron (daily)▶ │  Worker (rollup)      │ ──▶ Aggregates yesterday's raw
                │                       │     polls into server_polls_daily,
                │                       │     deletes raw rows >30d
                └───────────────────────┘
```

One Worker project, multiple entrypoints sharing the same D1 binding. Static assets are served directly from Workers Assets (not through the Worker fetch handler) so they don't consume CPU quota.

## Module layout

```
crates/
  app/                       ← Leptos app (shared SSR + hydrate)
    src/
      lib.rs                 ← <App/> root, routes, hydrate entry
      components/
        server_card.rs       ← card + uptime grid + sparkline
        server_grid.rs       ← category tabs + sort + grid container
        site_header.rs
      server_fns.rs          ← #[server] functions: list_servers, server_sparkline
      types.rs               ← Server, PollSummary, SparkPoint (Serde)
      style/                 ← scss owned by cargo-leptos

  worker/                    ← Cloudflare Worker entrypoints
    src/
      lib.rs                 ← fetch handler → Leptos SSR
      poller.rs              ← scheduled() handler: poll fan-out
      rollup.rs              ← scheduled() handler: daily aggregate + prune
      db.rs                  ← typed D1 query helpers

  migrations/                ← wrangler d1 migrations (numbered .sql files)
public/                      ← favicon, logo, server icons (Workers Assets)
wrangler.toml                ← bindings, cron triggers, assets, env
Cargo.toml                   ← workspace
```

The two-crate split is deliberate: `app` knows nothing about Workers (pure Leptos + Serde), `worker` knows only about D1 / Cron / fetch. This keeps the UI testable natively and the Workers boilerplate out of UI code.

## Data flow

**Initial page load (SSR):**

1. `GET /` → Worker SSR entrypoint.
2. Worker renders Leptos `<App/>`. Server functions `list_servers("pserver")` and `list_servers("realm-like")` run in parallel against D1.
3. HTML streams back with hydration markers + script tag pointing to `/pkg/realmdex.js`.
4. Browser fetches `/pkg/realmdex.js`, `/pkg/realmdex_bg.wasm`, CSS, and images from Workers Assets.
5. WASM bundle hydrates; signals come alive; no re-fetch on first paint.

**Live updates (client-side):**

6. A Leptos `Resource` re-runs `list_servers(current_category)` every 30s.
7. On resolve, signals update and cards re-render in place — no flash, no reload.
8. `server_sparkline(server_id)` refreshes on the same cadence.
9. If a fetch fails, the resource keeps the previous value and retries on the next tick. No error UI in this spec.

**Polling (Cron Trigger, every 60s):**

10. Cron fires `scheduled()` → `poller.rs`.
11. `SELECT id, host FROM servers WHERE polled = 1`.
12. `futures::join_all` over per-server async tasks; each `fetch()` has its own 10s timeout via `AbortController`.
13. Collect into `Vec<(server_id, online, players)>`.
14. Single batched `INSERT INTO server_polls (...) VALUES (...), (...), ...` to D1.

**Rollup (Cron Trigger, daily ~03:00 UTC):**

15. Aggregates yesterday's `server_polls` rows into `server_polls_daily` (per-server total_checks, up_checks, peak_players, day).
16. `INSERT ... ON CONFLICT(server_id, day) DO UPDATE` so retries are idempotent.
17. Deletes raw `server_polls` rows older than 30 days.

## Public API surface (Leptos server functions)

All POST, JSON, behind the security baseline below.

- `list_servers(category: Category) -> Vec<ServerCardData>` — everything one card needs in one round trip: identity, status, current players, 24h peak, last 14 days of daily uptime, last hour sparkline points. Single round trip per category. Returned **unsorted** (or sorted by `id`); ordering is a client concern.
- `server_sparkline(server_id: i64) -> Vec<SparkPoint>` — last hour, ~60 points. Used if a future feature wants a higher-resolution sparkline detached from the main payload.

**Sorting:** All sort logic lives on the client as a Leptos signal-based derivation over the resource's data. Switching sort mode does not re-fetch. The "default order on first paint" matches the current PHP behavior — online servers sorted by player count desc, then offline, then WIP — and is produced by the same client-side derivation running during SSR. No sort-related server-fn parameters.

**Security baseline (public read API):**

- CORS: `Access-Control-Allow-Origin: https://realmdex.com` only.
- `Origin` / `Referer` check on every server-fn request; reject mismatches with 403.
- Cloudflare Rate Limiting Rule: per-IP quota per minute on `/api/*` and Leptos server-fn paths.
- Minimal payload: never return `host`, internal IDs only if necessary, no admin fields.
- Hook (commented placeholder only, no code): a `verify_turnstile()` no-op stub in the request middleware. If abuse appears later, swap the stub for the real call without restructuring.

## D1 schema + migrations

```sql
-- 0001_init.sql

CREATE TABLE servers (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL,
    icon_path     TEXT,
    discord_link  TEXT,
    host          TEXT NOT NULL,                     -- full URL to player-count endpoint
    category      TEXT NOT NULL DEFAULT 'pserver',   -- 'pserver' | 'realm-like'
    is_wip        INTEGER NOT NULL DEFAULT 0,        -- 0/1
    polled        INTEGER NOT NULL DEFAULT 1,        -- 0 = skip in poller (covers WIP + paused)
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
    day           TEXT NOT NULL,                     -- 'YYYY-MM-DD' UTC
    total_checks  INTEGER NOT NULL,
    up_checks     INTEGER NOT NULL,
    peak_players  INTEGER NOT NULL,
    PRIMARY KEY (server_id, day)
);
```

**Changes vs. current PHP/SQLite schema:**

- `host` made non-nullable (matches actual usage — `poll.php` already assumes it's present).
- `is_wip` and `category` consistently typed and defaulted.
- New `polled` flag — explicit single-source-of-truth for "should the cron hit this?" Replaces the implicit "WIP means don't poll" rule. WIP servers default `polled = 0`.
- New `server_polls_daily` rollup. Page queries for the uptime grid hit this small table (~17 servers × 14 days = ~240 rows) instead of summing thousands of raw poll rows.
- `created_at` so the future admin UI can show when a server was added.

**Migration plan:**

- `wrangler d1 migrations` numbered `.sql` files in `migrations/`.
- `0001_init.sql` is the schema above.
- `0002_backfill.sql` is a one-shot data migration to be run only when promoting from the existing PHP system: import existing `data/uptime.db` content, set `polled = (is_wip = 0 ? 1 : 0)`, back-fill `server_polls_daily` from existing raw polls, delete raw polls older than 30 days.
- Local dev uses `wrangler d1 ... --local` (real SQLite file under `.wrangler/`).
- Production migrations run as part of the deploy job, after `wrangler deploy`, against `--env production`.

## Errors, retries, observability

**Poller error matrix (per-server, never aborts the batch):**

| Outcome | Recorded as |
|---|---|
| HTTP 200, body parses as int | `online=1, players=N` |
| HTTP 200, body unparseable | `online=1, players=0` + `tracing::warn` |
| Non-200 status | `online=0, players=0` |
| Timeout (10s) or network error | `online=0, players=0` |
| Unexpected panic in our code | `online=0, players=0` + `tracing::error` |

No in-tick retries (next tick is 60s away; retries inside a tick risk Workers sub-request limits). D1 write failure is logged and the tick is skipped — dropping a single minute is preferred to risking inconsistent state.

**Observability:**

- `tracing` crate with a `WorkersLayer` forwarding spans to `console.log` (visible via `wrangler tail`).
- Worker Analytics Engine binding (free tier) — one data point per poll tick: `(server_id, online, latency_ms)`.
- No external observability service in this spec.

**Client-side fetch errors:** Leptos `Resource` keeps previous value and silently retries on the next 30s tick. A "last updated Xs ago" indicator is a future enhancement, not in this spec.

## Secure deploy pipeline (baseline)

Full CI/CD platform is its own follow-up spec; this is the baseline the core rewrite needs to land safely on a public repo.

**Workflows:**

- `.github/workflows/ci.yml` — runs on every push + PR (including forks). **No secrets.** Builds Rust + WASM, runs `cargo test`, runs `cargo leptos build`, runs a migration dry-run against ephemeral local SQLite. Outputs build artifact.
- `.github/workflows/deploy.yml` — runs on `push` to `main` only. Requires the GitHub `production` environment, which is configured with a **required reviewer** (one human approval click per deploy). Uses **GitHub OIDC → Cloudflare** for short-lived credentials; no long-lived Cloudflare API token in GitHub Secrets. Steps: `wrangler deploy --env production`, then `wrangler d1 migrations apply --env production`. `permissions: id-token: write, contents: read`. Never runs on `pull_request`.

**Threat → mitigation matrix:**

| Threat | Mitigation |
|---|---|
| Forked PR running arbitrary code with secrets | `pull_request` event only (not `pull_request_target`); fork PRs receive no secrets and use the base repo's workflow files. `deploy.yml` doesn't fire on PRs at all. |
| Maintainer account compromise → silent deploy | `production` environment with required reviewer. |
| Stolen long-lived API token | OIDC; nothing long-lived in GitHub Secrets. |
| Malicious action tag movement | Pin all third-party actions by commit SHA. Renovate/Dependabot manages updates. |
| Overly broad workflow token | Workflow-level `permissions: contents: read`; per-job opt-in to anything more. |
| Direct push to main | Branch protection: required PR review, required status checks (build + test), no force push, no admin bypass. |
| Secret leakage in code or logs | Production secrets via `wrangler secret put` on Cloudflare, not in GitHub. `.env` in `.gitignore`. Only `example.env` committed. |
| Migration hitting the wrong DB | Separate `[env.production]` + `[env.preview]` D1 database IDs in `wrangler.toml`. Default (no `--env`) is local-only. |

**One-time setup checklist (documented in README during implementation):**

- Create Cloudflare service token bound to the GitHub repo via OIDC.
- Configure `production` environment in GitHub with required reviewer.
- Configure branch protection on `main`.
- `wrangler d1 create realmdex-prod` and put the ID in `wrangler.toml` under `[env.production]`.

## Testing + local dev loop

**Tests:**

- **Unit tests** (in `app` crate, `cargo test` natively, no Workers runtime):
  - `getUptimeColor` port + edge cases (0%, 50%, 75%, 100%).
  - Sort comparators (players-desc, players-asc, uptime-desc, random determinism).
  - Sparkline downsampling.
  - HTML rendering snapshots for `ServerCard` with known fixture data.
- **Integration tests** (in `worker` crate, `wrangler dev --local --test`):
  - DB query helpers against a seeded local D1.
  - Poller fan-out against a mock target HTTP server.
  - Rollup correctness on synthetic poll data spanning a day boundary.
- **CI smoke test:** `wrangler dev --local` boots, `GET /` returns 200 with "RealmDex" in body.

**Local dev:**

- `cargo leptos watch` for hot-reload of the `app` crate.
- `wrangler dev --local` for the Worker + local D1.
- `make seed` runs ported `system/seed.sql` against local D1 for fixture data.
- One-shot script to import the existing `data/uptime.db` into local D1, so day-one development uses real historical data.

## Out-of-scope hooks (left clean for follow-up specs)

- **Ingestion API** (sub-project 2): admin tokens, `POST /admin/servers`, `PATCH /admin/servers/:id`. Schema already has `polled` and `created_at` to support this without changes.
- **Admin dashboard** (sub-project 3): Leptos routes under `/admin/*`, Cloudflare Access in front. Same Leptos app crate, so it'll share types and components.
- **CI/CD platform** (sub-project 4): per-PR preview deploys, staging env, rollback, dependency bot.
