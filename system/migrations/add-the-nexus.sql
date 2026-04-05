-- Add The Nexus as a realm-like server
INSERT INTO servers (name, icon_path, discord_link, host, is_wip, category)
VALUES (
    'The Nexus',
    '/content/images/The_Nexus.png',
    'https://discord.gg/RPWPAeZrEy',
    'https://function-bun-production-dc7d.up.railway.app/player-count',
    0,
    'realm-like'
);
