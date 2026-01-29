// Event Handlers
$(document).on('click', '.server-icon', function () {
    const discordUrl = $(this).data('discord');
    window.open(discordUrl, '_blank');
});

// Uptime tooltip
let tooltipTimeout;
$(document).on('mouseenter', '.uptime-day', function (e) {
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

$(document).on('mouseleave', '.uptime-day', function () {
    $('.uptime-tooltip').remove();
});

// Show extended uptime on card hover
$(document).on('mouseenter', '.server-card', function () {
    clearTimeout(tooltipTimeout);
    const $extended = $(this).find('.uptime-extended');
    tooltipTimeout = setTimeout(() => {
        $extended.slideDown(200);
    }, 500); // Show after 500ms hover
});

$(document).on('mouseleave', '.server-card', function () {
    clearTimeout(tooltipTimeout);
    $(this).find('.uptime-extended').slideUp(200);
});