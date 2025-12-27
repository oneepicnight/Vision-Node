//! Lottery Eligibility Module
//!
//! Centralized logic for determining if a node can participate in block reward lottery.
//!
//! **Philosophy:**
//! - Mining = lottery ticket
//! - Must be close to tip + see same chain
//! - Anchors = backbone/truth keepers (strict requirements)
//! - Regular miners = outbound-only is fine (relaxed requirements)
//!
//! **No more "must be publicly reachable" for regular miners.**

use crate::auto_sync::SyncHealthSnapshot;
use crate::vision_constants::MAX_MINING_LAG_BLOCKS;
use crate::role::{current_node_role, NodeRole};

/// Decide if this node is eligible to participate in block rewards (lottery)
///
/// # Strict Rules (Lottery Purge System):
/// - Node must be within 2 blocks of network tip (MAX_MINING_LAG_BLOCKS = 2)
/// - Auto-purged from lottery if falling behind (protects against bad internet)
/// - Node must not be ahead of network (prevents isolated mining)
/// - Must have at least 1 synced peer with matching chain_id
/// - Anchor nodes: Must be publicly reachable + have 3+ peers (backbone)
/// - Regular miners: Outbound-only is fine, just need 1+ peer (leaf nodes)
///
/// **P2P Health Integration:**
/// Uses SyncHealthSnapshot from auto_sync to verify:
/// - Current sync lag (height_lag)
/// - Peer connectivity status
/// - Chain ID match verification
///
/// # Arguments
/// * `snapshot` - Current sync health snapshot from P2P health system
///
/// # Returns
/// * `true` - Node can participate in lottery (synced within 2 blocks)
/// * `false` - Node purged from lottery pool (too far behind or ahead)
pub fn is_reward_eligible(snapshot: &SyncHealthSnapshot) -> bool {
    let lag = snapshot.height_lag();
    
    // Check if genesis mode (MIN_PEERS_FOR_MINING=0)
    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let genesis_mode = min_peers == 0;
    
    let has_peers = if genesis_mode {
        true // Genesis node is always eligible even with 0 peers
    } else {
        snapshot.peer_count() >= 1
    };

    // Node must not be ahead of network, and not too far behind
    // lag >= 0 means we're behind or at tip (good)
    // lag <= MAX_MINING_LAG_BLOCKS (2) means we're within lottery window
    let in_sync_window = lag >= 0 && lag <= MAX_MINING_LAG_BLOCKS;
    
    if !in_sync_window {
        tracing::debug!(
            target: "lottery",
            "🚫 PURGED from lottery: lag={} blocks (max={}), ahead={}",
            lag.abs(), MAX_MINING_LAG_BLOCKS, lag < 0
        );
    }

    // PROOF-OF-SYNC: Single truth - no role-based complexity
    let chain_matches = snapshot.chain_id_matches;
    let eligible = in_sync_window && has_peers && chain_matches;
    
    if !eligible {
        tracing::debug!(
            target: "lottery",
            "🔍 Lottery eligibility: in_sync={}, peers={}, lag={} blocks, chain_match={}",
            in_sync_window, snapshot.peer_count(), lag, chain_matches
        );
    }
    
    eligible
}

/// Quick mining eligibility check (for miner worker loop)
/// NON-BLOCKING: Only checks basic local state without network I/O
/// Returns true if conditions allow mining to proceed
pub fn is_mining_eligible() -> bool {
    // FAST PATH: No blocking snapshot calls - just check basics
    
    // Check if mining address is set
    let mining_address = crate::MINER_ADDRESS.lock().clone();
    if mining_address.is_empty() {
        return false;
    }
    
    // Check MIN_PEERS requirement (fast, no network I/O)
    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    
    // Genesis mode (MIN_PEERS=0) always eligible if address set
    if min_peers == 0 {
        return true;
    }
    
    // Quick local chain check (no network I/O)
    let local_height = {
        let chain = crate::CHAIN.lock();
        chain.blocks.len() as u64
    };
    
    // If chain is very short, probably not synced yet
    if local_height < 10 {
        return false;
    }
    
    // Default to eligible if basic checks pass
    // Detailed eligibility enforced by lottery system
    true
}

/// Get detailed lottery status message for API/logging
pub fn lottery_status_message(snapshot: &SyncHealthSnapshot) -> String {
    let lag = snapshot.height_lag();
    
    // Check chain ID match
    if !snapshot.chain_id_matches {
        return "❌ Lottery disabled: Chain ID mismatch".to_string();
    }
    
    // Check if we're ahead of network (isolated mining)
    if lag < 0 {
        return format!("⚠️ Lottery disabled: Too far ahead ({} blocks)", lag.abs());
    }
    
    // Check if we're too far behind
    if lag > MAX_MINING_LAG_BLOCKS {
        return format!("⏳ Lottery disabled: Too far behind ({} blocks, max={})", lag, MAX_MINING_LAG_BLOCKS);
    }
    
    // Check peer count based on auto-detected role
    let role = current_node_role();
    match role {
        NodeRole::Anchor => {
            if snapshot.peer_count() < 3 {
                return format!("🛰️ Anchor lottery disabled: Need 3+ peers (have {})", snapshot.peer_count());
            }
            
            if !snapshot.public_reachable.unwrap_or(false) {
                return "🛰️ Anchor lottery disabled: Not publicly reachable".to_string();
            }
            
            "✅ Anchor lottery ready: Full eligibility".to_string()
        }
        NodeRole::Edge => {
            if snapshot.peer_count() < 1 {
                return "🎫 Lottery disabled: No peer connections".to_string();
            }
            
            "✅ Lottery ready: Full eligibility".to_string()
        }
    }
}

/// Get lottery eligibility statistics for debugging/monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct LotteryEligibilityInfo {
    pub eligible: bool,
    pub role: String,
    pub lag_blocks: i64,
    pub peer_count: usize,
    pub public_reachable: Option<bool>,
    pub reason: String,
}

impl LotteryEligibilityInfo {
    pub fn from_snapshot(snapshot: &SyncHealthSnapshot) -> Self {
        let eligible = is_reward_eligible(snapshot);
        let node_role = current_node_role();
        let role = match node_role {
            NodeRole::Anchor => "anchor",
            NodeRole::Edge => "edge-miner",
        };
        
        Self {
            eligible,
            role: role.to_string(),
            lag_blocks: snapshot.height_lag(),
            peer_count: snapshot.peer_count(),
            public_reachable: snapshot.public_reachable,
            reason: lottery_status_message(snapshot),
        }
    }
}
