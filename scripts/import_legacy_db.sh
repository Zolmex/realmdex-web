#!/usr/bin/env bash
set -euo pipefail

# Import the legacy PHP-era uptime.db SQLite file into the local D1 dev database.
# Re-runnable: clears existing servers/polls first.

SRC="legacy/uptime.db"
[ -f "$SRC" ] || { echo "missing $SRC — copy your legacy uptime.db there first"; exit 1; }

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

export PATH="$HOME/.cargo/bin:$PATH"

echo "==> legacy schema:"
sqlite3 "$SRC" ".schema servers"
sqlite3 "$SRC" ".schema server_polls"

echo "==> legacy row counts:"
sqlite3 "$SRC" "SELECT 'servers='||COUNT(*) FROM servers; SELECT 'polls='||COUNT(*) FROM server_polls;"

# Detect whether legacy schema has the newer columns (category, is_wip).
# The original PHP schema doesn't, so we default category='pserver' and is_wip=0.
HAS_CATEGORY=$(sqlite3 "$SRC" "SELECT COUNT(*) FROM pragma_table_info('servers') WHERE name='category';")
HAS_IS_WIP=$(sqlite3 "$SRC" "SELECT COUNT(*) FROM pragma_table_info('servers') WHERE name='is_wip';")

CAT_EXPR="quote('pserver')"
[ "$HAS_CATEGORY" = "1" ] && CAT_EXPR="quote(coalesce(category,'pserver'))"

WIP_EXPR="0"
[ "$HAS_IS_WIP" = "1" ] && WIP_EXPR="coalesce(is_wip,0)"

echo "==> clearing dev seed rows from local D1..."
npx wrangler d1 execute realmdex --local --command "DELETE FROM server_polls; DELETE FROM server_polls_daily; DELETE FROM servers;"

echo "==> generating servers import (category_expr=$CAT_EXPR, is_wip_expr=$WIP_EXPR)..."
sqlite3 "$SRC" "
SELECT 'INSERT INTO servers (id,name,icon_path,discord_link,host,category,is_wip,polled) VALUES ('
  || id || ','
  || quote(name) || ','
  || quote(coalesce(icon_path,'')) || ','
  || quote(coalesce(discord_link,'')) || ','
  || quote(coalesce(host,'')) || ','
  || $CAT_EXPR || ','
  || $WIP_EXPR || ','
  || (CASE $WIP_EXPR WHEN 1 THEN 0 ELSE 1 END)
  || ');'
FROM servers;
" > "$WORK/servers.sql"

echo "==> generating polls import..."
sqlite3 "$SRC" "
SELECT 'INSERT INTO server_polls (server_id,online,players,time) VALUES ('
  || server_id || ','
  || online || ','
  || coalesce(players,0) || ','
  || quote(time)
  || ');'
FROM server_polls;
" > "$WORK/polls.sql"

SERVERS_LINES=$(wc -l < "$WORK/servers.sql" | tr -d ' ')
POLLS_LINES=$(wc -l < "$WORK/polls.sql" | tr -d ' ')

echo "==> applying servers ($SERVERS_LINES rows)..."
npx wrangler d1 execute realmdex --local --file "$WORK/servers.sql"

# Chunk polls.sql if it's large — wrangler d1 execute --file can choke on big inputs.
CHUNK_SIZE=500
if [ "$POLLS_LINES" -gt "$CHUNK_SIZE" ]; then
  echo "==> applying polls in chunks of $CHUNK_SIZE ($POLLS_LINES total)..."
  split -l "$CHUNK_SIZE" "$WORK/polls.sql" "$WORK/polls_chunk_"
  for chunk in "$WORK"/polls_chunk_*; do
    echo "    applying $(basename "$chunk") ($(wc -l < "$chunk" | tr -d ' ') rows)..."
    npx wrangler d1 execute realmdex --local --file "$chunk"
  done
else
  echo "==> applying polls ($POLLS_LINES rows)..."
  npx wrangler d1 execute realmdex --local --file "$WORK/polls.sql"
fi

echo "==> back-filling daily rollup..."
npx wrangler d1 execute realmdex --local --command "
INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
SELECT server_id, date(time), COUNT(*), SUM(online), MAX(players)
FROM server_polls
WHERE date(time) < date('now')
GROUP BY server_id, date(time)
ON CONFLICT(server_id, day) DO NOTHING;
"

echo "==> final counts:"
npx wrangler d1 execute realmdex --local --command "
SELECT 'servers='||COUNT(*) FROM servers;
SELECT 'polls='||COUNT(*) FROM server_polls;
SELECT 'daily='||COUNT(*) FROM server_polls_daily;
"
echo "==> done."
