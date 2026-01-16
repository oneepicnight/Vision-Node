use axum::response::Json;
use std::sync::Arc;

use crate::external_rpc::{health_check_rpc, EXTERNAL_RPC_CLIENTS};

///GET /api/external_rpc/status
/// Returns the health status of all configured external RPC endpoints
pub async fn get_external_rpc_status() -> Json<serde_json::Value> {
    let clients = EXTERNAL_RPC_CLIENTS.lock().unwrap();

    // Check BTC
    let btc_status = if let Some(client) = clients.get(&crate::external_rpc::ExternalChain::Btc) {
        let client_clone = Arc::clone(client);
        drop(clients); // Release lock before async call
        match health_check_rpc(&client_clone).await {
            Ok(tip) => serde_json::json!({
                "ok": true,
                "tip": tip
            }),
            Err(e) => serde_json::json!({
                "ok": false,
                "error": format!("{}", e)
            }),
        }
    } else {
        serde_json::json!({
            "ok": false,
            "error": "Not configured",
            "disabled": true
        })
    };

    // Re-acquire lock for BCH
    let clients = EXTERNAL_RPC_CLIENTS.lock().unwrap();
    let bch_status = if let Some(client) = clients.get(&crate::external_rpc::ExternalChain::Bch) {
        let client_clone = Arc::clone(client);
        drop(clients);
        match health_check_rpc(&client_clone).await {
            Ok(tip) => serde_json::json!({
                "ok": true,
                "tip": tip
            }),
            Err(e) => serde_json::json!({
                "ok": false,
                "error": format!("{}", e)
            }),
        }
    } else {
        serde_json::json!({
            "ok": false,
            "error": "Not configured",
            "disabled": true
        })
    };

    // Re-acquire lock for DOGE
    let clients = EXTERNAL_RPC_CLIENTS.lock().unwrap();
    let doge_status = if let Some(client) = clients.get(&crate::external_rpc::ExternalChain::Doge) {
        let client_clone = Arc::clone(client);
        drop(clients);
        match health_check_rpc(&client_clone).await {
            Ok(tip) => serde_json::json!({
                "ok": true,
                "tip": tip
            }),
            Err(e) => serde_json::json!({
                "ok": false,
                "error": format!("{}", e)
            }),
        }
    } else {
        serde_json::json!({
            "ok": false,
            "error": "Not configured",
            "disabled": true
        })
    };

    Json(serde_json::json!({
        "btc": btc_status,
        "bch": bch_status,
        "doge": doge_status
    }))
}
