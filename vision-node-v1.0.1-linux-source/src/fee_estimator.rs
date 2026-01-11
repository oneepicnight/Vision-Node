//! Dynamic Fee Estimation Module
//! 
//! Provides smart fee estimation using blockchain RPC's estimatesmartfee.
//! Caches fee estimates to reduce RPC load and provides multiple fee tiers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::external_rpc::{ExternalChain, RpcClient};

/// Fee tier for transaction priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FeeTier {
    /// Low priority - may take hours
    Economy,
    /// Normal priority - typically next block or two
    Normal,
    /// High priority - aim for next block
    Priority,
}

impl FeeTier {
    /// Get the confirmation target in blocks for this tier
    pub fn target_blocks(&self) -> u32 {
        match self {
            FeeTier::Economy => 6,    // ~1 hour
            FeeTier::Normal => 3,     // ~30 minutes
            FeeTier::Priority => 1,   // ~10 minutes (next block)
        }
    }
}

/// Fee estimate for a specific chain and tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    /// Chain identifier (btc, bch, doge)
    pub chain: String,
    /// Fee tier
    pub tier: FeeTier,
    /// Estimated fee per byte in satoshis
    pub fee_per_byte: u64,
    /// Total estimated fee for a typical transaction (in satoshis)
    pub typical_tx_fee: u64,
    /// Number of blocks target
    pub blocks: u32,
    /// Unix timestamp when estimate was fetched
    #[serde(skip)]
    pub fetched_at: u64,
}

/// Cached fee estimates with expiration
struct FeeCache {
    estimates: HashMap<(String, FeeTier), FeeEstimate>,
    ttl: Duration,
}

impl FeeCache {
    fn new(ttl: Duration) -> Self {
        Self {
            estimates: HashMap::new(),
            ttl,
        }
    }
    
    fn get(&self, chain: &str, tier: FeeTier) -> Option<FeeEstimate> {
        let key = (chain.to_string(), tier);
        if let Some(estimate) = self.estimates.get(&key) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            if now - estimate.fetched_at < self.ttl.as_secs() {
                return Some(estimate.clone());
            }
        }
        None
    }
    
    fn set(&mut self, estimate: FeeEstimate) {
        let key = (estimate.chain.clone(), estimate.tier);
        self.estimates.insert(key, estimate);
    }
    
    fn clear_expired(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.estimates.retain(|_, estimate| {
            now - estimate.fetched_at < self.ttl.as_secs()
        });
    }
}

/// Fee estimator with RPC integration and caching
pub struct FeeEstimator {
    cache: Arc<Mutex<FeeCache>>,
    rpc_clients: Arc<crate::external_rpc::RpcClients>,
}

impl FeeEstimator {
    /// Create a new fee estimator with 10-minute cache TTL
    pub fn new(rpc_clients: Arc<crate::external_rpc::RpcClients>) -> Self {
        Self {
            cache: Arc::new(Mutex::new(FeeCache::new(Duration::from_secs(600)))),
            rpc_clients,
        }
    }
    
