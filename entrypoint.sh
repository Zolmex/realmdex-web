#!/bin/sh

# Ensure the database directory exists and is writable by the web server
mkdir -p /var/www/html/data
chmod 777 /var/www/html/data

# Initialize database if it doesn't exist
if [ ! -f /var/www/html/data/uptime.db ]; then
    echo "Initializing database..."
    sqlite3 /var/www/html/data/uptime.db < /var/www/html/system/schema.sql
fi

# Ensure the database file itself is writable
if [ -f /var/www/html/data/uptime.db ]; then
    chmod 666 /var/www/html/data/uptime.db
fi

# Start the polling script in the background
(
    while true; do
        echo "Running poll..."
        php /var/www/html/system/poll.php
        sleep 60
    done
) &

# Start Apache in the foreground
exec apache2-foreground
