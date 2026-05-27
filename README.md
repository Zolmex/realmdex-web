# RealmDex

This is the public repository for [Realmdex.com](https://realmdex.com/).

![image](/realmdex-com.png)

RealmDex tracks server status, player counts, and uptime for RotMG private servers and Realm-Like games.

The site runs on Cloudflare Workers (Rust + Leptos SSR) backed by Cloudflare D1. Every minute, a Cron Trigger fans out parallel polls to each listed server and writes the results to D1. The page renders server-side with the latest data and refreshes on a 30s client-side timer.

## Getting Your Server Listed

To get your server on RealmDex, you need to provide two things:

1. **A player count API endpoint**
2. **Server metadata** (name, icon, links)

### 1. Player Count Endpoint (Required)

RealmDex polls your server every 60 seconds. Your endpoint must:

- Return the **current online player count as a plain integer** in the response body (e.g. `42`)
- Return **HTTP 200** when the server is online
- Any non-200 response (or timeout after 10 seconds) is treated as **offline**

**Example response:**
```
HTTP/1.1 200 OK
Content-Type: text/plain

42
```

### 2. Server Metadata (Required)

| Field | Description | Example |
|---|---|---|
| **Name** | Your server's display name | `Valor` |
| **Icon** | Square image (PNG or WebP, recommended 100x100+) | `valor.webp` |
| **Link** | Discord invite or homepage URL | `https://discord.gg/valormc` |
| **Endpoint** | Full URL to your player count endpoint | `https://yourserver.com/api/players` |
| **Category** | `pserver` (private server) or `realm-like` (Realm-Like game) | `pserver` |

### What RealmDex Tracks Automatically

- **Online/Offline status** — based on whether your endpoint returns HTTP 200
- **Current player count** — parsed from your endpoint's response body
- **24-hour peak players** — highest player count in the last 24 hours
- **Last-hour sparkline** — small per-card chart of recent player count
- **Uptime history** — daily uptime percentage over the past 14 days

### WIP Servers

If your server is still in development, it can be listed with a **WIP** (Work in Progress) status. WIP servers appear in a separate section and are not polled.

## Development

### Prerequisites

- Rust (stable). Install with [rustup](https://rustup.rs).
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`.
- Node 20+ (for `wrangler` and `sass`).
- `worker-build` (installed automatically by `scripts/build-worker.sh` on first run).

> **Note on PATH:** if you have both Homebrew Rust and rustup installed, make sure `~/.cargo/bin` comes first on your `PATH` so the rustup toolchain wins. The build script already handles this when invoked via `wrangler dev`, but for direct `cargo` invocations you may need to prepend it yourself. See the comment block at the top of `wrangler.toml`.

### Running locally

First-time setup:

```bash
npx wrangler d1 migrations apply realmdex --local
./scripts/build-styles.sh
```

Then start the dev server:

```bash
npx wrangler dev --local
```

Open http://localhost:8787 (or whichever port `wrangler` reports).

### Importing real production data

If you want to develop against the real server list rather than the dev seed:

```bash
# Drop legacy/uptime.db (or uptime.db.live) into legacy/, then:
./scripts/import_legacy_db.sh
```

This clears the dev seed and replaces it with the real production list.

### Tests

```bash
cargo test --workspace
```

### Resetting local data

```bash
rm -rf .wrangler
npx wrangler d1 migrations apply realmdex --local
```

## Architecture

- **`crates/app/`** — Leptos app: components, types, server functions, D1 query helpers (the `db` module is ssr-gated).
- **`crates/worker/`** — Cloudflare Worker entrypoints: the `fetch` handler for SSR + JSON API routes, the `scheduled` handler for the per-minute poller and the daily rollup. Worker-rs 0.8.
- **`migrations/`** — D1 migrations applied with `wrangler d1 migrations apply`.
- **`public/`** — static assets (CSS, images, favicon) served by Workers Assets.
- **`scripts/`** — `build-worker.sh` (used by `wrangler.toml`'s build command), `build-styles.sh`, `import_legacy_db.sh`.

### Architectural reality check

Leptos's `#[server]` macro is not currently usable on Cloudflare Workers because worker-rs's D1 futures are `!Send` while the server-fn machinery requires `Send`. As a result, `/api/list_servers` and `/api/server_sparkline` are exposed as plain `async fn`s with a hand-rolled JSON dispatcher in `crates/worker/src/lib.rs`, and the client-side live-update layer is an inlined vanilla JS controller rather than a Leptos reactive layer. The initial page render is real SSR with D1 data baked in via Leptos context. See the comments in `crates/app/src/server_fns.rs` for the swap-back plan if this gap closes upstream.

## Deployment

See:

- [`docs/setup/cloudflare-oidc.md`](docs/setup/cloudflare-oidc.md) — GitHub Actions → Cloudflare auth setup using OIDC (no long-lived API token in repo secrets).
- [`docs/setup/rate-limiting.md`](docs/setup/rate-limiting.md) — edge rate-limiting rule for `/api/*`.
- [`docs/setup/production-migration.md`](docs/setup/production-migration.md) — one-shot procedure for the PHP → Workers cutover.

### Public-repo deploy safety

- The deploy workflow (`.github/workflows/deploy.yml`) only runs `on: push` to `main`. Fork PRs cannot trigger it.
- The `production` GitHub Environment requires reviewer approval. Every deploy pauses for a human click, even legitimate `main` pushes — defense against compromised accounts and accidental merges.
- Authentication to Cloudflare is via GitHub OIDC. No long-lived `CLOUDFLARE_API_TOKEN` lives in repo secrets.
- Third-party actions are pinned to commit SHAs (not tags) to defend against tag movement.
- Branch protection on `main`: required PR review, required CI status check, no force pushes, no admin bypass.

## Credits

- Repository contributors.
- [RotMG Wallpaper by Bohrokki](https://wall.alphacoders.com/big.php?i=1039035)
