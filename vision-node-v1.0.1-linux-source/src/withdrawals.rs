// WALLET SEND MODULE
//
// Generic "send" feature for moving coins from Vision wallet to any external address.
// Supports: BTC, BCH, DOGE (and future: LAND)
//
// Steps:
// 1. Locate the existing withdrawal handler(s) for BTC/BCH/DOGE.
//    - They probably validate balance, construct a raw transaction hex, and currently
//      either stub out broadcasting or use a direct RPC client.
// 2. For each chain, replace any direct RPC usage with:
//       let clients = EXTERNAL_RPC_CLIENTS.lock().unwrap();
//       let client = clients.get(ExternalChain::Btc /* or Bch/Doge */)
//           .ok_or_else(|| anyhow!("BTC RPC not configured"))?;
// 3. Add a helper function, e.g. `broadcast_raw_tx(chain: ExternalChain, hex: &str) -> Result<String>`
//    that:
//       - Calls `client.call("sendrawtransaction", json!([hex]))`
//       - On success, returns txid as String
//       - On failure, logs a warning and propagates an error
// 4. Update the withdrawal handler flow to:
//       - Lock / reserve the user's internal balance
//       - Build the raw transaction hex
//       - Call `broadcast_raw_tx(...)`
//       - On success: store the txid in the withdrawal record and finalize the internal state
//       - On failure: roll back or mark withdrawal as FAILED and release reserved balance
// 5. Ensure all errors are logged with chain + tx context, e.g.:
//       warn!("BTC withdrawal broadcast failed: user_id={:?}, error={:?}", user_id, err);
//
// Result:
// - A user-initiated withdrawal will result in a real on-chain send via the external RPC system.
// - Missing RPC config for a chain should cleanly prevent withdrawals for that asset instead of crashing.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::external_rpc::ExternalChain;
use crate::market::engine::QuoteAsset;
use crate::market::wallet::{WALLETS, UserWallet};
use crate::tx_builder::TransactionBuilder;
use crate::utxo_manager::UtxoManager;

/// Send request - generic wallet send to external address
#[derive(Debug, Deserialize)]
pub struct SendRequest {
    pub user_id: String,
    pub chain: String,      // "btc" | "bch" | "doge" | "land"
    pub to_address: String, // Destination wallet address
    pub amount: String,     // Amount to send (as string to preserve precision)
}

/// Send response with transaction ID
#[derive(Debug, Serialize, Clone)]
pub struct SendResponse {
    pub success: bool,
    pub txid: Option<String>,
    pub status: String,     // "broadcast" | "error"
    pub message: Option<String>,
}

/// Legacy withdrawal types (kept for backward compatibility)
#[derive(Debug, Deserialize)]
pub struct WithdrawRequest {
    pub user_id: String,
    pub asset: QuoteAsset,
    pub address: String,
    pub amount: f64,
}

#[derive(Debug, Serialize)]
pub struct WithdrawResponse {
    pub success: bool,
    pub txid: Option<String>,
    pub error: Option<String>,
}

/// Transaction status tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Broadcasting,
    Broadcast(String),  // txid
    Confirmed(String),  // txid with confirmations
    Failed(String),     // error message
}

/// Broadcast raw transaction to blockchain via external RPC
pub async fn broadcast_raw_tx(chain: ExternalChain, hex: &str) -> Result<String> {
    let clients = crate::EXTERNAL_RPC_CLIENTS.lock()
        .map_err(|e| anyhow!("Failed to lock RPC clients: {}", e))?;
    let client = clients.get(&chain)
        .ok_or_else(|| anyhow!("{} RPC not configured", chain.as_str()))?
        .clone();
    
    tracing::info!("Broadcasting {} transaction, hex length: {} bytes", chain.as_str(), hex.len());
    
    let result = client.call("sendrawtransaction", serde_json::json!([hex])).await
        .map_err(|e| {
            tracing::warn!("{} withdrawal broadcast failed: {}", chain.as_str(), e);
            anyhow!("Failed to broadcast transaction: {}", e)
        })?;
    
    let txid = result.as_str()
        .ok_or_else(|| anyhow!("Invalid txid response from RPC"))?
        .to_string();
    
    tracing::info!("✅ {} transaction broadcast successful: {}", chain.as_str(), txid);
    Ok(txid)
}

