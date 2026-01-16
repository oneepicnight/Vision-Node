#![cfg(feature = "farm")]
#![allow(dead_code)]

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::Response,
    Json,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use uuid::Uuid;
use zip::write::{FileOptions, ZipWriter};

use crate::config::mining_endpoints::MiningEndpointConfig;
#[cfg(feature = "farm")]
use crate::farm::farmhand_config::{FarmHandConfig, FarmHandEndpointMode};
#[cfg(feature = "farm")]
use crate::farm::{FarmRig, FarmState};

#[derive(Clone)]
pub struct FarmHandState {
    pub mining_endpoints: Arc<RwLock<MiningEndpointConfig>>,
    pub farm_state: Arc<RwLock<FarmState>>,
    pub db: sled::Db,
}

#[derive(Deserialize)]
pub struct CreateFarmHandRequest {
    pub rig_name: String,
    pub wallet_address: Option<String>,
    pub connection_mode: String, // "local" or "public"
    pub default_threads: Option<u32>,
}

#[derive(Serialize)]
pub struct CreateFarmHandResponse {
    pub ok: bool,
    pub error: Option<String>,
}

/// POST /admin/farm/farmhand/create - Create new FarmHand bundle
pub async fn create_farmhand(
    State(state): State<FarmHandState>,
    Json(req): Json<CreateFarmHandRequest>,
) -> Result<Response, (StatusCode, Json<CreateFarmHandResponse>)> {
    // Validate rig name
    if req.rig_name.is_empty() || req.rig_name.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(CreateFarmHandResponse {
                ok: false,
                error: Some("Invalid rig name (1-64 chars)".to_string()),
            }),
        ));
    }

    // Validate connection mode
    let endpoint_mode = match req.connection_mode.as_str() {
        "local" => FarmHandEndpointMode::Local,
        "public" => FarmHandEndpointMode::Public,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(
                        "Invalid connection_mode (must be 'local' or 'public')".to_string(),
                    ),
                }),
            ));
        }
    };

    // Load mining endpoints
    let endpoints = state.mining_endpoints.read().clone();

    // Determine WebSocket URL based on connection mode
    let controller_ws_url = match endpoint_mode {
        FarmHandEndpointMode::Local => endpoints.local_farm_ws_url().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(
                        "Local node URL not configured. Please set it in Mining Endpoints."
                            .to_string(),
                    ),
                }),
            )
        })?,
        FarmHandEndpointMode::Public => endpoints.public_farm_ws_url().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(
                        "Public farm URL not configured. Please set it in Mining Endpoints."
                            .to_string(),
                    ),
                }),
            )
        })?,
    };

    // Generate rig ID and auth token
    let rig_id = format!(
        "RIG-{}",
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap()
            .to_uppercase()
    );
    let auth_token = Uuid::new_v4().to_string().replace('-', "");

    // Create FarmRig with "pending" status
    let farm_rig = FarmRig {
        rig_id: rig_id.clone(),
        name: req.rig_name.clone(),
        os: "Unknown".to_string(),
        cpu_threads: req.default_threads.unwrap_or(0),
        status: "pending".to_string(),
        hashrate: 0.0,
        last_heartbeat: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        profile: None,
        endpoint_mode: Some(req.connection_mode.clone()),
    };

    // Save auth token to database
    let auth_key = format!("farm/rig_auth/{}", rig_id);
    state
        .db
        .insert(auth_key.as_bytes(), auth_token.as_bytes())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("Database error: {}", e)),
                }),
            )
        })?;

    // Save rig to database (will be registered via WebSocket when it connects)
    let rig_key = format!("farm/rigs/{}", rig_id);
    let rig_json = serde_json::to_string(&farm_rig).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateFarmHandResponse {
                ok: false,
                error: Some(format!("Serialization error: {}", e)),
            }),
        )
    })?;
    state
        .db
        .insert(rig_key.as_bytes(), rig_json.as_bytes())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("Database error: {}", e)),
                }),
            )
        })?;

    // Create FarmHandConfig
    let farmhand_config = FarmHandConfig {
        rig_id: rig_id.clone(),
        rig_name: req.rig_name.clone(),
        controller_ws_url,
        endpoint_mode,
        auth_token: auth_token.clone(),
        wallet_address: req.wallet_address.clone(),
        default_threads: req.default_threads,
    };

    // Generate TOML config
    let config_toml = farmhand_config.to_toml().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateFarmHandResponse {
                ok: false,
                error: Some(format!("Config serialization error: {}", e)),
            }),
        )
    })?;

    // Create ZIP bundle in memory
    let mut zip_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o755);

        // Add config file
        zip.start_file("farmhand.toml", options).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP error: {}", e)),
                }),
            )
        })?;
        zip.write_all(config_toml.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP write error: {}", e)),
                }),
            )
        })?;

        // Add README
        let readme = format!(
            r#"# FarmHand Rig: {}

## Rig Details
- Rig ID: {}
- Connection Mode: {}
- Controller URL: {}

## Setup Instructions

### Windows:
1. Extract this ZIP to a folder (e.g., C:\FarmHand)
2. Copy farmhand.exe to this folder
3. Double-click START-FARMHAND.bat

### Linux:
1. Extract this ZIP: tar -xzf farmhand_{}.tar.gz
2. Copy farmhand binary to this folder
3. Make executable: chmod +x farmhand
4. Run: ./farmhand

## Configuration

The farmhand.toml file contains your rig configuration.
DO NOT share this file - it contains your authentication token!

## Troubleshooting

Connection failed?
- Check firewall settings
- Verify controller is running
- Check network connectivity
{}

For support, contact your farm administrator.
"#,
            req.rig_name,
            rig_id,
            req.connection_mode,
            farmhand_config.controller_ws_url,
            req.rig_name,
            if req.connection_mode == "local" {
                "- Ensure you're on the same LAN as the controller"
            } else {
                "- Verify public endpoint is accessible from internet"
            }
        );

        zip.start_file("README.txt", options).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP error: {}", e)),
                }),
            )
        })?;
        zip.write_all(readme.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP write error: {}", e)),
                }),
            )
        })?;

        // Add batch file for Windows
        let batch_script = format!(
            r#"@echo off
title FarmHand - {}
echo.
echo ========================================
echo   FarmHand Mining Rig
echo ========================================
echo   Rig: {}
echo   Mode: {}
echo ========================================
echo.

if not exist farmhand.exe (
    echo ERROR: farmhand.exe not found!
    echo Please copy farmhand.exe to this folder.
    pause
    exit /b 1
)

echo Starting FarmHand...
farmhand.exe
pause
"#,
            req.rig_name, req.rig_name, req.connection_mode
        );

        zip.start_file("START-FARMHAND.bat", options).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP error: {}", e)),
                }),
            )
        })?;
        zip.write_all(batch_script.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP write error: {}", e)),
                }),
            )
        })?;

        // Add shell script for Linux
        let shell_script = format!(
            r#"#!/bin/bash
echo ""
echo "========================================"
echo "  FarmHand Mining Rig"
echo "========================================"
echo "  Rig: {}"
echo "  Mode: {}"
echo "========================================"
echo ""

if [ ! -f ./farmhand ]; then
    echo "ERROR: farmhand binary not found!"
    echo "Please copy farmhand to this folder."
    exit 1
fi

chmod +x ./farmhand
echo "Starting FarmHand..."
./farmhand
"#,
            req.rig_name, req.connection_mode
        );

        zip.start_file("start-farmhand.sh", options).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP error: {}", e)),
                }),
            )
        })?;
        zip.write_all(shell_script.as_bytes()).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP write error: {}", e)),
                }),
            )
        })?;

        zip.finish().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateFarmHandResponse {
                    ok: false,
                    error: Some(format!("ZIP finalization error: {}", e)),
                }),
            )
        })?;
    }

    tracing::info!(
        "✅ FarmHand bundle created: {} ({}, {})",
        rig_id,
        req.rig_name,
        req.connection_mode
    );

    // Return ZIP as downloadable file
    let filename = format!("farmhand_{}_{}.zip", req.rig_name.replace(' ', "_"), rig_id);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(zip_buffer))
        .unwrap())
}
