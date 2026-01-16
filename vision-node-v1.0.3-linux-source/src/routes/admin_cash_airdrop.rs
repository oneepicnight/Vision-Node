//! Admin routes for CASH airdrop system
//!
//! Guardian/Admin-only endpoints to preview and execute CASH airdrops.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::airdrop::{
    execute_cash_airdrop, get_cash_total_supply, CashAirdropLimits, CashAirdropRequest,
};
use crate::guardian::is_creator_address;

/// State for CASH airdrop routes
#[derive(Clone)]
pub struct AirdropRouteState {
    pub db: sled::Db,
    pub limits: CashAirdropLimits,
    pub balances_tree: String,
}

/// Response for preview endpoint
#[derive(Debug, Serialize)]
pub struct AirdropPreviewResponse {
    pub ok: bool,
    pub total_recipients: usize,
    pub total_cash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_confirmation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for execute endpoint
#[derive(Debug, Serialize)]
pub struct AirdropExecuteResponse {
    pub ok: bool,
    pub total_recipients: usize,
    pub total_cash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<Vec<FailedRecipient>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FailedRecipient {
    pub address: String,
    pub error: String,
}

/// POST /admin/airdrop/cash/preview
///
/// Preview a CASH airdrop without executing it.
/// Returns totals and whether confirmation is required.
///
/// Auth: Requires Guardian/Admin token
pub async fn preview_cash_airdrop(
    headers: HeaderMap,
    State(state): State<Arc<AirdropRouteState>>,
    Json(request): Json<CashAirdropRequest>,
) -> impl IntoResponse {
    // Guardian auth check
    let wallet_address = match extract_wallet_from_headers(&headers) {
        Some(addr) => addr,
        None => {
            warn!("[CASH AIRDROP PREVIEW] No valid authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized: Guardian authentication required"
                })),
            )
                .into_response();
        }
    };

    if !is_creator_address(&state.db, &wallet_address) {
        warn!(
            "[CASH AIRDROP PREVIEW] Unauthorized attempt by: {}",
            wallet_address
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Forbidden: Only the Guardian creator can preview CASH airdrops"
            })),
        )
            .into_response();
    }

    let limits = &state.limits;

    // Validate request
    match crate::airdrop::validate_airdrop_request(&request, limits) {
        Ok(total_cash) => {
            let requires_confirmation = limits.require_confirm_phrase
                && total_cash >= limits.confirm_threshold
                && request.confirm_phrase.is_none();

            info!(
                "[CASH AIRDROP PREVIEW] {} recipients, {} CASH, confirmation required: {}",
                request.recipients.len(),
                total_cash,
                requires_confirmation
            );

            Json(AirdropPreviewResponse {
                ok: true,
                total_recipients: request.recipients.len(),
                total_cash: total_cash.to_string(),
                requires_confirmation: if requires_confirmation {
                    Some(true)
                } else {
                    None
                },
                error: None,
            })
            .into_response()
        }
        Err(e) => {
            error!("[CASH AIRDROP PREVIEW] Validation failed: {}", e);
            Json(AirdropPreviewResponse {
                ok: false,
                total_recipients: 0,
                total_cash: "0".to_string(),
                requires_confirmation: None,
                error: Some(e.to_string()),
            })
            .into_response()
        }
    }
}

/// Extract wallet address from Authorization header
fn extract_wallet_from_headers(headers: &HeaderMap) -> Option<String> {
    let auth_header = headers
        .get("authorization")
        .or_else(|| headers.get("Authorization"))?;

    let auth_str = auth_header.to_str().ok()?;

    // If it starts with "Bearer ", extract token
    if auth_str.starts_with("Bearer ") {
        let token = auth_str.strip_prefix("Bearer ").unwrap_or("");

        // Simple JWT parsing - get payload without verification
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() == 3 {
            use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
            if let Ok(decoded) = URL_SAFE_NO_PAD.decode(parts[1]) {
                if let Ok(payload) = String::from_utf8(decoded) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload) {
                        if let Some(wallet) = json.get("wallet").and_then(|v| v.as_str()) {
                            return Some(wallet.to_string());
                        }
                        if let Some(addr) = json.get("address").and_then(|v| v.as_str()) {
                            return Some(addr.to_string());
                        }
                    }
                }
            }
        }
    }

    // Fallback: treat entire header as wallet address
    if auth_str.starts_with("0x") && auth_str.len() >= 42 {
        return Some(auth_str.to_string());
    }

    None
}

