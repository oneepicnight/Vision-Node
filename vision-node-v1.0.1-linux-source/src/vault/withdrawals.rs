// Vault Withdrawal System - Send Founder crypto to their configured addresses
//
// Withdraws accumulated BTC/BCH/DOGE from Founder1 and Founder2 vault buckets
// and broadcasts to their configured addresses in token_accounts.toml

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::external_rpc::ExternalChain;
use crate::foundation_config;
use crate::market::engine::QuoteAsset;
use crate::receipts::{write_receipt, Receipt};
use crate::vault::store::{VaultBucket, VaultStore};
use crate::withdrawals::broadcast_raw_tx;
use crate::utxo_signing::{SigningInput, TransactionSigner, WifKey};

/// Founder identifier for withdrawal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Founder {
    Founder1,
    Founder2,
}

impl Founder {
    fn as_str(&self) -> &'static str {
        match self {
            Founder::Founder1 => "founder1",
            Founder::Founder2 => "founder2",
        }
    }

    fn to_bucket(&self) -> VaultBucket {
        match self {
            Founder::Founder1 => VaultBucket::Founder1,
            Founder::Founder2 => VaultBucket::Founder2,
        }
    }

    /// Get configured crypto address for this founder and asset
    fn get_address(&self, asset: QuoteAsset) -> Option<String> {
        // Note: foundation_config only has {founder1,founder2}_{btc,bch,doge}_address
        // which return Option<String>, not the generic founder1_address() which returns String
        match self {
            Founder::Founder1 => match asset {
                QuoteAsset::Btc => crate::foundation_config::founder1_btc_address(),
                QuoteAsset::Bch => crate::foundation_config::founder1_bch_address(),
                QuoteAsset::Doge => crate::foundation_config::founder1_doge_address(),
                QuoteAsset::Land => None,
            },
            Founder::Founder2 => match asset {
                QuoteAsset::Btc => crate::foundation_config::founder2_btc_address(),
                QuoteAsset::Bch => crate::foundation_config::founder2_bch_address(),
                QuoteAsset::Doge => crate::foundation_config::founder2_doge_address(),
                QuoteAsset::Land => None,
            },
        }
    }
}

/// Withdrawal request
#[derive(Debug, Deserialize)]
pub struct VaultWithdrawalRequest {
    pub founder: String, // "founder1" or "founder2"
    pub asset: String,   // "BTC", "BCH", "DOGE"
    pub amount: Option<f64>, // Optional: withdraw specific amount (default: all)
}

/// Withdrawal response
#[derive(Debug, Serialize)]
pub struct VaultWithdrawalResponse {
    pub success: bool,
    pub founder: String,
    pub asset: String,
    pub amount: f64,
    pub txid: Option<String>,
    pub to_address: String,
    pub error: Option<String>,
}

/// Parse founder from string
fn parse_founder(s: &str) -> Result<Founder> {
    match s.to_lowercase().as_str() {
        "founder1" | "1" => Ok(Founder::Founder1),
        "founder2" | "2" => Ok(Founder::Founder2),
        _ => Err(anyhow!("Invalid founder: must be 'founder1' or 'founder2'")),
    }
}

/// Parse asset from string
fn parse_asset(s: &str) -> Result<QuoteAsset> {
    match s.to_uppercase().as_str() {
        "BTC" => Ok(QuoteAsset::Btc),
        "BCH" => Ok(QuoteAsset::Bch),
        "DOGE" => Ok(QuoteAsset::Doge),
        _ => Err(anyhow!("Invalid asset: must be BTC, BCH, or DOGE")),
    }
}

/// Convert QuoteAsset to ExternalChain
fn asset_to_chain(asset: QuoteAsset) -> Result<ExternalChain> {
    match asset {
        QuoteAsset::Btc => Ok(ExternalChain::Btc),
        QuoteAsset::Bch => Ok(ExternalChain::Bch),
        QuoteAsset::Doge => Ok(ExternalChain::Doge),
        QuoteAsset::Land => Err(anyhow!("LAND is not an external chain")),
    }
}

/// Estimate network fee for vault withdrawal transaction
fn estimate_vault_fee(chain: ExternalChain) -> f64 {
    // Conservative fee estimation for vault withdrawals
    // In production, query fee rate from RPC (estimatesmartfee)
    match chain {
        ExternalChain::Btc => 0.00001,  // ~$0.50 at $50k BTC
        ExternalChain::Bch => 0.00001,  // Very low BCH fees
        ExternalChain::Doge => 0.5,     // DOGE has higher nominal fees but cheap in USD
    }
}

