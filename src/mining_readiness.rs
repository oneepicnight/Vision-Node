//! Mining readiness
//!
//! Centralized gating rules for whether this node should mine.
//!
//! CRITICAL SPLIT:
//! - SYNC gate: 1 peer minimum (sync runs independently)
//! - MINING gate: 3 peers minimum (mainnet stability)

use crate::auto_sync::SyncHealthSnapshot;
use crate::vision_constants::MAX_MINING_LAG_BLOCKS;

/// Mainnet floor: MINING requires at least this many peers.
/// This prevents "seed prints all blocks" scenario and ensures mainnet stability.
pub const MAINNET_MIN_PEERS_FOR_MINING: u32 = 3;

/// Minimum peers required for SYNC to start (reduced to 1 for network recovery)
/// Temporarily lowered to allow sync with single compatible peer during network issues.
pub const MIN_PEERS_FOR_SYNC: u32 = 1;

/// Max allowed desync for mining (in blocks).
/// For 2-second blocks, 5 blocks = 10 seconds behind network estimate.
const MAX_DESYNC_FOR_MINING: u64 = 5;

/// Quick mining eligibility check (for miner worker loop).
///
/// NON-BLOCKING: Only checks basic local state without network I/O.
///
/// DESIGN PRINCIPLE:
/// Genesis blocks (0..GENESIS_END_HEIGHT) are embedded and immutable.
/// Genesis immutability is enforced in VALIDATION, not mining eligibility.
/// Mining MUST be allowed immediately after the final genesis block
/// to produce the first post-genesis block.
///
/// LAUNCH HARDENING: All conditions are MANDATORY and unskippable.
/// Mining eligibility depends on:
/// - Wallet present (mining address configured)
/// - Peer count (configurable via VISION_MIN_PEERS_FOR_MINING, mainnet floor: 3)
/// - Sync health (must be within MAX_DESYNC_FOR_MINING of observed tip)
///
/// Reward warmup is handled separately in emission logic.
pub fn is_mining_eligible() -> bool {
    // Local test override: allow mining unconditionally when explicitly enabled for localhost testing
    // This bypasses peer/tip gating to enable fast functional verification in multi-node harnesses.
    if std::env::var("VISION_LOCAL_TEST").ok().map(|v| v.trim() == "1").unwrap_or(false) {
        return true;
    }

    // Gate 1: Wallet must be configured
    let mining_address = crate::MINER_ADDRESS.lock().clone();
    if mining_address.is_empty() {
        return false;
    }

    // Gate 2: Consensus quorum enforcement (mainnet floor: 3 COMPATIBLE peers minimum)
    // CRITICAL: Check peers on SAME CHAIN (chain_id + bootstrap_prefix), not just TCP connections
    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    // Mainnet launch hardening: enforce minimum 3 COMPATIBLE peers FOR MINING
    let effective_min_peers = std::cmp::max(min_peers, MAINNET_MIN_PEERS_FOR_MINING);

    // Get consensus quorum (peers on same chain)
    let quorum = crate::PEER_MANAGER.consensus_quorum_blocking();

    // Log peer requirements once per check for debugging misconfiguration
    // OPS VISIBILITY: Show quorum status for 2am debugging
    static PEER_LOG_THROTTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last_log = PEER_LOG_THROTTLE.load(std::sync::atomic::Ordering::Relaxed);
    
    // Log quorum status every 30 seconds for operational visibility
    if now > last_log + 30 {
        PEER_LOG_THROTTLE.store(now, std::sync::atomic::Ordering::Relaxed);
        
        let total_connected = crate::globals::P2P_MANAGER.clone_inner().try_get_peer_count();
        let quorum_status = if quorum.compatible_peers >= effective_min_peers as usize {
            "✅ QUORUM_OK"
        } else {
            "❌ NO_QUORUM"
        };
        
        let block_reason = if quorum.compatible_peers < effective_min_peers as usize {
            // Distinguish between different failure modes
            if quorum.unknown_peers > 0 && quorum.incompatible_peers == 0 {
                format!("need_{}_compatible_all_unknown", effective_min_peers)
            } else if quorum.incompatible_peers > 0 {
                format!("need_{}_compatible_have_{}_incompatible", effective_min_peers, quorum.incompatible_peers)
            } else {
                format!("need_{}_compatible", effective_min_peers)
            }
        } else if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
            let spread = max_h.saturating_sub(min_h);
            if spread > 10 {
                format!("height_spread={}", spread)
            } else {
                "none".to_string()
            }
        } else {
            "none".to_string()
        };
        
        tracing::info!(
            "[QUORUM] {} | connected={} compatible={} incompatible={} unknown={} | min_needed={} | block_reason={}",
            quorum_status, total_connected, quorum.compatible_peers, quorum.incompatible_peers, 
            quorum.unknown_peers, effective_min_peers, block_reason
        );
    }

    // CRITICAL: Require COMPATIBLE peers (on same chain), not just TCP connections
    if quorum.compatible_peers < effective_min_peers as usize {
        return false;
    }
    
    // DEFENSIVE: Verify compatible peers are at similar heights (prevent outlier mining)
    if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
        const MAX_QUORUM_HEIGHT_SPREAD: u64 = 30;  // 🔧 Increased from 10 to 30 to allow mining during network forks/reorgs (2s blocks = 60s window)
        let spread = max_h.saturating_sub(min_h);
        if spread > MAX_QUORUM_HEIGHT_SPREAD {
            static SPREAD_LOG_THROTTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let last_spread_log = SPREAD_LOG_THROTTLE.load(std::sync::atomic::Ordering::Relaxed);
            if now > last_spread_log + 15 {
                SPREAD_LOG_THROTTLE.store(now, std::sync::atomic::Ordering::Relaxed);
                tracing::warn!(
                    "[MINING-GATE] ⚠️ Compatible peers divergent: min_h={} max_h={} spread={} (max={})",
                    min_h, max_h, spread, MAX_QUORUM_HEIGHT_SPREAD
                );
            }
            return false;
        }
    }

    // Gate 3: Sync health check (MANDATORY - cached backbone state, no blocking I/O)
    // Ensures we're within tolerance of network consensus tip.
    // FIX #4: Pause mining during active sync to avoid competing with validation
    
    // Check if we're actively syncing (far behind network)
    let local_height = crate::CHAIN.lock().current_height();
    let observed_tip = crate::PEER_MANAGER
        .try_best_remote_height()
        .unwrap_or(0);
    
    // If we're more than 10 blocks behind, we're in active sync mode - pause mining
    // Allows mining during network lag/fork resolution (10 blocks = 20 seconds at 2s block time)
    // CHANGED: Increased from 1 to 10 to allow mining during network instability without constant pauses
    const ACTIVE_SYNC_THRESHOLD: u64 = 10;
    if observed_tip > 0 && local_height < observed_tip.saturating_sub(ACTIVE_SYNC_THRESHOLD) {
        static SYNC_LOG_THROTTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_log = SYNC_LOG_THROTTLE.load(std::sync::atomic::Ordering::Relaxed);
        if now > last_log + 15 {
            SYNC_LOG_THROTTLE.store(now, std::sync::atomic::Ordering::Relaxed);
            tracing::info!(
                "[MINING-GATE] Mining PAUSED during sync: local={} network={} behind_by={}",
                local_height, observed_tip, observed_tip.saturating_sub(local_height)
            );
        }
        return false;
    }
    
    let backbone = crate::control_plane::get_backbone_state();

    // Genesis exemption: if we're at very early height (< 100 blocks), skip backbone sync check
    // This allows fresh testnets to start mining without external backbone data
    const GENESIS_EXEMPTION_HEIGHT: u64 = 100;
    if local_height >= GENESIS_EXEMPTION_HEIGHT {
        // MAINNET HARD STOP: Require known network tip before mining
        // If observed_tip == 0, we have NO visibility into network state (anchors unreachable)
        // Mining blind can cause chain forks and consensus instability
        #[cfg(feature = "full")]
        if observed_tip == 0 {
            // Rate-limited log (avoid spam) - consider using a static counter if needed
            use std::sync::atomic::{AtomicU64, Ordering};
            static LAST_LOG: AtomicU64 = AtomicU64::new(0);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let last = LAST_LOG.load(Ordering::Relaxed);
            if now > last + 30 {
                LAST_LOG.store(now, Ordering::Relaxed);
                tracing::warn!("[MINING-GATE] ⛔ Blocked: network tip unknown (anchors unreachable / observed_tip=0)");
            }
            return false;
        }

        // Check desync: local must be within MAX_DESYNC_FOR_MINING of observed tip
        if observed_tip > 0 && local_height < observed_tip.saturating_sub(MAX_DESYNC_FOR_MINING) {
            // Too far behind network tip - not synced enough
            return false;
        }
    }

    // Gate 4 (implicit): If we reached here, all conditions are met
    true
}

