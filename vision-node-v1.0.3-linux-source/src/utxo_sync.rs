//! UTXO Background Sync Module
//! 
//! Periodically syncs UTXOs from blockchain nodes for all users.
//! Tracks confirmations, detects reorgs, and updates UTXO states.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;
use anyhow::{Result, anyhow};
use chrono::Utc;

use crate::market::engine::QuoteAsset;
use crate::external_rpc::ExternalChain;
use crate::utxo_manager::{Utxo, UtxoManager, USER_UTXOS};

/// Configuration for UTXO sync behavior
#[derive(Debug, Clone)]
pub struct UtxoSyncConfig {
    /// Interval between sync cycles (default: 30 seconds)
    pub sync_interval: Duration,
    /// Minimum confirmations to consider a UTXO safe (default: 1)
    pub min_confirmations: u32,
    /// Maximum confirmations to track (default: 100)
    pub max_confirmations: u32,
    /// Whether to sync unconfirmed UTXOs (default: true)
    pub sync_unconfirmed: bool,
}

impl Default for UtxoSyncConfig {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(30),
            min_confirmations: 1,
            max_confirmations: 100,
            sync_unconfirmed: true,
        }
    }
}

/// Sync statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    pub total_syncs: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub utxos_updated: u64,
    pub utxos_added: u64,
    pub utxos_removed: u64,
    pub last_sync_timestamp: i64,
    pub last_error: Option<String>,
}

/// Global sync statistics
static SYNC_STATS: once_cell::sync::Lazy<Arc<Mutex<SyncStats>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(SyncStats::default())));

/// UTXO Background Sync Service
pub struct UtxoSyncService {
    config: UtxoSyncConfig,
    running: Arc<Mutex<bool>>,
}

