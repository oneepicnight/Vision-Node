use axum::{extract::State, http::StatusCode, Json};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::mining_endpoints::MiningEndpointConfig;

#[derive(Clone)]
pub struct AdminEndpointsState {
    pub mining_endpoints: Arc<RwLock<MiningEndpointConfig>>,
    pub db: sled::Db,
}

/// GET /admin/mining/endpoints - Get current mining endpoint configuration
pub async fn get_endpoints(State(state): State<AdminEndpointsState>) -> Json<MiningEndpointConfig> {
    let config = state.mining_endpoints.read().clone();
    Json(config)
}

#[derive(Deserialize)]
pub struct UpdateEndpointsRequest {
    pub public_pool_url: Option<String>,
    pub local_node_url: Option<String>,
    pub public_farm_base_url: Option<String>,
}

#[derive(Serialize)]
pub struct UpdateEndpointsResponse {
    pub ok: bool,
    pub message: String,
    pub config: MiningEndpointConfig,
}

/// POST /admin/mining/endpoints - Update mining endpoint configuration
pub async fn update_endpoints(
    State(state): State<AdminEndpointsState>,
    Json(req): Json<UpdateEndpointsRequest>,
) -> Result<Json<UpdateEndpointsResponse>, (StatusCode, String)> {
    // Update config
    let mut config = state.mining_endpoints.write();

    if let Some(public_url) = req.public_pool_url {
        if !public_url.is_empty() {
            config
                .set_public_pool_url(public_url)
                .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        } else {
            config.public_pool_url = None;
        }
    }

    if let Some(local_url) = req.local_node_url {
        if !local_url.is_empty() {
            config
                .set_local_node_url(local_url)
                .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        } else {
            config.local_node_url = None;
        }
    }

    if let Some(farm_url) = req.public_farm_base_url {
        if !farm_url.is_empty() {
            config
                .set_public_farm_base_url(farm_url)
                .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
        } else {
            config.public_farm_base_url = None;
        }
    }

    // Validate
    config
        .validate()
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Persist to database
    let json = config.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Serialization error: {}", e),
        )
    })?;

    state
        .db
        .insert("mining_endpoints", json.as_bytes())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    let response_config = config.clone();
    drop(config);

    tracing::info!("✅ Mining endpoints updated");

    Ok(Json(UpdateEndpointsResponse {
        ok: true,
        message: "Mining endpoints updated successfully".to_string(),
        config: response_config,
    }))
}
