// =================== Legendary Wallet Transfer API Endpoints ===================

use crate::legendary_wallet::{
    AccountFlags, OfferStatus, TransferWalletStatusTx, WalletOffer, WalletStatusError,
    apply_transfer_wallet_status, validate_transfer_wallet_status,
};
use crate::{acct_key, CHAIN};
use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// Offer storage uses database with "offer:" prefix
// Offers are loaded from DB on demand and persisted immediately

// =================== Request/Response Types ===================

#[derive(Debug, Deserialize)]
pub struct MarkTransferableRequest {
    pub transferable: bool,
    /// Message signed by wallet owner: "mark-transferable:{address}:{transferable}:{timestamp}"
    pub signature: String,
    /// Unix timestamp (must be within 5 minutes)
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct MarkTransferableResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOfferRequest {
    pub move_legendary: bool,
    pub move_immortal_node: bool,
    pub move_balance: bool,
    pub price_land: u128,
    /// Message signed by wallet owner: "create-offer:{address}:{price}:{timestamp}"
    pub signature: String,
    /// Unix timestamp (must be within 5 minutes)
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct CreateOfferResponse {
    pub offer_id: Uuid,
    pub offer: WalletOffer,
}

#[derive(Debug, Deserialize)]
pub struct CompleteTransferRequest {
    pub offer_id: Uuid,
    pub new_wallet_address: String,
    /// Message signed by buyer wallet: "complete-transfer:{offer_id}:{new_address}:{timestamp}"
    pub signature: String,
    /// Unix timestamp (must be within 5 minutes)
    pub timestamp: u64,
}

#[derive(Debug, Serialize)]
pub struct CompleteTransferResponse {
    pub success: bool,
    pub transaction_hash: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct AccountStatusResponse {
    pub address: String,
    pub balance: u128,
    pub flags: AccountFlags,
}

#[derive(Debug, Serialize)]
pub struct ListOffersResponse {
    pub offers: Vec<WalletOffer>,
}

// =================== Helper Functions ===================

/// Get account flags from chain state, or default if not found
fn get_account_flags(address: &str) -> AccountFlags {
    let chain = CHAIN.lock();
    let key = acct_key(address);
    chain
        .account_flags
        .get(&key)
        .copied()
        .unwrap_or_default()
}

/// Set account flags in chain state and persist to db
fn set_account_flags(address: &str, flags: AccountFlags) -> Result<(), String> {
    let mut chain = CHAIN.lock();
    let key = acct_key(address);
    
    // Update in-memory
    chain.account_flags.insert(key.clone(), flags);
    
    // Persist to database
    let db_key = format!("acctflags:{}", key);
    let flags_bytes = flags.to_bytes();
    chain
        .db
        .insert(db_key.as_bytes(), flags_bytes.as_slice())
        .map_err(|e| format!("DB error: {}", e))?;
    
    chain.db.flush().map_err(|e| format!("DB flush error: {}", e))?;
    
    Ok(())
}

/// Check if legendary transfer feature is enabled
fn is_feature_enabled() -> bool {
    // Can be controlled via environment variable
    std::env::var("VISION_LEGENDARY_TRANSFER_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase()
        == "true"
}

/// Save offer to database
fn save_offer_to_db(offer: &WalletOffer) -> Result<(), String> {
    let chain = CHAIN.lock();
    let db_key = format!("offer:{}", offer.id);
    let offer_json = serde_json::to_string(offer)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    chain
        .db
        .insert(db_key.as_bytes(), offer_json.as_bytes())
        .map_err(|e| format!("DB error: {}", e))?;
    
    chain.db.flush().map_err(|e| format!("DB flush error: {}", e))?;
    Ok(())
}

/// Load offer from database
fn load_offer_from_db(offer_id: &Uuid) -> Result<Option<WalletOffer>, String> {
    let chain = CHAIN.lock();
    let db_key = format!("offer:{}", offer_id);
    
    match chain.db.get(db_key.as_bytes()) {
        Ok(Some(bytes)) => {
            let offer_json = String::from_utf8(bytes.to_vec())
                .map_err(|e| format!("UTF-8 error: {}", e))?;
            let offer: WalletOffer = serde_json::from_str(&offer_json)
                .map_err(|e| format!("Deserialization error: {}", e))?;
            Ok(Some(offer))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("DB error: {}", e)),
    }
}

/// List all offers from database
fn list_all_offers() -> Result<Vec<WalletOffer>, String> {
    let chain = CHAIN.lock();
    let mut offers = Vec::new();
    
    let prefix = b"offer:";
    let iter = chain.db.scan_prefix(prefix);
    
    for item in iter {
        match item {
            Ok((_, value)) => {
                let offer_json = String::from_utf8(value.to_vec())
                    .map_err(|e| format!("UTF-8 error: {}", e))?;
                let offer: WalletOffer = serde_json::from_str(&offer_json)
                    .map_err(|e| format!("Deserialization error: {}", e))?;
                offers.push(offer);
            }
            Err(e) => return Err(format!("DB iteration error: {}", e)),
        }
    }
    
    Ok(offers)
}

/// Verify signature for a message using ECDSA secp256k1
/// address should be in "land1..." format
/// message is the exact string that was signed
/// signature is hex-encoded
pub fn verify_wallet_signature(address: &str, message: &str, signature_hex: &str) -> Result<bool, String> {
    use secp256k1::{Message as SecpMessage, PublicKey, Secp256k1};
    use sha2::{Digest, Sha256};
    
    // Decode signature from hex
    let sig_bytes = hex::decode(signature_hex)
        .map_err(|e| format!("Invalid signature hex: {}", e))?;
    
    if sig_bytes.len() != 65 {
        return Err("Signature must be 65 bytes (r + s + v)".to_string());
    }
    
    // Extract r, s, v components
    let r = &sig_bytes[0..32];
    let s = &sig_bytes[32..64];
    let v = sig_bytes[64];
    
    // Hash the message
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    let message_hash = hasher.finalize();
    
    let secp = Secp256k1::new();
    let message_secp = SecpMessage::from_slice(&message_hash)
        .map_err(|e| format!("Invalid message hash: {}", e))?;
    
    // Recover public key from signature
    let recovery_id = secp256k1::ecdsa::RecoveryId::from_i32((v - 27) as i32)
        .map_err(|e| format!("Invalid recovery id: {}", e))?;
    
    let mut sig_compact = [0u8; 64];
    sig_compact[..32].copy_from_slice(r);
    sig_compact[32..].copy_from_slice(s);
    
    let recoverable_sig = secp256k1::ecdsa::RecoverableSignature::from_compact(&sig_compact, recovery_id)
        .map_err(|e| format!("Invalid signature format: {}", e))?;
    
    let pubkey = secp.recover_ecdsa(&message_secp, &recoverable_sig)
        .map_err(|e| format!("Failed to recover public key: {}", e))?;
    
    // Derive address from public key
    let pubkey_bytes = pubkey.serialize_uncompressed();
    let mut addr_hasher = Sha256::new();
    addr_hasher.update(&pubkey_bytes[1..]); // Skip the 0x04 prefix
    let addr_hash = addr_hasher.finalize();
    let derived_address = format!("land1{}", hex::encode(&addr_hash[..20]));
    
    // Compare addresses
    Ok(derived_address.eq_ignore_ascii_case(address))
}

/// Verify timestamp is recent (within 5 minutes)
fn verify_timestamp(timestamp: u64) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;
    
    let now_secs = now.as_secs();
    let age = if now_secs > timestamp {
        now_secs - timestamp
    } else {
        timestamp - now_secs
    };
    
    if age > 300 {
        return Err(format!("Timestamp too old or in future (age: {}s)", age));
    }
    
    Ok(())
}

/// Transfer LAND tokens from buyer to seller
fn transfer_payment(from: &str, to: &str, amount: u128) -> Result<(), String> {
    let mut chain = CHAIN.lock();
    let from_key = acct_key(from);
    let to_key = acct_key(to);
    
    // Check sender balance
    let from_balance = chain.balances.get(&from_key).copied().unwrap_or(0);
    if from_balance < amount {
        return Err(format!(
            "Insufficient balance: has {} LAND, needs {} LAND",
            from_balance, amount
        ));
    }
    
    // Deduct from sender
    chain.balances.insert(from_key.clone(), from_balance - amount);
    
    // Add to receiver
    let to_balance = chain.balances.get(&to_key).copied().unwrap_or(0);
    let new_to_balance = to_balance.checked_add(amount)
        .ok_or_else(|| "Balance overflow".to_string())?;
    chain.balances.insert(to_key.clone(), new_to_balance);
    
    // Persist balances
    chain
        .db
        .insert(from_key.as_bytes(), from_balance.to_le_bytes().as_slice())
        .map_err(|e| format!("DB error: {}", e))?;
    
    chain
        .db
        .insert(to_key.as_bytes(), new_to_balance.to_le_bytes().as_slice())
        .map_err(|e| format!("DB error: {}", e))?;
    
    chain.db.flush().map_err(|e| format!("DB flush error: {}", e))?;
    
    Ok(())
}

// =================== API Endpoint Handlers ===================

/// GET /api/wallets/:address/status
/// Get wallet status including legendary/immortal flags
pub async fn get_wallet_status(
    Path(address): Path<String>,
) -> Result<Json<AccountStatusResponse>, (StatusCode, String)> {
    let chain = CHAIN.lock();
    let key = acct_key(&address);
    
    let balance = chain.balances.get(&key).copied().unwrap_or(0);
    let flags = chain
        .account_flags
        .get(&key)
        .copied()
        .unwrap_or_default();
    
    Ok(Json(AccountStatusResponse {
        address,
        balance,
        flags,
    }))
}

/// POST /api/wallets/:address/mark-transferable
/// Mark a wallet as transferable (seller opt-in)
/// Requires valid signature from wallet owner
pub async fn mark_transferable(
    Path(address): Path<String>,
    Json(req): Json<MarkTransferableRequest>,
) -> Result<Json<MarkTransferableResponse>, (StatusCode, String)> {
    if !is_feature_enabled() {
        return Err((
            StatusCode::FORBIDDEN,
            "Legendary transfer feature is disabled".to_string(),
        ));
    }
    
    // Verify timestamp
    verify_timestamp(req.timestamp)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e)))?;
    
