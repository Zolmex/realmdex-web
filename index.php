<?php
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
            <!-- Cards will be dynamically generated here -->
        </div>
    </main>
    <script type="module" src="/scripts/index.js"></script>
</body>

</html>
