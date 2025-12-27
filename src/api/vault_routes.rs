use crate::market::engine::QuoteAsset;
#[cfg(debug_assertions)]
use crate::vault::store::VaultBucket;
use crate::vault::store::VaultStore;
use axum::{extract::ConnectInfo, response::IntoResponse, routing::get, Json, Router};
use std::net::SocketAddr;
use std::sync::Arc;

use crate::api::security;
use crate::metrics;
use crate::vision_constants::{allow_seed_export, allow_seed_import};

#[derive(Clone)]
pub struct AppState {
    pub dbctx: Arc<metrics::DbCtx>,
    pub metrics: Arc<metrics::Metrics>,
}

pub fn router() -> Router {
    Router::new()
        .route("/vault", get(get_stats))
        .route("/vault/history", get(get_history))
        .route("/vault/epoch", get(get_epoch))
        .route("/vault/debug/balances", get(get_all_balances))
        .route("/vault/miners/addresses", get(get_miners_addresses))
        .route("/vault/miners/multisig", get(get_miners_multisig))
        // MAINNET LOCKDOWN: Seed operations moved to /admin namespace with security
        .route(
            "/admin/wallet/external/export",
            get(export_external_seed_secure),
        )
        .route(
            "/admin/wallet/external/import",
            axum::routing::post(import_external_seed_secure),
        )
        .route("/wallet/mode", get(get_wallet_mode))
        .merge(dev_router())
}

async fn get_stats() -> Json<serde_json::Value> {
    // Use VaultStore as source of truth
    let db = crate::CHAIN.lock().db.clone();
    let store = VaultStore::new(db);

    match store.get_all_balances() {
        Ok(balances) => Json(serde_json::json!({
            "ok": true,
            "source": "VaultStore (single source of truth)",
            "buckets": {
                "miners": {
                    "LAND": balances.miners.land.to_string(),
                    "BTC": balances.miners.btc.to_string(),
                    "BCH": balances.miners.bch.to_string(),
                    "DOGE": balances.miners.doge.to_string(),
                },
                "devops": {
                    "LAND": balances.devops.land.to_string(),
                    "BTC": balances.devops.btc.to_string(),
                    "BCH": balances.devops.bch.to_string(),
                    "DOGE": balances.devops.doge.to_string(),
                },
                "founders": {
                    "LAND": balances.founders.land.to_string(),
                    "BTC": balances.founders.btc.to_string(),
                    "BCH": balances.founders.bch.to_string(),
                    "DOGE": balances.founders.doge.to_string(),
                }
            }
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to read VaultStore: {}", e)
        })),
    }
}

async fn get_history() -> Json<serde_json::Value> {
    // History tracking not yet implemented in VaultStore
    // In staged build, old vault.rs has this; in launch-core we use VaultStore
    Json(serde_json::json!({
        "ok": true,
        "message": "Vault history tracking not yet implemented",
        "events": []
    }))
}

async fn get_epoch() -> impl IntoResponse {
    // Get current chain height
    let height = {
        let chain = crate::CHAIN.lock();
        chain.blocks.len() as u64
    };

    // Get DB reference and create VaultStore (single source of truth)
    let db = crate::CHAIN.lock().db.clone();
    let store = VaultStore::new(db.clone());

    // Read vault totals from VaultStore instead of legacy supply: keys
    let vault_land = store.total_vault_balance(QuoteAsset::Land).unwrap_or(0);
    let vault_btc = store.total_vault_balance(QuoteAsset::Btc).unwrap_or(0);
    let vault_bch = store.total_vault_balance(QuoteAsset::Bch).unwrap_or(0);
    let vault_doge = store.total_vault_balance(QuoteAsset::Doge).unwrap_or(0);

    // Get epoch status from vault_epoch module
    let epoch_status = match crate::vault_epoch::get_epoch_status(&db, height) {
        Ok(status) => status,
        Err(_) => {
            return Json(serde_json::json!({
                "error": "Failed to fetch epoch status"
            }))
        }
    };

    Json(serde_json::json!({
        "epoch_index": epoch_status.epoch_index,
        "last_payout_height": epoch_status.last_payout_height,
        "last_payout_at_ms": epoch_status.last_payout_at_ms,
        "vault_balances": {
            "LAND": vault_land.to_string(),
            "BTC": vault_btc.to_string(),
            "BCH": vault_bch.to_string(),
            "DOGE": vault_doge.to_string(),
        },
        "total_weight": epoch_status.total_weight.to_string(),
        "due": epoch_status.due,
        "height": height
    }))
}

