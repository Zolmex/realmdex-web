-- Production migration: add category column to servers table
-- Run on container: docker exec server-count-prod sqlite3 /var/www/html/data/uptime.db < /var/www/html/system/seed-prod.sql

PRAGMA foreign_keys = ON;

-- Add category column if it doesn't exist
-- SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so this will error if already present
ALTER TABLE servers ADD COLUMN category TEXT NOT NULL DEFAULT 'pserver';

-- Set realm-like categories for specific servers
UPDATE servers SET category = 'realm-like' WHERE name IN (
    'Realm of Valthor',
    'Hessel''s Realm',
    'Alvin Realms'
);
