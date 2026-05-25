use leptos::prelude::*;
use crate::types::{DailyUptime, ServerCardData, Status};
use crate::uptime::uptime_color;
use super::sparkline::Sparkline;

#[component]
pub fn ServerCard(data: ServerCardData) -> impl IntoView {
    let status_class = match data.status {
        Status::Online => "online",
        Status::Offline => "offline",
        Status::Wip => "wip",
    };
    let status_text = match data.status {
        Status::Online => "Online",
        Status::Offline => "Offline",
        Status::Wip => "WIP",
    };
    let is_wip = matches!(data.status, Status::Wip);
    let link_text = if data.link.contains("discord") { "Join Discord" } else { "Visit Homepage" };

    let week: Vec<DailyUptime> = data.uptime_14d.iter().take(7).cloned().collect();
    let two_week: Vec<DailyUptime> = data.uptime_14d.clone();

    let spark_points = RwSignal::new(data.sparkline.clone());

    let players_value = if is_wip { "-".to_string() } else { data.current_players.to_string() };
    let peak_value = if is_wip { "-".to_string() } else { data.peak_24h.to_string() };

    let id = data.id;
    let icon = data.icon_path.clone();
    let name = data.name.clone();
    let link = data.link.clone();

    view! {
        <div class="server-card" data-server-id=id>
            <div class="card-header">
                <img src=icon.clone() alt=name.clone() class="server-icon" data-discord=link.clone()/>
                <div class="server-info">
                    <h3 class="server-name">{name}</h3>
                    <a href=link class="server-discord" target="_blank" rel="noopener noreferrer">{link_text}</a>
                </div>
                <div class="status-container">
                    <div class=format!("status-indicator {status_class}") title=status_text></div>
                    <span class=format!("status-text {status_class}")>{status_text}</span>
                </div>
            </div>

            <div class="card-stats">
                <div class="stat-row">
                    <span class="stat-label">"Players"</span>
                    <span class="stat-value">{players_value}</span>
                </div>
                <div class="stat-row">
                    <span class="stat-label">"24h Peak"</span>
                    <span class="stat-value">{peak_value}</span>
                </div>
            </div>

            { (!is_wip).then(|| view! {
                <div class="sparkline-wrapper">
                    <Sparkline points=spark_points.into()/>
                </div>
                <div class="uptime-section">
                    <div class="uptime-wrapper">
                        <div class="uptime-labels">
                            <div class="uptime-label uptime-label-week">"Uptime (Past Week)"</div>
                            <div class="uptime-label uptime-label-2week">"Uptime (Past 2 Weeks)"</div>
                        </div>
                        <div class="uptime-grids">
                            <div class="uptime-week">{uptime_grid(&week)}</div>
                            <div class="uptime-2week">{uptime_grid(&two_week)}</div>
                        </div>
                    </div>
                </div>
            }) }
        </div>
    }
}

fn uptime_grid(days: &[DailyUptime]) -> impl IntoView {
    let mut cells = Vec::with_capacity(days.len());
    for (i, d) in days.iter().enumerate() {
        let color = uptime_color(d.uptime_percent);
        cells.push(view! {
            <div class="uptime-day"
                 style=format!("background-color: {color}")
                 data-uptime=d.uptime_percent
                 data-day=(i + 1) as i32></div>
        });
    }
    view! { <div class="uptime-grid">{cells}</div> }
}
