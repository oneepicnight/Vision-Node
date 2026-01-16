// WALLET SEND MODULE - Clean Axum 0.7 Compatible Implementation
//
// This is a fresh, simple HTTP interface for sending crypto (BTC/BCH/DOGE)
// from Vision wallets to external addresses. It uses the existing Phase 2 engines:
// - UtxoManager: Select and lock UTXOs
// - TransactionBuilder: Build raw transactions via RPC
// - KeyManager: Sign transactions (dev-signing feature)
// - External RPC: Broadcast to blockchain
//
// The handler is deliberately thin - all business logic lives in process_send().

use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::external_rpc::ExternalChain;
use crate::market::engine::QuoteAsset;
use crate::market::wallet::WALLETS;
use crate::tx_builder::TransactionBuilder;
use crate::utxo_manager::UtxoManager;

#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub user_id: String,
    pub chain: String,      // "btc" | "bch" | "doge"
    pub to_address: String,
    pub amount: String,     // Amount as string to preserve precision
    #[serde(default)]
    pub simulate: Option<bool>, // Simulation mode - validate without broadcasting
}

#[derive(Debug, Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub txid: Option<String>,
    pub status: String,     // "broadcast" | "simulated" | "error"
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_fee: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_spent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Main HTTP handler - This is the ONLY function Axum sees
/// 
/// Signature is compatible with Axum 0.7 stateless routing:
/// - Takes Json<SendRequest> as input
/// - Returns (StatusCode, Json<SendResponse>)
/// - No State extraction needed (we use global WALLETS, EXTERNAL_RPC_CLIENTS)
pub async fn wallet_send_external(
    Json(req): Json<SendRequest>,
) -> impl IntoResponse {
    // Spawn the async work to avoid type inference issues with StatusCode
    let response = tokio::task::spawn(async move {
        match process_send(req).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Send failed: {}", e);
                SendResponse {
                    success: false,
                    txid: None,
                    status: "error".to_string(),
                    message: Some(format!("Send failed: {}", e)),
                    estimated_fee: None,
                    total_spent: None,
                    error_code: Some("internal_error".to_string()),
                }
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("Task panicked: {}", e);
        SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some("Internal server error".to_string()),
            estimated_fee: None,
            total_spent: None,
            error_code: Some("internal_error".to_string()),
        }
    });
    
    // Explicitly type StatusCode to force axum::http::StatusCode
    let status: axum::http::StatusCode = if response.success {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_REQUEST
    };
    
    (status, Json(response))
}