/// Debug endpoint: Get all vault balances by bucket and asset
async fn get_all_balances() -> impl IntoResponse {
    let db = crate::CHAIN.lock().db.clone();
    let store = VaultStore::new(db);

    match store.get_all_balances() {
        Ok(balances) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "miners": {
                    "LAND": balances.miners.land.to_string(),
                    "BTC": balances.miners.btc.to_string(),
                    "BCH": balances.miners.bch.to_string(),
                    "DOGE": balances.miners.doge.to_string(),
                },
                "devops": {
                    "LAND": balances.devops.land.to_string(),
                    "BTC": balances.devops.btc.to_string(),
                    "BCH": balances.devops.bch.to_string(),
                    "DOGE": balances.devops.doge.to_string(),
                },
                "founders": {
                    "LAND": balances.founders.land.to_string(),
                    "BTC": balances.founders.btc.to_string(),
                    "BCH": balances.founders.bch.to_string(),
                    "DOGE": balances.founders.doge.to_string(),
                }
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Failed to read VaultStore: {}", e)
            })),
        ),
    }
}

/// GET /api/vault/miners/addresses - Read-only endpoint showing miners vault deposit addresses
async fn get_miners_addresses() -> Json<serde_json::Value> {
    let btc_addr = crate::foundation_config::miners_btc_address()
        .unwrap_or_else(|| "[not configured]".to_string());
    let bch_addr = crate::foundation_config::miners_bch_address()
        .unwrap_or_else(|| "[not configured]".to_string());
    let doge_addr = crate::foundation_config::miners_doge_address()
        .unwrap_or_else(|| "[not configured]".to_string());

    Json(serde_json::json!({
        "ok": true,
        "purpose": "Read-only display of where mining fees are deposited",
        "addresses": {
            "btc": btc_addr,
            "bch": bch_addr,
            "doge": doge_addr,
        }
    }))
}

/// GET /api/vault/miners/multisig - Show miners multisig fee addresses (NO SEEDS, pubkeys only)
/// These addresses collect ONLY exchange fees, never user deposits
async fn get_miners_multisig() -> Json<serde_json::Value> {
    match crate::vault::miners_multisig::get_miners_multisig_addresses() {
        Ok((btc_addr, bch_addr, doge_addr, config)) => Json(serde_json::json!({
            "ok": true,
            "purpose": "Multisig addresses for collecting exchange fees ONLY (non-custodial)",
            "warning": "These addresses require offline signing. No seeds stored on node.",
            "addresses": {
                "btc": btc_addr,
                "bch": bch_addr,
                "doge": doge_addr,
            },
            "multisig": {
                "m": config.m,
                "n": config.n(),
                "pubkeys": config.pubkeys,
            }
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to generate multisig addresses: {}", e),
            "hint": "Set VISION_MINERS_MULTISIG_PUBKEYS and VISION_MINERS_MULTISIG_M environment variables"
        })),
    }
}

#[derive(serde::Deserialize)]
struct ImportSeedRequest {
    seed_hex: String,
}

