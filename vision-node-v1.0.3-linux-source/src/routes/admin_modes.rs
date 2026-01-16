#![allow(dead_code)]

use axum::{extract::State, http::StatusCode, Json};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::runtime_mode::RuntimeModeConfig;

#[derive(Clone)]
pub struct AdminModesState {
    pub mode_config: Arc<RwLock<RuntimeModeConfig>>,
    pub mode_notify: Arc<tokio::sync::Notify>,
    pub db: sled::Db,
}

/// GET /admin/modes - Get current runtime mode configuration
pub async fn get_modes(State(state): State<AdminModesState>) -> Json<RuntimeModeConfig> {
    let config = state.mode_config.read().clone();
    Json(config)
}

#[derive(Deserialize)]
pub struct EmptyBody {}

/// POST /admin/modes/solo/enable - Enable solo mining mode
pub async fn enable_solo(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(&state, |config| config.solo_enabled = true, "solo enabled").await
}

/// POST /admin/modes/solo/disable - Disable solo mining mode
pub async fn disable_solo(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(
        &state,
        |config| config.solo_enabled = false,
        "solo disabled",
    )
    .await
}

/// POST /admin/modes/pool/enable - Enable pool hosting mode
pub async fn enable_pool(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(&state, |config| config.pool_enabled = true, "pool enabled").await
}

/// POST /admin/modes/pool/disable - Disable pool hosting mode
pub async fn disable_pool(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(
        &state,
        |config| config.pool_enabled = false,
        "pool disabled",
    )
    .await
}

/// POST /admin/modes/farm/enable - Enable farm controller mode
pub async fn enable_farm(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(&state, |config| config.farm_enabled = true, "farm enabled").await
}

/// POST /admin/modes/farm/disable - Disable farm controller mode
pub async fn disable_farm(
    State(state): State<AdminModesState>,
    Json(_): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    update_mode(
        &state,
        |config| config.farm_enabled = false,
        "farm disabled",
    )
    .await
}

#[derive(Serialize)]
pub struct ModeResponse {
    pub ok: bool,
    pub message: String,
    pub config: RuntimeModeConfig,
}

/// Helper to update mode configuration
async fn update_mode<F>(
    state: &AdminModesState,
    update_fn: F,
    message: &str,
) -> Result<Json<ModeResponse>, (StatusCode, String)>
where
    F: FnOnce(&mut RuntimeModeConfig),
{
    // Update config
    let mut config = state.mode_config.write();
    update_fn(&mut config);

    // Validate
    if let Err(e) = config.validate() {
        return Err((StatusCode::BAD_REQUEST, e));
    }

    // Persist to database
    let json = config.to_json().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Serialization error: {}", e),
        )
    })?;

    state
        .db
        .insert("runtime_mode_config", json.as_bytes())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
        })?;

    let response_config = config.clone();
    drop(config);

    // Notify mode watcher
    state.mode_notify.notify_waiters();

    tracing::info!("✅ Mode updated: {}", message);

    Ok(Json(ModeResponse {
        ok: true,
        message: message.to_string(),
        config: response_config,
    }))
}

// ============================================================================
// BACKWARD COMPATIBILITY WRAPPERS (for old function names used in main.rs)
// ============================================================================

#[derive(Serialize)]
pub struct ModeStatusResponse {
    pub ok: bool,
    pub config: RuntimeModeConfig,
}

/// Compatibility wrapper: get_mode_status -> get_modes
pub async fn get_mode_status(State(state): State<AdminModesState>) -> Json<ModeStatusResponse> {
    let config = get_modes(State(state)).await.0;
    Json(ModeStatusResponse { ok: true, config })
}

/// Compatibility wrapper: enable_mode -> enable_pool
pub async fn enable_mode(
    State(state): State<AdminModesState>,
    Json(body): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    enable_pool(State(state), Json(body)).await
}

/// Compatibility wrapper: disable_mode -> disable_pool
pub async fn disable_mode(
    State(state): State<AdminModesState>,
    Json(body): Json<EmptyBody>,
) -> Result<Json<ModeResponse>, (StatusCode, String)> {
    disable_pool(State(state), Json(body)).await
}
