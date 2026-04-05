# RealmDex
This is the public repository for [Realmdex.com](https://realmdex.com/).

![image](/realmdex-com.png)

RealmDex tracks server status, player counts, and uptime for RotMG private servers and Realm-Like games.

## Getting Your Server Listed

To get your server on RealmDex, you need to provide two things:

1. **A player count API endpoint**
2. **Server metadata** (name, icon, links)

### 1. Player Count Endpoint (Required)

RealmDex polls your server every 60 seconds to check its status. Your server must expose an HTTP endpoint that:

- Returns the **current online player count as a plain integer** in the response body (e.g. `42`)
- Returns **HTTP 200** when the server is online
- Any non-200 response (or timeout after 10 seconds) is treated as **offline**

**Example response:**
```
HTTP/1.1 200 OK
Content-Type: text/plain

42
```

This is the only technical integration required. How you implement this endpoint is up to you — a simple route in your game server, a standalone status service, etc.

### 2. Server Metadata (Required)

Provide the following information when requesting a listing:

| Field | Description | Example |
|---|---|---|
| **Name** | Your server's display name | `Valor` |
| **Icon** | Square image (PNG or WebP, recommended 100x100+) | `valor.webp` |
| **Link** | Discord invite or homepage URL | `https://discord.gg/valor` |
| **Endpoint** | Full URL to your player count endpoint | `https://yourserver.com/api/players` |
| **Category** | `pserver` (private server) or `realm-like` (Realm-Like game) | `pserver` |

### What RealmDex Tracks Automatically

Once your server is listed, the following is collected and displayed automatically:

- **Online/Offline status** — based on whether your endpoint returns HTTP 200
- **Current player count** — parsed from your endpoint's response body
- **24-hour peak players** — highest player count in the last 24 hours
- **Uptime history** — daily uptime percentage over the past 14 days

### WIP Servers

If your server is still in development, it can be listed with a **WIP** (Work in Progress) status. WIP servers appear in a separate section and are not polled for status data. Once you're ready to go live, provide your player count endpoint and the listing will be updated.

## Development

### Prerequisites

- [Docker](https://www.docker.com/products/docker-desktop/)

### Running Locally

```bash
docker compose up --build
```

The app will be available at `http://localhost:8080`.

The `SEED_DB=1` environment variable in `docker-compose.yml` populates the database with mock servers and 14 days of poll history on first run. Remove it or set to `0` for production.

To reset the database and re-seed:

```bash
docker compose down -v
docker compose up --build
```

## Credits
- Repository contributors.
- [RotMG Wallpaper by Bohrokki](https://wall.alphacoders.com/big.php?i=1039035)
