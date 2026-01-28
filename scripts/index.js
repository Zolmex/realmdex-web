$(document).ready(function() {
    // Sample data - replace with your PHP/AJAX call
    const servers = [
        {
            id: 1,
            name: "Server Alpha",
            isOnline: true,
            icon: "/content/images/server1.webp",
            discordUrl: "https://discord.gg/server1",
            currentPlayers: 45,
            maxPlayers: 100,
            max24h: 78,
            uptimeWeek: [100, 98, 100, 95, 100, 88, 100], // Last 7 days
            uptimeExtended: [92, 88, 95, 100, 100, 98, 100, 95, 100, 88, 100, 90, 85, 100] // Last 14 days
        },
        {
            id: 2,
            name: "Server Beta",
            isOnline: false,
            icon: "/content/images/server2.webp",
            discordUrl: "https://discord.gg/server2",
            currentPlayers: 0,
            maxPlayers: 50,
            max24h: 32,
            uptimeWeek: [100, 100, 85, 90, 0, 0, 0],
            uptimeExtended: [100, 95, 88, 100, 100, 85, 90, 0, 0, 0, 75, 80, 90, 95]
        }
    ];

    function getUptimeColor(percentage) {
        if (percentage >= 75) {
            // Green range
            const greenIntensity = Math.floor(((percentage - 75) / 25) * 255);
            return `rgb(${255 - greenIntensity}, 255, 0)`;
        } else if (percentage >= 50) {
            // Yellow to green range
            const ratio = (percentage - 50) / 25;
            return `rgb(255, ${255}, ${Math.floor(ratio * 255)})`;
        } else if (percentage > 0) {
            // Red to yellow range
            const ratio = percentage / 50;
            return `rgb(255, ${Math.floor(ratio * 255)}, 0)`;
        } else {
            // Pure red for 0%
            return 'rgb(255, 0, 0)';
        }
    }

    function createUptimeGrid(uptimeData, extended = false) {
        let html = '<div class="uptime-grid">';
        uptimeData.forEach((uptime, index) => {
            const color = getUptimeColor(uptime);
            html += `<div class="uptime-day" style="background-color: ${color}" data-uptime="${uptime}" data-day="${index + 1}"></div>`;
        });
        html += '</div>';
        return html;
    }

    function createServerCard(server) {
        const statusClass = server.isOnline ? 'online' : 'offline';
        const statusText = server.isOnline ? 'Online' : 'Offline';
        
        return `
            <div class="server-card" data-server-id="${server.id}">
                <div class="card-header">
                    <div class="status-indicator ${statusClass}" title="${statusText}"></div>
                    <img src="${server.icon}" alt="${server.name}" class="server-icon" data-discord="${server.discordUrl}">
                    <h3 class="server-name">${server.name}</h3>
                </div>
                
                <div class="card-stats">
                    <div class="stat-row">
                        <span class="stat-label">Players</span>
                        <span class="stat-value">${server.currentPlayers} / ${server.maxPlayers}</span>
                    </div>
                    <div class="stat-row">
                        <span class="stat-label">24h Peak</span>
                        <span class="stat-value">${server.max24h}</span>
                    </div>
                </div>
                
                <div class="uptime-section">
                    <div class="uptime-label">Uptime (Past Week)</div>
                    ${createUptimeGrid(server.uptimeWeek)}
                    <div class="uptime-extended">
                        <div class="uptime-label">Uptime (Past 2 Weeks)</div>
                        ${createUptimeGrid(server.uptimeExtended, true)}
                    </div>
                </div>
            </div>
        `;
    }

    function renderServers() {
        const $grid = $('#server-grid');
        $grid.empty();
        
        servers.forEach(server => {
            $grid.append(createServerCard(server));
        });
    }

    // Event Handlers
    $(document).on('click', '.server-icon', function() {
        const discordUrl = $(this).data('discord');
        window.open(discordUrl, '_blank');
    });

    // Uptime tooltip
    let tooltipTimeout;
    $(document).on('mouseenter', '.uptime-day', function(e) {
        const uptime = $(this).data('uptime');
        const day = $(this).data('day');
        const $tooltip = $('<div class="uptime-tooltip show"></div>');
        $tooltip.text(`Day ${day}: ${uptime}% uptime`);
        
        const rect = this.getBoundingClientRect();
        $tooltip.css({
            left: rect.left + (rect.width / 2) + 'px',
            top: (rect.top - 35) + 'px',
            transform: 'translateX(-50%)'
        });
        
        $('body').append($tooltip);
    });

    $(document).on('mouseleave', '.uptime-day', function() {
        $('.uptime-tooltip').remove();
    });

    // Show extended uptime on card hover
    $(document).on('mouseenter', '.server-card', function() {
        clearTimeout(tooltipTimeout);
        const $extended = $(this).find('.uptime-extended');
        tooltipTimeout = setTimeout(() => {
            $extended.slideDown(200);
        }, 500); // Show after 500ms hover
    });

    $(document).on('mouseleave', '.server-card', function() {
        clearTimeout(tooltipTimeout);
        $(this).find('.uptime-extended').slideUp(200);
    });

    renderServers();

    // Auto-refresh every 30 seconds
    setInterval(fetchServers, 30000);
});

function fetchServers() {
    $.ajax({
        url: '/api/servers.php',
        method: 'GET',
        dataType: 'json',
        success: function(data) {
            servers = data;
            renderServers();
        },
        error: function(error) {
            console.error('Error fetching servers:', error);
        }
    });
}