// Server sorting functionality
document.addEventListener('DOMContentLoaded', function() {
    const sortSelect = document.getElementById('sort-select');
    const serverGrid = document.getElementById('server-grid');
    
    // Store original order for random sorting
    let originalOnlineServers = [];
    
    // Initialize - capture initial online server order
    function initializeServers() {
        const allCards = Array.from(serverGrid.children);
        const dividers = allCards.filter(el => el.classList.contains('wip-divider'));
        const offlineDividerIndex = dividers.findIndex(el => el.classList.contains('offline-divider'));
        
        // Get online servers (before any divider)
        if (offlineDividerIndex !== -1) {
            const offlineDivider = dividers.find(el => el.classList.contains('offline-divider'));
            const offlineDividerPos = allCards.indexOf(offlineDivider);
            originalOnlineServers = allCards.slice(0, offlineDividerPos);
        } else if (dividers.length > 0) {
            // Only WIP divider exists
            const firstDividerPos = allCards.indexOf(dividers[0]);
            originalOnlineServers = allCards.slice(0, firstDividerPos);
        } else {
            // No dividers, all are online
            originalOnlineServers = allCards.filter(el => 
                el.classList.contains('server-card') && 
                el.querySelector('.status-indicator.online')
            );
        }
    }
    
    // Extract server data from card
    function getServerData(card) {
        const playersText = card.querySelector('.stat-value')?.textContent || '0';
        const players = parseInt(playersText) || 0;
        
        // Calculate average uptime from all uptime days
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
    
    // Sort servers based on selected option
    function sortServers(sortType) {
        const allCards = Array.from(serverGrid.children);
        
        // Find dividers
        const offlineDivider = allCards.find(el => 
            el.classList.contains('wip-divider') && el.classList.contains('offline-divider')
        );
        const wipDivider = allCards.find(el => 
            el.classList.contains('wip-divider') && !el.classList.contains('offline-divider')
        );
        
        // Get current online servers
        let onlineCards = [];
        if (offlineDivider) {
            const offlineDividerPos = allCards.indexOf(offlineDivider);
            onlineCards = allCards.slice(0, offlineDividerPos);
        } else if (wipDivider) {
            const wipDividerPos = allCards.indexOf(wipDivider);
            onlineCards = allCards.slice(0, wipDividerPos);
        } else {
            onlineCards = allCards.filter(el => 
                el.classList.contains('server-card') && 
                el.querySelector('.status-indicator.online')
            );
        }
        
        // Get offline and WIP servers
        let offlineCards = [];
        let wipCards = [];
        
        if (offlineDivider && wipDivider) {
            const offlineDividerPos = allCards.indexOf(offlineDivider);
            const wipDividerPos = allCards.indexOf(wipDivider);
            offlineCards = allCards.slice(offlineDividerPos + 1, wipDividerPos);
            wipCards = allCards.slice(wipDividerPos + 1);
        } else if (offlineDivider) {
            const offlineDividerPos = allCards.indexOf(offlineDivider);
            offlineCards = allCards.slice(offlineDividerPos + 1);
        } else if (wipDivider) {
            const wipDividerPos = allCards.indexOf(wipDivider);
            wipCards = allCards.slice(wipDividerPos + 1);
        }
        
        // Sort only online servers
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
                // Fisher-Yates shuffle
                for (let i = sortedOnline.length - 1; i > 0; i--) {
                    const j = Math.floor(Math.random() * (i + 1));
                    [sortedOnline[i], sortedOnline[j]] = [sortedOnline[j], sortedOnline[i]];
                }
                break;
        }
        
        // Clear grid
        serverGrid.innerHTML = '';
        
        // Re-add sorted online servers
        sortedOnline.forEach(data => {
            serverGrid.appendChild(data.element);
        });
        
        // Re-add offline section if exists
        if (offlineCards.length > 0) {
            serverGrid.appendChild(offlineDivider);
            offlineCards.forEach(card => {
                serverGrid.appendChild(card);
            });
        }
        
        // Re-add WIP section if exists
        if (wipCards.length > 0) {
            serverGrid.appendChild(wipDivider);
            wipCards.forEach(card => {
                serverGrid.appendChild(card);
            });
        }
    }
    
    // Initialize on page load
    initializeServers();
    
    // Add event listener for sort changes
    if (sortSelect) {
        sortSelect.addEventListener('change', function() {
            sortServers(this.value);
        });
    }
});