    // Verify signature
    let message = format!("mark-transferable:{}:{}:{}", address, req.transferable, req.timestamp);
    let signature_valid = verify_wallet_signature(&address, &message, &req.signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Signature verification failed: {}", e)))?;
    
    if !signature_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid signature: does not match wallet address".to_string(),
        ));
    }
    
    // Get current flags
    let mut flags = get_account_flags(&address);
    
    // Wallet must have special status to be transferable
    if !flags.has_special_status() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Wallet does not have legendary or immortal status".to_string(),
        ));
    }
    
    // Update transferable flag
    flags.transferable = req.transferable;
    
    // Save
    set_account_flags(&address, flags).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    
    Ok(Json(MarkTransferableResponse {
        success: true,
        message: format!(
            "Wallet {} {} for transfer",
            address,
            if req.transferable { "enabled" } else { "disabled" }
        ),
    }))
}

/// POST /api/wallets/:address/create-legendary-offer
/// Create a marketplace offer to sell wallet status
/// Requires valid signature from wallet owner
pub async fn create_legendary_offer(
    Path(address): Path<String>,
    Json(req): Json<CreateOfferRequest>,
) -> Result<Json<CreateOfferResponse>, (StatusCode, String)> {
    if !is_feature_enabled() {
        return Err((
            StatusCode::FORBIDDEN,
            "Legendary transfer feature is disabled".to_string(),
        ));
    }
    
    // Verify timestamp
    verify_timestamp(req.timestamp)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e)))?;
    
    // Verify signature
    let message = format!("create-offer:{}:{}:{}", address, req.price_land, req.timestamp);
    let signature_valid = verify_wallet_signature(&address, &message, &req.signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Signature verification failed: {}", e)))?;
    
    if !signature_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid signature: does not match wallet address".to_string(),
        ));
    }
    
    // Validate wallet has the required flags
    let flags = get_account_flags(&address);
    
    if req.move_legendary && !flags.legendary {
        return Err((
            StatusCode::BAD_REQUEST,
            "Wallet does not have legendary status".to_string(),
        ));
    }
    
    if req.move_immortal_node && !flags.immortal_node {
        return Err((
            StatusCode::BAD_REQUEST,
            "Wallet does not have immortal node status".to_string(),
        ));
    }
    
    if !flags.transferable {
        return Err((
            StatusCode::BAD_REQUEST,
            "Wallet is not marked as transferable".to_string(),
        ));
    }
    
    // Validate price is reasonable (prevent overflows)
    if req.price_land == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Price must be greater than 0".to_string(),
        ));
    }
    
    // Create offer
    let offer = WalletOffer::new(
        address,
        req.move_legendary,
        req.move_immortal_node,
        req.move_balance,
        req.price_land,
    );
    
    let offer_id = offer.id;
    
    // Store offer in database
    save_offer_to_db(&offer)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save offer: {}", e)))?;
    
    tracing::info!(
        "[LEGENDARY_WALLET] Created offer {} for wallet {} (price: {} LAND)",
        offer_id,
        offer.from,
        req.price_land
    );
    
    Ok(Json(CreateOfferResponse { offer_id, offer }))
}

