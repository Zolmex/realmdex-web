use leptos::prelude::*;

#[component]
pub fn SiteHeader() -> impl IntoView {
    view! {
        <header class="site-header" role="banner">
            <div id="h-realmdex-icon">
                <img src="/content/images/logo.webp" alt="RealmDex logo"/>
                <p>"RealmDex"</p>
            </div>
            <nav class="header-nav">
                <a href="https://discord.realmdex.com" target="_blank" rel="noopener noreferrer" class="discord-btn" aria-label="Join our Discord">
                    <svg width="20" height="15" viewBox="0 0 71 55" fill="currentColor" xmlns="http://www.w3.org/2000/svg">
                        <path d="M60.1 4.9A58.5 58.5 0 0 0 45.4.2a.2.2 0 0 0-.2.1 40.8 40.8 0 0 0-1.8 3.7 54 54 0 0 0-16.2 0A37.4 37.4 0 0 0 25.4.3a.2.2 0 0 0-.2-.1A58.4 58.4 0 0 0 10.5 4.9a.2.2 0 0 0-.1.1C1.5 18.7-.9 32.2.3 45.5v.2a58.9 58.9 0 0 0 17.7 9a.2.2 0 0 0 .3-.1 42.1 42.1 0 0 0 3.6-5.9.2.2 0 0 0-.1-.3 38.8 38.8 0 0 1-5.5-2.7.2.2 0 0 1 0-.4l1.1-.9a.2.2 0 0 1 .2 0 42 42 0 0 0 35.6 0 .2.2 0 0 1 .2 0l1.1.9a.2.2 0 0 1 0 .4 36.4 36.4 0 0 1-5.5 2.7.2.2 0 0 0-.1.3 47.2 47.2 0 0 0 3.6 5.9.2.2 0 0 0 .3.1A58.7 58.7 0 0 0 70.4 45.7v-.2c1.4-15-2.3-28.1-9.8-39.7a.2.2 0 0 0-.1 0ZM23.7 37.3c-3.4 0-6.3-3.2-6.3-7s2.8-7 6.3-7 6.3 3.1 6.3 7-2.8 7-6.3 7Zm23.2 0c-3.4 0-6.3-3.2-6.3-7s2.8-7 6.3-7 6.3 3.1 6.3 7-2.8 7-6.3 7Z"/>
                    </svg>
                    "Discord"
                </a>
            </nav>
        </header>
    }
}
