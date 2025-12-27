#![cfg(feature = "farm")]
#![allow(dead_code)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "farm")]
use crate::farm::profile::{FarmProfileConfig, FarmSchedule, RigConfig};
#[cfg(feature = "farm")]
use crate::farm::{FarmCommand, FarmRig, FarmState, FarmStats};

#[derive(Clone)]
pub struct AdminFarmState {
    pub farm_state: Arc<RwLock<FarmState>>,
    pub db: sled::Db,
}

/// GET /admin/farm/rigs - Get list of all farm rigs
pub async fn get_rigs(State(state): State<AdminFarmState>) -> Json<RigsResponse> {
    let farm = state.farm_state.read();
    let rigs = farm.get_all_rigs();
    let stats = farm.get_stats();

    Json(RigsResponse {
        ok: true,
        rigs,
        stats,
    })
}

#[derive(Serialize)]
pub struct RigsResponse {
    pub ok: bool,
    pub rigs: Vec<FarmRig>,
    pub stats: FarmStats,
}

/// Compatibility wrapper: list_rigs -> get_rigs
pub async fn list_rigs(State(state): State<AdminFarmState>) -> Json<RigsResponse> {
    get_rigs(State(state)).await
}

/// POST /admin/farm/rigs/{rig_id}/start - Start mining on a rig
pub async fn start_rig(
    Path((rig_id,)): Path<(String,)>,
    State(state): State<AdminFarmState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let tx_opt = {
        let farm = state.farm_state.read();
        farm.commands.get(&rig_id).cloned()
    };

    let result = if let Some(tx) = tx_opt {
        tx.send(FarmCommand::StartMining)
            .await
            .map_err(|e| format!("Failed to send command: {}", e))
    } else {
        Err(format!("Rig {} not found", rig_id))
    };

    match result {
        Ok(_) => {
            tracing::info!("▶️  Started mining on rig: {}", rig_id);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "message": format!("Started mining on rig {}", rig_id)
                })),
            )
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

/// POST /admin/farm/rigs/{rig_id}/stop - Stop mining on a rig
pub async fn stop_rig(
    Path((rig_id,)): Path<(String,)>,
    State(state): State<AdminFarmState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let tx_opt = {
        let farm = state.farm_state.read();
        farm.commands.get(&rig_id).cloned()
    };

    let result = if let Some(tx) = tx_opt {
        tx.send(FarmCommand::StopMining)
            .await
            .map_err(|e| format!("Failed to send command: {}", e))
    } else {
        Err(format!("Rig {} not found", rig_id))
    };

    match result {
        Ok(_) => {
            tracing::info!("⏸️  Stopped mining on rig: {}", rig_id);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "message": format!("Stopped mining on rig {}", rig_id)
                })),
            )
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub ok: bool,
    pub message: String,
}

/// GET /admin/farm/rigs/{rig_id}/config - Get rig configuration
pub async fn get_rig_config(
    Path((rig_id,)): Path<(String,)>,
    State(state): State<AdminFarmState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let config_key = format!("farm_rig_config/{}", rig_id);

    match state.db.get(&config_key) {
        Ok(Some(bytes)) => match serde_json::from_slice::<RigConfig>(&bytes) {
            Ok(config) => (StatusCode::OK, Json(serde_json::to_value(config).unwrap())),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Parse error: {}", e)
                })),
            ),
        },
        Ok(None) => {
            // Return default config
            let config = RigConfig::new(rig_id);
            (StatusCode::OK, Json(serde_json::to_value(config).unwrap()))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            })),
        ),
    }
}

#[derive(Deserialize)]
pub struct UpdateRigConfigRequest {
    pub profile: Option<FarmProfileConfig>,
    pub schedule: Option<FarmSchedule>,
    pub auto_restart_on_error: Option<bool>,
    pub min_hashrate_threshold: Option<f64>,
}

/// POST /admin/farm/rigs/{rig_id}/config - Update rig configuration
pub async fn update_rig_config(
    Path((rig_id,)): Path<(String,)>,
    State(state): State<AdminFarmState>,
    Json(req): Json<UpdateRigConfigRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Load existing config or create new
    let config_key = format!("farm_rig_config/{}", rig_id);
    let mut config = match state.db.get(&config_key) {
        Ok(Some(bytes)) => serde_json::from_slice::<RigConfig>(&bytes)
            .unwrap_or_else(|_| RigConfig::new(rig_id.clone())),
        _ => RigConfig::new(rig_id.clone()),
    };

    // Update fields
    if let Some(profile) = req.profile {
        config.profile = Some(profile);
    }
    if let Some(schedule) = req.schedule {
        config.schedule = Some(schedule);
    }
    if let Some(auto_restart) = req.auto_restart_on_error {
        config.auto_restart_on_error = auto_restart;
    }
    if let Some(threshold) = req.min_hashrate_threshold {
        config.min_hashrate_threshold = Some(threshold);
    }

    // Persist to database
    let json = match config.to_json() {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Serialization error: {}", e)
                })),
            )
        }
    };

    if let Err(e) = state.db.insert(&config_key, json.as_bytes()) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Database error: {}", e)
            })),
        );
    }

    tracing::info!("⚙️  Updated config for rig: {}", rig_id);

    // If profile was updated, apply it immediately
    if let Some(ref profile) = config.profile {
        let tx_opt = {
            let farm = state.farm_state.read();
            farm.commands.get(&rig_id).cloned()
        };

        let result = if let Some(tx) = tx_opt {
            tx.send(FarmCommand::ApplyProfile {
                config: profile.clone(),
            })
            .await
            .map_err(|e| format!("Failed to send command: {}", e))
        } else {
            Err(format!("Rig {} not found", rig_id))
        };
        if let Err(e) = result {
            tracing::warn!("Failed to apply profile to rig {}: {}", rig_id, e);
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "message": "Rig configuration updated successfully",
            "config": config
        })),
    )
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub ok: bool,
    pub message: String,
    pub config: RigConfig,
}