/// GET /api/wallets/legendary-offers
/// List all open legendary wallet offers
pub async fn list_legendary_offers() -> Result<Json<ListOffersResponse>, (StatusCode, String)> {
    let all_offers = list_all_offers()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load offers: {}", e)))?;
    
    let open_offers: Vec<WalletOffer> = all_offers
        .into_iter()
        .filter(|o| o.status == OfferStatus::Open)
        .collect();
    
    Ok(Json(ListOffersResponse {
        offers: open_offers,
    }))
}

/// GET /api/wallets/legendary-offers/:offer_id
/// Get details of a specific offer
pub async fn get_legendary_offer(
    Path(offer_id): Path<Uuid>,
) -> Result<Json<WalletOffer>, (StatusCode, String)> {
    let offer = load_offer_from_db(&offer_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load offer: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Offer not found".to_string()))?;
    
    Ok(Json(offer))
}

/// POST /api/wallets/complete-status-transfer
/// Complete a legendary wallet transfer (buyer activates with new wallet)
/// Requires payment in LAND tokens and signature from buyer
pub async fn complete_status_transfer(
    Json(req): Json<CompleteTransferRequest>,
) -> Result<Json<CompleteTransferResponse>, (StatusCode, String)> {
    if !is_feature_enabled() {
        return Err((
            StatusCode::FORBIDDEN,
            "Legendary transfer feature is disabled".to_string(),
        ));
    }
    
    // Verify timestamp
    verify_timestamp(req.timestamp)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e)))?;
    
    // Get offer
    let mut offer = load_offer_from_db(&req.offer_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load offer: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Offer not found".to_string()))?;
    
    if offer.status != OfferStatus::Open {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Offer is not open (status: {:?})", offer.status),
        ));
    }
    
    // Verify new wallet address is different
    if offer.from == req.new_wallet_address {
        return Err((
            StatusCode::BAD_REQUEST,
            "New wallet address must be different from seller's address".to_string(),
        ));
    }
    
    // Verify signature from buyer
    let message = format!("complete-transfer:{}:{}:{}", req.offer_id, req.new_wallet_address, req.timestamp);
    let signature_valid = verify_wallet_signature(&req.new_wallet_address, &message, &req.signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Signature verification failed: {}", e)))?;
    
    if !signature_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid signature: does not match buyer wallet address".to_string(),
        ));
    }
    
    // Transfer payment from buyer to seller
    transfer_payment(&req.new_wallet_address, &offer.from, offer.price_land)
        .map_err(|e| (StatusCode::PAYMENT_REQUIRED, format!("Payment failed: {}", e)))?;
    
    // Apply the transfer in chain state
    let mut chain = CHAIN.lock();
    
    let from_key = acct_key(&offer.from);
    let to_key = acct_key(&req.new_wallet_address);
    
    // Get current state
    let mut from_balance = chain.balances.get(&from_key).copied().unwrap_or(0);
    let mut from_flags = chain
        .account_flags
        .get(&from_key)
        .copied()
        .unwrap_or_default();
    
    let mut to_balance = chain.balances.get(&to_key).copied().unwrap_or(0);
    let mut to_flags = chain
        .account_flags
        .get(&to_key)
        .copied()
        .unwrap_or_default();
    
    // Create transaction
    let tx = TransferWalletStatusTx {
        from: offer.from.clone(),
        to: req.new_wallet_address.clone(),
        move_balance: offer.move_balance,
        move_legendary: offer.move_legendary,
        move_immortal_node: offer.move_immortal_node,
    };
    
    // Validate
    validate_transfer_wallet_status(&tx, from_balance, &from_flags, true)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    
    // Apply transfer
    apply_transfer_wallet_status(
        &tx,
        &mut from_balance,
        &mut from_flags,
        &mut to_balance,
        &mut to_flags,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    // Update chain state
    chain.balances.insert(from_key.clone(), from_balance);
    chain.balances.insert(to_key.clone(), to_balance);
    chain.account_flags.insert(from_key.clone(), from_flags);
    chain.account_flags.insert(to_key.clone(), to_flags);
    
    // Persist to database
    let from_bal_bytes = from_balance.to_be_bytes();
    let to_bal_bytes = to_balance.to_be_bytes();
    let from_flags_bytes = from_flags.to_bytes();
    let to_flags_bytes = to_flags.to_bytes();
    
    chain
        .db
        .insert(format!("bal:{}", from_key).as_bytes(), from_bal_bytes.as_slice())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
    
    chain
        .db
        .insert(format!("bal:{}", to_key).as_bytes(), to_bal_bytes.as_slice())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
    
    chain
        .db
        .insert(format!("acctflags:{}", from_key).as_bytes(), from_flags_bytes.as_slice())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
    
    chain
        .db
        .insert(format!("acctflags:{}", to_key).as_bytes(), to_flags_bytes.as_slice())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {}", e)))?;
    
    chain
        .db
        .flush()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB flush error: {}", e)))?;
    
    drop(chain);
    
    // Mark offer as completed and save
    offer.status = OfferStatus::Completed;
    save_offer_to_db(&offer)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update offer: {}", e)))?;
    
    // Simple hash of transaction (in production, use proper transaction hash)
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    let tx_data = format!("{}:{}:{}:{}", tx.from, tx.to, tx.move_legendary, tx.move_immortal_node);
    let mut hasher = RandomState::new().build_hasher();
    tx_data.hash(&mut hasher);
    let tx_hash = format!("{:x}", hasher.finish());
    
    tracing::info!(
        "[LEGENDARY_WALLET] Transfer completed: {} -> {} (offer: {}, paid: {} LAND)",
        tx.from,
        tx.to,
        req.offer_id,
        offer.price_land
    );
    
    Ok(Json(CompleteTransferResponse {
        success: true,
        transaction_hash: tx_hash,
        message: format!(
            "Legendary wallet status transferred from {} to {}. Payment of {} LAND processed.",
            tx.from, tx.to, offer.price_land
        ),
    }))
}