/// Process vault withdrawal - sends founder's accumulated crypto to their configured address
pub async fn process_vault_withdrawal(req: VaultWithdrawalRequest) -> Result<VaultWithdrawalResponse> {
    let founder = match parse_founder(&req.founder) {
        Ok(f) => f,
        Err(e) => return Err(anyhow!("Invalid founder: {}", e)),
    };
    
    let asset = match parse_asset(&req.asset) {
        Ok(a) => a,
        Err(e) => return Err(anyhow!("Invalid asset: {}", e)),
    };
    
    let chain = match asset_to_chain(asset) {
        Ok(c) => c,
        Err(e) => return Err(anyhow!("Invalid chain: {}", e)),
    };

    // Get founder's configured address
    let to_address = match founder.get_address(asset) {
        Some(addr) => addr,
        None => return Err(anyhow!("No payout address configured for {} {}", founder.as_str(), asset.as_str())),
    };

    // Get database
    let db = {
        let chain_guard = crate::CHAIN.lock();
        chain_guard.db.clone()
    };

    let store = VaultStore::new(db.clone());
    let bucket = founder.to_bucket();

    // Get current balance
    let balance_units = match store.get_bucket_balance(bucket, asset) {
        Ok(balance) => balance,
        Err(e) => return Err(anyhow!("Failed to get balance: {}", e)),
    };
    let balance_f64 = balance_units as f64 / 100_000_000.0;

    if balance_units == 0 {
        return Err(anyhow!("No {} balance to withdraw", asset.as_str()));
    }

    // Determine withdrawal amount
    let withdraw_amount = match req.amount {
        Some(amt) if amt > 0.0 && amt <= balance_f64 => amt,
        Some(_) => {
            return Err(anyhow!("Invalid amount, max balance: {:.8}", balance_f64));
        }
        None => balance_f64, // Withdraw all
    };

    let withdraw_units = (withdraw_amount * 100_000_000.0) as u128;

    tracing::info!(
        "🏦 Vault withdrawal: {} withdrawing {:.8} {} → {}",
        founder.as_str(),
        withdraw_amount,
        asset.as_str(),
        to_address
    );

    // Get vault hot wallet private key (WIF format)
    // In production, this would come from secure key storage (HSM, KMS, etc.)
    let vault_wif = match get_vault_private_key(founder, asset) {
        Ok(wif) => wif,
        Err(e) => {
            tracing::error!(
                "❌ Failed to get vault private key: {} {} - {}",
                founder.as_str(),
                asset.as_str(),
                e
            );
            return Err(anyhow!("Withdrawal failed"));
        }
    };

    // Parse WIF key
    let wif_key = match WifKey::from_wif(&vault_wif) {
        Ok(key) => key,
        Err(e) => {
            tracing::error!(
                "❌ Invalid vault WIF key: {} {} - {}",
                founder.as_str(),
                asset.as_str(),
                e
            );
            return Err(anyhow!("Invalid vault WIF key: {}", e));
        }
    };

    // Build unsigned transaction
    // For now, this is a placeholder - full implementation would:
    // 1. Select UTXOs for this founder's vault
    // 2. Build raw transaction with createrawtransaction
    // 3. Sign with client-side signing
    // 4. Broadcast
    
    // Get vault UTXO for this founder
    let vault_user_id = format!("vault:{}", founder.as_str());
    
    // Estimate fee
    let fee = estimate_vault_fee(chain);
    
    // Build unsigned transaction hex (simplified)
    let (raw_tx_hex, signing_inputs) = match build_unsigned_vault_transaction(
        &vault_user_id,
        asset,
        &to_address,
        withdraw_amount,
        fee,
        chain,
    ).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(
                "❌ Failed to build unsigned transaction: {} {} - {}",
                founder.as_str(),
                asset.as_str(),
                e
            );
            return Err(anyhow!("Failed to build transaction: {}", e));
        }
    };

    tracing::info!(
        "📦 Built unsigned vault withdrawal tx: {} bytes, {} inputs",
        raw_tx_hex.len() / 2,
        signing_inputs.len()
    );

    // Sign each input with client-side signing
    let mut signatures = Vec::new();
    for (idx, signing_input) in signing_inputs.iter().enumerate() {
        let signature = match TransactionSigner::sign_input(
            &raw_tx_hex,
            idx,
            signing_input,
            &wif_key,
            chain,
        ) {
            Ok(sig) => sig,
            Err(e) => {
                tracing::error!(
                    "❌ Failed to sign input {}: {} {} - {}",
                    idx,
                    founder.as_str(),
                    asset.as_str(),
                    e
                );
                return Err(anyhow!("Failed to sign input {}: {}", idx, e));
            }
        };
        
        let pubkey = wif_key.public_key();
        signatures.push((idx, signature, pubkey));
    }

    // Apply signatures to build final signed transaction
    let signed_tx_hex = match TransactionSigner::apply_signatures(
        &raw_tx_hex,
        signatures,
        wif_key.compressed,
    ) {
        Ok(hex) => hex,
        Err(e) => {
            tracing::error!(
                "❌ Failed to apply signatures: {} {} - {}",
                founder.as_str(),
                asset.as_str(),
                e
            );
            return Err(anyhow!("Failed to apply signatures: {}", e));
        }
    };

    tracing::info!(
        "✍️ Signed vault withdrawal tx: {} bytes",
        signed_tx_hex.len() / 2
    );

    // Broadcast transaction
    match broadcast_raw_tx(chain, &signed_tx_hex).await {
            Ok(txid) => {
                // Debit vault on successful broadcast
                if let Err(e) = store.debit_vault(bucket, asset, withdraw_units) {
                    tracing::error!(
                        "⚠️ Vault withdrawal broadcast succeeded but debit failed: {} {} {} - {}",
                        founder.as_str(),
                        asset.as_str(),
                        txid,
                        e
                    );
                }

                tracing::info!(
                    "✅ Vault withdrawal successful: {} {} {} → {} (txid: {})",
                    founder.as_str(),
                    withdraw_amount,
                    asset.as_str(),
                    to_address,
                    txid
                );

                // Write receipt
                let receipt = Receipt {
                    id: String::new(),
                    ts_ms: 0,
                    kind: "vault_withdrawal".to_string(),
                    from: format!("vault:{}:{}", founder.as_str(), asset.as_str()),
                    to: to_address.clone(),
                    amount: withdraw_units.to_string(),
                    fee: "0".to_string(), // TODO: Calculate actual network fee
                    memo: Some(format!("Vault withdrawal: {} {}", founder.as_str(), asset.as_str())),
                    txid: Some(txid.clone()),
                    ok: true,
                    note: None,
                };

                if let Err(e) = write_receipt(&db, None, receipt) {
                    tracing::warn!("Failed to write withdrawal receipt: {}", e);
                }

                Ok(VaultWithdrawalResponse {
                    success: true,
                    founder: founder.as_str().to_string(),
                    asset: asset.as_str().to_string(),
                    amount: withdraw_amount,
                    txid: Some(txid),
                    to_address,
                    error: None,
                })
            }
            Err(e) => {
                tracing::error!(
                    "❌ Vault withdrawal broadcast failed: {} {} {} - {}",
                    founder.as_str(),
                    asset.as_str(),
                    withdraw_amount,
                    e
                );

                Err(anyhow!("Broadcast failed: {}", e))
            }
        }
}