/// POST /admin/airdrop/cash
///
/// Execute a CASH airdrop.
/// Credits CASH to recipient wallets and updates global supply.
///
/// Auth: Requires Guardian/Admin token
pub async fn execute_cash_airdrop_handler(
    headers: HeaderMap,
    State(state): State<Arc<AirdropRouteState>>,
    Json(request): Json<CashAirdropRequest>,
) -> impl IntoResponse {
    // Guardian auth check
    let wallet_address = match extract_wallet_from_headers(&headers) {
        Some(addr) => addr,
        None => {
            warn!("[CASH AIRDROP] No valid authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized: Guardian authentication required"
                })),
            )
                .into_response();
        }
    };

    if !is_creator_address(&state.db, &wallet_address) {
        warn!("[CASH AIRDROP] Unauthorized attempt by: {}", wallet_address);
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Forbidden: Only the Guardian creator can execute CASH airdrops"
            })),
        )
            .into_response();
    }

    info!(
        "[CASH AIRDROP] Guardian {} authenticated for airdrop execution",
        wallet_address
    );

    let limits = &state.limits;

    info!(
        "[CASH AIRDROP] Execute request from {}: {} recipients, reason: {:?}",
        request.requested_by,
        request.recipients.len(),
        request.reason
    );

    // Execute airdrop
    match execute_cash_airdrop(&state.db, &state.balances_tree, &request, limits) {
        Ok(result) => {
            let has_failures = !result.failed.is_empty();

            let failed_records: Option<Vec<FailedRecipient>> = if has_failures {
                Some(
                    result
                        .failed
                        .iter()
                        .map(|(addr, err)| FailedRecipient {
                            address: addr.clone(),
                            error: err.clone(),
                        })
                        .collect(),
                )
            } else {
                None
            };

            let message = if has_failures {
                format!(
                    "CASH airdrop completed with {} successes and {} failures. Total {} CASH distributed.",
                    result.total_recipients,
                    result.failed.len(),
                    result.total_cash
                )
            } else {
                format!(
                    "CASH airdrop executed successfully via Guardian. {} CASH distributed to {} recipients.",
                    result.total_cash, result.total_recipients
                )
            };

            info!("[CASH AIRDROP] {}", message);

            // Log to Chronicle for Guardian Control Room
            if let Err(e) = crate::routes::guardian_control::log_guardian_action(
                &state.db,
                "CASH_AIRDROP",
                &format!("Distributed to {} recipients", result.total_recipients),
                Some(format!("{} CASH", result.total_cash)),
                None,
            ) {
                warn!("[CHRONICLE] Failed to log airdrop action: {}", e);
            }

            (
                StatusCode::OK,
                Json(AirdropExecuteResponse {
                    ok: true,
                    total_recipients: result.total_recipients,
                    total_cash: result.total_cash.to_string(),
                    message: Some(message),
                    failed: failed_records,
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("[CASH AIRDROP] Execution failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(AirdropExecuteResponse {
                    ok: false,
                    total_recipients: 0,
                    total_cash: "0".to_string(),
                    message: None,
                    failed: None,
                    error: Some(e.to_string()),
                }),
            )
                .into_response()
        }
    }
}

/// Response for supply endpoint
#[derive(Debug, Serialize)]
pub struct SupplyResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_supply: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GET /admin/airdrop/cash/supply
///
/// Get current total CASH supply
pub async fn get_cash_supply(State(state): State<Arc<AirdropRouteState>>) -> impl IntoResponse {
    match get_cash_total_supply(&state.db) {
        Ok(supply) => Json(SupplyResponse {
            ok: true,
            total_supply: Some(supply.to_string()),
            error: None,
        }),
        Err(e) => Json(SupplyResponse {
            ok: false,
            total_supply: None,
            error: Some(e),
        }),
    }
}

/// Create CASH airdrop router with configuration
pub fn airdrop_router(db: sled::Db, limits: CashAirdropLimits, balances_tree: String) -> Router {
    let state = Arc::new(AirdropRouteState {
        db,
        limits,
        balances_tree,
    });

    Router::new()
        .route("/admin/airdrop/cash/preview", post(preview_cash_airdrop))
        .route("/admin/airdrop/cash", post(execute_cash_airdrop_handler))
        .route("/admin/airdrop/cash/supply", get(get_cash_supply))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add integration tests with test AirdropRouteState
}
