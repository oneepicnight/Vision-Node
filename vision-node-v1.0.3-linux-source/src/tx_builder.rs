// Transaction Construction System
// Builds, signs, and serializes transactions for BTC, BCH, DOGE
//
// NOTE: This is a DEVELOPMENT-ONLY implementation with simplified transaction building.
// For production, integrate with bitcoincore-rpc's wallet functions or implement
// full PSBT (Partially Signed Bitcoin Transaction) support.

use anyhow::{Result, anyhow};

use crate::market::engine::QuoteAsset;
use crate::external_rpc::ExternalChain;
use crate::utxo_manager::UtxoManager;

/// Transaction builder for external chains
/// Uses RPC wallet functions for development simplicity
pub struct TransactionBuilder;

impl TransactionBuilder {
    /// Build, sign, and serialize a transaction for sending funds
    /// Returns hex-encoded signed transaction ready for broadcast
    /// 
    /// DEVELOPMENT APPROACH: Uses RPC wallet functions (sendtoaddress, createrawtransaction)
    /// instead of manual signing. This requires the RPC wallet to have funds.
    #[cfg(feature = "dev-signing")]
    pub async fn build_send_transaction(
        user_id: &str,
        asset: QuoteAsset,
        to_address: &str,
        amount: f64,
        fee: f64,
    ) -> Result<String> {
        tracing::info!(
            "🔨 Building transaction for user {}: {:.8} {} to {}",
            user_id, amount, asset.as_str(), to_address
        );
        
        // For development: Use Bitcoin Core's createrawtransaction + signrawtransactionwithwallet
        // This avoids complex manual signing implementation
        
        // Step 1: Select UTXOs
        let (selected_utxos, total_input, change_amount) = 
            UtxoManager::select_utxos(user_id, asset, amount, fee)?;
        
        tracing::info!(
            "📦 Selected {} inputs: {:.8} total, {:.8} change",
            selected_utxos.len(),
            total_input,
            change_amount
        );
        
        // Step 2: Build inputs array for createrawtransaction
        let inputs: Vec<serde_json::Value> = selected_utxos.iter().map(|utxo| {
            serde_json::json!({
                "txid": utxo.txid,
                "vout": utxo.vout
            })
        }).collect();
        
        // Step 3: Build outputs object
        let mut outputs = serde_json::Map::new();
        outputs.insert(to_address.to_string(), serde_json::json!(amount));
        
        // Add change output if significant
        if change_amount > Self::get_dust_threshold(asset) {
            let change_addr = Self::get_change_address_string(user_id, asset)?;
            outputs.insert(change_addr, serde_json::json!(change_amount));
            tracing::debug!("💰 Change output: {:.8}", change_amount);
        }
        
        // Step 4: Create raw transaction via RPC
        let chain = match asset {
            QuoteAsset::Btc => ExternalChain::Btc,
            QuoteAsset::Bch => ExternalChain::Bch,
            QuoteAsset::Doge => ExternalChain::Doge,
            QuoteAsset::Land => return Err(anyhow!("LAND is not an external chain")),
        };
        
        // Clone client to avoid holding lock across await
        let client = {
            let clients = crate::EXTERNAL_RPC_CLIENTS.lock();
            clients.get(chain)
                .ok_or_else(|| anyhow!("RPC client not available for {}", chain.as_str()))?
                .clone()
        }; // Lock dropped here
        
        let raw_tx = client.call(
            "createrawtransaction",
            serde_json::json!([inputs, outputs])
        ).await?;
        
        let unsigned_hex = raw_tx.as_str()
            .ok_or_else(|| anyhow!("Invalid createrawtransaction response"))?;
        
        tracing::debug!("📝 Created unsigned tx: {} bytes", unsigned_hex.len() / 2);
        
        // Step 5: Sign the transaction using wallet
        // NOTE: This requires the RPC wallet to have imported the private keys
        let signed_result = client.call(
            "signrawtransactionwithwallet",
            serde_json::json!([unsigned_hex])
        ).await?;
        
        let complete = signed_result["complete"].as_bool().unwrap_or(false);
        if !complete {
            let errors = &signed_result["errors"];
            return Err(anyhow!("Transaction signing incomplete: {:?}", errors));
        }
        
        let signed_hex = signed_result["hex"].as_str()
            .ok_or_else(|| anyhow!("No hex in sign response"))?
            .to_string();
        
        tracing::info!("✍️ Signed transaction: {} bytes", signed_hex.len() / 2);
        
        Ok(signed_hex)
    }
    
    /// Get change address string for user (simplified)
    fn get_change_address_string(user_id: &str, asset: QuoteAsset) -> Result<String> {
        // In production, derive from HD wallet or get from user's deposit address
        // For development, use a placeholder or the user's deposit address
        
        use crate::market::wallet::WALLETS;
        
        if let Ok(wallets) = WALLETS.lock() {
            if let Some(wallet) = wallets.get(user_id) {
                if let Some(addr) = wallet.get_deposit_address(asset) {
                    tracing::debug!("Using user deposit address as change: {}", addr);
                    return Ok(addr);
                }
            }
        }
        
        // Fallback to placeholder
        let placeholder = match asset {
            QuoteAsset::Btc => "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            QuoteAsset::Bch => "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            QuoteAsset::Doge => "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L",
            QuoteAsset::Land => return Err(anyhow!("LAND does not use addresses")),
        };
        
        tracing::warn!(
            "⚠️ Using placeholder change address for user {} - no deposit address found",
            user_id
        );
        
        Ok(placeholder.to_string())
    }
    
    /// Get dust threshold for a chain (minimum output value)
    fn get_dust_threshold(asset: QuoteAsset) -> f64 {
        match asset {
            QuoteAsset::Btc => 0.00000546, // 546 satoshis
            QuoteAsset::Bch => 0.00000546, // Same as BTC
            QuoteAsset::Doge => 0.01,      // 1,000,000 dogeoshi (DOGE has high dust)
            QuoteAsset::Land => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dust_threshold() {
        assert!(TransactionBuilder::get_dust_threshold(QuoteAsset::Btc) > 0.0);
        assert!(TransactionBuilder::get_dust_threshold(QuoteAsset::Doge) > 
                TransactionBuilder::get_dust_threshold(QuoteAsset::Btc));
    }
}