/// Get vault private key for a founder (WIF format)
/// In production, this would retrieve from secure storage (HSM, KMS, encrypted vault)
fn get_vault_private_key(founder: Founder, asset: QuoteAsset) -> Result<String> {
    // For now, check environment variables
    // In production: integrate with key_manager.rs or HSM
    let env_key = format!(
        "VAULT_{}_{}",
        founder.as_str().to_uppercase(),
        asset.as_str().to_uppercase()
    );
    
    std::env::var(&env_key)
        .map_err(|_| anyhow!(
            "Vault private key not found. Set {} environment variable with WIF private key.",
            env_key
        ))
}

/// Build unsigned transaction for vault withdrawal
/// Returns (raw_tx_hex, signing_inputs)
async fn build_unsigned_vault_transaction(
    vault_user_id: &str,
    asset: QuoteAsset,
    to_address: &str,
    amount: f64,
    fee: f64,
    chain: ExternalChain,
) -> Result<(String, Vec<SigningInput>)> {
    // Get RPC client
    let client = {
        let clients = crate::EXTERNAL_RPC_CLIENTS.lock()
            .map_err(|e| anyhow!("Failed to lock RPC clients: {}", e))?;
        clients.get(&chain)
            .ok_or_else(|| anyhow!("RPC client not available for {}", chain.as_str()))?
            .clone()
    };

    // Select UTXOs (simplified - uses listunspent for vault addresses)
    let vault_address = get_vault_deposit_address(vault_user_id, asset)?;
    
    let unspent_result = client.call(
        "listunspent",
        serde_json::json!([0, 9999999, [vault_address]])
    ).await?;
    
    let utxos = unspent_result.as_array()
        .ok_or_else(|| anyhow!("Invalid listunspent response"))?;
    
    if utxos.is_empty() {
        return Err(anyhow!("No UTXOs available for vault"));
    }

    // Select enough UTXOs to cover amount + fee
    let total_needed = amount + fee;
    let amount_sats = (amount * 100_000_000.0) as u64;
    let fee_sats = (fee * 100_000_000.0) as u64;
    let mut selected_utxos = Vec::new();
    let mut total_selected: u64 = 0;

    for utxo in utxos {
        let value = utxo["amount"].as_f64().unwrap_or(0.0);
        let value_sats = (value * 100_000_000.0) as u64;
        
        selected_utxos.push(utxo.clone());
        total_selected += value_sats;
        
        if total_selected >= (amount_sats + fee_sats) {
            break;
        }
    }

    if total_selected < (amount_sats + fee_sats) {
        return Err(anyhow!(
            "Insufficient UTXOs: have {} sats, need {} sats",
            total_selected,
            amount_sats + fee_sats
        ));
    }

    // Build inputs array
    let inputs: Vec<serde_json::Value> = selected_utxos.iter().map(|utxo| {
        serde_json::json!({
            "txid": utxo["txid"].as_str().unwrap_or(""),
            "vout": utxo["vout"].as_u64().unwrap_or(0)
        })
    }).collect();

    // Build outputs
    let mut outputs = serde_json::Map::new();
    outputs.insert(to_address.to_string(), serde_json::json!(amount));
    
    // Add change if necessary
    let change_sats = total_selected - amount_sats - fee_sats;
    if change_sats > 546 {  // Dust threshold
        let change_btc = change_sats as f64 / 100_000_000.0;
        outputs.insert(vault_address.clone(), serde_json::json!(change_btc));
    }

    // Create raw transaction
    let raw_tx_result = client.call(
        "createrawtransaction",
        serde_json::json!([inputs, outputs])
    ).await?;
    
    let raw_tx_hex = raw_tx_result.as_str()
        .ok_or_else(|| anyhow!("Invalid createrawtransaction response"))?
        .to_string();

    // Build signing inputs
    let signing_inputs: Vec<SigningInput> = selected_utxos.iter().map(|utxo| {
        let txid = utxo["txid"].as_str().unwrap_or("");
        let vout = utxo["vout"].as_u64().unwrap_or(0) as u32;
        let script_pubkey_hex = utxo["scriptPubKey"].as_str().unwrap_or("");
        let amount_btc = utxo["amount"].as_f64().unwrap_or(0.0);
        let amount_sats = (amount_btc * 100_000_000.0) as u64;
        
        // Decode and reverse txid for signing
        let mut prev_tx_hash = hex::decode(txid).unwrap_or_default();
        prev_tx_hash.reverse();
        
        SigningInput {
            prev_tx_hash,
            prev_vout: vout,
            script_pubkey: hex::decode(script_pubkey_hex).unwrap_or_default(),
            amount_satoshis: Some(amount_sats),
            sequence: 0xffffffff,
        }
    }).collect();

    Ok((raw_tx_hex, signing_inputs))
}