/// GET /api/admin/wallet/external/export - Export external master seed (HEX)
///
/// ⚠️ MAINNET SECURITY LOCKDOWN ⚠️
/// - Requires localhost access ONLY (127.0.0.1 or ::1)
/// - Requires VISION_ALLOW_SEED_EXPORT=true env flag (default OFF)
/// - Requires X-Admin-Token header matching VISION_ADMIN_TOKEN
/// - Returns 404 if not enabled (security through obscurity for port scanners)
///
/// CRITICAL: This is the ONLY backup mechanism for user funds
/// WARNING: Anyone with this seed can derive all user addresses and spend funds
async fn export_external_seed_secure(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // MAINNET: Feature must be explicitly enabled
    if !allow_seed_export() {
        tracing::warn!(
            "[SECURITY BLOCK] Seed export attempt from {} - VISION_ALLOW_SEED_EXPORT not enabled",
            addr
        );
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Not found"
            })),
        );
    }

    // MAINNET: Must be localhost
    if !security::is_localhost(&addr) {
        tracing::error!(
            "[SECURITY BLOCK] Seed export attempt from REMOTE address: {}",
            addr
        );
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Access denied"
            })),
        );
    }

    // MAINNET: Must have admin token
    let token = headers.get("x-admin-token").and_then(|v| v.to_str().ok());

    if !security::verify_admin_token(token) {
        tracing::warn!(
            "[SECURITY BLOCK] Seed export from localhost {} - invalid admin token",
            addr
        );
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized"
            })),
        );
    }

    tracing::info!(
        "[SECURITY AUDIT] Seed export authorized from localhost {}",
        addr
    );

    match crate::market::deposits::export_external_seed() {
        Ok(seed_hex) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "seed_hex": seed_hex,
                "warning": "BACKUP THIS SEED IMMEDIATELY! Store in secure offline location.",
                "danger": "Anyone with this seed can spend user funds from derived addresses.",
                "usage": "POST to /api/admin/wallet/external/import with {\"seed_hex\": \"...\"} to restore"
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Failed to export seed: {}", e)
            })),
        ),
    }
}

/// POST /api/admin/wallet/external/import - Import external master seed from hex
///
/// ⚠️ MAINNET SECURITY LOCKDOWN ⚠️
/// - Requires localhost access ONLY (127.0.0.1 or ::1)
/// - Requires VISION_ALLOW_SEED_IMPORT=true env flag (default OFF)
/// - Requires X-Admin-Token header matching VISION_ADMIN_TOKEN
/// - Returns 404 if not enabled (security through obscurity for port scanners)
///
/// DANGER: This OVERWRITES the existing seed - all previous addresses become inaccessible
/// Only use when restoring from backup or migrating node
async fn import_external_seed_secure(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ImportSeedRequest>,
) -> impl IntoResponse {
    // MAINNET: Feature must be explicitly enabled
    if !allow_seed_import() {
        tracing::warn!(
            "[SECURITY BLOCK] Seed import attempt from {} - VISION_ALLOW_SEED_IMPORT not enabled",
            addr
        );
        return (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Not found"
            })),
        );
    }

    // MAINNET: Must be localhost
    if !security::is_localhost(&addr) {
        tracing::error!(
            "[SECURITY BLOCK] Seed import attempt from REMOTE address: {}",
            addr
        );
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Access denied"
            })),
        );
    }

    // MAINNET: Must have admin token
    let token = headers.get("x-admin-token").and_then(|v| v.to_str().ok());

    if !security::verify_admin_token(token) {
        tracing::warn!(
            "[SECURITY BLOCK] Seed import from localhost {} - invalid admin token",
            addr
        );
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized"
            })),
        );
    }

    tracing::info!(
        "[SECURITY AUDIT] Seed import authorized from localhost {}",
        addr
    );

    match crate::market::deposits::import_external_seed(&req.seed_hex) {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "message": "Seed imported successfully",
                "warning": "ALL PREVIOUS ADDRESSES ARE NOW INACCESSIBLE",
                "action_required": "Restart node to regenerate addresses from new seed"
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Failed to import seed: {}", e)
            })),
        ),
    }
}

// ==================== DEV/DEBUG ENDPOINTS (debug builds only) ====================