/// Get external chain from quote asset
fn asset_to_chain(asset: &QuoteAsset) -> Result<ExternalChain> {
    match asset {
        QuoteAsset::Btc => Ok(ExternalChain::Btc),
        QuoteAsset::Bch => Ok(ExternalChain::Bch),
        QuoteAsset::Doge => Ok(ExternalChain::Doge),
        QuoteAsset::Land => Err(anyhow!("LAND is not a withdrawable external asset")),
    }
}

/// Build raw transaction hex
/// Steps:
/// 1. Select UTXOs from wallet
/// 2. Build transaction with bitcoin crate
/// 3. Sign with private keys
/// 4. Serialize to hex
async fn build_raw_transaction(
    asset: &QuoteAsset,
    to_address: &str,
    amount: f64,
    user_id: &str,
) -> Result<String> {
    #[cfg(feature = "dev-signing")]
    {
        // Get fee for the transaction
        let chain = match asset {
            QuoteAsset::Btc => ExternalChain::Btc,
            QuoteAsset::Bch => ExternalChain::Bch,
            QuoteAsset::Doge => ExternalChain::Doge,
            QuoteAsset::Land => return Err(anyhow!("LAND is not an external chain")),
        };
        
        let fee = estimate_fee(chain, amount);
        
        tracing::info!(
            "🔨 Building transaction: user={}, amount={:.8}, fee={:.8}, to={}",
            user_id, amount, fee, to_address
        );
        
        // Build signed transaction using TransactionBuilder
        let tx_hex = TransactionBuilder::build_send_transaction(
            user_id,
            *asset,
            to_address,
            amount,
            fee,
        ).await?;
        
        Ok(tx_hex)
    }
    
    #[cfg(not(feature = "dev-signing"))]
    {
        Err(anyhow!(
            "Transaction signing disabled. Enable 'dev-signing' feature to use server-side signing.\n\
             WARNING: Server-side signing is for development only. Production should use client-side signing."
        ))
    }
}

/// Validate withdrawal address for the given chain
fn validate_address(chain: ExternalChain, address: &str) -> Result<()> {
    // Basic validation - in production, use proper address parsing
    match chain {
        ExternalChain::Btc => {
            if !address.starts_with("bc1") && !address.starts_with("1") && !address.starts_with("3") {
                return Err(anyhow!("Invalid Bitcoin address format"));
            }
        }
        ExternalChain::Bch => {
            if !address.starts_with("bitcoincash:") && !address.starts_with("q") {
                return Err(anyhow!("Invalid Bitcoin Cash address format"));
            }
        }
        ExternalChain::Doge => {
            if !address.starts_with("D") {
                return Err(anyhow!("Invalid Dogecoin address format"));
            }
        }
    }
    Ok(())
}

/// Check if RPC is available for the given chain
pub async fn check_withdrawal_available(asset: &QuoteAsset) -> Result<bool> {
    let chain = asset_to_chain(asset)?;
    
    let clients = crate::EXTERNAL_RPC_CLIENTS.lock()
        .map_err(|e| anyhow!("Failed to lock RPC clients: {}", e))?;
    Ok(clients.contains_key(&chain))
}

/// Parse chain string to ExternalChain enum
fn parse_chain(chain_str: &str) -> Result<ExternalChain> {
    match chain_str.to_lowercase().as_str() {
        "btc" | "bitcoin" => Ok(ExternalChain::Btc),
        "bch" | "bitcoincash" => Ok(ExternalChain::Bch),
        "doge" | "dogecoin" => Ok(ExternalChain::Doge),
        _ => Err(anyhow!("Unsupported chain: {}. Supported: btc, bch, doge", chain_str)),
    }
}

