use leptos::prelude::*;

#[component]
pub fn SiteHeader() -> impl IntoView {
    view! {
        <header class="site-header" role="banner">
            <div id="h-realmdex-icon">
                <img src="/content/images/logo.webp" alt="RealmDex logo"/>
                <p>"RealmDex"</p>
            </div>
        </header>
    }
}