/// DELETE /api/dev/vault/reset - Reset all vault balances to zero (local testing only)
/// Only available in debug builds to prevent accidents
#[cfg(debug_assertions)]
async fn dev_reset_vault() -> impl IntoResponse {
    let db = crate::CHAIN.lock().db.clone();
    let store = VaultStore::new(db);

    // Reset all bucket balances
    for bucket in &[
        VaultBucket::Miners,
        VaultBucket::DevOps,
        VaultBucket::Founders,
    ] {
        for asset in &[
            QuoteAsset::Land,
            QuoteAsset::Btc,
            QuoteAsset::Bch,
            QuoteAsset::Doge,
        ] {
            if let Err(e) = store.debit_vault(*bucket, *asset, u128::MAX) {
                tracing::warn!("Failed to reset vault: {}", e);
            }
        }
    }

    tracing::info!("🔧 DEV: Vault reset to zero");
    Json(serde_json::json!({
        "ok": true,
        "message": "Vault balances reset to zero"
    }))
}

#[derive(serde::Deserialize)]
struct SimulateTradeRequest {
    asset: String, // "BTC", "BCH", "DOGE", or "LAND"
    amount: u128,  // amount in smallest units
}

/// POST /api/dev/exchange/simulate_trade - Force one known trade for testing
/// Only available in debug builds
#[cfg(debug_assertions)]
async fn dev_simulate_trade(Json(req): Json<SimulateTradeRequest>) -> impl IntoResponse {
    let asset = match req.asset.to_uppercase().as_str() {
        "BTC" => QuoteAsset::Btc,
        "BCH" => QuoteAsset::Bch,
        "DOGE" => QuoteAsset::Doge,
        "LAND" => QuoteAsset::Land,
        _ => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "Invalid asset. Use: BTC, BCH, DOGE, LAND"
                })),
            )
        }
    };

    let db = crate::CHAIN.lock().db.clone();

    // Route through vault as if it were an exchange fee
    match crate::market::settlement::route_exchange_fee(asset, req.amount as f64 / 100_000_000.0) {
        Ok(_) => {
            let store = VaultStore::new(db);
            let miners_bal = store
                .get_bucket_balance(VaultBucket::Miners, asset)
                .unwrap_or(0);
            let devops_bal = store
                .get_bucket_balance(VaultBucket::DevOps, asset)
                .unwrap_or(0);
            let founders_bal = store
                .get_bucket_balance(VaultBucket::Founders, asset)
                .unwrap_or(0);

            (
                axum::http::StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "message": format!("Simulated trade: {} {}", req.amount, asset.as_str()),
                    "vault_after": {
                        "miners": miners_bal.to_string(),
                        "devops": devops_bal.to_string(),
                        "founders": founders_bal.to_string(),
                    }
                })),
            )
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Simulation failed: {}", e)
            })),
        ),
    }
}

/// GET /api/wallet/mode - Get wallet mode status
async fn get_wallet_mode() -> Json<serde_json::Value> {
    let status = crate::swap::WalletModeStatus::current();

    Json(serde_json::json!({
        "mode": status.mode.as_str(),
        "can_sign": status.can_sign,
        "message": status.message,
        "capabilities": {
            "swap_initiation": status.capabilities.swap_initiation,
            "refund_signing": status.capabilities.refund_signing,
            "key_export": status.capabilities.key_export,
            "balance_viewing": status.capabilities.balance_viewing,
            "swap_monitoring": status.capabilities.swap_monitoring,
            "confirmation_tracking": status.capabilities.confirmation_tracking,
        },
        "instructions": if status.can_sign {
            "This node has full signing capability."
        } else {
            "To enable signing: POST /api/wallet/external/import with a valid seed_hex"
        }
    }))
}

#[cfg(debug_assertions)]
pub fn dev_router() -> Router {
    Router::new()
        .route("/dev/vault/reset", axum::routing::delete(dev_reset_vault))
        .route(
            "/dev/exchange/simulate_trade",
            axum::routing::post(dev_simulate_trade),
        )
}

#[cfg(not(debug_assertions))]
pub fn dev_router() -> Router {
    Router::new()
}