/// Convert ExternalChain to QuoteAsset
fn chain_to_asset(chain: ExternalChain) -> QuoteAsset {
    match chain {
        ExternalChain::Btc => QuoteAsset::Btc,
        ExternalChain::Bch => QuoteAsset::Bch,
        ExternalChain::Doge => QuoteAsset::Doge,
    }
}

/// Estimate network fee for transaction
fn estimate_fee(chain: ExternalChain, _amount: f64) -> f64 {
    // Simple fixed fee estimation
    // In production, query fee rate from RPC (estimatesmartfee)
    match chain {
        ExternalChain::Btc => 0.00001,  // ~$0.50 at $50k BTC
        ExternalChain::Bch => 0.00001,  // Very low BCH fees
        ExternalChain::Doge => 0.5,     // DOGE has higher nominal fees but cheap in USD
    }
}

/// Get user's available balance for a chain
fn get_user_balance(user_id: &str, chain: ExternalChain) -> f64 {
    if let Ok(wallets) = WALLETS.lock() {
        if let Some(wallet) = wallets.get(user_id) {
            let asset = chain_to_asset(chain);
            return wallet.get_available(asset);
        }
    }
    0.0
}

/// Reserve balance for a send (move from available to locked)
fn reserve_balance(user_id: &str, chain: ExternalChain, amount: f64) -> Result<()> {
    let mut wallets = WALLETS.lock()
        .map_err(|e| anyhow!("Failed to lock wallets: {}", e))?;
    let wallet = wallets.entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));
    
    let asset = chain_to_asset(chain);
    let available = wallet.get_available(asset);
    
    if available < amount {
        return Err(anyhow!("Insufficient balance: have {}, need {}", available, amount));
    }
    
    // Move from available to locked
    match chain {
        ExternalChain::Btc => {
            wallet.btc_available -= amount;
            wallet.btc_locked += amount;
        }
        ExternalChain::Bch => {
            wallet.bch_available -= amount;
            wallet.bch_locked += amount;
        }
        ExternalChain::Doge => {
            wallet.doge_available -= amount;
            wallet.doge_locked += amount;
        }
    }
    
    tracing::info!("🔒 Reserved {} {} for user {}", amount, chain.as_str(), user_id);
    Ok(())
}

/// Release reserved balance (move from locked back to available)
fn release_balance(user_id: &str, chain: ExternalChain, amount: f64) {
    if let Ok(mut wallets) = WALLETS.lock() {
    if let Some(wallet) = wallets.get_mut(user_id) {
        match chain {
            ExternalChain::Btc => {
                wallet.btc_locked -= amount;
                wallet.btc_available += amount;
            }
            ExternalChain::Bch => {
                wallet.bch_locked -= amount;
                wallet.bch_available += amount;
            }
            ExternalChain::Doge => {
                wallet.doge_locked -= amount;
                wallet.doge_available += amount;
            }
        }
            tracing::info!("🔓 Released {} {} for user {}", amount, chain.as_str(), user_id);
        }
    }
}

/// Finalize send (deduct from locked balance permanently)
fn finalize_send(user_id: &str, chain: ExternalChain, amount: f64) {
    if let Ok(mut wallets) = WALLETS.lock() {
    if let Some(wallet) = wallets.get_mut(user_id) {
        match chain {
            ExternalChain::Btc => {
                wallet.btc_locked -= amount;
            }
            ExternalChain::Bch => {
                wallet.bch_locked -= amount;
            }
            ExternalChain::Doge => {
                wallet.doge_locked -= amount;
            }
        }
            tracing::info!("✅ Finalized send of {} {} for user {}", amount, chain.as_str(), user_id);
        }
    }
}

