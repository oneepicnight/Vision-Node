// UTXO Management System
// Tracks spendable outputs for BTC, BCH, and DOGE

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

use crate::market::engine::QuoteAsset;
use crate::external_rpc::ExternalChain;

/// A single UTXO (Unspent Transaction Output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    /// Transaction ID
    pub txid: String,
    /// Output index (vout)
    pub vout: u32,
    /// Amount in base units (BTC, BCH, DOGE - not satoshis)
    pub amount: f64,
    /// Script pubkey hex
    pub script_pubkey: String,
    /// Number of confirmations
    pub confirmations: u32,
    /// Whether this UTXO is spendable
    pub spendable: bool,
    /// Whether this UTXO is currently locked for a pending transaction
    pub locked: bool,
    /// Address that controls this UTXO
    pub address: String,
    /// When this UTXO was last updated
    pub last_updated: DateTime<Utc>,
}

impl Utxo {
    /// Check if UTXO is ready to spend (confirmed and not locked)
    pub fn is_available(&self) -> bool {
        self.spendable && !self.locked && self.confirmations >= 1
    }
    
    /// Convert amount to satoshis (or equivalent base unit)
    pub fn amount_satoshis(&self) -> u64 {
        (self.amount * 100_000_000.0) as u64
    }
}

/// UTXO storage per user and asset
/// Structure: user_id -> asset -> Vec<Utxo>
pub static USER_UTXOS: Lazy<Arc<Mutex<HashMap<String, HashMap<QuoteAsset, Vec<Utxo>>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// UTXO Manager for tracking and selecting spendable outputs
pub struct UtxoManager;

impl UtxoManager {
    /// Get all UTXOs for a user and asset
    pub fn get_user_utxos(user_id: &str, asset: QuoteAsset) -> Vec<Utxo> {
        USER_UTXOS.lock()
            .ok()
            .and_then(|utxos| {
                utxos.get(user_id)
                    .and_then(|user_utxos| user_utxos.get(&asset))
                    .map(|u| u.clone())
            })
            .unwrap_or_default()
    }
    
    /// Get available (spendable and unlocked) UTXOs
    pub fn get_available_utxos(user_id: &str, asset: QuoteAsset) -> Vec<Utxo> {
        Self::get_user_utxos(user_id, asset)
            .into_iter()
            .filter(|u| u.is_available())
            .collect()
    }
    
    /// Calculate total available balance from UTXOs
    pub fn calculate_available_balance(user_id: &str, asset: QuoteAsset) -> f64 {
        Self::get_available_utxos(user_id, asset)
            .iter()
            .map(|u| u.amount)
            .sum()
    }
    
    /// Select UTXOs for a transaction using largest-first strategy
    /// Returns (selected_utxos, total_amount, change_amount)
    pub fn select_utxos(
        user_id: &str,
        asset: QuoteAsset,
        target_amount: f64,
        fee: f64,
    ) -> Result<(Vec<Utxo>, f64, f64)> {
        let mut available = Self::get_available_utxos(user_id, asset);
        
        if available.is_empty() {
            return Err(anyhow!("No UTXOs available for spending"));
        }
        
        let total_needed = target_amount + fee;
        
        // Sort by amount descending (largest first)
        available.sort_by(|a, b| b.amount.partial_cmp(&a.amount).unwrap());
        
        let mut selected = Vec::new();
        let mut total = 0.0;
        
        for utxo in available {
            selected.push(utxo.clone());
            total += utxo.amount;
            
            if total >= total_needed {
                break;
            }
        }
        
        if total < total_needed {
            return Err(anyhow!(
                "Insufficient UTXOs: have {:.8}, need {:.8}",
                total,
                total_needed
            ));
        }
        
        let change = total - total_needed;
        
        Ok((selected, total, change))
    }
    
    /// Lock UTXOs for a pending transaction
    pub fn lock_utxos(user_id: &str, asset: QuoteAsset, utxos: &[Utxo]) -> Result<()> {
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        let user_utxos = storage.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(asset)
            .or_insert_with(Vec::new);
        
        for utxo_to_lock in utxos {
            if let Some(utxo) = user_utxos.iter_mut().find(|u| {
                u.txid == utxo_to_lock.txid && u.vout == utxo_to_lock.vout
            }) {
                utxo.locked = true;
            }
        }
        
        tracing::debug!("🔒 Locked {} UTXOs for user {}", utxos.len(), user_id);
        Ok(())
    }
    
    /// Unlock UTXOs (e.g., when transaction fails)
    pub fn unlock_utxos(user_id: &str, asset: QuoteAsset, utxos: &[Utxo]) -> Result<()> {
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        if let Some(user_utxos) = storage.get_mut(user_id).and_then(|u| u.get_mut(&asset)) {
            for utxo_to_unlock in utxos {
                if let Some(utxo) = user_utxos.iter_mut().find(|u| {
                    u.txid == utxo_to_unlock.txid && u.vout == utxo_to_unlock.vout
                }) {
                    utxo.locked = false;
                }
            }
        }
        
        tracing::debug!("🔓 Unlocked {} UTXOs for user {}", utxos.len(), user_id);
        Ok(())
    }
    
    /// Mark UTXOs as spent (remove from storage after successful broadcast)
    pub fn mark_spent(user_id: &str, asset: QuoteAsset, utxos: &[Utxo]) -> Result<()> {
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        if let Some(user_utxos) = storage.get_mut(user_id).and_then(|u| u.get_mut(&asset)) {
            user_utxos.retain(|u| {
                !utxos.iter().any(|spent| spent.txid == u.txid && spent.vout == u.vout)
            });
        }
        
        tracing::info!("✅ Marked {} UTXOs as spent for user {}", utxos.len(), user_id);
        Ok(())
    }
    