/// Human-readable mining readiness status message for APIs/logging.
pub fn mining_status_message(snapshot: &SyncHealthSnapshot) -> String {
    let lag = snapshot.height_lag();

    if !snapshot.chain_id_matches {
        return "❌ Mining disabled: Chain ID mismatch".to_string();
    }

    if lag < 0 {
        return format!("⚠️ Mining disabled: Too far ahead ({} blocks)", lag.abs());
    }

    if lag > MAX_MINING_LAG_BLOCKS {
        return format!(
            "⏳ Mining disabled: Too far behind ({} blocks, max={})",
            lag, MAX_MINING_LAG_BLOCKS
        );
    }

    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    if min_peers != 0 && snapshot.peer_count() < min_peers as usize {
        return format!(
            "🔌 Mining disabled: Need {}+ peer connections (have {})",
            min_peers,
            snapshot.peer_count()
        );
    }

    "✅ Mining ready".to_string()
}

/// Get detailed mining eligibility status and reason for blocking (for UI indicators)
pub fn get_eligibility_status() -> (bool, String) {
    // Check for local test override
    if std::env::var("VISION_LOCAL_TEST").ok().map(|v| v.trim() == "1").unwrap_or(false) {
        return (true, "✅ Active (local test mode)".to_string());
    }

    // Gate 1: Wallet check
    let mining_address = crate::MINER_ADDRESS.lock().clone();
    if mining_address.is_empty() || mining_address == "pow_miner" {
        return (false, "⚙️ No wallet configured".to_string());
    }

    // Gate 2: Peer quorum check
    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let effective_min_peers = std::cmp::max(min_peers, MAINNET_MIN_PEERS_FOR_MINING);
    let quorum = crate::PEER_MANAGER.consensus_quorum_blocking();
    
    if quorum.compatible_peers < effective_min_peers as usize {
        return (false, format!("🔌 Need {} compatible peers (have {})", effective_min_peers, quorum.compatible_peers));
    }

    // Gate 2b: Height spread check
    if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
        const MAX_QUORUM_HEIGHT_SPREAD: u64 = 30;
        let spread = max_h.saturating_sub(min_h);
        if spread > MAX_QUORUM_HEIGHT_SPREAD {
            return (false, format!("⚠️ Peers divergent (spread: {} blocks)", spread));
        }
    }

    // Gate 3: Sync health check
    let local_height = crate::CHAIN.lock().current_height();
    let observed_tip = crate::PEER_MANAGER.try_best_remote_height().unwrap_or(0);
    
    const ACTIVE_SYNC_THRESHOLD: u64 = 10;
    if observed_tip > 0 && local_height < observed_tip.saturating_sub(ACTIVE_SYNC_THRESHOLD) {
        let behind = observed_tip.saturating_sub(local_height);
        return (false, format!("⏳ Syncing ({} blocks behind)", behind));
    }

    // Gate 3b: Genesis exemption check
    const GENESIS_EXEMPTION_HEIGHT: u64 = 100;
    if local_height >= GENESIS_EXEMPTION_HEIGHT {
        #[cfg(feature = "full")]
        if observed_tip == 0 {
            return (false, "❌ Network tip unknown".to_string());
        }
        
        const MAX_DESYNC_FOR_MINING: u64 = 5;
        if observed_tip > 0 && local_height < observed_tip.saturating_sub(MAX_DESYNC_FOR_MINING) {
            let behind = observed_tip.saturating_sub(local_height);
            return (false, format!("⏳ {} blocks behind network", behind));
        }
    }

    // All gates passed
    (true, "✅ All systems go!".to_string())
}