/// Process generic send request - main entry point for wallet sends
pub async fn process_send(request: SendRequest) -> Result<SendResponse> {
    let chain = parse_chain(&request.chain)?;
    
    // Validate destination address format
    validate_address(chain, &request.to_address)?;
    
    // Check if RPC is configured and healthy
    let clients = crate::EXTERNAL_RPC_CLIENTS.lock()
        .map_err(|e| anyhow!("Failed to lock RPC clients: {}", e))?;
    if !clients.contains_key(&chain) {
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some(format!("{} RPC not configured or unavailable", chain.as_str())),
        });
    }
    drop(clients);
    
    // Parse and validate amount
    let amount = request.amount.parse::<f64>()
        .map_err(|_| anyhow!("Invalid amount format"))?;
    
    if amount <= 0.0 {
        return Ok(SendResponse {
            success: false,
            txid: None,
            status: "error".to_string(),
            message: Some("Amount must be greater than zero".to_string()),
        });
    }
    
    tracing::info!(
        "🔄 Processing {} send: user={}, amount={}, to={}",
        chain.as_str(), request.user_id, amount, request.to_address
    );
    
    // Step 1: Check user balance including estimated fee
    let balance = get_user_balance(&request.user_id, chain);
    let estimated_fee = estimate_fee(chain, amount);
    let total_needed = amount + estimated_fee;
    
    if balance < total_needed {
        return Ok(SendResponse {
            success: false,
            status: "error".to_string(),
            message: Some(format!(
                "Insufficient balance. Available: {:.8}, Required: {:.8} (amount: {:.8} + fee: {:.8})",
                balance, total_needed, amount, estimated_fee
            )),
            txid: None,
        });
    }
    
    tracing::info!("✅ Balance check passed: {} >= {} (amount + fee)", balance, total_needed);
    
    // Step 2: Reserve balance (lock it during transaction processing)
    if let Err(e) = reserve_balance(&request.user_id, chain, total_needed) {
        return Ok(SendResponse {
            success: false,
            status: "error".to_string(),
            message: Some(format!("Failed to reserve balance: {}", e)),
            txid: None,
        });
    }
    
    // Step 3: Build raw transaction
    let asset = chain_to_asset(chain);
    let raw_tx = match build_raw_transaction(&asset, &request.to_address, amount, &request.user_id).await {
        Ok(tx) => tx,
        Err(e) => {
            // Release reserved balance on failure
            release_balance(&request.user_id, chain, total_needed);
            tracing::error!("❌ Failed to build transaction: {}", e);
            return Ok(SendResponse {
                success: false,
                status: "error".to_string(),
                message: Some(format!("Failed to build transaction: {}", e)),
                txid: None,
            });
        }
    };
    
    tracing::info!("📦 Built raw transaction: {} bytes", raw_tx.len());
    
    // Step 4: Broadcast transaction
    match broadcast_raw_tx(chain, &raw_tx).await {
        Ok(txid) => {
            // Finalize send - permanently deduct from locked balance
            finalize_send(&request.user_id, chain, total_needed);
            
            let txid_clone = txid.clone();
            
            tracing::info!(
                "✅ {} send successful: user={}, txid={}, amount={:.8}",
                chain.as_str(), request.user_id, txid_clone, amount
            );
            
            Ok(SendResponse {
                success: true,
                txid: Some(txid_clone.clone()),
                status: "broadcast".to_string(),
                message: Some(format!("Transaction broadcast successfully. TXID: {}", txid_clone)),
            })
        }
        Err(e) => {
            // Release reserved balance on broadcast failure
            release_balance(&request.user_id, chain, total_needed);
            
            tracing::error!(
                "❌ {} send broadcast failed: user={}, error={}",
                chain.as_str(), request.user_id, e
            );
            
            Ok(SendResponse {
                success: false,
                txid: None,
                status: "error".to_string(),
                message: Some(format!("Broadcast failed: {}", e)),
            })
        }
    }
}

