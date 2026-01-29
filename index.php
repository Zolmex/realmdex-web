<?php

function getUptimeColor($percentage)
{
    if ($percentage >= 75) {
        // Green range
        $greenIntensity = floor((($percentage - 75) / 25) * 255);
        return 'rgb(' . (255 - $greenIntensity) . ', 255, 0)';
    } else if ($percentage >= 50) {
        // Yellow to green range
        $ratio = ($percentage - 50) / 25;
        return 'rgb(255, 255, ' . floor($ratio * 255);
    } else if ($percentage > 0) {
        // Red to yellow range
        $ratio = $percentage / 50;
        return 'rgb(255, ' . floor($ratio * 255);
    } else {
        // Pure red for 0%
        return 'rgb(255, 0, 0)';
    }
}

function createUptimeGrid($uptime)
{
    $ret = '<div class="uptime-grid">';
    $index = 0;
    foreach ($uptime as $day) {
        $percent = (float) $day['uptime_percent'];
        $color = getUptimeColor($percent);

        $ret .= '<div class="uptime-day" style="background-color: '.$color.'" data-uptime="'.$percent.'" data-day="' . ($index + 1) . '"></div>';
        $index++;
    }
    $ret .= "</div>";
    return $ret;
}

function fetchUptime(PDO $db, int $serverId, int $days): array
{
    $stmt = $db->prepare(
        "SELECT
            date(time) AS day,
            COUNT(*) AS total_checks,
            SUM(online) AS up_checks,
            ROUND(100.0 * SUM(online) / COUNT(*), 2) AS uptime_percent
         FROM server_polls
         WHERE server_id = :server_id
           AND time >= datetime('now', :interval)
         GROUP BY date(time)
         ORDER BY day ASC"
    );

    $stmt->execute([
        ':server_id' => $serverId,
        ':interval' => "-$days days",
    ]);

    return $stmt->fetchAll(PDO::FETCH_ASSOC);
}

function generateServerGrid()
{
    global $db;
    $dbPath = 'C:\Maty\SQLite\RealmDex\uptime.db';
    $db = new PDO("sqlite:$dbPath");
    $db->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

    $servers = $db->query( // Query all servers
        "SELECT * FROM servers"
    )->fetchAll(PDO::FETCH_ASSOC);

    foreach ($servers as $server) {
        $serverId = $server['id'];
        $host = $server['host'];
        $serverName = $server['name'];
        $serverIcon = $server['icon_path'];
        $serverDiscord = $server['discord_link'];

        $uptime = fetchUptime($db, $serverId, 7);
        $uptimeExtended = fetchUptime($db, $serverId, 14);

        $stmtPeak = $db->prepare(
            "SELECT MAX(players)
             FROM server_polls
             WHERE server_id = :server_id
               AND time >= datetime('now', '-1 day')"
        );

        $stmtPeak->execute([
            ':server_id' => $serverId,
        ]);

        $server24hPeak = $stmtPeak->fetchColumn();
        $server24hPeak = $server24hPeak !== null ? (int) $server24hPeak : 0;

        $stmtLast = $db->prepare(
            "SELECT players, online
             FROM server_polls
             WHERE server_id = :server_id
             ORDER BY time DESC
             LIMIT 1"
        );

        $stmtLast->execute([
            ':server_id' => $serverId,
        ]);

        $row = $stmtLast->fetch(PDO::FETCH_ASSOC);

        $serverPlayers = $row ? $row['players'] : 0;
        $serverPlayers = $serverPlayers !== null ? (int) $serverPlayers : 0;
        $isOnline = $row ? $row['online'] : false;

        $statusClass = $isOnline ? 'online' : 'offline';
        $statusText = $isOnline ? 'Online' : 'Offline';

        echo '
    <div class="server-card" data-server-id="'.$serverId.'">
        <div class="card-header">
            <div class="status-indicator ' . $statusClass . '" title="' . $statusText . '"></div>
            <img src="' . $serverIcon . '" alt="' . $serverName . '" class="server-icon" data-discord="' . $serverDiscord . '">
            <h3 class="server-name">' . $serverName . '</h3>
        </div>

        <div class="card-stats">
            <div class="stat-row">
                <span class="stat-label">Players</span>
                <span class="stat-value">' . $serverPlayers . '</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">24h Peak</span>
                <span class="stat-value">' . $server24hPeak . '</span>
            </div>
        </div>

        <div class="uptime-section">
            <div class="uptime-label">Uptime (Past Week)</div>
' . createUptimeGrid($uptime) . '<div class="uptime-extended">
                <div class="uptime-label">Uptime (Past 2 Weeks)</div>
' . createUptimeGrid($uptimeExtended) . '</div>
        </div>
    </div>
';
    }
}

?>

<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="/styles/index.css">
    <title>RealmDex PServer Stats</title>
    <link rel="icon" type="image/x-icon" href="/favicon.ico">
    <script src="https://code.jquery.com/jquery-4.0.0.js" integrity="sha256-9fsHeVnKBvqh3FB2HYu7g2xseAZ5MlN6Kz/qnkASV8U=" crossorigin="anonymous"></script>
</head>

<body>
    <header class="site-header" role="banner">
        <div id="h-realmdex-icon">
            <img src="/content/images/logo.webp" alt="RealmDex logo">
            <p>RealmDex</p>
        </div>
    </header>
    <main class="server-grid-container" role="main">
        <div id="server-grid">
            <?php generateServerGrid(); ?>
        </div>
    </main>
    <script type="module" src="/scripts/index.js"></script>
</body>

</html>