/// Pure business logic - NO Axum types here
/// 
/// Steps:
/// 1. Parse and validate chain
/// 2. Validate destination address format
/// 3. Parse amount to smallest unit (satoshis, etc.)
/// 4. Check user balance and reserve funds
/// 5. Select UTXOs via UtxoManager
/// 6. Build raw transaction via TransactionBuilder (uses RPC)
/// 7. Broadcast via external RPC (or skip if simulate mode)
/// 8. Finalize balance deduction or rollback on error
async fn process_send(req: SendRequest) -> anyhow::Result<SendResponse> {
    let simulate = req.simulate.unwrap_or(false);
    
    tracing::info!(
        "Processing send: user_id={}, chain={}, to={}, amount={}, simulate={}",
        req.user_id, req.chain, req.to_address, req.amount, simulate
    );
    
    // 0. Security checks (skip in simulation mode)
    if !simulate {
        // Rate limiting
        let rate_config = crate::security::RateLimitConfig::default();
        if let Err(e) = crate::security::SecurityManager::check_rate_limit(&req.user_id, &rate_config) {
            tracing::warn!("Rate limit check failed for {}: {}", req.user_id, e);
            return Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(e.to_string()),
                estimated_fee: None,
                total_spent: None,
                error_code: Some("rate_limit_exceeded".to_string()),
            });
        }
    }
    
    // 1. Map chain string -> QuoteAsset enum
    let chain = match chain_to_asset(&req.chain) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid chain: user_id={}, chain={}, error={:?}", req.user_id, req.chain, e);
            return Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(format!("Invalid chain '{}'. Supported: btc, bch, doge", req.chain)),
                estimated_fee: None,
                total_spent: None,
                error_code: Some("invalid_chain".to_string()),
            });
        }
    };

    // 2. Validate chain is supported for external sends
    if !matches!(chain, QuoteAsset::Btc | QuoteAsset::Bch | QuoteAsset::Doge) {
        tracing::error!("Unsupported chain for external send: user_id={}, chain={:?}", req.user_id, chain);
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some(format!("Chain {} not supported for external sends. Use BTC, BCH, or DOGE.", req.chain)),
            estimated_fee: None,
            total_spent: None,
            error_code: Some("unsupported_chain".to_string()),
        });
    }

    // 3. Validate address format (basic check - full validation happens in RPC)
    if req.to_address.is_empty() || req.to_address.len() < 26 {
        tracing::error!("Invalid address format: user_id={}, address={}", req.user_id, req.to_address);
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some("Invalid destination address format".to_string()),
            estimated_fee: None,
            total_spent: None,
            error_code: Some("invalid_address".to_string()),
        });
    }

    // 4. Parse amount to smallest unit (satoshis for BTC/BCH, koinus for DOGE)
    let amount: u64 = match req.amount.parse() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Invalid amount format: user_id={}, amount={}, error={:?}", req.user_id, req.amount, e);
            return Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(format!("Invalid amount format: {}", e)),
                estimated_fee: None,
                total_spent: None,
                error_code: Some("invalid_amount".to_string()),
            });
        }
    };

    if amount == 0 {
        tracing::error!("Zero amount send attempted: user_id={}", req.user_id);
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some("Amount must be greater than zero".to_string()),
            estimated_fee: None,
            total_spent: None,
            error_code: Some("invalid_amount".to_string()),
        });
    }

    // 4.5 Security: Check amount limits (skip in simulation mode)
    if !simulate {
        let amount_f64 = amount as f64 / 100_000_000.0;
        let limit_config = match chain {
            QuoteAsset::Btc => crate::security::AmountLimitConfig::btc_default(),
            QuoteAsset::Bch => crate::security::AmountLimitConfig::bch_default(),
            QuoteAsset::Doge => crate::security::AmountLimitConfig::doge_default(),
            _ => crate::security::AmountLimitConfig::btc_default(),
        };
        
        if let Err(e) = crate::security::SecurityManager::check_amount_limit(
            &req.user_id,
            chain,
            amount_f64,
            &limit_config,
        ) {
            tracing::warn!("Amount limit check failed for {}: {}", req.user_id, e);
            return Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(e.to_string()),
                estimated_fee: None,
                total_spent: None,
                error_code: Some("amount_limit_exceeded".to_string()),
            });
        }
        
        // Log audit: send initiated
        crate::security::SecurityManager::log_audit(
            crate::security::AuditEntry::new(&req.user_id, crate::security::AuditEventType::SendInitiated)
                .with_asset(chain)
                .with_amount(amount_f64)
                .with_address(&req.to_address)
        );
    }

    // 5. Check dust threshold
    let dust_threshold = get_dust_threshold(chain);
    if amount < dust_threshold {
        tracing::warn!("Amount below dust threshold: user_id={}, amount={}, dust={}", req.user_id, amount, dust_threshold);
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some(format!(
                "Amount {} is below dust threshold ({} for {})",
                amount, dust_threshold, req.chain.to_uppercase()
            )),
            estimated_fee: None,
            total_spent: None,
            error_code: Some("amount_too_small".to_string()),
        });
    }

    // 6. Estimate fee (fixed per chain for now)
    let fee = estimate_fee(chain);

    // 7. Check user balance (including fee)
    let balance = match get_user_balance(&req.user_id, chain) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to get user balance: user_id={}, chain={:?}, error={:?}", req.user_id, chain, e);
            return Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some("Failed to retrieve balance".to_string()),
                estimated_fee: None,
                total_spent: None,
                error_code: Some("internal_error".to_string()),
            });
        }
    };
    
    let total_needed = amount + fee;

    if balance < total_needed {
        tracing::warn!(
            "Insufficient balance: user_id={}, chain={:?}, balance={}, needed={} (amount={}, fee={})",
            req.user_id, chain, balance, total_needed, amount, fee
        );
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some(format!(
                "Insufficient balance. Need {} (amount: {}, fee: {}), have {}",
                total_needed, amount, fee, balance
            )),
            estimated_fee: Some(fee),
            total_spent: Some(total_needed),
            error_code: Some("insufficient_funds".to_string()),
        });
    }

    // SIMULATION MODE: Stop here if simulate flag is set
    if simulate {
        tracing::info!(
            "Simulation successful: user_id={}, chain={:?}, to={}, amount={}, fee={}, total={}",
            req.user_id, chain, req.to_address, amount, fee, total_needed
        );
        return Ok(SendResponse {
            success: true,
            txid: Some("simulation-only".to_string()),
            status: "simulated".to_string(),
            message: Some(format!(
                "Simulation successful. Would send {} + {} fee = {} total",
                amount, fee, total_needed
            )),
            estimated_fee: Some(fee),
            total_spent: Some(total_needed),
            error_code: None,
        });
    }

    // 8. Reserve balance (lock funds during transaction)
    if let Err(e) = reserve_balance(&req.user_id, chain, total_needed) {
        tracing::error!("Failed to reserve balance: user_id={}, chain={:?}, amount={}, error={:?}", 
                       req.user_id, chain, total_needed, e);
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some("Failed to reserve balance".to_string()),
            estimated_fee: Some(fee),
            total_spent: Some(total_needed),
            error_code: Some("internal_error".to_string()),
        });
    }

    // 9. Build and broadcast transaction - if this fails, we rollback the balance reservation
    #[cfg(feature = "dev-signing")]
    let build_result = build_and_broadcast(&req.user_id, chain, &req.to_address, amount, fee).await;
    
    #[cfg(not(feature = "dev-signing"))]
    let build_result: Result<String, anyhow::Error> = Err(anyhow::anyhow!("Transaction signing requires dev-signing feature"));
    
    match build_result {
        Ok(txid) => {
            // Success! Finalize the balance deduction
            if let Err(e) = finalize_send(&req.user_id, chain, total_needed) {
                tracing::error!("Failed to finalize send: user_id={}, txid={}, error={:?}", req.user_id, txid, e);
            }

            tracing::info!(
                "Send successful: user_id={}, chain={:?}, to={}, amount={}, fee={}, txid={}",
                req.user_id, chain, req.to_address, amount, fee, txid
            );

            // Store transaction record in history
            let tx_record = crate::tx_history::TransactionRecord::new(
                &req.user_id,
                &txid,
                chain,
                crate::tx_history::TxType::Send,
                amount as f64 / 100_000_000.0,
                fee as f64 / 100_000_000.0,
                "wallet", // from_address - will be improved later
                &req.to_address,
            );
            
            if let Err(e) = crate::tx_history::TxHistoryManager::add_transaction(tx_record) {
                tracing::error!("Failed to store tx history: txid={}, error={:?}", txid, e);
            }

            // Store transaction record (legacy - keep for backward compatibility)
            if let Err(e) = store_tx_record(&req.user_id, &req.chain, &req.to_address, amount, &txid, "broadcast").await {
                tracing::error!("Failed to store tx record: txid={}, error={:?}", txid, e);
            }

            // Log audit: send completed
            crate::security::SecurityManager::log_audit(
                crate::security::AuditEntry::new(&req.user_id, crate::security::AuditEventType::SendCompleted)
                    .with_asset(chain)
                    .with_amount(amount as f64 / 100_000_000.0)
                    .with_address(&req.to_address)
                    .with_txid(&txid)
            );

            Ok(SendResponse {
                success: true,
                txid: Some(txid.clone()),
                status: "broadcast".to_string(),
                message: Some(format!("Transaction broadcast successfully: {}", txid)),
                estimated_fee: Some(fee),
                total_spent: Some(total_needed),
                error_code: None,
            })
        }
        Err(e) => {
            // Failed! Release the reserved balance
            if let Err(release_err) = release_balance(&req.user_id, chain, total_needed) {
                tracing::error!("Failed to release balance after error: user_id={}, error={:?}", req.user_id, release_err);
            }

            tracing::error!(
                "Send failed: user_id={}, chain={:?}, to={}, amount={}, error={:?}",
                req.user_id, chain, req.to_address, amount, e
            );

            let error_msg = e.to_string();
            let error_code = if error_msg.contains("RPC") || error_msg.contains("not configured") {
                "rpc_unavailable"
            } else if error_msg.contains("insufficient") {
                "insufficient_funds"
            } else {
                "transaction_failed"
            };

            // Log audit: send failed
            crate::security::SecurityManager::log_audit(
                crate::security::AuditEntry::new(&req.user_id, crate::security::AuditEventType::SendFailed)
                    .with_asset(chain)
                    .with_amount(amount as f64 / 100_000_000.0)
                    .with_address(&req.to_address)
                    .with_error(&error_msg)
            );

            Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(format!("Transaction failed: {}", error_msg)),
                estimated_fee: Some(fee),
                total_spent: Some(total_needed),
                error_code: Some(error_code.to_string()),
            })
        }
    }
}

