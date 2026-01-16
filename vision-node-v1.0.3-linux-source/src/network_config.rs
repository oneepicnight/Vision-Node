// NOTE: Deprecated.
//
// This module implemented env-selected network/testnet behavior (e.g. VISION_NETWORK).
// Under the ONE-chain model, it must not be referenced by the compiled crate.
// It is intentionally kept as a historical reference only.
/// Network configuration and genesis validation
use serde::{Deserialize, Serialize};

/// Network type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkType {
    Testnet,
    Mainnet,
    Testnet48hV080, // "vision-testnet-48h-v0.8.0" - 48-hour lifecycle test
}

impl NetworkType {
    pub fn from_env() -> Self {
        match std::env::var("VISION_NETWORK").ok().as_deref() {
            Some("mainnet") => Self::Mainnet,
            Some("vision-testnet-48h-v0.8.0") => Self::Testnet48hV080,
            _ => Self::Testnet, // Default to testnet for safety
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
            Self::Testnet48hV080 => "vision-testnet-48h-v0.8.0",
        }
    }

    pub fn genesis_hash(&self) -> &'static str {
        match self {
            Self::Testnet => GENESIS_HASH_TESTNET,
            Self::Mainnet => GENESIS_HASH_MAINNET,
            Self::Testnet48hV080 => GENESIS_HASH_TESTNET_48H,
        }
    }

    pub fn sunset_height(&self) -> Option<u64> {
        match self {
            Self::Testnet => Some(TESTNET_SUNSET_HEIGHT),
            Self::Mainnet => None, // Mainnet never sunsets
            Self::Testnet48hV080 => None, // Handled by testnet_48h_bounds
        }
    }

    pub fn is_sunset(&self, height: u64) -> bool {
        match self.sunset_height() {
            Some(sunset) => height >= sunset,
            None => false,
        }
    }
}

// Genesis hashes for network separation
pub const GENESIS_HASH_TESTNET: &str = "0000000000000000000000000000000000000000000000000000000000000000"; // To be filled with actual genesis block hash
pub const GENESIS_HASH_MAINNET: &str = "0000000000000000000000000000000000000000000000000000000000000000"; // To be filled with actual genesis block hash
pub const GENESIS_HASH_TESTNET_48H: &str = "0000000000000000000000000000000000000000000000000000000000000000"; // Testnet 48h v0.8.0

// Testnet 48h v0.8.0 constants
// Target: 48 hours of blocks at 2 seconds per block = 86,400 blocks
pub const TESTNET_48H_TOTAL_BLOCKS: u64 = 86_400;
pub const TESTNET_48H_MIDPOINT_PERCENT: f64 = 0.5;

/// Check if network is the 48h testnet v0.8.0
pub fn is_testnet_48h_v080(network: NetworkType) -> bool {
    matches!(network, NetworkType::Testnet48hV080)
}

/// Get (start_height, mid_height, end_height) for 48h testnet
/// 
/// - start: genesis height (typically 0)
/// - mid: halfway point (50% - transition from mining to hybrid)
/// - end: sunset height (100% - mining ends, only guardian stakes)
pub fn testnet_48h_bounds(genesis_height: u64) -> (u64, u64, u64) {
    let total = TESTNET_48H_TOTAL_BLOCKS;
    let start = genesis_height;
    let end = start + total;
    let mid = start + (total as f64 * TESTNET_48H_MIDPOINT_PERCENT) as u64;
    (start, mid, end)
}

/// Get the current phase for 48h testnet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Testnet48hPhase {
    Mining,       // height < mid: Full mining rewards
    Hybrid,       // mid <= height < end: 50/50 mining + staking
    StakingOnly,  // height >= end: Staking only (guardian)
    Complete,     // height >= end (non-guardian - shutdown)
}

impl Testnet48hPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mining => "mining",
            Self::Hybrid => "hybrid",
            Self::StakingOnly => "staking-only",
            Self::Complete => "complete",
        }
    }
}

/// Determine the phase for 48h testnet based on height and guardian status
pub fn testnet_48h_phase(height: u64, genesis_height: u64, is_guardian: bool) -> Testnet48hPhase {
    let (start, mid, end) = testnet_48h_bounds(genesis_height);
    
    if height < mid {
        Testnet48hPhase::Mining
    } else if height < end {
        Testnet48hPhase::Hybrid
    } else if is_guardian {
        Testnet48hPhase::StakingOnly
    } else {
        Testnet48hPhase::Complete
    }
}

