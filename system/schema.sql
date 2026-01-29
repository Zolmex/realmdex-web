-- Written for SQLite
PRAGMA foreign_keys = ON;

CREATE TABLE servers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    icon_path TEXT,
    discord_link TEXT
);

CREATE TABLE server_polls (
    id INTEGER PRIMARY KEY,
    server_id INTEGER NOT NULL,
    online INTEGER NOT NULL CHECK (online IN (0, 1)), -- Boolean
    players INTEGER,
    time TEXT NOT NULL,

    FOREIGN KEY (server_id)
        REFERENCES servers(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_server_polls_server_time
ON server_polls(server_id, time);

ALTER TABLE servers -- Forgot host column when creating database now it stays as an ALTER TABLE command. Deal with it
ADD COLUMN host TEXT NOT NULL;