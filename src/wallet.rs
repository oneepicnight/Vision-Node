#![allow(dead_code)]

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::metrics;
use crate::receipts::{write_receipt, Receipt};

#[derive(Clone)]
pub struct AppState {
    pub dbctx: Arc<metrics::DbCtx>,
    pub metrics: Arc<metrics::Metrics>,
    // include other fields already present in your AppState (p2p, mempool handle, etc.)
}

// --- Balance model ---

#[derive(Debug, Serialize)]
pub struct BalanceResp {
    pub address: String,
    pub balance: String, // decimal string
}

// --- Nonce model ---

#[derive(Debug, Serialize)]
pub struct NonceResp {
    pub address: String,
    pub nonce: u64,
}

// --- Transfer model ---

#[derive(Debug, Deserialize)]
pub struct TransferReq {
    pub from: String,
    pub to: String,
    pub amount: String, // decimal string -> u128
    #[serde(default)]
    pub fee: Option<String>,
    #[serde(default)]
    pub memo: Option<String>,
    /// Client-side signature (hex-encoded 64-byte Ed25519 signature)
    pub signature: String,
    /// Nonce for replay protection (must be exactly expected_nonce + 1)
    pub nonce: u64,
    /// Public key of sender (hex-encoded 32-byte Ed25519 public key)
    pub public_key: String,
}

#[derive(Debug, Serialize)]
pub struct TransferResp {
    pub status: &'static str,
    pub receipt_id: String,
}

// --- Routes ---