// Testnet sunset configuration
pub const TESTNET_SUNSET_HEIGHT: u64 = 1_000_000;
pub const META_TESTNET_SUNSET_DONE: &str = "meta:testnet_sunset_done";

// CASH genesis drop configuration
pub const CASH_GENESIS_HEIGHT: u64 = 1_000_000;

// Anti-fork protection constants
pub const MAX_TIME_DRIFT_SECS: i64 = 10;
pub const MAX_REORG_DEPTH: u64 = 64;

// Mining stability constants
pub const TARGET_BLOCK_TIME_SECS: u64 = 2;
pub const DIFFICULTY_ADJUSTMENT_WINDOW: u64 = 100;

/// Check if peer's genesis hash matches ours
pub fn validate_peer_genesis(peer_genesis: &str, our_network: NetworkType) -> Result<(), String> {
    let expected = our_network.genesis_hash();
    if peer_genesis != expected {
        return Err(format!(
            "Genesis hash mismatch: peer={}, expected={} (network={})",
            peer_genesis,
            expected,
            our_network.as_str()
        ));
    }
    Ok(())
}

/// Check if network sunset has occurred
pub fn check_sunset(height: u64, network: NetworkType) -> Result<(), String> {
    if network.is_sunset(height) {
        return Err(format!(
            "Network {} has reached sunset at height {}. Please migrate to mainnet.",
            network.as_str(),
            height
        ));
    }
    Ok(())
}

/// Export wallet keys for migration
/// 
/// Exports all keys and balances from the database to a JSON file.
/// The exported file can be used to migrate testnet wallets to mainnet.
/// 
/// Returns the JSON string containing:
/// - network: "testnet"
/// - export_height: Block height at export
/// - timestamp: RFC3339 timestamp
/// - keys: HashMap of address -> {key, balance}
pub fn export_migration_keys(db: &sled::Db) -> anyhow::Result<String> {
    use std::collections::HashMap;
    
    let keys_tree = db.open_tree("keys")
        .map_err(|e| anyhow::anyhow!("Failed to open keys tree: {}", e))?;
    let balances_tree = db.open_tree("balances")
        .map_err(|e| anyhow::anyhow!("Failed to open balances tree: {}", e))?;
    
    let mut export = HashMap::new();
    let mut key_count = 0;
    
    for item in keys_tree.iter() {
        let (key, value) = item
            .map_err(|e| anyhow::anyhow!("Failed to read key from database: {}", e))?;
        
        let address = String::from_utf8_lossy(&key).to_string();
        let balance = balances_tree
            .get(&key)
            .map_err(|e| anyhow::anyhow!("Failed to read balance for {}: {}", address, e))?
            .map(|v| {
                if v.len() == 16 {
                    let mut bytes = [0u8; 16];
                    bytes.copy_from_slice(&v);
                    u128::from_le_bytes(bytes) // Fixed: should be LE not BE
                } else {
                    0
                }
            })
            .unwrap_or(0);
        
        export.insert(address.clone(), serde_json::json!({
            "key": hex::encode(&value),
            "balance": balance.to_string(),
        }));
        
        key_count += 1;
    }
    
    tracing::info!("Exported {} wallet keys for migration", key_count);
    
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "network": "testnet",
        "export_height": 1_000_000u64,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "key_count": key_count,
        "keys": export,
    }))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_type() {
        let testnet = NetworkType::Testnet;
        let mainnet = NetworkType::Mainnet;
        
        assert_eq!(testnet.as_str(), "testnet");
        assert_eq!(mainnet.as_str(), "mainnet");
        
        assert_eq!(testnet.genesis_hash(), GENESIS_HASH_TESTNET);
        assert_eq!(mainnet.genesis_hash(), GENESIS_HASH_MAINNET);
    }

    #[test]
    fn test_testnet_sunset() {
        let testnet = NetworkType::Testnet;
        assert!(!testnet.is_sunset(999_999));
        assert!(testnet.is_sunset(1_000_000));
        assert!(testnet.is_sunset(1_000_001));
    }

    #[test]
    fn test_mainnet_no_sunset() {
        let mainnet = NetworkType::Mainnet;
        assert!(!mainnet.is_sunset(999_999));
        assert!(!mainnet.is_sunset(1_000_000));
        assert!(!mainnet.is_sunset(10_000_000));
    }
}
