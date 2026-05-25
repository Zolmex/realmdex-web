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