/// Get vault deposit address for user
fn get_vault_deposit_address(vault_user_id: &str, asset: QuoteAsset) -> Result<String> {
    crate::market::deposits::deposit_address_for_user(vault_user_id, asset)
        .map_err(|e| anyhow!("Failed to get vault deposit address: {}", e))
}

/// Get withdrawal status for a founder - shows balances available for withdrawal
pub fn get_withdrawal_status(founder_str: &str) -> Result<serde_json::Value> {
    let founder = parse_founder(founder_str)?;
    let bucket = founder.to_bucket();

    let db = {
        let chain_guard = crate::CHAIN.lock();
        chain_guard.db.clone()
    };

    let store = VaultStore::new(db);

    let btc_units = store.get_bucket_balance(bucket, QuoteAsset::Btc).unwrap_or(0);
    let bch_units = store.get_bucket_balance(bucket, QuoteAsset::Bch).unwrap_or(0);
    let doge_units = store.get_bucket_balance(bucket, QuoteAsset::Doge).unwrap_or(0);

    let btc_address = founder.get_address(QuoteAsset::Btc);
    let bch_address = founder.get_address(QuoteAsset::Bch);
    let doge_address = founder.get_address(QuoteAsset::Doge);

    Ok(serde_json::json!({
        "founder": founder.as_str(),
        "balances": {
            "BTC": {
                "units": btc_units.to_string(),
                "amount": (btc_units as f64 / 100_000_000.0),
                "address": btc_address,
                "withdrawable": btc_units > 0
            },
            "BCH": {
                "units": bch_units.to_string(),
                "amount": (bch_units as f64 / 100_000_000.0),
                "address": bch_address,
                "withdrawable": bch_units > 0
            },
            "DOGE": {
                "units": doge_units.to_string(),
                "amount": (doge_units as f64 / 100_000_000.0),
                "address": doge_address,
                "withdrawable": doge_units > 0
            }
        }
    }))
}