    /// Get fee estimate for a specific chain and tier
    pub async fn estimate_fee(&self, chain: &str, tier: FeeTier) -> Result<FeeEstimate, String> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(estimate) = cache.get(chain, tier) {
                tracing::debug!("Fee estimate cache hit for {}/{:?}", chain, tier);
                return Ok(estimate);
            }
        }
        
        // Cache miss - fetch from RPC
        tracing::debug!("Fee estimate cache miss for {}/{:?}, fetching from RPC", chain, tier);
        let estimate = self.fetch_from_rpc(chain, tier).await?;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.set(estimate.clone());
        }
        
        Ok(estimate)
    }
    
    /// Get all fee tiers for a chain
    pub async fn estimate_all_tiers(&self, chain: &str) -> Result<Vec<FeeEstimate>, String> {
        let mut estimates = Vec::new();
        
        for tier in [FeeTier::Economy, FeeTier::Normal, FeeTier::Priority] {
            match self.estimate_fee(chain, tier).await {
                Ok(estimate) => estimates.push(estimate),
                Err(e) => {
                    tracing::warn!("Failed to estimate {:?} fee for {}: {}", tier, chain, e);
                    // Use fallback if RPC fails
                    estimates.push(self.fallback_estimate(chain, tier));
                }
            }
        }
        
        Ok(estimates)
    }
    
    /// Fetch fee estimate from blockchain RPC
    async fn fetch_from_rpc(&self, chain: &str, tier: FeeTier) -> Result<FeeEstimate, String> {
        let external_chain = match chain {
            "btc" => ExternalChain::Btc,
            "bch" => ExternalChain::Bch,
            "doge" => ExternalChain::Doge,
            _ => return Err(format!("Unsupported chain: {}", chain)),
        };
        
        let client = self.rpc_clients.get(external_chain)
            .ok_or_else(|| format!("RPC client not configured for {}", chain))?;
        
        let conf_target = tier.target_blocks();
        
        // Call estimatesmartfee RPC
        let response: serde_json::Value = client
            .call("estimatesmartfee", serde_json::json!([conf_target]))
            .await
            .map_err(|e| format!("RPC call failed: {}", e))?;
        
        // Parse response
        // Response format: {"feerate": 0.00001234, "blocks": 6}
        // feerate is in BTC/kB, we need sat/byte
        let feerate_btc_per_kb = response
            .get("feerate")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| "Missing or invalid feerate in response".to_string())?;
        
        // Convert BTC/kB to sat/byte
        // 1 BTC = 100,000,000 satoshis
        // 1 kB = 1000 bytes
        let fee_per_byte = ((feerate_btc_per_kb * 100_000_000.0) / 1000.0).ceil() as u64;
        
        // Ensure minimum fee
        let fee_per_byte = fee_per_byte.max(1);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(FeeEstimate {
            chain: chain.to_string(),
            tier,
            fee_per_byte,
            typical_tx_fee: fee_per_byte * 250, // Typical tx is ~250 bytes
            blocks: conf_target,
            fetched_at: now,
        })
    }
    
    /// Provide fallback fee estimates when RPC is unavailable
    fn fallback_estimate(&self, chain: &str, tier: FeeTier) -> FeeEstimate {
        // Conservative fallback fees (sat/byte)
        let fee_per_byte = match (chain, tier) {
            ("btc", FeeTier::Priority) => 20,
            ("btc", FeeTier::Normal) => 10,
            ("btc", FeeTier::Economy) => 5,
            ("bch", FeeTier::Priority) => 2,
            ("bch", FeeTier::Normal) => 1,
            ("bch", FeeTier::Economy) => 1,
            ("doge", FeeTier::Priority) => 1000,  // DOGE has very low per-coin value
            ("doge", FeeTier::Normal) => 500,
            ("doge", FeeTier::Economy) => 100,
            _ => 10,
        };
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        tracing::warn!("Using fallback fee estimate for {}/{:?}: {} sat/byte", 
                       chain, tier, fee_per_byte);
        
        FeeEstimate {
            chain: chain.to_string(),
            tier,
            fee_per_byte,
            typical_tx_fee: fee_per_byte * 250,
            blocks: tier.target_blocks(),
            fetched_at: now,
        }
    }
    
    /// Clean up expired cache entries
    pub fn cleanup_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear_expired();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_tier_blocks() {
        assert_eq!(FeeTier::Economy.target_blocks(), 6);
        assert_eq!(FeeTier::Normal.target_blocks(), 3);
        assert_eq!(FeeTier::Priority.target_blocks(), 1);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = FeeCache::new(Duration::from_secs(60));
        
        // Test that cache stores and retrieves values
        let estimate = FeeEstimate {
            chain: "btc".to_string(),
            tier: FeeTier::Normal,
            fee_per_byte: 10,
            typical_tx_fee: 2500,
            blocks: 3,
            fetched_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let mut cache = cache;
        cache.set(estimate.clone());
        
        let retrieved = cache.get("btc", FeeTier::Normal);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().fee_per_byte, 10);
    }
}
