// Approach: SSR-only with a small embedded vanilla-JS controller for tabs,
// sort, and 30s live updates. Reason: Leptos 0.8's idiomatic `Resource` +
// `#[server]` flow assumes server fn futures are `Send`, but worker-rs 0.8
// D1 futures are `!Send` (see server_fns.rs). Wiring up hydration without the
// macro is non-trivial; SSR renders the whole page with real data and a small
// script handles category switching, sort, and refetch from /api/list_servers.
use crate::types::{Category, InitialServers, ServerCardData, Status};
use leptos::prelude::*;
use super::server_card::ServerCard;

fn partition(data: &[ServerCardData]) -> (Vec<ServerCardData>, Vec<ServerCardData>, Vec<ServerCardData>) {
    let mut online = Vec::new();
    let mut offline = Vec::new();
    let mut wip = Vec::new();
    for s in data {
        match s.status {
            Status::Online => online.push(s.clone()),
            Status::Offline => offline.push(s.clone()),
            Status::Wip => wip.push(s.clone()),
        }
    }
    // default sort: players desc
    online.sort_by(|a, b| b.current_players.cmp(&a.current_players));
    (online, offline, wip)
}

fn render_category(cat: Category, data: &[ServerCardData]) -> impl IntoView {
    let (online, offline, wip) = partition(data);
    let cat_str = cat.as_db_str();
    let has_offline = !offline.is_empty();
    let has_wip = !wip.is_empty();

    let online_cards: Vec<_> = online.into_iter().map(|s| view! { <ServerCard data=s/> }).collect();
    let offline_cards: Vec<_> = offline.into_iter().map(|s| view! { <ServerCard data=s/> }).collect();
    let wip_cards: Vec<_> = wip.into_iter().map(|s| view! { <ServerCard data=s/> }).collect();

    view! {
        <div class="server-grid" data-category=cat_str>
            {online_cards}
            { has_offline.then(|| view! {
                <div class="wip-divider offline-divider"><span>"Offline"</span></div>
            }) }
            {offline_cards}
            { has_wip.then(|| view! {
                <div class="wip-divider"><span>"Work in Progress"</span></div>
            }) }
            {wip_cards}
        </div>
    }
}

#[component]
pub fn ServerGrid() -> impl IntoView {
    // Read initial data from context (provided by the worker fetch handler).
    // Falls back to an empty struct if missing (shouldn't happen in normal flow).
    let initial = use_context::<InitialServers>().unwrap_or_default();
    let pserver_view = render_category(Category::Pserver, &initial.pserver);

    view! {
        <main class="server-grid-container" role="main">
            <div class="category-tabs">
                <button class="category-tab active" data-category="pserver">"Private Servers"</button>
                <button class="category-tab" data-category="realm-like">"Realm-Likes"</button>
            </div>
            <div class="controls-bar">
                <label for="sort-select" class="sort-label">"Sort by:"</label>
                <select id="sort-select" class="sort-select">
                    <option value="players-desc">"Players (High to Low)"</option>
                    <option value="players-asc">"Players (Low to High)"</option>
                    <option value="uptime-desc">"Uptime (High to Low)"</option>
                    <option value="random">"Random"</option>
                </select>
            </div>
            {pserver_view}
        </main>
    }
}
