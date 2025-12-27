//! Tip API Routes - "Buy Me a Drink" Feature
//!
//! Endpoints for one-time tipping system.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::tip::{load_tip_state, save_tip_state, usd_to_coin_amount, TipConfig};

/// State for tip routes
#[derive(Clone)]
pub struct TipRouteState {
    pub db: sled::Db,
    pub config: TipConfig,
}

/// Request to send a tip
#[derive(Debug, Deserialize)]
pub struct TipRequest {
    pub coin: String,
    pub wallet_address: String, // From auth/session in production
}

/// Response for tip status query
#[derive(Debug, Serialize)]
pub struct TipStatusResponse {
    pub has_tipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>, // String for large numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_tip_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_label: Option<String>,
}

/// Response for tip submission
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TipSubmitResponse {
    Success {
        ok: bool,
        coin: String,
        amount: String,
        message: String,
    },
    Error {
        error: String,
    },
}

/// GET /wallet/tip/status - Check if wallet has tipped
pub async fn get_tip_status(
    State(state): State<Arc<TipRouteState>>,
    Json(req): Json<TipStatusQuery>,
) -> impl IntoResponse {
    match load_tip_state(&state.db, &req.wallet_address) {
        Ok(tip_state) => {
            let badge_label = if tip_state.has_tipped {
                Some("Thank you for believing in my dream".to_string())
            } else {
                None
            };

            let response = TipStatusResponse {
                has_tipped: tip_state.has_tipped,
                coin: tip_state.coin,
                amount: tip_state.amount.map(|a| a.to_string()),
                last_tip_at: tip_state.last_tip_at,
                badge_label,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            error!("[TIP] Failed to load tip status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TipStatusResponse {
                    has_tipped: false,
                    coin: None,
                    amount: None,
                    last_tip_at: None,
                    badge_label: None,
                }),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TipStatusQuery {
    pub wallet_address: String,
}

/// POST /wallet/tip - Send a tip
pub async fn send_tip(
    State(state): State<Arc<TipRouteState>>,
    Json(req): Json<TipRequest>,
) -> (StatusCode, Json<TipSubmitResponse>) {
    // Load tip state
    let mut tip_state = match load_tip_state(&state.db, &req.wallet_address) {
        Ok(state) => state,
        Err(e) => {
            error!("[TIP] Failed to load tip state: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TipSubmitResponse::Error {
                    error: "Failed to load tip state".to_string(),
                }),
            );
        }
    };

    // Check if already tipped
    if tip_state.has_tipped {
        return (
            StatusCode::CONFLICT,
            Json(TipSubmitResponse::Error {
                error: "You already tipped. One drink per wallet, cheapskate.".to_string(),
            }),
        );
    }

    // Validate coin is allowed
    if !state.config.tip_allowed_coins.contains(&req.coin) {
        return (
            StatusCode::BAD_REQUEST,
            Json(TipSubmitResponse::Error {
                error: format!(
                    "Coin {} not supported for tips. Use BTC, BCH, or DOGE.",
                    req.coin
                ),
            }),
        );
    }

    // Convert USD to coin amount
    let tip_amount = match usd_to_coin_amount(&req.coin, state.config.tip_usd_amount) {
        Ok(amount) => amount,
        Err(e) => {
            error!("[TIP] Price conversion failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TipSubmitResponse::Error {
                    error: format!("Failed to calculate tip amount: {}", e),
                }),
            );
        }
    };

    // TODO: In production, integrate with wallet service to:
    // 1. Check user's balance for this coin
    // 2. Execute transfer to config.tip_address
    //
    // For now, we'll simulate success for testing
    // Uncomment and implement when wallet service is integrated:
    /*
    match check_balance_for_tip(user_balance, tip_amount, &req.coin) {
        Ok(()) => {},
        Err(TipError::InsufficientBalance) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TipErrorResponse {
                    error: "Not enough balance. You're broke, I get it.".to_string(),
                }),
            );
        }
        Err(e) => {
            error!("[TIP] Balance check failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TipErrorResponse {
                    error: format!("Balance check failed: {}", e),
                }),
            );
        }
    }

    // Execute transfer
    match execute_transfer(&req.wallet_address, &state.config.tip_address, tip_amount, &req.coin) {
        Ok(()) => {},
        Err(e) => {
            error!("[TIP] Transfer failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TipErrorResponse {
                    error: format!("Transfer failed: {}", e),
                }),
            );
        }
    }
    */

    // Mark as tipped
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    tip_state.tipped(&req.coin, tip_amount, now);

    if let Err(e) = save_tip_state(&state.db, &tip_state) {
        error!("[TIP] Failed to save tip state: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(TipSubmitResponse::Error {
                error: "Failed to record tip".to_string(),
            }),
        );
    }

    info!(
        "[TIP] 🍺 Wallet {} tipped {} {} (${}) to {}",
        req.wallet_address,
        tip_amount,
        req.coin,
        state.config.tip_usd_amount,
        state.config.tip_address
    );

    (
        StatusCode::OK,
        Json(TipSubmitResponse::Success {
            ok: true,
            coin: req.coin.clone(),
            amount: tip_amount.to_string(),
            message: "Thanks for the drink. You are officially not an asshole.".to_string(),
        }),
    )
}

/// Create the tip router
pub fn tip_router(db: sled::Db, config: TipConfig) -> Router {
    let state = Arc::new(TipRouteState { db, config });

    Router::new()
        .route("/wallet/tip/status", post(get_tip_status))
        .route("/wallet/tip", post(send_tip))
        .with_state(state)
}