/// Build raw transaction and broadcast to blockchain
#[cfg(feature = "dev-signing")]
async fn build_and_broadcast(
    user_id: &str,
    chain: QuoteAsset,
    to_address: &str,
    amount: u64,
    fee: u64,
) -> anyhow::Result<String> {
    // 1. Sync UTXOs from blockchain (requires addresses - use placeholder for now)
    let addresses = vec![]; // TODO: Get actual user addresses
    UtxoManager::sync_user_utxos(user_id, chain, addresses).await?;

    // 2. Build transaction using TransactionBuilder (RPC-based)
    // Convert u64 sats to f64 for tx_builder
    let amount_f64 = amount as f64;
    let fee_f64 = fee as f64;
    
    let tx_hex = TransactionBuilder::build_send_transaction(
        user_id,
        chain,
        to_address,
        amount_f64,
        fee_f64,
    )
    .await?;

    // 3. Broadcast via external RPC
    let external_chain = match chain {
        QuoteAsset::Btc => ExternalChain::Btc,
        QuoteAsset::Bch => ExternalChain::Bch,
        QuoteAsset::Doge => ExternalChain::Doge,
        _ => return Err(anyhow::anyhow!("Invalid chain for external send")),
    };

    // Clone the client to avoid holding the lock across await
    let client = {
        let clients = crate::EXTERNAL_RPC_CLIENTS.lock();
        clients
            .get(external_chain)
            .ok_or_else(|| anyhow::anyhow!("{:?} RPC client not configured", external_chain))?
            .clone()
    }; // Lock is dropped here

    let result: serde_json::Value = client
        .call("sendrawtransaction", serde_json::json!([tx_hex]))
        .await
        .map_err(|e| anyhow::anyhow!("RPC sendrawtransaction failed: {}", e))?;

    let txid = result
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("RPC returned non-string txid"))?
        .to_string();

    tracing::info!(
        "✅ Sent {} {} to {}: txid={}",
        amount,
        format!("{:?}", chain),
        to_address,
        txid
    );

    Ok(txid)
}

