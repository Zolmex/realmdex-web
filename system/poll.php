<?php
declare(strict_types=1);

$dbPath = '/opt/uptime/uptime.db';
$timeoutSeconds = 10;

$db = new PDO("sqlite:$dbPath");
$db->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);

$servers = $db->query(
    "SELECT id, host FROM servers"
)->fetchAll(PDO::FETCH_ASSOC);

$insert = $db->prepare(
    "INSERT INTO server_polls
     (server_id, online, players, time)
     VALUES (?, ?, ?, datetime('now'))"
);

foreach ($servers as $server) {
    $host = $server['host'];
    $url = "http://$host/status";

    $online = 0;
    $players = 0;

    try {
        $ch = curl_init($url);
        curl_setopt_array($ch, [
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => $timeoutSeconds,
            CURLOPT_CONNECTTIMEOUT => $timeoutSeconds,
        ]);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);

        if ($response !== false && $httpCode === 200) {
            $data = json_decode($response, true);

            $online = 1;
            $players = $data['players'];
        }

        curl_close($ch);
    } catch (Throwable $e) {
        // Server considered offline
        $online = 0;
        $players = 0;
    }

    $insert->execute([
        $server['id'],
        $online,
        $players
    ]);
}
