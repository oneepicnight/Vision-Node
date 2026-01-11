#![allow(dead_code)]

#[cfg(feature = "staging")]
pub mod discord_oauth;

#[cfg(feature = "staging")]
pub mod node_approval_api;

pub mod external_rpc_api;
pub mod peers_api;
pub mod routing_api;
pub mod security;
pub mod snapshot;
pub mod upstream;
pub mod vault_routes;
pub mod website_api;

use tracing::info;

// Minimal public re-exports used by other modules
pub use discord_oauth::init_discord_links_db;
mod discord_oauth;

pub fn log_upstream_mode() {
    use std::env;

    if let Ok(base) = env::var("VISION_UPSTREAM_HTTP_BASE") {
        info!("[UPSTREAM] HTTP upstream enabled: {base}");
    } else {
        info!("[UPSTREAM] No HTTP upstream configured (local-only mode).");
    }
}