/// POST /api/wallets/legendary-offers/:offer_id/cancel
/// Cancel an open offer
/// Requires signature from seller
#[derive(Debug, Deserialize)]
pub struct CancelOfferRequest {
    /// Message signed by seller: "cancel-offer:{offer_id}:{timestamp}"
    pub signature: String,
    /// Unix timestamp (must be within 5 minutes)
    pub timestamp: u64,
}

pub async fn cancel_legendary_offer(
    Path(offer_id): Path<Uuid>,
    Json(req): Json<CancelOfferRequest>,
) -> Result<Json<MarkTransferableResponse>, (StatusCode, String)> {
    // Verify timestamp
    verify_timestamp(req.timestamp)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid timestamp: {}", e)))?;
    
    // Get offer
    let mut offer = load_offer_from_db(&offer_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load offer: {}", e)))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Offer not found".to_string()))?;
    
    if offer.status != OfferStatus::Open {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Offer is not open (status: {:?})", offer.status),
        ));
    }
    
    // Verify signature from seller
    let message = format!("cancel-offer:{}:{}", offer_id, req.timestamp);
    let signature_valid = verify_wallet_signature(&offer.from, &message, &req.signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Signature verification failed: {}", e)))?;
    
    if !signature_valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid signature: does not match seller wallet address".to_string(),
        ));
    }
    
    // Cancel offer
    offer.status = OfferStatus::Cancelled;
    save_offer_to_db(&offer)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update offer: {}", e)))?;
    
    tracing::info!(
        "[LEGENDARY_WALLET] Offer {} cancelled by {}",
        offer_id,
        offer.from
    );
    
    Ok(Json(MarkTransferableResponse {
        success: true,
        message: format!("Offer {} cancelled", offer_id),
    }))
}

