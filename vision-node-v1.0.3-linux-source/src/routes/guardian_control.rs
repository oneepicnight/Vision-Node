//! Guardian Control Room API endpoints
//!
//! Private endpoints for the creator to access god-tier controls.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::airdrop::get_cash_total_supply;
use crate::guardian::{is_creator_address, load_creator_config};
use crate::{CHAIN, PROM_P2P_PEERS};

/// State for Guardian Control Room routes
#[derive(Clone)]
pub struct GuardianControlState {
    pub db: sled::Db,
}

/// Response for /me/guardian endpoint
#[derive(Debug, Serialize)]
pub struct GuardianAuthResponse {
    pub is_guardian_creator: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_title: Option<String>,
}

/// Chain overview for Guardian page
#[derive(Debug, Serialize)]
pub struct ChainOverview {
    pub era: String, // "Mining" or "Staking"
    pub height: u64,
    pub total_land_supply: String,
    pub total_cash_supply: String,
    pub guardians_online: u64,
    pub constellation_nodes: u64,
}

/// Recent action log entry for Guardian page
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GuardianAction {
    pub timestamp: u64,
    pub action_type: String, // "CASH_AIRDROP", "CONFIG_CHANGE", "SYSTEM_EVENT"
    pub description: String,
    pub amount: Option<String>,
    pub target: Option<String>,
}