    /// Add a new UTXO (e.g., from a deposit)
    pub fn add_utxo(user_id: &str, asset: QuoteAsset, utxo: Utxo) -> Result<()> {
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        let user_utxos = storage.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(asset)
            .or_insert_with(Vec::new);
        
        // Check if UTXO already exists
        if !user_utxos.iter().any(|u| u.txid == utxo.txid && u.vout == utxo.vout) {
            user_utxos.push(utxo);
            tracing::info!("➕ Added new UTXO for user {}", user_id);
        }
        
        Ok(())
    }
    
    /// Sync UTXOs from blockchain via RPC (listunspent)
    pub async fn sync_user_utxos(user_id: &str, asset: QuoteAsset, addresses: Vec<String>) -> Result<()> {
        if addresses.is_empty() {
            return Ok(());
        }
        
        let chain = match asset {
            QuoteAsset::Btc => ExternalChain::Btc,
            QuoteAsset::Bch => ExternalChain::Bch,
            QuoteAsset::Doge => ExternalChain::Doge,
            QuoteAsset::Land => return Err(anyhow!("LAND is not an external chain")),
        };
        
        // Clone client to avoid holding lock across await
        let client = {
            let clients = crate::EXTERNAL_RPC_CLIENTS.lock()
                .map_err(|e| anyhow!("Failed to lock RPC clients: {}", e))?;
            clients.get(&chain)
                .ok_or_else(|| anyhow!("RPC client not available for {}", chain.as_str()))?
                .clone()
        }; // Lock dropped here
        
        // Call listunspent with minimum 0 confirmations to see all UTXOs
        let result = client.call(
            "listunspent",
            serde_json::json!([0, 9999999, addresses])
        ).await?;
        
        // Parse UTXOs from response
        let utxo_array = result.as_array()
            .ok_or_else(|| anyhow!("Invalid listunspent response"))?;
        
        let mut new_utxos = Vec::new();
        
        for utxo_json in utxo_array {
            let utxo = Utxo {
                txid: utxo_json["txid"].as_str().unwrap_or("").to_string(),
                vout: utxo_json["vout"].as_u64().unwrap_or(0) as u32,
                amount: utxo_json["amount"].as_f64().unwrap_or(0.0),
                script_pubkey: utxo_json["scriptPubKey"].as_str().unwrap_or("").to_string(),
                confirmations: utxo_json["confirmations"].as_u64().unwrap_or(0) as u32,
                spendable: utxo_json["spendable"].as_bool().unwrap_or(true),
                locked: false,
                address: utxo_json["address"].as_str().unwrap_or("").to_string(),
                last_updated: Utc::now(),
            };
            
            if !utxo.txid.is_empty() {
                new_utxos.push(utxo);
            }
        }
        
        // Update storage
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        storage.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(asset, new_utxos);
        
        tracing::info!("🔄 Synced UTXOs for user {}: {} found", user_id, utxo_array.len());
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_utxo_selection() {
        let user_id = "test_user";
        let asset = QuoteAsset::Btc;
        
        // Setup test UTXOs
        let mut storage = USER_UTXOS.lock().unwrap();
        let user_utxos = storage.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(asset)
            .or_insert_with(Vec::new);
        
        user_utxos.push(Utxo {
            txid: "tx1".to_string(),
            vout: 0,
            amount: 1.0,
            script_pubkey: "".to_string(),
            confirmations: 6,
            spendable: true,
            locked: false,
            address: "addr1".to_string(),
            last_updated: Utc::now(),
        });
        
        user_utxos.push(Utxo {
            txid: "tx2".to_string(),
            vout: 0,
            amount: 0.5,
            script_pubkey: "".to_string(),
            confirmations: 3,
            spendable: true,
            locked: false,
            address: "addr2".to_string(),
            last_updated: Utc::now(),
        });
        
        drop(storage);
        
        // Test selection
        let result = UtxoManager::select_utxos(user_id, asset, 0.8, 0.0001);
        assert!(result.is_ok());
        
        let (selected, total, change) = result.unwrap();
        assert_eq!(selected.len(), 1); // Should select the 1.0 BTC UTXO
        assert_eq!(total, 1.0);
        assert!((change - 0.1999).abs() < 0.0001);
    }
    
    #[test]
    fn test_utxo_locking() {
        let user_id = "test_user_2";
        let asset = QuoteAsset::Btc;
        
        let utxo = Utxo {
            txid: "tx1".to_string(),
            vout: 0,
            amount: 1.0,
            script_pubkey: "".to_string(),
            confirmations: 6,
            spendable: true,
            locked: false,
            address: "addr1".to_string(),
            last_updated: Utc::now(),
        };
        
        // Add UTXO
        UtxoManager::add_utxo(user_id, asset, utxo.clone()).unwrap();
        
        // Lock it
        UtxoManager::lock_utxos(user_id, asset, &[utxo.clone()]).unwrap();
        
        // Check it's locked
        let available = UtxoManager::get_available_utxos(user_id, asset);
        assert_eq!(available.len(), 0);
        
        // Unlock it
        UtxoManager::unlock_utxos(user_id, asset, &[utxo.clone()]).unwrap();
        
        // Check it's available again
        let available = UtxoManager::get_available_utxos(user_id, asset);
        assert_eq!(available.len(), 1);
    }
}