/// GET /wallet/:addr/balance
pub async fn get_balance(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> impl IntoResponse {
    // Validate address (best-effort)
    if !is_valid_addr(&addr) {
        return api_err(400, "invalid_address");
    }

    let db = &state.dbctx.db;
    let balances = match db.open_tree("balances") {
        Ok(t) => t,
        Err(e) => return api_err(500, &format!("db_open: {e}")),
    };

    let bal = read_u128_le(&balances, addr.as_bytes()).unwrap_or(0);
    let resp = BalanceResp {
        address: addr,
        balance: bal.to_string(),
    };
    Json(resp).into_response()
}

/// GET /wallet/:addr/nonce
/// Query the current nonce for an address (for replay protection)
pub async fn get_nonce(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> impl IntoResponse {
    // Validate address
    if !is_valid_addr(&addr) {
        return api_err(400, "invalid_address");
    }

    let db = &state.dbctx.db;
    let nonces = match db.open_tree("wallet_nonces") {
        Ok(t) => t,
        Err(e) => return api_err(500, &format!("db_open_nonces: {e}")),
    };

    let nonce = read_u64_le(&nonces, addr.as_bytes()).unwrap_or(0);
    let resp = NonceResp {
        address: addr,
        nonce,
    };
    Json(resp).into_response()
}

/// POST /wallet/transfer
pub async fn post_transfer(
    State(state): State<AppState>,
    Json(req): Json<TransferReq>,
) -> impl IntoResponse {
    // 1. Basic validation
    if !is_valid_addr(&req.from) || !is_valid_addr(&req.to) {
        return api_err(400, "invalid_address");
    }

    let Ok(amount) = parse_u128(&req.amount) else {
        return api_err(400, "invalid_amount");
    };
    let fee_u128 = match &req.fee {
        Some(f) => match parse_u128(f) {
            Ok(v) => v,
            Err(_) => return api_err(400, "invalid_fee"),
        },
        None => 0,
    };

    if amount == 0 {
        return api_err(400, "amount_zero");
    }
    if req.from == req.to {
        return api_err(400, "same_sender_recipient");
    }

    // 2. Verify signature (cryptographic proof of authorization)
    if let Err(e) = verify_transfer_signature(&req) {
        return api_err(401, &format!("signature_verification_failed: {e}"));
    }

    let db = &state.dbctx.db;

    // 3. Verify and increment nonce (replay attack prevention)
    let nonces_tree = match db.open_tree("wallet_nonces") {
        Ok(t) => t,
        Err(e) => return api_err(500, &format!("db_open_nonces: {e}")),
    };

    let current_nonce = read_u64_le(&nonces_tree, req.from.as_bytes()).unwrap_or(0);
    let expected_nonce = current_nonce + 1;

    if req.nonce != expected_nonce {
        return api_err(
            400,
            &format!(
                "invalid_nonce: expected {expected_nonce}, got {}",
                req.nonce
            ),
        );
    }

    // 4. Open balances tree outside transaction
    let balances = match db.open_tree("balances") {
        Ok(t) => t,
        Err(e) => return api_err(500, &format!("db_open: {e}")),
    };

    // 5. ATOMIC TRANSACTION: Ensures all-or-nothing balance updates
    let result = balances.transaction(|tx_balances| {
        // Read current balances within transaction
        let from_bal = tx_balances
            .get(req.from.as_bytes())?
            .map(|v| decode_u128_le(&v))
            .unwrap_or(0);

        if from_bal < amount + fee_u128 {
            // Abort transaction with custom error
            return sled::transaction::abort("insufficient_funds");
        }

        let to_bal = tx_balances
            .get(req.to.as_bytes())?
            .map(|v| decode_u128_le(&v))
            .unwrap_or(0);

        let fee_collector = b"__fees__";
        let fees_bal = tx_balances
            .get(fee_collector)?
            .map(|v| decode_u128_le(&v))
            .unwrap_or(0);

        // Write new balances atomically
        let new_from = from_bal - amount - fee_u128;
        let new_to = to_bal + amount;
        let new_fees = fees_bal + fee_u128;

        tx_balances.insert(req.from.as_bytes(), &new_from.to_le_bytes()[..])?;
        tx_balances.insert(req.to.as_bytes(), &new_to.to_le_bytes()[..])?;
        tx_balances.insert(fee_collector, &new_fees.to_le_bytes()[..])?;

        Ok(())
    });

    // 6. Handle transaction result
    if let Err(e) = result {
        let msg = match e {
            sled::transaction::TransactionError::Abort(ref s) => s.to_string(),
            sled::transaction::TransactionError::Storage(ref err) => format!("storage: {err}"),
        };
        let code = if msg == "insufficient_funds" {
            402
        } else {
            500
        };
        return api_err(code, &msg);
    }

    // 7. Increment nonce AFTER successful transfer (commit point)
    if let Err(e) = write_u64_le(&nonces_tree, req.from.as_bytes(), expected_nonce) {
        eprintln!("[wallet] failed to write nonce: {e}");
        return api_err(500, "nonce_write_failed");
    }

    // 8. Update Prometheus metrics
    state.metrics.wallet_transfers_total.inc();
    state
        .metrics
        .wallet_transfer_volume
        .inc_by(amount.min(u64::MAX as u128) as u64);
    state
        .metrics
        .wallet_fees_collected
        .inc_by(fee_u128.min(u64::MAX as u128) as u64);

    // 9. Emit receipt (best-effort; do not fail transfer if receipt write fails)
    let rec = Receipt {
        id: String::new(),
        ts_ms: 0,
        kind: "transfer".to_string(),
        from: req.from.clone(),
        to: req.to.clone(),
        amount: amount.to_string(),
        fee: fee_u128.to_string(),
        memo: req.memo.clone(),
        txid: None,
        ok: true,
        note: None,
    };
    if let Err(e) = write_receipt(&state.dbctx.db, Some(&state.metrics), rec) {
        eprintln!("[wallet] write_receipt error: {e}");
        // continue
    }

    let resp = TransferResp {
        status: "ok",
        receipt_id: "latest".to_string(),
    };
    Json(resp).into_response()
}

// --- Helpers ---

/// Verify Ed25519 signature for a transfer request
///
/// Follows the same pattern as main blockchain tx verification:
/// 1. Decode public key and signature from hex
/// 2. Verify public key derives to the 'from' address
/// 3. Construct signable message (canonical representation)
/// 4. Verify Ed25519 signature
fn verify_transfer_signature(req: &TransferReq) -> Result<(), String> {
    // 1. Decode public key (32 bytes)
    let pubkey_bytes =
        decode_hex32(&req.public_key).map_err(|e| format!("invalid_public_key: {e}"))?;

    let pubkey = VerifyingKey::from_bytes(&pubkey_bytes)
        .map_err(|e| format!("invalid_public_key_format: {e}"))?;

    // 2. Verify public key derives to 'from' address
    let derived_addr = hex::encode(pubkey_bytes);
    if derived_addr != req.from {
        return Err(format!(
            "public_key_mismatch: derived {derived_addr}, expected {}",
            req.from
        ));
    }

    // 3. Decode signature (64 bytes)
    let sig_bytes = decode_hex64(&req.signature).map_err(|e| format!("invalid_signature: {e}"))?;

    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| format!("invalid_signature_bytes"))?;
    let signature = Signature::from_bytes(&sig_array);

    // 4. Construct canonical message to sign
    let message = signable_transfer_bytes(req);

    // 5. Verify signature
    pubkey
        .verify(&message, &signature)
        .map_err(|e| format!("signature_verification_failed: {e}"))?;

    Ok(())
}

/// Construct canonical message for signing a transfer
///
/// Message format (prevents replay attacks and ambiguity):
/// - from (32 bytes, raw)
/// - to (32 bytes, raw)
/// - amount (16 bytes, LE)
/// - fee (16 bytes, LE)
/// - nonce (8 bytes, LE)
/// - memo (optional, UTF-8 bytes)
fn signable_transfer_bytes(req: &TransferReq) -> Vec<u8> {
    let mut msg = Vec::with_capacity(128);

    // from address (32 bytes hex -> 32 bytes raw)
    if let Ok(from_bytes) = hex::decode(&req.from) {
        msg.extend_from_slice(&from_bytes);
    }

    // to address (32 bytes hex -> 32 bytes raw)
    if let Ok(to_bytes) = hex::decode(&req.to) {
        msg.extend_from_slice(&to_bytes);
    }

    // amount (u128 LE)
    if let Ok(amt) = parse_u128(&req.amount) {
        msg.extend_from_slice(&amt.to_le_bytes());
    }

    // fee (u128 LE)
    let fee = req
        .fee
        .as_ref()
        .and_then(|f| parse_u128(f).ok())
        .unwrap_or(0);
    msg.extend_from_slice(&fee.to_le_bytes());

    // nonce (u64 LE)
    msg.extend_from_slice(&req.nonce.to_le_bytes());

    // memo (optional UTF-8)
    if let Some(ref memo) = req.memo {
        msg.extend_from_slice(memo.as_bytes());
    }

    msg
}

/// Decode 32-byte hex string to fixed array
fn decode_hex32(s: &str) -> Result<[u8; 32], String> {
    let v = hex::decode(s).map_err(|e| e.to_string())?;
    if v.len() != 32 {
        return Err(format!("expected 32 bytes, got {}", v.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&v);
    Ok(arr)
}

/// Decode 64-byte hex string to fixed array
fn decode_hex64(s: &str) -> Result<[u8; 64], String> {
    let v = hex::decode(s).map_err(|e| e.to_string())?;
    if v.len() != 64 {
        return Err(format!("expected 64 bytes, got {}", v.len()));
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&v);
    Ok(arr)
}

fn is_valid_addr(s: &str) -> bool {
    // Vision addresses are 64-character hex strings (32 bytes)
    if s.len() != 64 {
        return false;
    }
    // Verify all characters are valid hex
    s.chars().all(|c| c.is_ascii_hexdigit())
}

fn parse_u128(s: &str) -> Result<u128, ()> {
    s.parse::<u128>().map_err(|_| ())
}

fn read_u128_le(tree: &sled::Tree, key: &[u8]) -> anyhow::Result<u128> {
    if let Some(ivec) = tree.get(key)? {
        Ok(decode_u128_le(ivec.as_ref()))
    } else {
        Ok(0)
    }
}

fn write_u128_le(tree: &sled::Tree, key: &[u8], v: u128) -> anyhow::Result<()> {
    let mut buf = [0u8; 16];
    buf.copy_from_slice(&v.to_le_bytes());
    tree.insert(key, buf.to_vec())?;
    Ok(())
}

fn decode_u128_le(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    let take = bytes.len().min(16);
    buf[..take].copy_from_slice(&bytes[..take]);
    u128::from_le_bytes(buf)
}

/// Read u64 from sled tree (LE encoding)
fn read_u64_le(tree: &sled::Tree, key: &[u8]) -> anyhow::Result<u64> {
    if let Some(ivec) = tree.get(key)? {
        Ok(decode_u64_le(ivec.as_ref()))
    } else {
        Ok(0)
    }
}

/// Write u64 to sled tree (LE encoding)
fn write_u64_le(tree: &sled::Tree, key: &[u8], v: u64) -> anyhow::Result<()> {
    tree.insert(key, v.to_le_bytes().to_vec())?;
    Ok(())
}

/// Decode u64 from LE bytes
fn decode_u64_le(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let take = bytes.len().min(8);
    buf[..take].copy_from_slice(&bytes[..take]);
    u64::from_le_bytes(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};

    #[test]
    fn test_decode_hex32_valid() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = decode_hex32(hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_decode_hex32_invalid_length() {
        let hex = "0123456789abcdef"; // Only 16 chars = 8 bytes
        let result = decode_hex32(hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 32 bytes"));
    }

    #[test]
    fn test_decode_hex64_valid() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\
                   0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = decode_hex64(hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn test_decode_hex64_invalid_length() {
        let hex = "0123456789abcdef0123456789abcdef"; // Only 32 chars = 16 bytes
        let result = decode_hex64(hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 64 bytes"));
    }

    #[test]
    fn test_is_valid_addr() {
        // Valid 64-char hex
        assert!(is_valid_addr(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));

        // Invalid: too short
        assert!(!is_valid_addr("0123456789abcdef"));

        // Invalid: too long
        assert!(!is_valid_addr(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef00"
        ));

        // Invalid: non-hex character
        assert!(!is_valid_addr(
            "0123456789abcdefg123456789abcdef0123456789abcdef0123456789abcde"
        ));
    }

    #[test]
    fn test_parse_u128() {
        assert_eq!(parse_u128("0"), Ok(0));
        assert_eq!(parse_u128("12345"), Ok(12345));
        assert_eq!(
            parse_u128("340282366920938463463374607431768211455"),
            Ok(u128::MAX)
        );
        assert!(parse_u128("invalid").is_err());
        assert!(parse_u128("340282366920938463463374607431768211456").is_err());
        // Overflow
    }

    #[test]
    fn test_decode_u128_le() {
        // Test zero
        let bytes = [0u8; 16];
        assert_eq!(decode_u128_le(&bytes), 0);

        // Test max value
        let bytes = [0xFF; 16];
        assert_eq!(decode_u128_le(&bytes), u128::MAX);

        // Test specific value (5000 in LE)
        let bytes = 5000u128.to_le_bytes();
        assert_eq!(decode_u128_le(&bytes), 5000);
    }

    #[test]
    fn test_decode_u64_le() {
        // Test zero
        let bytes = [0u8; 8];
        assert_eq!(decode_u64_le(&bytes), 0);

        // Test max value
        let bytes = [0xFF; 8];
        assert_eq!(decode_u64_le(&bytes), u64::MAX);

        // Test specific value (12345 in LE)
        let bytes = 12345u64.to_le_bytes();
        assert_eq!(decode_u64_le(&bytes), 12345);
    }

    #[test]
    fn test_signable_transfer_bytes_deterministic() {
        let req1 = TransferReq {
            from: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: Some("test".to_string()),
            signature: String::new(),
            nonce: 1,
            public_key: String::new(),
        };

        let req2 = TransferReq {
            from: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: Some("test".to_string()),
            signature: String::new(),
            nonce: 1,
            public_key: String::new(),
        };

        // Same inputs should produce identical messages
        let msg1 = signable_transfer_bytes(&req1);
        let msg2 = signable_transfer_bytes(&req2);
        assert_eq!(msg1, msg2);

        // Expected length: 32 (from) + 32 (to) + 16 (amount) + 16 (fee) + 8 (nonce) + 4 (memo) = 108
        assert_eq!(msg1.len(), 108);
    }

    #[test]
    fn test_signable_transfer_bytes_different_nonces() {
        let req1 = TransferReq {
            from: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1,
            public_key: String::new(),
        };

        let req2 = TransferReq {
            from: "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 2,
            public_key: String::new(),
        };

        // Different nonces should produce different messages
        let msg1 = signable_transfer_bytes(&req1);
        let msg2 = signable_transfer_bytes(&req2);
        assert_ne!(msg1, msg2);
    }

    #[test]
    fn test_verify_transfer_signature_valid() {
        // Generate a test keypair
        let mut rng = rand::rngs::OsRng;
        let keypair = Keypair::generate(&mut rng);
        let public_key_hex = hex::encode(keypair.public.as_bytes());

        // Create a transfer request (without signature yet)
        let mut req = TransferReq {
            from: public_key_hex.clone(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: Some("test".to_string()),
            signature: String::new(), // Will set below
            nonce: 1,
            public_key: public_key_hex.clone(),
        };

        // Sign the message
        let message = signable_transfer_bytes(&req);
        let signature = keypair.sign(&message);
        req.signature = hex::encode(signature.to_bytes());

        // Verification should pass
        let result = verify_transfer_signature(&req);
        assert!(
            result.is_ok(),
            "Signature verification should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_verify_transfer_signature_invalid_signature() {
        // Generate a test keypair
        let mut rng = rand::rngs::OsRng;
        let keypair = Keypair::generate(&mut rng);
        let public_key_hex = hex::encode(keypair.public.as_bytes());

        let req = TransferReq {
            from: public_key_hex.clone(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: Some("test".to_string()),
            signature: "0".repeat(128), // Invalid signature
            nonce: 1,
            public_key: public_key_hex,
        };

        // Verification should fail
        let result = verify_transfer_signature(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("signature"));
    }

    #[test]
    fn test_verify_transfer_signature_wrong_key() {
        // Generate two keypairs
        let mut rng = rand::rngs::OsRng;
        let keypair1 = Keypair::generate(&mut rng);
        let keypair2 = Keypair::generate(&mut rng);

        let public_key1_hex = hex::encode(keypair1.public.as_bytes());
        let public_key2_hex = hex::encode(keypair2.public.as_bytes());

        // Create request claiming to be from address 1
        let mut req = TransferReq {
            from: public_key1_hex.clone(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: None,
            signature: String::new(),
            nonce: 1,
            public_key: public_key2_hex, // But using public key 2
        };

        // Sign with keypair2
        let message = signable_transfer_bytes(&req);
        let signature = keypair2.sign(&message);
        req.signature = hex::encode(signature.to_bytes());

        // Verification should fail (public key mismatch)
        let result = verify_transfer_signature(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("public_key_mismatch"));
    }

    #[test]
    fn test_verify_transfer_signature_tampered_message() {
        // Generate a test keypair
        let mut rng = rand::rngs::OsRng;
        let keypair = Keypair::generate(&mut rng);
        let public_key_hex = hex::encode(keypair.public.as_bytes());

        // Create and sign a transfer
        let mut req = TransferReq {
            from: public_key_hex.clone(),
            to: "0000000000000000000000000000000000000000000000000000000000000002".to_string(),
            amount: "5000".to_string(),
            fee: Some("50".to_string()),
            memo: Some("original".to_string()),
            signature: String::new(),
            nonce: 1,
            public_key: public_key_hex.clone(),
        };

        let message = signable_transfer_bytes(&req);
        let signature = keypair.sign(&message);
        req.signature = hex::encode(signature.to_bytes());

        // Tamper with the amount after signing
        req.amount = "10000".to_string();

        // Verification should fail (message doesn't match signature)
        let result = verify_transfer_signature(&req);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("signature_verification_failed"));
    }
}

// Uniform JSON error per api_error_schema.md
fn api_err(code: u16, err: &str) -> axum::response::Response {
    use axum::http::{HeaderValue, StatusCode};
    use serde_json::json;

    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = json!({
        "status": "rejected",
        "code": code,
        "error": err
    })
    .to_string();

    let mut resp = (status, body).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}