impl UtxoSyncService {
    /// Create a new UTXO sync service with default config
    pub fn new() -> Self {
        Self {
            config: UtxoSyncConfig::default(),
            running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: UtxoSyncConfig) -> Self {
        Self {
            config,
            running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Start the background sync task
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let running = Arc::clone(&self.running);
        
        // Mark as running
        *running.lock().unwrap() = true;
        
        tokio::spawn(async move {
            tracing::info!("🔄 UTXO background sync service started (interval: {:?})", config.sync_interval);
            
            let mut interval = time::interval(config.sync_interval);
            
            loop {
                interval.tick().await;
                
                // Check if still running
                if !*running.lock().unwrap() {
                    tracing::info!("🛑 UTXO sync service stopping");
                    break;
                }
                
                // Update stats
                {
                    let mut stats = SYNC_STATS.lock().unwrap();
                    stats.total_syncs += 1;
                    stats.last_sync_timestamp = Utc::now().timestamp();
                }
                
                // Perform sync
                match Self::sync_all_users(&config).await {
                    Ok(updated) => {
                        let mut stats = SYNC_STATS.lock().unwrap();
                        stats.successful_syncs += 1;
                        stats.utxos_updated += updated as u64;
                        tracing::debug!("✅ UTXO sync completed: {} UTXOs updated", updated);
                    }
                    Err(e) => {
                        let mut stats = SYNC_STATS.lock().unwrap();
                        stats.failed_syncs += 1;
                        stats.last_error = Some(e.to_string());
                        tracing::error!("❌ UTXO sync failed: {}", e);
                    }
                }
            }
            
            tracing::info!("🛑 UTXO background sync service stopped");
        })
    }
    
    /// Stop the background sync task
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
    }
    
    /// Sync all users' UTXOs
    async fn sync_all_users(config: &UtxoSyncConfig) -> Result<usize> {
        let mut total_updated = 0;
        
        // Get snapshot of all users and their assets
        let users_snapshot = {
            let storage = USER_UTXOS.lock()
                .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
            
            let mut users = Vec::new();
            for (user_id, assets) in storage.iter() {
                for asset in assets.keys() {
                    users.push((user_id.clone(), *asset));
                }
            }
            users
        };
        
        if users_snapshot.is_empty() {
            return Ok(0);
        }
        
        tracing::debug!("🔄 Syncing UTXOs for {} user-asset pairs", users_snapshot.len());
        
        // Sync each user-asset pair
        for (user_id, asset) in users_snapshot {
            match Self::sync_user_asset(&user_id, asset, config).await {
                Ok(count) => total_updated += count,
                Err(e) => {
                    tracing::warn!("Failed to sync UTXOs for user {} asset {:?}: {}", user_id, asset, e);
                }
            }
        }
        
        Ok(total_updated)
    }
    
    /// Sync UTXOs for a specific user and asset
    async fn sync_user_asset(user_id: &str, asset: QuoteAsset, config: &UtxoSyncConfig) -> Result<usize> {
        // Skip LAND (not an external chain)
        if asset == QuoteAsset::Land {
            return Ok(0);
        }
        
        let chain = match asset {
            QuoteAsset::Btc => ExternalChain::Btc,
            QuoteAsset::Bch => ExternalChain::Bch,
            QuoteAsset::Doge => ExternalChain::Doge,
            QuoteAsset::Land => return Ok(0),
        };
        
        // Get existing UTXOs to extract addresses
        let existing_utxos = UtxoManager::get_user_utxos(user_id, asset);
        if existing_utxos.is_empty() {
            return Ok(0);
        }
        
        // Extract unique addresses
        let addresses: Vec<String> = existing_utxos
            .iter()
            .map(|u| u.address.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        
        if addresses.is_empty() {
            return Ok(0);
        }
        
        // Get RPC client
        let client = {
            let clients = crate::EXTERNAL_RPC_CLIENTS.lock();
            clients.get(chain)
                .ok_or_else(|| anyhow!("RPC client not available for {:?}", chain))?
                .clone()
        };
        
        // Call listunspent with appropriate confirmation range
        let min_conf = if config.sync_unconfirmed { 0 } else { config.min_confirmations };
        let max_conf = config.max_confirmations;
        
        let result = client.call(
            "listunspent",
            serde_json::json!([min_conf, max_conf, addresses])
        ).await?;
        
        let utxo_array = result.as_array()
            .ok_or_else(|| anyhow!("Invalid listunspent response"))?;
        
        // Parse new UTXO data
        let mut new_utxos = Vec::new();
        for utxo_json in utxo_array {
            let utxo = Utxo {
                txid: utxo_json["txid"].as_str().unwrap_or("").to_string(),
                vout: utxo_json["vout"].as_u64().unwrap_or(0) as u32,
                amount: utxo_json["amount"].as_f64().unwrap_or(0.0),
                script_pubkey: utxo_json["scriptPubKey"].as_str().unwrap_or("").to_string(),
                confirmations: utxo_json["confirmations"].as_u64().unwrap_or(0) as u32,
                spendable: utxo_json["spendable"].as_bool().unwrap_or(true),
                locked: false, // Preserve locked state from existing UTXOs
                address: utxo_json["address"].as_str().unwrap_or("").to_string(),
                last_updated: Utc::now(),
            };
            
            if !utxo.txid.is_empty() {
                new_utxos.push(utxo);
            }
        }
        
        // Update storage with reorg detection
        let updated_count = Self::merge_utxos(user_id, asset, &existing_utxos, new_utxos)?;
        
        Ok(updated_count)
    }
    
    /// Merge new UTXOs with existing ones, detecting reorgs and updates
    fn merge_utxos(
        user_id: &str,
        asset: QuoteAsset,
        existing: &[Utxo],
        new: Vec<Utxo>,
    ) -> Result<usize> {
        let mut storage = USER_UTXOS.lock()
            .map_err(|e| anyhow!("Failed to lock UTXO storage: {}", e))?;
        
        let user_utxos = storage.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .entry(asset)
            .or_insert_with(Vec::new);
        
        let mut updated_count = 0;
        
        // Create a map of new UTXOs for quick lookup
        let new_map: HashMap<(String, u32), &Utxo> = new
            .iter()
            .map(|u| ((u.txid.clone(), u.vout), u))
            .collect();
        
        // Update existing UTXOs
        for utxo in user_utxos.iter_mut() {
            if let Some(new_utxo) = new_map.get(&(utxo.txid.clone(), utxo.vout)) {
                // UTXO still exists - update confirmations
                if utxo.confirmations != new_utxo.confirmations {
                    utxo.confirmations = new_utxo.confirmations;
                    utxo.last_updated = Utc::now();
                    updated_count += 1;
                }
            }
        }
        
        // Remove UTXOs that disappeared (spent or reorg'd)
        let before_count = user_utxos.len();
        let removed_utxos: Vec<_> = user_utxos.iter()
            .filter(|u| !u.locked && !new_map.contains_key(&(u.txid.clone(), u.vout)))
            .map(|u| (u.txid.clone(), u.vout))
            .collect();
        
        user_utxos.retain(|u| {
            // Keep locked UTXOs even if they disappeared (might be in mempool)
            if u.locked {
                return true;
            }
            // Keep UTXOs that still exist in blockchain
            new_map.contains_key(&(u.txid.clone(), u.vout))
        });
        let removed = before_count - user_utxos.len();
        
        if removed > 0 {
            let mut stats = SYNC_STATS.lock().unwrap();
            stats.utxos_removed += removed as u64;
            tracing::info!("🗑️ Removed {} spent/reorg'd UTXOs for user {}", removed, user_id);
            
            // Send WebSocket notifications for removed UTXOs
            for (txid, vout) in removed_utxos {
                crate::ws_notifications::WsNotificationManager::notify_utxo_removed(
                    user_id,
                    &txid,
                    vout,
                    asset,
                );
            }
        }
        
        // Add new UTXOs that don't exist yet
        for new_utxo in new {
            let exists = user_utxos.iter().any(|u| {
                u.txid == new_utxo.txid && u.vout == new_utxo.vout
            });
            
            if !exists {
                user_utxos.push(new_utxo.clone());
                let mut stats = SYNC_STATS.lock().unwrap();
                stats.utxos_added += 1;
                updated_count += 1;
                tracing::info!("➕ New UTXO detected for user {}", user_id);
                
                // Send WebSocket notification for new UTXO
                crate::ws_notifications::WsNotificationManager::notify_utxo_added(
                    user_id,
                    &new_utxo.txid,
                    new_utxo.vout,
                    asset,
                    new_utxo.amount,
                    new_utxo.confirmations,
                );
            }
        }
        
        Ok(updated_count)
    }
    
    /// Get current sync statistics
    pub fn get_stats() -> SyncStats {
        SYNC_STATS.lock().unwrap().clone()
    }
    
    /// Reset sync statistics
    pub fn reset_stats() {
        let mut stats = SYNC_STATS.lock().unwrap();
        *stats = SyncStats::default();
    }
}

impl Default for UtxoSyncService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_defaults() {
        let config = UtxoSyncConfig::default();
        assert_eq!(config.sync_interval, Duration::from_secs(30));
        assert_eq!(config.min_confirmations, 1);
        assert_eq!(config.max_confirmations, 100);
        assert!(config.sync_unconfirmed);
    }

    #[test]
    fn test_sync_stats() {
        UtxoSyncService::reset_stats();
        let stats = UtxoSyncService::get_stats();
        assert_eq!(stats.total_syncs, 0);
        assert_eq!(stats.successful_syncs, 0);
        assert_eq!(stats.failed_syncs, 0);
    }
}
