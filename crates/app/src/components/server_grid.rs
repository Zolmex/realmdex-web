use leptos::prelude::*;

#[component]
pub fn ServerGrid() -> impl IntoView {
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
            <div class="server-grid" data-category="pserver"></div>
        </main>
    }
}