// ==================== HELPER FUNCTIONS ====================

/// Convert chain string to QuoteAsset enum
fn chain_to_asset(chain_str: &str) -> anyhow::Result<QuoteAsset> {
    match chain_str.to_lowercase().as_str() {
        "btc" => Ok(QuoteAsset::Btc),
        "bch" => Ok(QuoteAsset::Bch),
        "doge" => Ok(QuoteAsset::Doge),
        "land" => Ok(QuoteAsset::Land),
        other => Err(anyhow::anyhow!("Unknown chain: {}", other)),
    }
}

/// Get dust threshold for chain (minimum spendable amount)
fn get_dust_threshold(chain: QuoteAsset) -> u64 {
    match chain {
        QuoteAsset::Btc => 546,           // BTC dust: 546 sats
        QuoteAsset::Doge => 1_000_000,    // DOGE dust: 0.01 DOGE (1M koinus)
        QuoteAsset::Bch => 546,           // BCH dust: 546 sats
        _ => 0,
    }
}

/// Estimate transaction fee (fixed for now, can be dynamic later)
fn estimate_fee(chain: QuoteAsset) -> u64 {
    match chain {
        QuoteAsset::Btc => 1000,      // 1000 sats (~$0.40 at $40k BTC)
        QuoteAsset::Bch => 500,       // 500 sats (BCH fees are lower)
        QuoteAsset::Doge => 100_000,  // 0.001 DOGE (100k koinus)
        _ => 1000,
    }
}

/// Get user's available balance for a chain
fn get_user_balance(user_id: &str, chain: QuoteAsset) -> anyhow::Result<u64> {
    let wallets = WALLETS.lock().map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .get(user_id)
        .ok_or_else(|| anyhow::anyhow!("User wallet not found: {}", user_id))?;

    let balance_f64 = match chain {
        QuoteAsset::Btc => wallet.btc_available,
        QuoteAsset::Bch => wallet.bch_available,
        QuoteAsset::Doge => wallet.doge_available,
        QuoteAsset::Land => wallet.land_available,
    };
    
    // Convert f64 to u64 satoshis (multiply by 1e8 for BTC-like coins)
    Ok((balance_f64 * 100_000_000.0) as u64)
}

