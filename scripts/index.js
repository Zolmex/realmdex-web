// Server sorting and category tab functionality
document.addEventListener('DOMContentLoaded', function() {
    const sortSelect = document.getElementById('sort-select');
    const categoryTabs = document.querySelectorAll('.category-tab');
    const serverGrids = document.querySelectorAll('.server-grid');

    // Extract server data from card
    function getServerData(card) {
        const playersText = card.querySelector('.stat-value')?.textContent || '0';
        const players = parseInt(playersText) || 0;

        const uptimeDays = card.querySelectorAll('.uptime-day');
        let totalUptime = 0;
        uptimeDays.forEach(day => {
            const uptime = parseFloat(day.dataset.uptime) || 0;
            totalUptime += uptime;
        });
        const avgUptime = uptimeDays.length > 0 ? totalUptime / uptimeDays.length : 0;

        return {
            element: card,
            players: players,
            uptime: avgUptime
        };
    }

    // Sort a specific grid's online servers
    function sortGrid(grid, sortType) {
        const allCards = Array.from(grid.children);

        const offlineDivider = allCards.find(el =>
            el.classList.contains('wip-divider') && el.classList.contains('offline-divider')
        );
        const wipDivider = allCards.find(el =>
            el.classList.contains('wip-divider') && !el.classList.contains('offline-divider')
        );

        let onlineCards = [];
        if (offlineDivider) {
            onlineCards = allCards.slice(0, allCards.indexOf(offlineDivider));
        } else if (wipDivider) {
            onlineCards = allCards.slice(0, allCards.indexOf(wipDivider));
        } else {
            onlineCards = allCards.filter(el =>
                el.classList.contains('server-card') &&
                el.querySelector('.status-indicator.online')
            );
        }

        let offlineCards = [];
        let wipCards = [];

        if (offlineDivider && wipDivider) {
            const offlinePos = allCards.indexOf(offlineDivider);
            const wipPos = allCards.indexOf(wipDivider);
            offlineCards = allCards.slice(offlinePos + 1, wipPos);
            wipCards = allCards.slice(wipPos + 1);
        } else if (offlineDivider) {
            offlineCards = allCards.slice(allCards.indexOf(offlineDivider) + 1);
        } else if (wipDivider) {
            wipCards = allCards.slice(allCards.indexOf(wipDivider) + 1);
        }

        let sortedOnline = onlineCards.map(card => getServerData(card));

        switch(sortType) {
            case 'players-desc':
                sortedOnline.sort((a, b) => b.players - a.players);
                break;
            case 'players-asc':
                sortedOnline.sort((a, b) => a.players - b.players);
                break;
            case 'uptime-desc':
                sortedOnline.sort((a, b) => b.uptime - a.uptime);
                break;
            case 'random':
                for (let i = sortedOnline.length - 1; i > 0; i--) {
                    const j = Math.floor(Math.random() * (i + 1));
                    [sortedOnline[i], sortedOnline[j]] = [sortedOnline[j], sortedOnline[i]];
                }
                break;
        }

        grid.innerHTML = '';

        sortedOnline.forEach(data => grid.appendChild(data.element));

        if (offlineCards.length > 0) {
            grid.appendChild(offlineDivider);
            offlineCards.forEach(card => grid.appendChild(card));
        }

        if (wipCards.length > 0) {
            grid.appendChild(wipDivider);
            wipCards.forEach(card => grid.appendChild(card));
        }
    }

    // Category tab switching
    categoryTabs.forEach(tab => {
        tab.addEventListener('click', function() {
            const category = this.dataset.category;

            categoryTabs.forEach(t => t.classList.remove('active'));
            this.classList.add('active');

            serverGrids.forEach(grid => {
                grid.style.display = grid.dataset.category === category ? '' : 'none';
            });
        });
    });

    // Sort change applies to the currently visible grid
    if (sortSelect) {
        sortSelect.addEventListener('change', function() {
            const activeGrid = document.querySelector('.server-grid[style=""], .server-grid:not([style*="display: none"])');
            if (activeGrid) {
                sortGrid(activeGrid, this.value);
            }
        });
    }
});
