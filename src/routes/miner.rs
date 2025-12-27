//! Miner control and monitoring routes
//!
//! Provides HTTP endpoints for:
//! - GET /miner/config - Get current thread configuration
//! - POST /miner/config - Update thread count
//! - GET /miner/speed - Get current hashrate and history
//! - GET /miner/stats - Get blocks found, rewards, etc.

use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::miner::ActiveMiner;
use crate::miner_manager::MinerSpeed;

/// Shared miner state
#[derive(Clone)]
pub struct MinerState {
    pub miner: Arc<ActiveMiner>,
}

/// Request body for updating miner config
#[derive(Debug, Deserialize)]
pub struct MinerConfigUpdate {
    pub threads: Option<usize>,
    pub mining_profile: Option<String>,
    pub mining_threads: Option<usize>,
    pub simd_batch_size: Option<u64>,
}

/// Response with max_threads info
#[derive(Debug, Serialize)]
pub struct MinerConfigResponse {
    pub threads: usize,
    pub enabled: bool,
    pub max_threads: usize,
    pub mining_profile: Option<String>, // "laptop", "balanced", "beast"
    pub mining_threads: Option<usize>,  // 0 or null = auto
    pub simd_batch_size: Option<u64>,   // 1-1024, default 4
}

/// GET /miner/config - Get current configuration
pub async fn get_miner_config(State(state): State<MinerState>) -> Json<MinerConfigResponse> {
    let threads = state.miner.get_threads();
    let enabled = state.miner.is_enabled();

    // Load performance tuning fields from config
    let config =
        crate::config::miner::MinerConfig::load_or_create("miner.json").unwrap_or_default();

    Json(MinerConfigResponse {
        threads,
        enabled,
        max_threads: num_cpus::get() * 2,
        mining_profile: config.mining_profile,
        mining_threads: config.mining_threads,
        simd_batch_size: config.simd_batch_size,
    })
}

/// POST /miner/config - Update thread count and performance settings
pub async fn set_miner_config(
    State(state): State<MinerState>,
    Json(req): Json<MinerConfigUpdate>,
) -> Result<Json<MinerConfigResponse>, StatusCode> {
    // Load existing config
    let mut config =
        crate::config::miner::MinerConfig::load_or_create("miner.json").unwrap_or_default();

    // Update fields if provided
    if let Some(threads) = req.threads {
        state.miner.set_threads(threads);
    }
    if let Some(profile) = req.mining_profile {
        config.mining_profile = Some(profile);
    }
    if let Some(threads) = req.mining_threads {
        config.mining_threads = Some(threads);
    }
    if let Some(batch_size) = req.simd_batch_size {
        config.simd_batch_size = Some(batch_size);
    }

    // Save updated config
    if let Err(e) = config.save("miner.json") {
        eprintln!("Failed to save miner config: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let threads = state.miner.get_threads();
    let enabled = state.miner.is_enabled();
    Ok(Json(MinerConfigResponse {
        threads,
        enabled,
        max_threads: num_cpus::get() * 2,
        mining_profile: config.mining_profile,
        mining_threads: config.mining_threads,
        simd_batch_size: config.simd_batch_size,
    }))
}

/// GET /miner/speed - Get current hashrate and history
pub async fn get_miner_speed(State(state): State<MinerState>) -> Json<MinerSpeed> {
    Json(state.miner.stats())
}

/// Mining statistics response
#[derive(Debug, Serialize)]
pub struct MiningStatsResponse {
    pub blocks_found: u64,
    pub blocks_accepted: u64,
    pub blocks_rejected: u64,
    pub last_block_time: Option<u64>,
    pub last_block_height: Option<u64>,
    pub total_rewards: u64,
    pub average_block_time: Option<f64>,
}

/// GET /miner/stats - Get mining statistics (blocks, rewards)
pub async fn get_miner_stats(State(state): State<MinerState>) -> Json<MiningStatsResponse> {
    let stats = state.miner.get_stats();
    Json(stats)
}

/// Create miner router with all endpoints
pub fn miner_router(state: MinerState) -> Router {
    Router::new()
        .route(
            "/miner/config",
            get(get_miner_config).post(set_miner_config),
        )
        .route("/miner/speed", get(get_miner_speed))
        .route("/miner/stats", get(get_miner_stats))
        // NOTE: /mining/status is in the main API router (comprehensive implementation)
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::miner_manager::MinerManager;

    #[tokio::test]
    async fn test_get_miner_config() {
        let manager = Arc::new(MinerManager::new());
        let config = manager.get_config();

        assert!(config.threads > 0);
        assert_eq!(config.enabled, true);
    }

    #[tokio::test]
    async fn test_set_threads() {
        let manager = Arc::new(MinerManager::new());
        manager.set_threads(4);

        assert_eq!(manager.get_threads(), 4);
    }

    #[tokio::test]
    async fn test_get_speed() {
        let manager = Arc::new(MinerManager::new());
        let stats = manager.stats();

        assert_eq!(stats.threads, manager.get_threads());
        assert_eq!(stats.history.len(), 120);
    }
}
