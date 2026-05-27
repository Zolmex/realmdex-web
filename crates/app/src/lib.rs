use leptos::prelude::*;

pub mod components;
pub mod types;
pub mod uptime;
pub mod validation;

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

