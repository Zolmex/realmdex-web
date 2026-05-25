INSERT INTO servers (id, name, icon_path, discord_link, host, category, is_wip, polled) VALUES
  (1, 'Valor',     '/content/images/valor.webp',     'https://discord.gg/valormc',     'http://localhost:9001/v', 'pserver', 0, 1),
  (2, 'Pserver A', '/content/images/placeholder.webp','https://discord.gg/a',          'http://localhost:9001/a', 'pserver', 0, 1),
  (3, 'WIP One',   '/content/images/placeholder.webp','https://discord.gg/w',          '',                        'pserver', 1, 0),
  (4, 'Realmlike', '/content/images/placeholder.webp','https://example.com',           'http://localhost:9001/r', 'realm-like', 0, 1);

INSERT INTO server_polls (server_id, online, players, time)
SELECT 1, 1, 42, datetime('now', '-' || (n * 60) || ' seconds') FROM (
  WITH RECURSIVE c(n) AS (SELECT 0 UNION ALL SELECT n+1 FROM c WHERE n < 60) SELECT n FROM c
);

INSERT INTO server_polls_daily (server_id, day, total_checks, up_checks, peak_players)
SELECT 1, date('now', '-' || n || ' days'), 1440, CASE WHEN n IN (3,7) THEN 1000 ELSE 1440 END, 50
FROM (WITH RECURSIVE c(n) AS (SELECT 0 UNION ALL SELECT n+1 FROM c WHERE n < 14) SELECT n FROM c);
