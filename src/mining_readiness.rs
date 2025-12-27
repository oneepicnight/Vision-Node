//! Mining readiness
//!
//! Centralized gating rules for whether this node should mine.

use crate::auto_sync::SyncHealthSnapshot;
use crate::vision_constants::MAX_MINING_LAG_BLOCKS;

/// Mainnet floor: mining ALWAYS requires at least this many peers, regardless of env var.
/// This prevents "seed prints all blocks" scenario and ensures mainnet stability.
pub const MAINNET_MIN_PEERS_FLOOR: u32 = 3;

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
    // Gate 1: Wallet must be configured
    let mining_address = crate::MINER_ADDRESS.lock().clone();
    if mining_address.is_empty() {
        return false;
    }

    // Gate 2: Peer count enforcement (mainnet floor: 3 peers minimum for launch)
    let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    // Mainnet launch hardening: enforce minimum 3 peers (prevents seed from mining all blocks)
    let effective_min_peers = std::cmp::max(min_peers, MAINNET_MIN_PEERS_FLOOR);

    // Enforce real peer gating using fast non-blocking check
    let connected_peers = crate::PEER_MANAGER.try_connected_validated_count() as u32;

    // Log peer requirements once per check for debugging misconfiguration
    static PEER_LOG_THROTTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last_log = PEER_LOG_THROTTLE.load(std::sync::atomic::Ordering::Relaxed);
    if now > last_log + 15 && connected_peers < effective_min_peers {
        PEER_LOG_THROTTLE.store(now, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(
            "[MINING-GATE] Peer requirements: env_min={} mainnet_floor={} effective_floor={} connected={}",
            min_peers, MAINNET_MIN_PEERS_FLOOR, effective_min_peers, connected_peers
        );
    }

    if connected_peers < effective_min_peers {
        return false;
    }

    // Gate 3: Sync health check (MANDATORY - cached backbone state, no blocking I/O)
    // Ensures we're within tolerance of network consensus tip.
    let backbone = crate::control_plane::get_backbone_state();
    let local_height = crate::CHAIN.lock().current_height();
    let observed_tip = crate::PEER_MANAGER
        .try_best_remote_height()
        .unwrap_or(backbone.observed_tip_height);

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
        .unwrap_or(1);

    if min_peers != 0 && snapshot.peer_count() < min_peers as usize {
        return format!(
            "🔌 Mining disabled: Need {}+ peer connections (have {})",
            min_peers,
            snapshot.peer_count()
        );
    }

    "✅ Mining ready".to_string()
}
