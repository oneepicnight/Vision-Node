//! Mining API endpoints
//!
//! Provides HTTP endpoints for mining status, leaderboard, and statistics.

use axum::{
    Router,
    routing::get,
    extract::State,
    Json,
    http::StatusCode,
};
use std::sync::{Arc, Mutex};
use serde_json::json;

use crate::miner::{MiningLeaderboard, ActiveMiner};

/// Shared mining state for API
pub struct MiningApiState {
    pub leaderboard: Arc<Mutex<MiningLeaderboard>>,
    pub miner: Arc<ActiveMiner>,
}

/// GET /mining/status
/// Returns current mining status
async fn get_mining_status(
    State(state): State<Arc<MiningApiState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let threads = state.miner.get_threads();
    let stats = state.miner.stats();
    let num_cores = num_cpus::get();
    let high_core_profile = num_cores >= 16;
    
    Ok(Json(json!({
        "threads": threads,
        "hashrate_hps": stats.current_hashrate,
        "epoch": 0, // TODO: Get from miner state
        "difficulty": 0, // TODO: Get from difficulty tracker
        "high_core_profile": high_core_profile,
        "total_hashes": 0, // TODO: Add total_hashes to MinerSpeed
        "enabled": threads > 0,
        "current_height": 0, // TODO: Get from chain state
    })))
}

/// GET /mining/leaderboard
/// Returns top miners by blocks found
async fn get_mining_leaderboard(
    State(state): State<Arc<MiningApiState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let leaderboard = state.leaderboard.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let top_miners = leaderboard.get_leaderboard(100);
    
    Ok(Json(json!({
        "miners": top_miners,
        "total": top_miners.len(),
    })))
}

/// GET /stats/miners
/// Alias for leaderboard with pool detection flags
async fn get_stats_miners(
    State(state): State<Arc<MiningApiState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    get_mining_leaderboard(State(state)).await
}

/// Create mining API router
pub fn mining_api_router(state: Arc<MiningApiState>) -> Router {
    Router::new()
        .route("/mining/status", get(get_mining_status))
        .route("/mining/leaderboard", get(get_mining_leaderboard))
        .route("/stats/miners", get(get_stats_miners))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pow::visionx::VisionXParams;
    use crate::consensus_pow::DifficultyConfig;
    
    #[tokio::test]
    async fn test_mining_api() {
        let leaderboard = Arc::new(Mutex::new(MiningLeaderboard::new(100)));
        
        let params = VisionXParams::default();
        let difficulty_config = DifficultyConfig {
            target_block_time: 2,
            adjustment_interval: 120,
            min_solve_divisor: 4,
            max_solve_multiplier: 10,
            max_change_up_percent: 110,
            max_change_down_percent: 90,
            max_adjustment_factor: 4.0,
            min_difficulty: 10000,
        };
        
        let miner = Arc::new(crate::miner::ActiveMiner::new_disabled(
            params,
            difficulty_config,
            10000,
            None,
        ));
        
        let state = Arc::new(MiningApiState {
            leaderboard,
            miner,
        });
        
        // Test status endpoint
        let result = get_mining_status(State(state.clone())).await;
        assert!(result.is_ok());
    }
}
