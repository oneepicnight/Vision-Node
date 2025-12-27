// tests are below to avoid interfering with module-level inner doc comments
//! Wallet route handlers
//!
//! Provides HTTP handlers for:
//! - GET /wallet/:addr/balance - Query token balance
//! - GET /wallet/:addr/nonce - Query current nonce for replay protection
//! - POST /wallet/transfer - Transfer tokens between addresses

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::{wallet, DB_CTX, PROM_METRICS};

/// Handler for GET /wallet/:addr/balance
///
/// Queries the balance for a given address from the 'balances' tree.
/// Returns JSON with balance as string (u128).
pub async fn wallet_balance_handler(Path(addr): Path<String>) -> impl IntoResponse {
    let state = wallet::AppState {
        dbctx: DB_CTX.clone(),
        metrics: PROM_METRICS.clone(),
    };
    wallet::get_balance(State(state), Path(addr)).await
}

/// Handler for GET /wallet/:addr/nonce
///
/// Queries the current nonce for a given address from the 'wallet_nonces' tree.
/// Returns JSON with nonce as u64. Used by clients for replay protection.
pub async fn wallet_nonce_handler(Path(addr): Path<String>) -> impl IntoResponse {
    let state = wallet::AppState {
        dbctx: DB_CTX.clone(),
        metrics: PROM_METRICS.clone(),
    };
    wallet::get_nonce(State(state), Path(addr)).await
}

/// Handler for POST /wallet/transfer
///
/// Executes an atomic token transfer from one address to another.
/// Deducts amount + fee from sender, credits amount to recipient,
/// credits fee to __fees__ account, and writes a receipt.
///
/// Request body: TransferReq { from, to, amount, fee }
pub async fn wallet_transfer_handler(Json(req): Json<wallet::TransferReq>) -> impl IntoResponse {
    let state = wallet::AppState {
        dbctx: DB_CTX.clone(),
        metrics: PROM_METRICS.clone(),
    };
    wallet::post_transfer(State(state), Json(req)).await
}

