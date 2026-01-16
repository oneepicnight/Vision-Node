//! Bootstrap endpoint - Bootstrap configuration and version info
//!
//! Provides bootstrap information including recommended version,
//! bootstrap peers, and network configuration.

use axum::response::{IntoResponse, Json};
use serde::Serialize;

/// Bootstrap configuration response
#[derive(Debug, Serialize)]
pub struct BootstrapInfo {
    pub recommended_version: String,
    pub welcome_message: String,
    pub bootstrap_peers: Vec<String>,
    pub testnet_phase: String,
    pub network_id: String,
}

/// GET /api/bootstrap
///
/// Returns bootstrap configuration for new nodes including
/// recommended version and bootstrap peer list.
pub async fn get_bootstrap() -> impl IntoResponse {
    // Get recommended version (current version)
    let recommended_version = crate::vision_constants::VISION_VERSION.to_string();
    
    // Get bootstrap peers from environment or use defaults
    let bootstrap_peers = get_bootstrap_peers();
    
    // Legacy field retained for compatibility; canonical chain has no testnet phases.
    let testnet_phase = "active".to_string();
    
    Json(BootstrapInfo {
        recommended_version,
        welcome_message: "Welcome to Vision World".to_string(),
        bootstrap_peers,
        testnet_phase,
        network_id: crate::vision_constants::VISION_NETWORK_ID.to_string(),
    })
}

/// Get bootstrap peers from configuration
fn get_bootstrap_peers() -> Vec<String> {
    // Try to get from environment first
    if let Ok(peers_str) = std::env::var("BOOTSTRAP_PEERS") {
        return peers_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    
    // Use hardcoded defaults (Guardian nodes)
    vec![
        "vision://VNODE-GUARDIAN-1@visionworld.tech:7070".to_string(),
        "vision://VNODE-GUARDIAN-2@backup.visionworld.tech:7070".to_string(),
    ]
}