/// Extract wallet address from Authorization header (JWT or Bearer token)
/// Expected format: "Bearer <token>" or "<wallet_address>"
fn extract_wallet_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers
        .get("authorization")
        .or_else(|| headers.get("Authorization"))?;

    let auth_str = auth_header.to_str().ok()?;

    // If it starts with "Bearer ", extract token
    if auth_str.starts_with("Bearer ") {
        let token = auth_str.strip_prefix("Bearer ").unwrap_or("");

        // Simple JWT parsing - get payload without verification (for now)
        // In production, use jsonwebtoken crate to verify signature
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            // Decode middle part (payload)
            use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
            if let Ok(decoded) = URL_SAFE_NO_PAD.decode(parts[1]) {
                if let Ok(payload) = String::from_utf8(decoded) {
                    // Parse JSON to extract wallet address
                    // Expected: {"wallet":"0x123...","exp":...}
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                        if let Some(wallet) = json.get("wallet").and_then(|v| v.as_str()) {
                            return Some(wallet.to_string());
                        }
                        // Try alternate field names
                        if let Some(addr) = json.get("address").and_then(|v| v.as_str()) {
                            return Some(addr.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback: treat entire header as wallet address (for simple auth)
    if auth_str.starts_with("0x") && auth_str.len() >= 42 {
        return Some(auth_str.to_string());
    }

    None
}

/// GET /me/guardian
///
/// Check if the current user is the creator/guardian.
/// This controls access to the Guardian Control Room page.
pub async fn check_guardian_auth(
    headers: HeaderMap,
    State(state): State<Arc<GuardianControlState>>,
) -> impl IntoResponse {
    // Extract wallet address from Authorization header
    let wallet_address = match extract_wallet_from_headers(&headers) {
        Some(addr) => addr,
        None => {
            warn!("[GUARDIAN AUTH] No valid authorization header");
            return Json(GuardianAuthResponse {
                is_guardian_creator: false,
                creator_name: None,
                creator_title: None,
            });
        }
    };

    let is_creator = is_creator_address(&state.db, &wallet_address);

    if is_creator {
        match load_creator_config(&state.db) {
            Ok(config) => {
                info!("[GUARDIAN AUTH] Creator authenticated: {}", wallet_address);
                Json(GuardianAuthResponse {
                    is_guardian_creator: true,
                    creator_name: Some(config.name),
                    creator_title: Some(config.title),
                })
            }
            Err(e) => {
                warn!("[GUARDIAN AUTH] Failed to load creator config: {}", e);
                Json(GuardianAuthResponse {
                    is_guardian_creator: true,
                    creator_name: None,
                    creator_title: None,
                })
            }
        }
    } else {
        info!(
            "[GUARDIAN AUTH] Non-creator access denied: {}",
            wallet_address
        );
        Json(GuardianAuthResponse {
            is_guardian_creator: false,
            creator_name: None,
            creator_title: None,
        })
    }
}

/// GET /status/overview
///
/// Get high-level chain statistics for the Guardian page.
pub async fn get_chain_overview(
    State(state): State<Arc<GuardianControlState>>,
) -> impl IntoResponse {
    // Get chain height and era
    let (height, era_name) = {
        let chain = CHAIN.lock();
        let h = chain.blocks.len().saturating_sub(1) as u64;
        let era = if h < 100_000 { "Mining" } else { "Staking" }; // Simplified era detection
        (h, era.to_string())
    };

    // Get CASH supply from airdrop module
    let cash_supply = match get_cash_total_supply(&state.db) {
        Ok(supply) => format!("{:.2}", supply as f64 / 1_000_000_000.0),
        Err(e) => {
            warn!("[GUARDIAN] Failed to get CASH supply: {}", e);
            "0.00".to_string()
        }
    };

    // Get LAND supply from token accounts or chain state
    // For now, use a calculated value based on blocks mined
    let land_supply = format!("{:.2}", (height as f64 * 50.0) / 1_000_000_000.0);

    // Get P2P peer counts
    let peer_count = PROM_P2P_PEERS.get() as u64;

    // Count guardians vs constellation nodes (simplified: all peers are constellation for now)
    let guardians = 0; // TODO: Count peers with role="guardian" from peer store
    let constellation = peer_count;

    let overview = ChainOverview {
        era: era_name,
        height,
        total_land_supply: land_supply,
        total_cash_supply: cash_supply,
        guardians_online: guardians,
        constellation_nodes: constellation,
    };

    Json(overview)
}

/// GET /guardian/actions/recent
///
/// Get recent Guardian actions from Chronicle (action log).
/// Returns last 50 actions sorted by timestamp descending.
pub async fn get_recent_actions(
    State(state): State<Arc<GuardianControlState>>,
) -> impl IntoResponse {
    // Open Chronicle tree (action log)
    let chronicle = match state.db.open_tree("chronicle") {
        Ok(tree) => tree,
        Err(e) => {
            error!("[GUARDIAN] Failed to open chronicle tree: {}", e);
            return Json(Vec::<GuardianAction>::new());
        }
    };

    let mut actions = Vec::new();

    // Iterate chronicle in reverse (newest first)
    for (key, value) in chronicle.iter().rev().take(50).flatten() {
        // Parse key as timestamp
        let _timestamp = u64::from_be_bytes(key.as_ref().try_into().unwrap_or([0u8; 8]));

        // Parse value as JSON action entry
        if let Ok(entry) = serde_json::from_slice::<GuardianAction>(&value) {
            actions.push(entry);
        }
    }

    Json(actions)
}

/// Log a Guardian action to Chronicle
/// Called by other modules when Guardian actions occur
pub fn log_guardian_action(
    db: &sled::Db,
    action_type: &str,
    description: &str,
    amount: Option<String>,
    target: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let chronicle = db.open_tree("chronicle")?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let action = GuardianAction {
        timestamp,
        action_type: action_type.to_string(),
        description: description.to_string(),
        amount,
        target,
    };

    let key = timestamp.to_be_bytes(); // Sortable by timestamp
    let value = serde_json::to_vec(&action)?;

    chronicle.insert(key, value)?;

    info!(
        "[CHRONICLE] Logged action: {} - {}",
        action_type, description
    );

    Ok(())
}

/// Create Guardian Control Room router
pub fn guardian_control_router(db: sled::Db) -> Router {
    let state = Arc::new(GuardianControlState { db });

    Router::new()
        .route("/me/guardian", get(check_guardian_auth))
        .route("/status/overview", get(get_chain_overview))
        .route("/guardian/actions/recent", get(get_recent_actions))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guardian_auth_response() {
        let response = GuardianAuthResponse {
            is_guardian_creator: true,
            creator_name: Some("Donald Etcher".to_string()),
            creator_title: Some("Architect of Chaos".to_string()),
        };

        assert!(response.is_guardian_creator);
        assert_eq!(response.creator_name.unwrap(), "Donald Etcher");
    }
}