// Unit tests for the router handlers
#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{DbCtx, Metrics};
    use axum::extract::State as AxumState;
    use axum::http::Request as AxumRequest;
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum::routing::post;
    use axum::{body::Body, Router};
    use ed25519_dalek::Signer;
    use serde_json::json;
    use sled::Config;
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn router_post_transfer_accepts_valid_signature() {
        // Build temporary sled DB and metrics
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());

        // Prepare an AppState for the handler
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Seed recipient key and sender key
        let balances = db.open_tree("balances").unwrap();
        let recipient_key = hex::encode([0x02u8; 32]);

        // Prepare a keypair and sign a transfer
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());

        // Insert sender balance and nonce
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Build the TransferReq and sign it
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "50000".to_string(),
            fee: Some("100".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };

        // Manually construct the signable bytes (match signable_transfer_bytes)
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        if let Some(ref memo) = req.memo {
            sign_msg.extend_from_slice(memo.as_bytes());
        }
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        // Call the handler directly (no HTTP router) so we can test behavior in-process
        let response = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        let status = response.status();
        assert!(status.is_success(), "status not success: {}", status);

        // Validate sender/recipient balances updated in DB
        let sender_final = balances
            .get(sender_addr.as_bytes())
            .unwrap()
            .map(|v| {
                let mut buf = [0u8; 16];
                buf.copy_from_slice(&v[..]);
                u128::from_le_bytes(buf)
            })
            .unwrap_or(0);
        let expected_sender = 100_000u128 - 50_000u128 - 100u128;
        assert_eq!(sender_final, expected_sender);

        let recip_final = balances
            .get(recipient_key.as_bytes())
            .unwrap()
            .map(|v| {
                let mut buf = [0u8; 16];
                buf.copy_from_slice(&v[..]);
                u128::from_le_bytes(buf)
            })
            .unwrap_or(0);
        assert_eq!(recip_final, 50_000u128);
    }

    #[tokio::test]
    async fn oneshot_router_transfer_accepts_valid_signature() {
        // Setup DB + metrics + state
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Seed DB
        let balances = db.open_tree("balances").unwrap();
        let recipient_key = hex::encode([0x02u8; 32]);

        // Create a keypair and seed sender balance + nonce
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Construct JSON request body and sign
        let req_json = json!({
            "from": sender_addr,
            "to": recipient_key,
            "amount": 50000u128.to_string(),
            "fee": 100u128.to_string(),
            "nonce": 1u64,
            "public_key": hex::encode(keypair.verifying_key().as_bytes()),
        });

        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(req_json["from"].as_str().unwrap()).unwrap());
        sign_msg.extend_from_slice(&hex::decode(req_json["to"].as_str().unwrap()).unwrap());
        sign_msg.extend_from_slice(&(50000u128).to_le_bytes());
        sign_msg.extend_from_slice(&(100u128).to_le_bytes());
        sign_msg.extend_from_slice(&1u64.to_le_bytes());
        let signature = keypair.sign(&sign_msg);

        // Add signature to req_json
        let mut req_body = req_json.clone();
        req_body["signature"] = json!(hex::encode(signature.to_bytes()));

        // Build router and oneshot request
        let app = Router::new()
            .route("/wallet/transfer", post(wallet::post_transfer))
            .with_state(state);

        let req = AxumRequest::builder()
            .method("POST")
            .uri("/wallet/transfer")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert!(resp.status().is_success());

        // Check balances were updated
        let sender_final = balances
            .get(hex::encode(keypair.verifying_key().as_bytes()))
            .unwrap()
            .map(|v| {
                let mut buf = [0u8; 16];
                buf.copy_from_slice(&v[..]);
                u128::from_le_bytes(buf)
            })
            .unwrap_or(0);
        let expected_sender = 100_000u128 - 50_000u128 - 100u128;
        assert_eq!(sender_final, expected_sender);
    }

    #[tokio::test]
    async fn oneshot_router_rejects_invalid_signature() {
        // Setup DB + metrics + state
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let balances = db.open_tree("balances").unwrap();
        let mut rng = rand::rngs::OsRng;
        let legit_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let wrong_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(legit_kp.verifying_key().as_bytes());
        let recipient_key = hex::encode([0xAAu8; 32]);
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Build JSON body signed with wrong keypair
        let mut req_json = json!({
            "from": sender_addr.clone(),
            "to": recipient_key.clone(),
            "amount": 1000u128.to_string(),
            "fee": 1u128.to_string(),
            "nonce": 1u64,
            "public_key": hex::encode(legit_kp.public.as_bytes()),
        });

        // Sign message with wrong key
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req_json["from"].as_str().unwrap()).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req_json["to"].as_str().unwrap()).unwrap());
        sign_msg.extend_from_slice(&(1000u128).to_le_bytes());
        sign_msg.extend_from_slice(&(1u128).to_le_bytes());
        sign_msg.extend_from_slice(&1u64.to_le_bytes());
        let sig = wrong_kp.sign(&sign_msg);
        req_json["signature"] = serde_json::Value::String(hex::encode(sig.to_bytes()));

        // Build router and oneshot request
        let app = Router::new()
            .route("/wallet/transfer", post(wallet::post_transfer))
            .with_state(state);
        let request = AxumRequest::builder()
            .method("POST")
            .uri("/wallet/transfer")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&req_json).unwrap()))
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_invalid_signature() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Prepare keypairs and seed
        let mut rng = rand::rngs::OsRng;
        let legit_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let wrong_kp = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(legit_kp.verifying_key().as_bytes());
        let recipient_key = hex::encode([0xABu8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Build TransferReq signed with wrong key
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "1000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(legit_kp.public.as_bytes()),
        };
        // Sign with wrong key
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        if let Some(ref memo) = req.memo {
            sign_msg.extend_from_slice(memo.as_bytes());
        }
        let signature = wrong_kp.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let response = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(response.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_invalid_nonce() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Setup
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x03u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Build request with wrong nonce (2 instead of 1)
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "1000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 2u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        if let Some(ref memo) = req.memo {
            sign_msg.extend_from_slice(memo.as_bytes());
        }
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_insufficient_funds() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Setup keypair and minimal balance
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x04u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        // Seed only 100 tokens
        balances
            .insert(&sender_addr, &100u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Attempt to transfer 10_000
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "10000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        if let Some(ref memo) = req.memo {
            sign_msg.extend_from_slice(memo.as_bytes());
        }
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_malformed_signature() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Setup basic accounts
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x05u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Create a request with malformed signature hex (not 128 chars)
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "1000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: "deadbeef".to_string(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_invalid_public_key_format() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x06u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Use a short public_key string (invalid) but sign correctly to ensure signature verification fails at pubkey parse stage
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "1000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: "deadbeef".to_string(),
        };
        // Create a signature to attach (from real key)
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_same_sender_recipient() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Same from/to
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: sender_addr.clone(),
            amount: "1000".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_rejects_amount_zero() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x07u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Build request with amount = 0
        let req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "0".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };

        let resp = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_client_error());
    }

    #[tokio::test]
    async fn handler_persists_receipt_on_success() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x08u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "500".to_string(),
            fee: Some("2".to_string()),
            memo: Some("receipt test".to_string()),
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        if let Some(ref memo) = req.memo {
            sign_msg.extend_from_slice(memo.as_bytes());
        }
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let response = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(response.status().is_success());

        // Check receipts tree contains at least one record with expected data
        let tree = db.open_tree("receipts").unwrap();
        let mut found = false;
        for item in tree.iter().rev() {
            let (_k, v) = item.unwrap();
            if let Ok(rec) = bincode::deserialize::<crate::receipts::Receipt>(&v) {
                if rec.kind == "transfer" && rec.from == sender_addr && rec.to == recipient_key {
                    assert_eq!(rec.amount, "500");
                    assert_eq!(rec.fee, "2");
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "Receipt not found in receipts tree");
    }

    #[tokio::test]
    async fn handler_increments_metrics_on_transfer() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x09u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &100_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Read initial counters
        fn read_counter_val(m: &Metrics, name: &str) -> u64 {
            match name {
                "wallet_transfers_total" => m.wallet_transfers_total.get(),
                "wallet_transfer_volume" => m.wallet_transfer_volume.get(),
                "wallet_fees_collected" => m.wallet_fees_collected.get(),
                "wallet_receipts_written" => m.wallet_receipts_written.get(),
                _ => 0,
            }
        }

        let before_transfers = read_counter_val(&metrics, "wallet_transfers_total");
        let before_volume = read_counter_val(&metrics, "wallet_transfer_volume");
        let before_fees = read_counter_val(&metrics, "wallet_fees_collected");
        let before_receipts = read_counter_val(&metrics, "wallet_receipts_written");

        // Perform a transfer
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "250".to_string(),
            fee: Some("5".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        let signature = keypair.sign(&sign_msg);
        req.signature = hex::encode(signature.to_bytes());

        let response = wallet::post_transfer(AxumState(state), axum::Json(req))
            .await
            .into_response();
        assert!(response.status().is_success());

        // Re-read counters
        let after_transfers = read_counter_val(&metrics, "wallet_transfers_total");
        let after_volume = read_counter_val(&metrics, "wallet_transfer_volume");
        let after_fees = read_counter_val(&metrics, "wallet_fees_collected");
        let after_receipts = read_counter_val(&metrics, "wallet_receipts_written");

        assert!(after_transfers > before_transfers);
        assert!(after_volume >= before_volume + 250);
        assert!(after_fees >= before_fees + 5);
        assert!(after_receipts > before_receipts);
    }

    #[tokio::test]
    async fn nonce_advanced_behavior_sequential_transfers() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Prepare keypair, balances and nonces
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x10u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &1_000_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Execute 3 sequential transfers using nonce 1,2,3
        for i in 1..=3u64 {
            let mut req = wallet::TransferReq {
                from: sender_addr.clone(),
                to: recipient_key.clone(),
                amount: "100".to_string(),
                fee: Some("1".to_string()),
                memo: Some(format!("seq-{}", i)),
                signature: String::new(),
                nonce: i,
                public_key: hex::encode(keypair.verifying_key().as_bytes()),
            };
            // sign
            let mut sign_msg = Vec::new();
            sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
            sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
            sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
            let fee_val: u128 = req
                .fee
                .as_ref()
                .and_then(|f| f.parse::<u128>().ok())
                .unwrap_or(0);
            sign_msg.extend_from_slice(&fee_val.to_le_bytes());
            sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
            if let Some(ref memo) = req.memo {
                sign_msg.extend_from_slice(memo.as_bytes());
            }
            let sig = keypair.sign(&sign_msg);
            req.signature = hex::encode(sig.to_bytes());

            let resp = wallet::post_transfer(AxumState(state.clone()), axum::Json(req))
                .await
                .into_response();
            assert!(
                resp.status().is_success(),
                "transfer {} failed: {:?}",
                i,
                resp.status()
            );
        }

        // Attempt to reuse nonce 2, should fail
        let mut req_reuse = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "50".to_string(),
            fee: Some("1".to_string()),
            memo: Some("reuse".to_string()),
            signature: String::new(),
            nonce: 2u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut msg_reuse = Vec::new();
        msg_reuse.extend_from_slice(&hex::decode(&req_reuse.from).unwrap());
        msg_reuse.extend_from_slice(&hex::decode(&req_reuse.to).unwrap());
        msg_reuse.extend_from_slice(&(req_reuse.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val: u128 = req_reuse
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        msg_reuse.extend_from_slice(&fee_val.to_le_bytes());
        msg_reuse.extend_from_slice(&req_reuse.nonce.to_le_bytes());
        if let Some(ref memo) = req_reuse.memo {
            msg_reuse.extend_from_slice(memo.as_bytes());
        }
        let sig2 = keypair.sign(&msg_reuse);
        req_reuse.signature = hex::encode(sig2.to_bytes());
        let resp_reuse = wallet::post_transfer(AxumState(state.clone()), axum::Json(req_reuse))
            .await
            .into_response();
        assert!(
            resp_reuse.status().is_client_error(),
            "expected client error for duplicate nonce"
        );

        // Check final nonce is 3
        // Read nonce directly from sled (LE u64)
        fn read_u64_from_tree(tree: &sled::Tree, key: &[u8]) -> u64 {
            if let Ok(Some(v)) = tree.get(key) {
                let mut buf = [0u8; 8];
                let take = v.len().min(8);
                buf[..take].copy_from_slice(&v[..take]);
                u64::from_le_bytes(buf)
            } else {
                0
            }
        }
        let final_nonce = read_u64_from_tree(&nonces, sender_addr.as_bytes());
        assert_eq!(final_nonce, 3);
    }

    #[tokio::test]
    async fn receipts_latest_pagination_and_ordering() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let wallet_state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };
        let receipts_state = crate::receipts::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Prepare keypair and seed
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x11u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &10_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();

        // Make 4 transfers with identifiable memos so we can check ordering
        for i in 1..=4u64 {
            let mut req = wallet::TransferReq {
                from: sender_addr.clone(),
                to: recipient_key.clone(),
                amount: "1".to_string(),
                fee: Some("0".to_string()),
                memo: Some(format!("rtest-{}", i)),
                signature: String::new(),
                nonce: i,
                public_key: hex::encode(keypair.verifying_key().as_bytes()),
            };
            let mut sign_msg = Vec::new();
            sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
            sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
            sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
            let fee_val: u128 = req
                .fee
                .as_ref()
                .and_then(|f| f.parse::<u128>().ok())
                .unwrap_or(0);
            sign_msg.extend_from_slice(&fee_val.to_le_bytes());
            sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
            if let Some(ref memo) = req.memo {
                sign_msg.extend_from_slice(memo.as_bytes());
            }
            let s = keypair.sign(&sign_msg);
            req.signature = hex::encode(s.to_bytes());
            let resp = wallet::post_transfer(AxumState(wallet_state.clone()), axum::Json(req))
                .await
                .into_response();
            assert!(resp.status().is_success());
        }

        // Build router for receipts endpoint and query latest limit 2
        let app = Router::new()
            .route("/receipts/latest", get(crate::receipts::get_latest))
            .with_state(receipts_state);
        // Request limit=2
        let request = AxumRequest::builder()
            .method("GET")
            .uri("/receipts/latest?limit=2")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(request).await.unwrap();
        assert!(resp.status().is_success());
        let body_bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let receipts = json.as_array().expect("receipts array");
        assert_eq!(receipts.len(), 2);
        // newest first: the last transfer had memo rtest-4, so receipts[0] should match
        assert_eq!(receipts[0]["memo"].as_str().unwrap(), "rtest-4");
        assert_eq!(receipts[1]["memo"].as_str().unwrap(), "rtest-3");
    }

    #[tokio::test]
    async fn metrics_http_endpoint_exposes_counters() {
        let db = Config::new().temporary(true).open().unwrap();
        let dbctx = Arc::new(DbCtx { db: db.clone() });
        let metrics = Arc::new(Metrics::new());
        let state = wallet::AppState {
            dbctx: dbctx.clone(),
            metrics: metrics.clone(),
        };

        // Seed a transfer to change counters
        let mut rng = rand::rngs::OsRng;
        let keypair = ed25519_dalek::SigningKey::generate(&mut rng);
        let sender_addr = hex::encode(keypair.verifying_key().as_bytes());
        let recipient_key = hex::encode([0x12u8; 32]);
        let balances = db.open_tree("balances").unwrap();
        balances
            .insert(&sender_addr, &10_000u128.to_le_bytes()[..])
            .unwrap();
        let nonces = db.open_tree("wallet_nonces").unwrap();
        nonces
            .insert(&sender_addr, &0u64.to_le_bytes()[..])
            .unwrap();
        let mut req = wallet::TransferReq {
            from: sender_addr.clone(),
            to: recipient_key.clone(),
            amount: "123".to_string(),
            fee: Some("1".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1u64,
            public_key: hex::encode(keypair.verifying_key().as_bytes()),
        };
        let mut sign_msg = Vec::new();
        sign_msg.extend_from_slice(&hex::decode(&req.from).unwrap());
        sign_msg.extend_from_slice(&hex::decode(&req.to).unwrap());
        sign_msg.extend_from_slice(&(req.amount.parse::<u128>().unwrap()).to_le_bytes());
        let fee_val = req
            .fee
            .as_ref()
            .and_then(|f| f.parse::<u128>().ok())
            .unwrap_or(0);
        sign_msg.extend_from_slice(&fee_val.to_le_bytes());
        sign_msg.extend_from_slice(&req.nonce.to_le_bytes());
        let sig = keypair.sign(&sign_msg);
        req.signature = hex::encode(sig.to_bytes());
        let resp = wallet::post_transfer(AxumState(state.clone()), axum::Json(req))
            .await
            .into_response();
        assert!(resp.status().is_success());

        // Now call metrics::metrics_handler directly and assert text payload contains metrics
        let resp = crate::metrics::metrics_handler(db, metrics.clone())
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(resp.into_body(), 65536).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        assert!(
            body_str.contains("wallet_transfers_total")
                || body_str.contains("vision_wallet_transfers_total")
        );
        assert!(
            body_str.contains("wallet_transfer_volume")
                || body_str.contains("vision_wallet_transfer_volume")
        );
        assert!(
            body_str.contains("wallet_fees_collected")
                || body_str.contains("vision_wallet_fees_collected")
        );
        assert!(
            body_str.contains("wallet_receipts_written")
                || body_str.contains("vision_wallet_receipts_written")
        );
    }
}

