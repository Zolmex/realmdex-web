-- Mock seed data for RealmDex development
-- Run after schema.sql to populate a realistic dev database

PRAGMA foreign_keys = ON;

-- Private Servers (mix of online, offline, and WIP)
INSERT INTO servers (id, name, icon_path, discord_link, host, is_wip, category) VALUES
(1, 'Valor',           '/content/images/valor.webp',           'https://discord.gg/valormc',    'http://mock-valor:8080/players',     0, 'pserver'),
(2, 'Cosmic Realms',   '/content/images/cosmic.webp',          'https://discord.gg/cosmic',     'http://mock-cosmic:8080/players',    0, 'pserver'),
(3, 'Old School',      '/content/images/oldschool.png',        'https://discord.gg/oldschool',  'http://mock-oldschool:8080/players', 0, 'pserver'),
(4, 'Anomaly',         '/content/images/anomaly.webp',         'https://discord.gg/anomaly',    'http://mock-anomaly:8080/players',   0, 'pserver'),
(5, 'Untiered',        '/content/images/untiered.webp',        'https://discord.gg/untiered',   'http://mock-untiered:8080/players',  0, 'pserver'),
(6, 'Astrum',          '/content/images/astrum.png',           'https://discord.gg/astrum',     'http://mock-astrum:8080/players',    0, 'pserver'),
(7, 'Evershade',       '/content/images/evershade.webp',       'https://discord.gg/evershade',  'http://mock-evershade:8080/players', 0, 'pserver'),
(8, 'Ruins of Valthor','/content/images/ruins-of-valthor.webp','https://ruinsofvalthor.com',    'http://mock-valthor:8080/players',   0, 'pserver'),
(9, 'Dominion',        '/content/images/dom.png',              'https://discord.gg/dominion',   'http://mock-dom:8080/players',       1, 'pserver'),
(10,'IcAS',            '/content/images/icas.webp',            'https://discord.gg/icas',       'http://mock-icas:8080/players',      1, 'pserver');

-- Realm-Like Games
INSERT INTO servers (id, name, icon_path, discord_link, host, is_wip, category) VALUES
(11, 'The Nexus',      '/content/images/logo.webp',            'https://discord.gg/thenexus',   'http://mock-nexus:8080/players',     0, 'realm-like'),
(12, 'Realm of Valthor','/content/images/realm-of-valthor.webp','https://discord.gg/valthor',   'http://mock-rov:8080/players',       0, 'realm-like'),
(13, 'Hessels',        '/content/images/hessels.webp',         'https://discord.gg/hessels',    'http://mock-hessels:8080/players',   0, 'realm-like'),
(14, 'Alvin',          '/content/images/alvin.webp',           'https://discord.gg/alvin',      'http://mock-alvin:8080/players',     1, 'realm-like');

-- Generate 14 days of poll data (one poll per hour = 24 per day = 336 per server)
-- Each server has different characteristics:
--   Valor:           high pop, very stable (99% uptime)
--   Cosmic Realms:   medium pop, stable (95% uptime)
--   Old School:      medium pop, stable (92% uptime)
--   Anomaly:         medium-low pop, good uptime (90%)
--   Untiered:        low pop, decent uptime (85%)
--   Astrum:          low pop, spotty uptime (70%)
--   Evershade:       very low pop, poor uptime (50%)
--   Ruins of Valthor: offline (0% recent uptime, was online earlier)

-- We use a recursive CTE to generate timestamps, then insert polls with
-- deterministic pseudo-random variation using the server_id and hour offset.

-- Valor (id=1): 99% uptime, 80-350 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    1,
    CASE WHEN (h * 7 + 3) % 100 < 99 THEN 1 ELSE 0 END,
    CASE WHEN (h * 7 + 3) % 100 < 99
        THEN 80 + ABS((h * 137 + 42) % 271)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Cosmic Realms (id=2): 95% uptime, 40-180 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    2,
    CASE WHEN (h * 13 + 5) % 100 < 95 THEN 1 ELSE 0 END,
    CASE WHEN (h * 13 + 5) % 100 < 95
        THEN 40 + ABS((h * 89 + 17) % 141)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Old School (id=3): 92% uptime, 30-150 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    3,
    CASE WHEN (h * 11 + 7) % 100 < 92 THEN 1 ELSE 0 END,
    CASE WHEN (h * 11 + 7) % 100 < 92
        THEN 30 + ABS((h * 73 + 29) % 121)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Anomaly (id=4): 90% uptime, 20-120 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    4,
    CASE WHEN (h * 17 + 11) % 100 < 90 THEN 1 ELSE 0 END,
    CASE WHEN (h * 17 + 11) % 100 < 90
        THEN 20 + ABS((h * 61 + 37) % 101)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Untiered (id=5): 85% uptime, 10-80 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    5,
    CASE WHEN (h * 19 + 2) % 100 < 85 THEN 1 ELSE 0 END,
    CASE WHEN (h * 19 + 2) % 100 < 85
        THEN 10 + ABS((h * 47 + 13) % 71)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Astrum (id=6): 70% uptime, 5-50 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    6,
    CASE WHEN (h * 23 + 9) % 100 < 70 THEN 1 ELSE 0 END,
    CASE WHEN (h * 23 + 9) % 100 < 70
        THEN 5 + ABS((h * 31 + 7) % 46)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Evershade (id=7): 50% uptime, 1-25 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    7,
    CASE WHEN (h * 29 + 4) % 100 < 50 THEN 1 ELSE 0 END,
    CASE WHEN (h * 29 + 4) % 100 < 50
        THEN 1 + ABS((h * 19 + 3) % 25)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Ruins of Valthor (id=8): was online first 7 days, offline last 7 days
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    8,
    CASE WHEN h < 168 THEN 1 ELSE 0 END,
    CASE WHEN h < 168
        THEN 15 + ABS((h * 53 + 11) % 86)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- =====================
-- Realm-Like Games
-- =====================

-- The Nexus (id=11): popular realm-like, 97% uptime, 60-250 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    11,
    CASE WHEN (h * 3 + 1) % 100 < 97 THEN 1 ELSE 0 END,
    CASE WHEN (h * 3 + 1) % 100 < 97
        THEN 60 + ABS((h * 113 + 31) % 191)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Realm of Valthor (id=12): medium realm-like, 88% uptime, 20-100 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    12,
    CASE WHEN (h * 37 + 13) % 100 < 88 THEN 1 ELSE 0 END,
    CASE WHEN (h * 37 + 13) % 100 < 88
        THEN 20 + ABS((h * 67 + 23) % 81)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;

-- Hessels (id=13): small realm-like, 75% uptime, 5-40 players
WITH RECURSIVE hours(h) AS (
    SELECT 0 UNION ALL SELECT h + 1 FROM hours WHERE h < 335
)
INSERT INTO server_polls (server_id, online, players, time)
SELECT
    13,
    CASE WHEN (h * 41 + 6) % 100 < 75 THEN 1 ELSE 0 END,
    CASE WHEN (h * 41 + 6) % 100 < 75
        THEN 5 + ABS((h * 29 + 11) % 36)
        ELSE 0 END,
    datetime('now', '-' || (335 - h) || ' hours')
FROM hours;
