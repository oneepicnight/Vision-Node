//! Admin seed balance route handler
//!
//! Provides HTTP handler for:
//! - POST /admin/seed-balance - Seed test balances (requires admin token)

use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

use crate::{api_error_struct, check_admin, CHAIN};

/// Request body for POST /admin/seed-balance
#[derive(Deserialize)]
pub struct SeedBalanceReq {
    pub address: String,
    pub amount: String, // decimal string -> u128
}

/// Handler for POST /admin/seed-balance
///
/// Seeds a balance for a given address (for testing/development).
/// Requires admin token via Authorization header or query param.
///
/// Request body: { "address": "64-char-hex", "amount": "123456" }
///
/// Security: Validates admin token, address format (64-char hex), and amount parsing.
pub async fn admin_seed_balance(
    headers: HeaderMap,
    Query(q): Query<HashMap<String, String>>,
    Json(req): Json<SeedBalanceReq>,
) -> impl IntoResponse {
    if !check_admin(headers, &q) {
        return api_error_struct(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "invalid or missing admin token",
        );
    }

    // Validate address (64-char hex)
    if req.address.len() != 64 || !req.address.chars().all(|c| c.is_ascii_hexdigit()) {
        return api_error_struct(
            StatusCode::BAD_REQUEST,
            "invalid_address",
            "address must be 64-character hex string",
        );
    }

    // Parse amount
    let amount: u128 = match req.amount.parse() {
        Ok(v) => v,
        Err(_) => {
            return api_error_struct(
                StatusCode::BAD_REQUEST,
                "invalid_amount",
                "amount must be valid u128",
            )
        }
    };

    // Write to balances tree
    let db = {
        let g = CHAIN.lock();
        g.db.clone()
    };

    match db.open_tree("balances") {
        Ok(balances) => {
            let mut buf = [0u8; 16];
            buf.copy_from_slice(&amount.to_le_bytes());
            match balances.insert(req.address.as_bytes(), &buf[..]) {
                Ok(_) => {
                    tracing::info!("Admin seeded balance: {} -> {}", req.address, amount);
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "ok": true,
                            "address": req.address,
                            "balance": amount.to_string(),
                        })),
                    )
                }
                Err(e) => api_error_struct(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "db_error",
                    &format!("failed to write: {}", e),
                ),
            }
        }
        Err(e) => api_error_struct(
            StatusCode::INTERNAL_SERVER_ERROR,
            "db_error",
            &format!("failed to open tree: {}", e),
        ),
    }
}