/// Process withdrawal request (legacy - use process_send for new code)
pub async fn process_withdrawal(request: WithdrawRequest) -> Result<WithdrawResponse> {
    let chain = asset_to_chain(&request.asset)
        .map_err(|e| anyhow!("Invalid withdrawal asset: {}", e))?;
    
    // Validate address format
    validate_address(chain, &request.address)?;
    
    // Check if RPC is configured
    if !check_withdrawal_available(&request.asset).await? {
        return Ok(WithdrawResponse {
            success: false,
            txid: None,
            error: Some(format!("{} RPC not configured", chain.as_str())),
        });
    }
    
    // Check minimum withdrawal amount
    if request.amount <= 0.0 {
        return Ok(WithdrawResponse {
            success: false,
            txid: None,
            error: Some("Amount must be greater than zero".to_string()),
        });
    }
    
    tracing::info!(
        "Processing {} withdrawal: user={}, amount={}, address={}",
        chain.as_str(), request.user_id, request.amount, request.address
    );
    
    // TODO: Step 1 - Validate and reserve user balance
    // let balance = get_user_balance(&request.user_id, &request.asset).await?;
    // if balance < request.amount {
    //     return Ok(WithdrawResponse {
    //         success: false,
    //         txid: None,
    //         error: Some("Insufficient balance".to_string()),
    //     });
    // }
    // reserve_balance(&request.user_id, &request.asset, request.amount).await?;
    
    // Step 2 - Build raw transaction
    let tx_hex = match build_raw_transaction(
        &request.asset,
        &request.address,
        request.amount,
        &request.user_id,
    ).await {
        Ok(hex) => hex,
        Err(e) => {
            tracing::error!(
                "{} withdrawal tx build failed: user={}, error={}",
                chain.as_str(), request.user_id, e
            );
            // TODO: Release reserved balance
            return Ok(WithdrawResponse {
                success: false,
                txid: None,
                error: Some(format!("Failed to build transaction: {}", e)),
            });
        }
    };
    
    // Step 3 - Broadcast transaction
    match broadcast_raw_tx(chain, &tx_hex).await {
        Ok(txid) => {
            tracing::info!(
                "✅ {} withdrawal successful: user={}, txid={}, amount={}",
                chain.as_str(), request.user_id, txid, request.amount
            );
            
            // TODO: Step 4 - Finalize withdrawal
            // - Deduct balance permanently
            // - Store withdrawal record with txid
            // - Update withdrawal status to Confirmed(txid)
            
            Ok(WithdrawResponse {
                success: true,
                txid: Some(txid),
                error: None,
            })
        }
        Err(e) => {
            tracing::error!(
                "❌ {} withdrawal broadcast failed: user={}, error={}",
                chain.as_str(), request.user_id, e
            );
            
            // TODO: Step 5 - Rollback on failure
            // - Release reserved balance
            // - Mark withdrawal as Failed(error)
            
            Ok(WithdrawResponse {
                success: false,
                txid: None,
                error: Some(format!("Broadcast failed: {}", e)),
            })
        }
    }
}

// NOTE: HTTP handler is in main.rs to avoid Axum Handler trait issues

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_asset_to_chain() {
        assert!(matches!(asset_to_chain(&QuoteAsset::Btc), Ok(ExternalChain::Btc)));
        assert!(matches!(asset_to_chain(&QuoteAsset::Bch), Ok(ExternalChain::Bch)));
        assert!(matches!(asset_to_chain(&QuoteAsset::Doge), Ok(ExternalChain::Doge)));
        assert!(asset_to_chain(&QuoteAsset::Land).is_err());
    }
    
    #[test]
    fn test_validate_address() {
        // Bitcoin
        assert!(validate_address(ExternalChain::Btc, "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh").is_ok());
        assert!(validate_address(ExternalChain::Btc, "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        assert!(validate_address(ExternalChain::Btc, "xyz123").is_err());
        
        // Bitcoin Cash
        assert!(validate_address(ExternalChain::Bch, "bitcoincash:qpm2qsznhks23z7629mms6s4cwef74vcwvy22gdx6a").is_ok());
        assert!(validate_address(ExternalChain::Bch, "D123").is_err());
        
        // Dogecoin
        assert!(validate_address(ExternalChain::Doge, "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L").is_ok());
        assert!(validate_address(ExternalChain::Doge, "bc1q").is_err());
    }
}
