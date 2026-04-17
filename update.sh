#!/bin/bash
# Update existing servers with new host URLs and is_wip flags

echo "Checking database schema..."

# Check if is_wip column exists, add it if it doesn't
COLUMN_EXISTS=$(docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "PRAGMA table_info(servers);" | grep -c "is_wip")

if [ "$COLUMN_EXISTS" -eq 0 ]; then
  echo "Adding is_wip column..."
  docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
    "ALTER TABLE servers ADD COLUMN is_wip INTEGER DEFAULT 0;"
  echo "✓ is_wip column added"
else
  echo "✓ is_wip column already exists"
fi

echo ""
echo "Updating server hosts and WIP status..."

# Update Domain of Magica
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET host = 'http://198.244.151.113:2051/realmdex/stats', is_wip = 0 WHERE name = 'Domain of Magica';"

# Update Valor (already correct, but set is_wip)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 0 WHERE name = 'Valor';"

# Update Attack the Boss (mark as WIP)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 1 WHERE name = 'Attack the Boss';"

# Update Path of Olympus
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 0 WHERE name = 'Path of Olympus';"

# Update Anomaly Realms (mark as WIP)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 1 WHERE name = 'Anomaly Realms';"

# Delete Cosmic Realms v1 (no longer in use)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "DELETE FROM servers WHERE name = 'Cosmic Realms v1';"

# Update Ruins of Valthor (new IP)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET host = 'http://77.90.14.79:2000/realmdex/stats', is_wip = 0 WHERE name = 'Ruins of Valthor';"

# Update Realm of Valthor
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 0 WHERE name = 'Realm of Valthor';"

# Update Ica's Realm
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 0 WHERE name = 'Ica''s Realm';"

# Update Evershade (mark as WIP)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 1 WHERE name = 'Evershade';"

# Update Hessel's Realm
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET is_wip = 0 WHERE name = 'Hessel''s Realm';"

# Update Untiered (new host)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET host = 'http://untiered.svera.xyz:8080/realmdex/stats', is_wip = 0 WHERE name = 'Untiered';"

# Update Alvin Realms (new host)
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db \
  "UPDATE servers SET host = 'http://116.203.91.218:5656/realmdex/stats', is_wip = 0 WHERE name = 'Alvin Realms';"

echo "Update complete!"
echo ""
echo "Verifying changes:"
docker exec -i server-count-prod sqlite3 /var/www/html/data/uptime.db "SELECT id, name, host, is_wip FROM servers ORDER BY id;"
