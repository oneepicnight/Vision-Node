//! Wallet Legacy API Routes
//!
//! HTTP endpoints for managing legacy routes (torch-passing)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::legacy::{legacy_message, LegacyManager, LegacyRoute, LegacyStatus};

/// Request to create or update a legacy route
#[derive(Debug, Deserialize)]
pub struct CreateLegacyRequest {
    pub owner_address: String,
    pub legacy_address: String,
    pub star_id: String,
    pub epitaph: Option<String>,
}

/// Request to arm or cancel a legacy route
#[derive(Debug, Deserialize)]
pub struct LegacyActionRequest {
    pub owner_address: String,
}

/// Query parameters for getting legacy route
#[derive(Debug, Deserialize)]
pub struct LegacyQuery {
    pub addr: String,
}

/// Response for legacy operations
#[derive(Debug, Serialize)]
pub struct LegacyResponse {
    pub success: bool,
    pub message: String,
    pub route: Option<LegacyRoute>,
}

/// GET /wallet/legacy?addr=xxx
/// Get the legacy route for an address
pub async fn get_legacy(
    Query(params): Query<LegacyQuery>,
    State(legacy_manager): State<Arc<LegacyManager>>,
) -> impl IntoResponse {
    match legacy_manager.get(&params.addr) {
        Ok(Some(route)) => {
            let message = legacy_message(route.status);

            (
                StatusCode::OK,
                Json(LegacyResponse {
                    success: true,
                    message: message.to_string(),
                    route: Some(route),
                }),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(LegacyResponse {
                success: false,
                message: "No legacy route found for this address".to_string(),
                route: None,
            }),
        ),
        Err(e) => {
            error!("[LEGACY] Failed to get legacy route: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LegacyResponse {
                    success: false,
                    message: format!("Error: {}", e),
                    route: None,
                }),
            )
        }
    }
}

/// POST /wallet/legacy
/// Create or update a legacy route
pub async fn create_legacy(
    State(legacy_manager): State<Arc<LegacyManager>>,
    Json(req): Json<CreateLegacyRequest>,
) -> impl IntoResponse {
    // Create new route
    let mut route = LegacyRoute::new(
        req.owner_address.clone(),
        req.legacy_address.clone(),
        req.star_id.clone(),
    );

    // Add epitaph if provided
    if let Some(epitaph) = req.epitaph {
        route.set_epitaph(epitaph);
    }

    // Validate
    if let Err(e) = legacy_manager.validate(&route) {
        return (
            StatusCode::BAD_REQUEST,
            Json(LegacyResponse {
                success: false,
                message: format!("Validation failed: {}", e),
                route: None,
            }),
        );
    }

    // Check if there's an existing route
    match legacy_manager.get(&req.owner_address) {
        Ok(Some(existing)) => {
            // Don't allow updating an executed route
            if existing.status == LegacyStatus::Executed {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(LegacyResponse {
                        success: false,
                        message: "Cannot modify an executed legacy route".to_string(),
                        route: Some(existing),
                    }),
                );
            }

            // Allow updating draft or cancelled routes
            if existing.status == LegacyStatus::Draft || existing.status == LegacyStatus::Cancelled
            {
                info!(
                    "[LEGACY] Updating legacy route for {} (old: {} → new: {})",
                    req.owner_address, existing.legacy_address, req.legacy_address
                );
            }
        }
        Ok(None) => {
            info!(
                "[LEGACY] Creating new legacy route: {} → {} (star: {})",
                req.owner_address, req.legacy_address, req.star_id
            );
        }
        Err(e) => {
            error!("[LEGACY] Error checking existing route: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LegacyResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                    route: None,
                }),
            );
        }
    }

    // Save the route
    match legacy_manager.save(&route) {
        Ok(_) => (
            StatusCode::OK,
            Json(LegacyResponse {
                success: true,
                message: "Legacy route created. Use /wallet/legacy/arm to activate it.".to_string(),
                route: Some(route),
            }),
        ),
        Err(e) => {
            error!("[LEGACY] Failed to save legacy route: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LegacyResponse {
                    success: false,
                    message: format!("Failed to save: {}", e),
                    route: None,
                }),
            )
        }
    }
}

/// POST /wallet/legacy/arm
/// Arm a legacy route (make it active)
pub async fn arm_legacy(
    State(legacy_manager): State<Arc<LegacyManager>>,
    Json(req): Json<LegacyActionRequest>,
) -> impl IntoResponse {
    // Get existing route
    let mut route = match legacy_manager.get(&req.owner_address) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(LegacyResponse {
                    success: false,
                    message: "No legacy route found. Create one first.".to_string(),
                    route: None,
                }),
            );
        }
        Err(e) => {
            error!("[LEGACY] Failed to get legacy route: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LegacyResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                    route: None,
                }),
            );
        }
    };

    // Arm it
    match route.arm() {
        Ok(_) => {
            // Save updated route
            match legacy_manager.save(&route) {
                Ok(_) => {
                    info!(
                        "[LEGACY] Legacy armed: {} → {}",
                        route.owner_address, route.legacy_address
                    );

                    (
                        StatusCode::OK,
                        Json(LegacyResponse {
                            success: true,
                            message: legacy_message(LegacyStatus::Armed).to_string(),
                            route: Some(route),
                        }),
                    )
                }
                Err(e) => {
                    error!("[LEGACY] Failed to save armed route: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(LegacyResponse {
                            success: false,
                            message: format!("Failed to save: {}", e),
                            route: None,
                        }),
                    )
                }
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(LegacyResponse {
                success: false,
                message: format!("Cannot arm: {}", e),
                route: Some(route),
            }),
        ),
    }
}

/// POST /wallet/legacy/cancel
/// Cancel a legacy route
pub async fn cancel_legacy(
    State(legacy_manager): State<Arc<LegacyManager>>,
    Json(req): Json<LegacyActionRequest>,
) -> impl IntoResponse {
    // Get existing route
    let mut route = match legacy_manager.get(&req.owner_address) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(LegacyResponse {
                    success: false,
                    message: "No legacy route found".to_string(),
                    route: None,
                }),
            );
        }
        Err(e) => {
            error!("[LEGACY] Failed to get legacy route: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(LegacyResponse {
                    success: false,
                    message: format!("Database error: {}", e),
                    route: None,
                }),
            );
        }
    };

    // Cancel it
    match route.cancel() {
        Ok(_) => {
            // Save updated route
            match legacy_manager.save(&route) {
                Ok(_) => {
                    info!("[LEGACY] Legacy cancelled: {}", route.owner_address);

                    (
                        StatusCode::OK,
                        Json(LegacyResponse {
                            success: true,
                            message: "Legacy route cancelled".to_string(),
                            route: Some(route),
                        }),
                    )
                }
                Err(e) => {
                    error!("[LEGACY] Failed to save cancelled route: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(LegacyResponse {
                            success: false,
                            message: format!("Failed to save: {}", e),
                            route: None,
                        }),
                    )
                }
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(LegacyResponse {
                success: false,
                message: format!("Cannot cancel: {}", e),
                route: Some(route),
            }),
        ),
    }
}

/// Create the wallet legacy router
pub fn wallet_legacy_router(legacy_manager: Arc<LegacyManager>) -> Router {
    Router::new()
        .route("/wallet/legacy", get(get_legacy).post(create_legacy))
        .route("/wallet/legacy/arm", post(arm_legacy))
        .route("/wallet/legacy/cancel", post(cancel_legacy))
        .with_state(legacy_manager)
}
