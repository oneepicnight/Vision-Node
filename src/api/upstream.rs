//! Guardian HTTP Upstream Module (DEPRECATED - NO LONGER USED)
//!
//! **REMOVED**: Guardian no longer syncs chain state from website HTTP.
//!
//! **Guardian Role**: Read-only observer (Discord, mood, consciousness, logging)
//! **Chain Authority**: P2P consensus only - no HTTP chain oracle
//! **Beacon Role**: Peer discovery only (not chain authority)
//!
//! The website /api/upstream/* routes should rotate through constellation peer APIs,
//! not provide special "trusted" chain state to Guardian nodes.
//!
//! This module is kept for backward compatibility but all functions are neutered.

// Re-export response types for compatibility
use super::website_api::{ChainStatusResponse, HealthPublicResponse, StatusResponse};

// =============================================================================
// ALL UPSTREAM CHAIN-SYNC FUNCTIONS REMOVED
// Guardian is an observer, not a chain oracle consumer
// =============================================================================

/// DEPRECATED: No longer fetches from upstream
pub async fn upstream_status() -> Option<StatusResponse> {
    None // Guardian uses local chain state only
}

/// DEPRECATED: No longer fetches from upstream
pub async fn upstream_chain_status() -> Option<ChainStatusResponse> {
    None // Guardian uses local chain state only
}

/// DEPRECATED: No longer fetches from upstream
pub async fn upstream_constellation() -> Option<serde_json::Value> {
    None // Guardian uses local chain state only
}

/// DEPRECATED: No longer fetches from upstream
pub async fn upstream_health_public() -> Option<HealthPublicResponse> {
    None // Guardian uses local chain state only
}

/// DEPRECATED: No longer fetches from upstream
pub async fn upstream_mood() -> Option<serde_json::Value> {
    None // Guardian uses local chain state only
}

/// DEPRECATED: Kept for backward compatibility only
/// Logs warning if VISION_UPSTREAM_HTTP_BASE is still set
pub fn log_upstream_mode() {
    if let Ok(base) = std::env::var("VISION_UPSTREAM_HTTP_BASE") {
        tracing::warn!(
            "[UPSTREAM] VISION_UPSTREAM_HTTP_BASE is set ({}) but ignored - \
             Guardian no longer syncs chain state via HTTP. \
             Use beacon for peer discovery only.",
            base
        );
    }
}
