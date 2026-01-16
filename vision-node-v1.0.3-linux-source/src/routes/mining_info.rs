use axum::{extract::State, Json};
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;

use crate::config::mining_endpoints::MiningEndpointConfig;
use crate::runtime_mode::RuntimeModeConfig;

#[derive(Clone)]
pub struct MiningInfoState {
    pub mode_config: Arc<RwLock<RuntimeModeConfig>>,
    pub mining_endpoints: Arc<RwLock<MiningEndpointConfig>>,
}

#[derive(Serialize)]
pub struct MiningInfo {
    pub solo_enabled: bool,
    pub pool_enabled: bool,
    pub farm_enabled: bool,
    pub public_pool_url: Option<String>,
    pub local_node_url: Option<String>,
    pub public_farm_base_url: Option<String>,
    pub local_farm_ws_url: Option<String>,
    pub public_farm_ws_url: Option<String>,
}

/// GET /mining/info - Public endpoint to get mining information
pub async fn get_mining_info(State(state): State<MiningInfoState>) -> Json<MiningInfo> {
    let mode_config = state.mode_config.read();
    let endpoints = state.mining_endpoints.read();

    Json(MiningInfo {
        solo_enabled: mode_config.solo_enabled,
        pool_enabled: mode_config.pool_enabled,
        farm_enabled: mode_config.farm_enabled,
        public_pool_url: endpoints.public_pool_url.clone(),
        local_node_url: endpoints.local_node_url.clone(),
        public_farm_base_url: endpoints.public_farm_base_url.clone(),
        local_farm_ws_url: endpoints.local_farm_ws_url(),
        public_farm_ws_url: endpoints.public_farm_ws_url(),
    })
}
