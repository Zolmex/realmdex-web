use leptos::prelude::*;

pub mod components;
pub mod types;
pub mod uptime;

#[cfg(feature = "ssr")]
pub mod db;

#[cfg(feature = "ssr")]
pub mod server_fns;

use crate::components::{server_grid::ServerGrid, site_header::SiteHeader};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <SiteHeader/>
        <ServerGrid/>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
