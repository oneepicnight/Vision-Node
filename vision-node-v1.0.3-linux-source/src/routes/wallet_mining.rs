use axum::{extract::State, http::StatusCode, Json};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::mining_endpoints::MiningEndpointConfig;
use crate::runtime_mode::RuntimeModeConfig;

#[derive(Clone)]
pub struct WalletMiningState {
    pub mode_config: Arc<RwLock<RuntimeModeConfig>>,
    pub mining_endpoints: Arc<RwLock<MiningEndpointConfig>>,
}

#[derive(Deserialize)]
pub struct JoinPoolRequest {
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct JoinPoolResponse {
    pub ok: bool,
    pub pool_url: String,
    pub wallet_address: String,
    pub recommended_threads: usize,
    pub command_line: String,
}

/// POST /wallet/mining/join-pool - Get information to join the pool
pub async fn join_pool(
    State(state): State<WalletMiningState>,
    Json(req): Json<JoinPoolRequest>,
) -> Result<Json<JoinPoolResponse>, (StatusCode, String)> {
    // Check if pool is enabled
    let mode_config = state.mode_config.read();
    if !mode_config.pool_enabled {
        return Err((
            StatusCode::BAD_REQUEST,
            "Pool is not enabled on this node".to_string(),
        ));
    }
    drop(mode_config);

    // Get public pool URL
    let endpoints = state.mining_endpoints.read();
    let pool_url = endpoints
        .get_public_pool_url()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Pool URL not configured".to_string(),
            )
        })?
        .to_string();
    drop(endpoints);

    // Validate wallet address (basic check)
    if req.wallet_address.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Wallet address is required".to_string(),
        ));
    }

    // Calculate recommended threads (leave 2 for system)
    let recommended_threads = num_cpus::get().saturating_sub(2).max(1);

    // Generate command line
    let command_line = format!(
        "vision-miner --pool {} --wallet {} --threads {}",
        pool_url, req.wallet_address, recommended_threads
    );

    tracing::info!(
        "📋 Generated pool join info for wallet: {}",
        req.wallet_address
    );

    Ok(Json(JoinPoolResponse {
        ok: true,
        pool_url,
        wallet_address: req.wallet_address,
        recommended_threads,
        command_line,
    }))
}