/// Reserve balance during transaction (prevent double-spend)
fn reserve_balance(user_id: &str, chain: QuoteAsset, amount: u64) -> anyhow::Result<()> {
    let mut wallets = WALLETS.lock().map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .get_mut(user_id)
        .ok_or_else(|| anyhow::anyhow!("User wallet not found: {}", user_id))?;

    // Convert u64 sats back to f64
    let amount_f64 = amount as f64 / 100_000_000.0;

    let (current_balance, balance_field) = match chain {
        QuoteAsset::Btc => (wallet.btc_available, &mut wallet.btc_available),
        QuoteAsset::Bch => (wallet.bch_available, &mut wallet.bch_available),
        QuoteAsset::Doge => (wallet.doge_available, &mut wallet.doge_available),
        QuoteAsset::Land => (wallet.land_available, &mut wallet.land_available),
    };

    if current_balance < amount_f64 {
        return Err(anyhow::anyhow!("Insufficient balance"));
    }

    // Deduct immediately (will be finalized or released later)
    *balance_field = current_balance - amount_f64;

    tracing::debug!("Reserved {} {} for user {}", amount, format!("{:?}", chain), user_id);
    Ok(())
}

/// Release reserved balance (rollback on error)
fn release_balance(user_id: &str, chain: QuoteAsset, amount: u64) -> anyhow::Result<()> {
    let mut wallets = WALLETS.lock().map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .get_mut(user_id)
        .ok_or_else(|| anyhow::anyhow!("User wallet not found: {}", user_id))?;

    // Convert u64 sats back to f64
    let amount_f64 = amount as f64 / 100_000_000.0;

    let balance_field = match chain {
        QuoteAsset::Btc => &mut wallet.btc_available,
        QuoteAsset::Bch => &mut wallet.bch_available,
        QuoteAsset::Doge => &mut wallet.doge_available,
        QuoteAsset::Land => &mut wallet.land_available,
    };

    *balance_field += amount_f64;

    tracing::warn!("Released {} {} back to user {}", amount, format!("{:?}", chain), user_id);
    Ok(())
}

/// Finalize send (confirm balance deduction)
fn finalize_send(user_id: &str, chain: QuoteAsset, amount: u64) -> anyhow::Result<()> {
    // Balance already deducted in reserve_balance, so nothing more to do
    tracing::info!("Finalized send of {} {} for user {}", amount, format!("{:?}", chain), user_id);
    Ok(())
}

// ==================== TRANSACTION HISTORY ====================

use std::sync::Mutex as StdMutex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TxRecord {
    pub id: u64,
    pub user_id: String,
    pub chain: String,
    pub to_address: String,
    pub amount: String,
    pub txid: String,
    pub status: String,
    pub created_at: String,
}

static TX_RECORDS: Lazy<StdMutex<Vec<TxRecord>>> = Lazy::new(|| StdMutex::new(Vec::new()));
static TX_COUNTER: Lazy<StdMutex<u64>> = Lazy::new(|| StdMutex::new(1));

/// Store a transaction record in memory (for now)
async fn store_tx_record(
    user_id: &str,
    chain: &str,
    to_address: &str,
    amount: u64,
    txid: &str,
    status: &str,
) -> anyhow::Result<()> {
    let id = {
        let mut counter = TX_COUNTER.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        let id = *counter;
        *counter += 1;
        id
    };

    let record = TxRecord {
        id,
        user_id: user_id.to_string(),
        chain: chain.to_string(),
        to_address: to_address.to_string(),
        amount: amount.to_string(),
        txid: txid.to_string(),
        status: status.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut records = TX_RECORDS.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
    records.push(record);

    tracing::debug!("Stored tx record: id={}, user_id={}, txid={}", id, user_id, txid);
    Ok(())
}

/// Get recent transaction records for a user
pub fn get_recent_sends(user_id: &str, limit: usize) -> anyhow::Result<Vec<TxRecord>> {
    let records = TX_RECORDS.lock().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
    
    let mut user_records: Vec<TxRecord> = records
        .iter()
        .filter(|r| r.user_id == user_id)
        .cloned()
        .collect();
    
    // Sort by ID descending (most recent first)
    user_records.sort_by(|a, b| b.id.cmp(&a.id));
    
    // Take only the requested limit
    user_records.truncate(limit);
    
    Ok(user_records)
}
