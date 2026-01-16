//! Node Approval API
//!
//! HTTP endpoints for wallet-signed node approval

use crate::identity::{local_fingerprint, local_node_id, local_pubkey_b64};
use crate::node_approval::{ApprovalSubmitRequest, NodeApproval};
use axum::{http::StatusCode, Json};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Verify Ed25519 signature for a message using base64-encoded signature and pubkey
fn verify_ed25519_b64(pubkey_b64: &str, message: &[u8], sig_b64: &str) -> Result<bool, String> {
    // Decode public key from base64
    let pubkey_bytes = general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| format!("Invalid pubkey base64: {}", e))?;

    if pubkey_bytes.len() != 32 {
        return Err(format!(
            "Public key must be 32 bytes, got {}",
            pubkey_bytes.len()
        ));
    }

    let mut pubkey_array = [0u8; 32];
    pubkey_array.copy_from_slice(&pubkey_bytes);

    let pubkey = VerifyingKey::from_bytes(&pubkey_array)
        .map_err(|e| format!("Invalid Ed25519 public key: {}", e))?;

    // Decode signature from base64
    let sig_bytes = general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|e| format!("Invalid signature base64: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err(format!(
            "Signature must be 64 bytes, got {}",
            sig_bytes.len()
        ));
    }

    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| format!("Invalid signature length"))?;
    let signature = Signature::from_bytes(&sig_array);

    // Verify signature
    match pubkey.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// GET /api/node/approval/status - Get node approval status
pub async fn get_approval_status() -> Json<ApprovalStatusResponse> {
    let node_id = local_node_id();
    let node_pubkey_b64 = local_pubkey_b64();
    let pubkey_fingerprint = local_fingerprint();

    match NodeApproval::load() {
        Ok(Some(approval)) => {
            // Verify approval is still valid
            match approval.verify(&node_id, &node_pubkey_b64) {
                Ok(_) => Json(ApprovalStatusResponse {
                    approved: true,
                    wallet_address: Some(approval.wallet_address),
                    node_id,
                    node_pubkey_b64,
                    pubkey_fingerprint,
                    last_error: None,
                }),
                Err(e) => Json(ApprovalStatusResponse {
                    approved: false,
                    wallet_address: None,
                    node_id,
                    node_pubkey_b64,
                    pubkey_fingerprint,
                    last_error: Some(format!("Approval invalid: {}", e)),
                }),
            }
        }
        Ok(None) => Json(ApprovalStatusResponse {
            approved: false,
            wallet_address: None,
            node_id,
            node_pubkey_b64,
            pubkey_fingerprint,
            last_error: None,
        }),
        Err(e) => Json(ApprovalStatusResponse {
            approved: false,
            wallet_address: None,
            node_id,
            node_pubkey_b64,
            pubkey_fingerprint,
            last_error: Some(format!("Failed to load approval: {}", e)),
        }),
    }
}

/// POST /api/node/approval/submit - Submit wallet-signed node approval
pub async fn submit_approval(
    Json(req): Json<ApprovalSubmitRequest>,
) -> Result<Json<ApprovalSubmitResponse>, (StatusCode, String)> {
    let node_id = local_node_id();
    let node_pubkey_b64 = local_pubkey_b64();

    // Build canonical message
    let message = NodeApproval::build_canonical_message(
        &req.wallet_address,
        &node_id,
        &node_pubkey_b64,
        req.ts_unix,
        &req.nonce_hex,
    );

    // Get wallet's public key from chain state to verify signature
    let wallet_pubkey = {
        let chain = crate::CHAIN.lock();
        chain
            .db
            .get(format!("wallet_pubkey:{}", req.wallet_address).as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
    };

    let wallet_pubkey = wallet_pubkey.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Wallet {} has no registered public key. Please create wallet first.",
                req.wallet_address
            ),
        )
    })?;

    // Verify Ed25519 signature
    match verify_ed25519_b64(&wallet_pubkey, message.as_bytes(), &req.signature_b64) {
        Ok(true) => {
            // Signature valid - create and save approval
            let approval = NodeApproval {
                wallet_address: req.wallet_address.clone(),
                node_id: node_id.clone(),
                node_pubkey_b64: node_pubkey_b64.clone(),
                ts_unix: req.ts_unix,
                nonce_hex: req.nonce_hex.clone(),
                signature_b64: req.signature_b64.clone(),
            };

            // Verify approval constraints (timestamp, node_id match, etc.)
            if let Err(e) = approval.verify(&node_id, &node_pubkey_b64) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Approval validation failed: {}", e),
                ));
            }

            // Save approval
            if let Err(e) = approval.save() {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save approval: {}", e),
                ));
            }

            // Bind wallet to node as primary wallet
            if let Err(e) = crate::bind_wallet_to_node(&req.wallet_address) {
                tracing::warn!("⚠️ Failed to bind wallet to node: {}", e);
                // Don't fail the approval - wallet might already be bound
            }

            tracing::info!("✅ Node approved by wallet: {}", req.wallet_address);

            Ok(Json(ApprovalSubmitResponse {
                success: true,
                message: "Node approval saved successfully".to_string(),
                wallet_address: req.wallet_address,
                node_id,
            }))
        }
        Ok(false) => Err((
            StatusCode::UNAUTHORIZED,
            "Invalid wallet signature".to_string(),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Signature verification failed: {}", e),
        )),
    }
}

#[derive(Serialize)]
pub struct ApprovalStatusResponse {
    pub approved: bool,
    pub wallet_address: Option<String>,
    pub node_id: String,
    pub node_pubkey_b64: String,
    pub pubkey_fingerprint: String,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
pub struct ApprovalSubmitResponse {
    pub success: bool,
    pub message: String,
    pub wallet_address: String,
    pub node_id: String,
}

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub wallet_address: String,
}

#[derive(Serialize)]
pub struct ChallengeResponse {
    pub message: String,
    pub expires_at: u64,
    pub nonce_hex: String,
    pub ts_unix: u64,
}

/// POST /api/node/approval/challenge - Generate challenge message for wallet approval
pub async fn get_challenge(Json(req): Json<ChallengeRequest>) -> Json<ChallengeResponse> {
    let node_id = local_node_id();
    let node_pubkey_b64 = local_pubkey_b64();

    // Generate random nonce
    let nonce_bytes: [u8; 16] = rand::random();
    let nonce_hex = hex::encode(nonce_bytes);

    // Current timestamp
    let ts_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Build canonical message
    let message = crate::node_approval::NodeApproval::build_canonical_message(
        &req.wallet_address,
        &node_id,
        &node_pubkey_b64,
        ts_unix,
        &nonce_hex,
    );

    // Challenge expires in 120 seconds
    let expires_at = ts_unix + 120;

    Json(ChallengeResponse {
        message,
        expires_at,
        nonce_hex,
        ts_unix,
    })
}