// =================== Vision-Native Message Signing ===================

#[derive(Debug, Deserialize)]
pub struct SignMessageRequest {
    pub message: String,
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct SignMessageResponse {
    pub signature_b64: String,
    pub pubkey_b64: String,
    pub wallet_address: String,
}

/// POST /api/wallet/sign_message - Sign a message with the wallet's Ed25519 key
/// 
/// This endpoint signs messages using the Vision wallet's native Ed25519 key.
/// Used for node approval and other cryptographic proofs.
pub async fn sign_message(
    Json(req): Json<SignMessageRequest>,
) -> Result<Json<SignMessageResponse>, (StatusCode, String)> {
    use ed25519_dalek::{Keypair, Signer};
    use base64::{Engine as _, engine::general_purpose};
    
    // Get wallet keypair from chain DB
    let chain = crate::CHAIN.lock();
    
    // Check if this is the primary wallet
    let primary_wallet = chain.db.get(b"primary_wallet_address")
        .ok()
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
    
    if primary_wallet.as_deref() != Some(req.wallet_address.as_str()) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Can only sign with primary wallet. Primary wallet: {:?}", primary_wallet),
        ));
    }
    
    // Get wallet keypair (stored as base64 secret key + pubkey)
    let secret_key_b64 = chain.db.get(format!("wallet_secret:{}", req.wallet_address).as_bytes())
        .ok()
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            format!("Wallet {} secret key not found. Wallet may not be created on this node.", req.wallet_address),
        ))?;
    
    let pubkey_b64 = chain.db.get(format!("wallet_pubkey:{}", req.wallet_address).as_bytes())
        .ok()
        .flatten()
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            format!("Wallet {} public key not found", req.wallet_address),
        ))?;
    
    drop(chain);
    
    // Decode secret key from base64
    let secret_bytes = general_purpose::STANDARD
        .decode(&secret_key_b64)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid secret key encoding: {}", e)))?;
    
    if secret_bytes.len() != 32 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Secret key must be 32 bytes, got {}", secret_bytes.len()),
        ));
    }
    
    // Decode public key from base64
    let pubkey_bytes = general_purpose::STANDARD
        .decode(&pubkey_b64)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid pubkey encoding: {}", e)))?;
    
    if pubkey_bytes.len() != 32 {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Public key must be 32 bytes, got {}", pubkey_bytes.len()),
        ));
    }
    
    // Reconstruct keypair (64 bytes: [32 secret][32 public])
    let mut keypair_bytes = secret_bytes.to_vec();
    keypair_bytes.extend_from_slice(&pubkey_bytes);
    
    let keypair_array: [u8; 64] = keypair_bytes.try_into().map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, "Invalid keypair bytes length".to_string())
    })?;
    
    let keypair = ed25519_dalek::SigningKey::from_keypair_bytes(&keypair_array)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid keypair: {}", e)))?;
    
    // Sign the message
    let message_bytes = req.message.as_bytes();
    let signature = keypair.sign(message_bytes);
    let signature_b64 = general_purpose::STANDARD.encode(signature.to_bytes());
    
    tracing::info!(
        "✅ Signed message for wallet {} (length: {} bytes)",
        req.wallet_address,
        message_bytes.len()
    );
    
    Ok(Json(SignMessageResponse {
        signature_b64,
        pubkey_b64,
        wallet_address: req.wallet_address,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RegisterWalletRequest {
    pub wallet_address: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterWalletResponse {
    pub success: bool,
    pub message: String,
    pub wallet_address: String,
}

/// POST /api/wallet/register - Register wallet and generate Ed25519 keys
/// 
/// This endpoint generates and stores a new Ed25519 keypair for the wallet,
/// enabling message signing and node approval functionality.
pub async fn register_wallet(
    Json(req): Json<RegisterWalletRequest>,
) -> Result<Json<RegisterWalletResponse>, (StatusCode, String)> {
    use ed25519_dalek::SigningKey;
    use base64::{Engine as _, engine::general_purpose};
    use rand::rngs::OsRng;
    
    // Check if wallet already registered
    let chain = crate::CHAIN.lock();
    let existing_pubkey = chain.db.get(format!("wallet_pubkey:{}", req.wallet_address).as_bytes())
        .ok()
        .flatten();
    
    if existing_pubkey.is_some() {
        drop(chain);
        return Ok(Json(RegisterWalletResponse {
            success: true,
            message: "Wallet already registered".to_string(),
            wallet_address: req.wallet_address,
        }));
    }
    
    // Generate new Ed25519 keypair
    
    let keypair = SigningKey::generate(&mut rand::rngs::OsRng);
    
    // Encode keys as base64
    let secret_key_b64 = general_purpose::STANDARD.encode(keypair.to_bytes());
    let pubkey_b64 = general_purpose::STANDARD.encode(keypair.verifying_key().to_bytes());
    
    // Store keys in database
    chain.db.insert(
        format!("wallet_pubkey:{}", req.wallet_address).as_bytes(),
        pubkey_b64.as_bytes()
    ).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to store pubkey: {}", e)
    ))?;
    
    chain.db.insert(
        format!("wallet_secret:{}", req.wallet_address).as_bytes(),
        secret_key_b64.as_bytes()
    ).map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to store secret key: {}", e)
    ))?;
    
    chain.db.flush().map_err(|e| (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to flush database: {}", e)
    ))?;
    
    drop(chain);
    
    // Bind wallet to node
    if let Err(e) = crate::bind_wallet_to_node(&req.wallet_address) {
        tracing::warn!("⚠️ Failed to bind wallet to node: {}", e);
        // Don't fail - wallet might already be bound
    }
    
    tracing::info!("✅ Registered wallet: {}", req.wallet_address);
    
    Ok(Json(RegisterWalletResponse {
        success: true,
        message: "Wallet registered successfully".to_string(),
        wallet_address: req.wallet_address,
    }))
}


