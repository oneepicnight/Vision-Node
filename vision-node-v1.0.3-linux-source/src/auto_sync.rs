//! Auto-Sync Module
//!
//! Background chain synchronization that runs INDEPENDENT of mining eligibility.
//! This ensures nodes stay synchronized with the network even when not mining.
//!
//! Core Rule: Auto-sync NEVER checks mining eligibility, reward gates, or quorum.
//! It only cares about: "Is there a taller compatible chain?" → "Sync to it."
#![allow(dead_code)]

use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use crate::vision_constants::{AUTO_SYNC_INTERVAL_SECS, AUTO_SYNC_MAX_LAG_BLOCKS};

// 🚀 OPTION A - Temporary Relief: Increased fork detection timeouts
// Network latency can cause false fork detection timeouts, especially under load.
// These values provide breathing room for legitimate peer responses while keeping
// the system responsive. Future: implement proper fork proof handshake (Option B).
const SYNC_FORK_TIMEOUT_SECS: u64 = 20;    // Initial tip comparison (increased for large gaps)
const SYNC_FORK_SEARCH_TIMEOUT_SECS: u64 = 15; // Binary search iterations (increased for network load)

/// Sync health snapshot for mining eligibility checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncHealthSnapshot {
    pub sync_height: u64,
    pub network_estimated_height: u64,
    pub network_tip_hash: String, // Canonical tip hash from anchors
    pub connected_peers: usize,
    pub is_syncing: bool,
    pub chain_id_matches: bool,
    pub is_too_far_ahead: bool,
    pub behind_blocks: i64,       // How far behind network (negative = ahead)
    pub slow_peers: usize,        // Count of peers lagging >= SLOW_PEER_LAG_BLOCKS
    pub avg_peer_lag_blocks: f32, // Average lag across all peers
    pub public_reachable: Option<bool>, // Whether this node is publicly reachable (for anchor eligibility)
    pub anchor_sampled: usize,          // Number of anchors successfully queried
    // Consensus quorum fields (for exchange gate)
    pub compatible_peers: usize,
    pub incompatible_peers: usize,
}

impl SyncHealthSnapshot {
    /// Alias for connected_peers (used by mining readiness gating)
    pub fn peer_count(&self) -> usize {
        self.connected_peers
    }
}

impl SyncHealthSnapshot {
    /// Get current sync health snapshot
    /// DEPRECATED: Use current_async() from async contexts or read from cache
    /// This sync version may block on network I/O - avoid in hot paths
    #[deprecated(note = "Use current_async() or cached snapshot to avoid blocking")]
    pub fn current() -> Self {
        // Check if genesis mode (MIN_PEERS=0) - use fast path
        let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let genesis_mode = min_peers == 0;

        tracing::debug!(
            "[SYNC_HEALTH] MIN_PEERS={}, genesis_mode={}",
            min_peers,
            genesis_mode
        );

        // GENESIS MODE: Skip chain lock to avoid deadlock!
        // (Block producer holds CHAIN lock when calling this)
        if genesis_mode {
            tracing::debug!("[SYNC_HEALTH] Genesis mode - using defaults (no chain lock)");
            return Self {
                sync_height: 10, // Dummy value - we're always synced in genesis mode
                network_estimated_height: 10,
                network_tip_hash: String::new(),
                connected_peers: 0,
                is_syncing: false,
                chain_id_matches: true,
                is_too_far_ahead: false,
                behind_blocks: 0,
                slow_peers: 0,
                avg_peer_lag_blocks: 0.0,
                public_reachable: Some(true),
                anchor_sampled: 0,
                compatible_peers: 0,
                incompatible_peers: 0,
            };
        }

        // Get local height and chain identity (non-genesis mode)
        let (sync_height, _our_chain_id, _our_genesis_hash) = {
            let chain = crate::CHAIN.lock();
            let chain_id = crate::vision_constants::expected_chain_id();
            let genesis_hash = if !chain.blocks.is_empty() {
                chain.blocks[0].header.pow_hash.clone()
            } else {
                String::new()
            };
            (chain.blocks.len() as u64, chain_id, genesis_hash)
        };

        // 🛰️ PRIMARY SOURCE OF TRUTH: Query anchors over HTTP (port 7070)
        let _anchor_seeds = crate::p2p::seed_peers::SeedPeerConfig::parse_anchor_seeds_from_env();

        // DEPRECATED PATH: Return minimal data to avoid blocking
        // Real implementations should use current_async()
        let (_anchor_height, network_tip_hash, anchor_sampled) = (0, String::new(), 0);
        tracing::warn!("[SYNC_HEALTH] sync current() called - returning minimal data, use current_async() instead");

        // 🌐 PRIMARY: Use control plane backbone state (HTTP 7070)
        let backbone = crate::control_plane::get_backbone_state();

        // DEPRECATED PATH: Return minimal data to avoid blocking
        // Use backbone if available, otherwise use local height
        let network_estimated_height = if backbone.connected && backbone.observed_tip_height > 0 {
            backbone.observed_tip_height
        } else {
            sync_height
        };
        let connected_peers = 0;
        let chain_id_matches = true;
        let slow_peers = 0;
        let avg_peer_lag_blocks = 0.0;

        tracing::warn!("[SYNC_HEALTH] Deprecated sync current() called - returning backbone-only data, use current_async() instead");

        // Check if actively syncing (behind by more than 2 blocks)
        let is_syncing = network_estimated_height > sync_height + 2;

        // Check if too far ahead of network consensus
        // This prevents mining in isolation when local chain has diverged
        let is_too_far_ahead = if connected_peers >= 2 {
            sync_height
                > network_estimated_height + crate::vision_constants::MAX_BLOCKS_AHEAD_OF_CONSENSUS
        } else {
            false // Don't check if we don't have enough peers for consensus
        };

        // Calculate how far behind (or ahead) we are
        let behind_blocks = network_estimated_height as i64 - sync_height as i64;

        // Check if we're publicly reachable (based on ADVERTISED_P2P_ADDRESS being set)
        let public_reachable = {
            let addr_guard = crate::ADVERTISED_P2P_ADDRESS.lock();
            if addr_guard.is_some() {
                Some(true)
            } else {
                Some(false)
            }
        };

        let snapshot = Self {
            sync_height,
            network_estimated_height,
            network_tip_hash,
            connected_peers,
            is_syncing,
            chain_id_matches,
            is_too_far_ahead,
            behind_blocks,
            slow_peers,
            avg_peer_lag_blocks,
            public_reachable,
            anchor_sampled,
            compatible_peers: 0,  // Not available in sync context
            incompatible_peers: 0,
        };

        // 🔄 Update node role based on current health
        crate::role::update_node_role(&snapshot);

        snapshot
    }

    /// Calculate how far behind (or ahead) the network tip we are
    /// Positive = behind, negative = ahead, zero = at tip
    pub fn height_lag(&self) -> i64 {
        self.network_estimated_height as i64 - self.sync_height as i64
    }

    /// Check if exchange is ready for trading
    /// Requires: sync_status="ready", compatible_peers>=2, not too far ahead (desync<=1)
    pub fn exchange_ready(&self) -> bool {
        // Must not be syncing
        if self.is_syncing {
            return false;
        }

        // CRITICAL: Must have at least 2 COMPATIBLE peers (same chain) for consensus
        // This prevents exchange operations when connected to wrong fork/network
        if self.compatible_peers < 2 {
            return false;
        }

        // Chain ID must match
        if !self.chain_id_matches {
            return false;
        }

        // Must not be too far ahead (diverged)
        if self.is_too_far_ahead {
            return false;
        }

        // Allow small desync (<=1 block)
        let desync = self.sync_height.abs_diff(self.network_estimated_height);

        desync <= 1
    }

    /// Check if node can mine with balanced readiness rules.
    /// Once synced and healthy → eligible to mine.
    /// While behind or unhealthy → no mining.
    pub fn can_mine_fair(&self, min_peers: u16) -> bool {
        use crate::vision_constants::MAX_DESYNC_FOR_MINING;

        // Must have Chain ID match
        if !self.chain_id_matches {
            return false;
        }

        // Must have minimum peers for consensus
        if self.connected_peers < min_peers as usize {
            return false;
        }

        // Must not be actively syncing
        if self.is_syncing {
            return false;
        }

        // Must not be too far ahead (diverged)
        if self.is_too_far_ahead {
            return false;
        }

        // Must be within allowed desync for mining (e.g., ≤2 blocks)
        if self.behind_blocks.abs() > MAX_DESYNC_FOR_MINING as i64 {
            return false;
        }

        // All checks passed.
        true
    }

    /// Get mining eligibility status message
    pub fn mining_status_message(&self, min_peers: u16) -> String {
        use crate::vision_constants::MAX_DESYNC_FOR_MINING;

        if !self.chain_id_matches {
            return "⛔ Mining disabled: Chain ID mismatch".to_string();
        }

        if self.connected_peers < min_peers as usize {
            return format!(
                "⛔ Mining disabled: Need {} peers (have {})",
                min_peers, self.connected_peers
            );
        }

        if self.is_syncing {
            return format!(
                "⏳ Mining disabled: Syncing ({}  blocks behind)",
                self.behind_blocks
            );
        }

        if self.is_too_far_ahead {
            return format!(
                "⚠️  Mining disabled: Too far ahead ({} blocks ahead of network)",
                -self.behind_blocks
            );
        }

        if self.behind_blocks.abs() > MAX_DESYNC_FOR_MINING as i64 {
            return format!(
                "⛔ Mining disabled: Desync too large ({} blocks)",
                self.behind_blocks.abs()
            );
        }

        "✅ Mining ready".to_string()
    }

    /// ASYNC VERSION: Get current sync health snapshot without blocking
    /// This is the preferred method - call from async contexts only
    /// NO BLOCKING CALLS - pure async implementation
    pub async fn current_async() -> Self {
        // Check if genesis mode (MIN_PEERS=0) - use fast path
        let min_peers: u32 = std::env::var("VISION_MIN_PEERS_FOR_MINING")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
        let genesis_mode = min_peers == 0;

        if genesis_mode {
            tracing::debug!("[SYNC_HEALTH] Genesis mode - using defaults");
            return Self {
                sync_height: 10,
                network_estimated_height: 10,
                network_tip_hash: String::new(),
                connected_peers: 0,
                is_syncing: false,
                chain_id_matches: true,
                is_too_far_ahead: false,
                behind_blocks: 0,
                slow_peers: 0,
                avg_peer_lag_blocks: 0.0,
                public_reachable: Some(true),
                anchor_sampled: 0,
                compatible_peers: 0,
                incompatible_peers: 0,
            };
        }

        // Get local height and chain identity (non-genesis mode)
        let (sync_height, our_chain_id, our_genesis_hash) = {
            let chain = crate::CHAIN.lock();
            let chain_id = crate::vision_constants::expected_chain_id();
            let genesis_hash = if !chain.blocks.is_empty() {
                chain.blocks[0].header.pow_hash.clone()
            } else {
                String::new()
            };
            (chain.blocks.len() as u64, chain_id, genesis_hash)
        };

        // 🛰️ Query anchors over HTTP (port 7070) - ASYNC, NO BLOCKING
        let anchor_seeds = crate::p2p::seed_peers::SeedPeerConfig::parse_anchor_seeds_from_env();

        let (anchor_height, network_tip_hash, anchor_sampled) = if !anchor_seeds.is_empty() {
            crate::anchor_client::query_anchor_consensus(
                &anchor_seeds,
                &our_chain_id,
                &our_genesis_hash,
            )
            .await
        } else {
            (0, String::new(), 0)
        };

        // 🌐 Get control plane backbone state
        let backbone = crate::control_plane::get_backbone_state();

        // 📡 Peer connectivity truth: validated connected peers
        let live_peer_count = crate::PEER_MANAGER.connected_validated_count().await;

        // Additional peer metadata (heights, compatibility) still comes from PEER_MANAGER
        let quorum = crate::PEER_MANAGER.consensus_quorum().await;
        let peers = crate::PEER_MANAGER.connected_peers().await;

        // Get best peer height
        let best_peer_height_opt = peers.iter().filter_map(|p| p.height).max();

        // PRIORITY ORDER for network tip height:
        // 1. P2P peer gossip (pure swarm first)
        // 2. Control plane backbone (HTTP 7070) - fallback
        // 3. Anchor HTTP fallback - last resort
        let network_height = if let Some(h) = best_peer_height_opt {
            tracing::debug!("[SYNC_HEALTH] Using P2P peer gossip tip height: {}", h);
            h.max(sync_height)
        } else if backbone.connected && backbone.observed_tip_height > 0 {
            tracing::debug!(
                "[SYNC_HEALTH] ✅ Using backbone tip height: {}",
                backbone.observed_tip_height
            );
            backbone.observed_tip_height
        } else if anchor_height > 0 {
            tracing::debug!(
                "[SYNC_HEALTH] Using anchor HTTP tip height: {}",
                anchor_height
            );
            anchor_height
        } else {
            tracing::debug!(
                "[SYNC_HEALTH] No external tip available; using local height: {}",
                sync_height
            );
            sync_height
        };

        // Calculate slow peer metrics
        let mut slow_count = 0;
        let mut lag_sum: u64 = 0;
        let mut lag_samples: u64 = 0;

        for peer in &peers {
            if let Some(h) = peer.last_reported_height {
                let lag = network_height.saturating_sub(h);
                lag_sum += lag;
                lag_samples += 1;

                if lag >= crate::vision_constants::SLOW_PEER_LAG_BLOCKS {
                    slow_count += 1;
                }
            }
        }

        let avg_lag = if lag_samples > 0 {
            (lag_sum as f32) / (lag_samples as f32)
        } else {
            0.0
        };

        let chain_matches = quorum.incompatible_peers == 0 || quorum.compatible_peers > 0;

        // Check if actively syncing (behind by more than 2 blocks)
        let is_syncing = network_height > sync_height + 2;

        // Check if too far ahead
        let is_too_far_ahead = if live_peer_count >= 2 {
            sync_height > network_height + crate::vision_constants::MAX_BLOCKS_AHEAD_OF_CONSENSUS
        } else {
            false
        };

        let behind_blocks = network_height as i64 - sync_height as i64;

        // Check if publicly reachable
        let public_reachable = {
            let addr_guard = crate::ADVERTISED_P2P_ADDRESS.lock();
            addr_guard.as_ref().map(|_| true)
        };

        let snapshot = Self {
            sync_height,
            network_estimated_height: network_height,
            network_tip_hash,
            connected_peers: live_peer_count,
            is_syncing,
            chain_id_matches: chain_matches,
            is_too_far_ahead,
            behind_blocks,
            slow_peers: slow_count,
            avg_peer_lag_blocks: avg_lag,
            public_reachable,
            anchor_sampled,
            compatible_peers: quorum.compatible_peers,
            incompatible_peers: quorum.incompatible_peers,
        };

        // Update node role based on current health
        crate::role::update_node_role(&snapshot);

        snapshot
    }
}

static HTTP: Lazy<Client> = Lazy::new(Client::new);

fn local_port() -> u16 {
    std::env::var("VISION_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7070)
}

/// Configuration for auto-sync behavior
#[derive(Debug, Clone)]
pub struct AutoSyncConfig {
    /// How often to check if sync is needed (seconds)
    pub poll_interval_secs: u64,
    /// How far behind we allow ourselves before triggering sync
    pub max_lag_before_sync: u64,
}

impl Default for AutoSyncConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: AUTO_SYNC_INTERVAL_SECS,
            max_lag_before_sync: AUTO_SYNC_MAX_LAG_BLOCKS,
        }
    }
}

/// Spawn background auto-sync task that runs forever
///
/// This task NEVER checks:
/// - Mining eligibility
/// - Reward gates
/// - Height quorum
///
/// It only checks: "Is there a taller compatible chain?" → "Sync to it."
pub fn spawn_auto_sync_task(config: AutoSyncConfig) {
    tracing::warn!(
        "[AUTO-SYNC] 🚀 SPAWNING AUTO-SYNC TASK! poll_interval={}s max_lag={}",
        config.poll_interval_secs,
        config.max_lag_before_sync
    );
    
    tokio::spawn(async move {
        tracing::warn!("[AUTO-SYNC] 🔄 AUTO-SYNC LOOP STARTED!");
        
        loop {
            if let Err(e) = auto_sync_step(&config).await {
                tracing::warn!("AUTO-SYNC error: {:?}", e);
            }

            sleep(Duration::from_secs(config.poll_interval_secs)).await;
        }
    });
}

/// Single auto-sync check: compare local vs remote height, sync if needed
/// 
/// SYNC GATE: Requires 3 COMPATIBLE peers (same chain_id + bootstrap_prefix)
/// CRITICAL: Validates consensus quorum, not just TCP connections
async fn auto_sync_step(config: &AutoSyncConfig) -> anyhow::Result<()> {
    // 0) Check consensus quorum (peers on same chain)
    let quorum = crate::PEER_MANAGER.consensus_quorum().await;
    
    tracing::warn!(
        "[AUTO-SYNC-TICK] ⏰ Sync check: compatible={} incompatible={} need={}",
        quorum.compatible_peers,
        quorum.incompatible_peers,
        crate::mining_readiness::MIN_PEERS_FOR_SYNC
    );
    
    // CRITICAL: Check COMPATIBLE peers (on same chain), not raw connections
    if quorum.compatible_peers < crate::mining_readiness::MIN_PEERS_FOR_SYNC as usize {
        tracing::debug!(
            "[SYNC-GATE] ⛔ Waiting for consensus quorum: compatible={} incompatible={} need={}",
            quorum.compatible_peers,
            quorum.incompatible_peers,
            crate::mining_readiness::MIN_PEERS_FOR_SYNC
        );
        return Ok(());
    }
    
    // DEFENSIVE: Check height spread among compatible peers (prevent wild divergence)
    if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
        const MAX_HEIGHT_SPREAD: u64 = 100;
        let spread = max_h.saturating_sub(min_h);
        if spread > MAX_HEIGHT_SPREAD {
            tracing::warn!(
                "[SYNC-GATE] ⚠️ Compatible peers have divergent heights: min={} max={} spread={} (max={})",
                min_h, max_h, spread, MAX_HEIGHT_SPREAD
            );
            // Still allow sync but log warning - they may converge
        }
    }
    
    // 1) Get best remote height from peer manager
    tracing::warn!("[AUTO-SYNC-TICK] 🔍 Querying PEER_MANAGER.best_remote_height()...");
    let best_remote = crate::PEER_MANAGER.best_remote_height().await;

    tracing::warn!(
        "[AUTO-SYNC-TICK] 📊 PEER_MANAGER returned: {:?}",
        best_remote
    );

    let best_remote = match best_remote {
        Some(h) => h,
        None => {
            // No peers with known height yet - THIS IS THE PROBLEM!
            tracing::warn!(
                "[AUTO-SYNC] ⚠️ NO PEER HEIGHTS KNOWN! compatible_peers={} but best_remote_height=None. Peers may not be reporting heights in handshake.",
                quorum.compatible_peers
            );
            return Ok(());
        }
    };

    // 2) Get local height from Chain
    let local_height = {
        let chain = crate::CHAIN.lock();
        chain.blocks.len() as u64
    };

    // 🛡️ SAFETY: Never sync DOWN to a lower chain or height-0 peer
    if best_remote == 0 && local_height > 0 {
        tracing::warn!(
            local_height,
            remote_height = best_remote,
            "⚠️ AUTO-SYNC: Refusing to sync from peer at height 0 while we have a non-zero chain. \
            This prevents accidental chain wipes."
        );
        return Ok(());
    }

    // 🛡️ SAFETY: Never sync backwards (only sync UP to higher chains)
    const SAFETY_MARGIN: u64 = 5; // Allow small reorgs
    if best_remote + SAFETY_MARGIN < local_height {
        tracing::warn!(
            local_height,
            remote_height = best_remote,
            "⚠️ AUTO-SYNC: Refusing to sync from peer significantly behind us (local ahead of remote). \
            Only forward syncing is allowed."
        );
        return Ok(());
    }

    // 3) Check if we're already at tip or ahead
    if best_remote <= local_height {
        tracing::debug!(
            "AUTO-SYNC: at tip or ahead (local={}, remote={})",
            local_height,
            best_remote
        );
        return Ok(());
    }

    // 4) Calculate lag
    let lag = best_remote.saturating_sub(local_height);

    // 5) If lag is small, just monitor (don't spam sync)
    if lag < config.max_lag_before_sync {
        tracing::debug!(
            "AUTO-SYNC: small lag={} (< {}), staying in monitor mode (local={}, remote={})",
            lag,
            config.max_lag_before_sync,
            local_height,
            best_remote
        );
        return Ok(());
    }

    // 6) We're behind! Trigger sync
    // Get best peer info for diagnostic logging
    let best_peer_addr = {
        let peers = crate::PEER_MANAGER.connected_peers().await;
        peers
            .iter()
            .filter_map(|p| p.height.map(|h| (format!("{}:{}", p.ip, p.port), h)))
            .max_by_key(|(_, h)| *h)
            .map(|(addr, _)| addr)
            .unwrap_or_else(|| "unknown".to_string())
    };

    tracing::warn!(
        "[SYNC_DECISION] local={} best_peer={} peer_h={} behind_by={} action=START_SYNC (trigger>=1)",
        local_height,
        best_peer_addr,
        best_remote,
        lag
    );

    // CRITICAL: Stop miner BEFORE sync attempt (not after)
    // Don't waste CPU mining wrong fork during fork detection + sync
    tracing::warn!(
        "[AUTO-SYNC] ⏸️  STOPPING MINER: Mining paused during sync (local={}, network={})",
        local_height,
        best_remote
    );
    crate::ACTIVE_MINER.stop();
    crate::ACTIVE_MINER.clear_job();

    tracing::info!(
        "AUTO-SYNC: behind by {} blocks (local={}, remote={}), starting catch-up",
        lag,
        local_height,
        best_remote
    );

    // 7) Perform chain catchup via P2P messages over port 7072 (NOT localhost HTTP!)
    perform_chain_catchup(local_height, best_remote).await?;

    Ok(())
}

/// Trigger chain catchup using P2P messages (NOT HTTP!)
/// This uses the P2P protocol over port 7072, not HTTP over localhost:7070
async fn perform_chain_catchup(local_height: u64, target_height: u64) -> anyhow::Result<()> {
    // Get connected peers with their heights from PEER_MANAGER
    let peers = crate::PEER_MANAGER.connected_peers().await;
    
    if peers.is_empty() {
        return Err(anyhow::anyhow!("no peers available for sync"));
    }

    // 🎯 SMART PEER SELECTION: Pick peer with highest advertised height
    // Filter out:
    // 1. Peers with height <= local_height (they're behind us or stale)
    // 2. Peers at height=1 on mainnet (fake seeds, not real sync sources)
    // 3. Peers with no known height
    let mut viable_peers: Vec<_> = peers
        .iter()
        .filter(|p| {
            if let Some(h) = p.height {
                // On mainnet, reject height=1 peers as sync sources
                // (they're either stale seeds or fresh nodes, not helpful)
                if h <= 1 && local_height > 1 {
                    tracing::debug!(
                        "[SYNC-FILTER] Rejecting peer {}:{} with height={} (below local={}, likely stale seed)",
                        p.ip,
                        p.port,
                        h,
                        local_height
                    );
                    return false;
                }
                // Only accept peers ahead of us
                h > local_height
            } else {
                false // No known height = not viable
            }
        })
        .collect();

    if viable_peers.is_empty() {
        // No peer is ahead of us
        tracing::warn!(
            "[SYNC-PEER-CHOICE] local={} best_seen={} picked=NONE reason=no_peers_ahead_of_us",
            local_height,
            target_height
        );
        return Err(anyhow::anyhow!("no peers ahead of local height"));
    }

    // Sort by height descending (highest first)
    viable_peers.sort_by(|a, b| {
        let h_a = a.height.unwrap_or(0);
        let h_b = b.height.unwrap_or(0);
        h_b.cmp(&h_a) // Descending
    });

    // Pick the peer with the highest height
    let best_peer = viable_peers[0];
    let picked_height = best_peer.height.unwrap_or(0);
    let picked_addr = format!("{}:{}", best_peer.ip, best_peer.port);
    
    // 📖 SMOKING GUN LOG: Show exactly what we're doing
    tracing::info!(
        "[SYNC-PEER-CHOICE] local={} best_seen={} picked={} picked_height={} reason=highest_height viable_peers={}",
        local_height,
        target_height,
        picked_addr,
        picked_height,
        viable_peers.len()
    );

    // 🚀 P2P-BASED SYNC: Request blocks via P2P messages (port 7072)
    // Try best peer first, then fallback to others if it fails
    for (idx, peer) in viable_peers.iter().enumerate() {
        let peer_height = peer.height.unwrap_or(0);
        let peer_addr = format!("{}:{}", peer.ip, peer.port);
        
        // 🔍 FORK SAFETY: Verify peer's chain matches ours before sync
        // Find common ancestor to avoid plugging Ford alternator into Chevy engine
        let common_ancestor = match find_common_ancestor(&peer_addr, local_height).await {
            Ok(height) => height,
            Err(e) => {
                tracing::warn!(
                    "[SYNC-FORK] Failed to find common ancestor with {}: {}",
                    peer_addr,
                    e
                );
                continue; // Try next peer
            }
        };
        
        if common_ancestor < local_height {
            tracing::warn!(
                "[SYNC-FORK] Peer {} diverged at height {} (local={})",
                peer_addr,
                common_ancestor,
                local_height
            );
            tracing::info!(
                "[SYNC-FORK] 🔄 Will sync from common ancestor {} - apply_block() will handle reorg automatically",
                common_ancestor
            );
            // DON'T skip the peer! Let apply_block() handle the reorg when blocks arrive.
            // The block acceptance logic will:
            // 1. Receive competing blocks from peer
            // 2. Calculate cumulative work
            // 3. Automatically reorg if peer's chain has more work
            // 4. Roll back to common ancestor and replay peer's blocks
        }
        
        tracing::info!(
            "[SYNC] -> P2P_SYNC peer={} from_height={} to_height={} (using P2P messages over port 7072)",
            peer_addr,
            local_height + 1,
            target_height
        );
        
        tracing::debug!(
            "[SYNC] Attempting peer {}/{}: {} (height={})",
            idx + 1,
            viable_peers.len(),
            peer_addr,
            peer_height
        );

        // Use P2P GetBlocks message (compatible with old peers)
        let msg = crate::p2p::connection::P2PMessage::GetBlocks {
            start_height: local_height + 1,
            end_height: target_height.min(local_height + 100), // Batch size: 100 blocks max
        };

        // Send via P2P connection manager
        match crate::P2P_MANAGER.send_to_peer(&peer_addr, msg).await {
            Ok(()) => {
                tracing::info!(
                    "[SYNC] ✅ P2P sync request sent to {} (height={}), waiting for blocks...",
                    peer_addr,
                    peer_height
                );
                
                // Poll for blocks to arrive (check every 500ms for up to 10s)
                let start_time = tokio::time::Instant::now();
                let timeout = tokio::time::Duration::from_secs(10);
                let poll_interval = tokio::time::Duration::from_millis(500);
                
                loop {
                    // Check if we made progress
                    let new_height = {
                        let chain = crate::CHAIN.lock();
                        chain.blocks.len() as u64
                    };
                    
                    if new_height > local_height {
                        let pulled = new_height - local_height;
                        tracing::info!(
                            "[AUTO-SYNC] ✅ Pulled {} blocks via P2P from {} (new_height={}) in {:?}",
                            pulled,
                            peer_addr,
                            new_height,
                            start_time.elapsed()
                        );
                        return Ok(());
                    }
                    
                    // Check timeout
                    if start_time.elapsed() >= timeout {
                        tracing::warn!(
                            "[AUTO-SYNC] ⚠️ No blocks received from {} after {:?}, trying next peer",
                            peer_addr,
                            timeout
                        );
                        break;
                    }
                    
                    // Wait before next check
                    tokio::time::sleep(poll_interval).await;
                }
            }
            Err(e) => {
                tracing::debug!(
                    "[AUTO-SYNC] ❌ Failed to send P2P sync request to {} (height={}): {}",
                    peer_addr,
                    peer_height,
                    e
                );
                continue;
            }
        }
    }

    Err(anyhow::anyhow!("failed to sync from any viable peer via P2P"))
}

/// Legacy function - kept for backward compatibility
/// Calls the new spawn_auto_sync_task() with default config
pub fn start_autosync() {
    spawn_auto_sync_task(AutoSyncConfig::default());
}

/// Find common ancestor height with peer using binary search
/// Returns the highest height where both our chain and peer's chain agree
async fn find_common_ancestor(peer_addr: &str, our_height: u64) -> anyhow::Result<u64> {
    // Quick check: does peer agree on our current tip?
    let our_tip_hash = {
        let chain = crate::CHAIN.lock();
        if our_height == 0 || our_height as usize > chain.blocks.len() {
            return Err(anyhow::anyhow!("invalid local height"));
        }
        let idx = (our_height - 1) as usize;
        chain.blocks.get(idx)
            .map(|b| crate::canon_hash(&b.header.pow_hash))
            .ok_or_else(|| anyhow::anyhow!("local tip not found"))?
    };
    
    // Request peer's hash at our height
    tracing::debug!(
        "[SYNC-FORK] Requesting hash at height {} from peer {}",
        our_height,
        peer_addr
    );
    let msg = crate::p2p::connection::P2PMessage::GetBlockHash { height: our_height };
    crate::P2P_MANAGER.send_to_peer(peer_addr, msg).await
        .map_err(|e| anyhow::anyhow!("send_to_peer failed: {}", e))?;
    
    // Wait for response (with timeout) - SYNC_FORK_TIMEOUT_SECS for network round-trip
    tokio::time::sleep(tokio::time::Duration::from_secs(SYNC_FORK_TIMEOUT_SECS)).await;
    
    let peer_hash = {
        let cache = crate::SYNC_HASH_CACHE.lock();
        cache.get(&(peer_addr.to_string(), our_height)).cloned()
    };
    
    match peer_hash {
        Some(hash) if hash == our_tip_hash => {
            // Perfect! Peer agrees on our tip
            tracing::debug!(
                "[SYNC-FORK] Peer {} agrees on height {} hash={}",
                peer_addr,
                our_height,
                &hash[..8]
            );
            return Ok(our_height);
        }
        Some(hash) => {
            // Fork detected! Binary search backwards to find common ancestor
            tracing::warn!(
                "[SYNC-FORK] Peer {} disagrees at height {}: ours={} theirs={}",
                peer_addr,
                our_height,
                &our_tip_hash[..8],
                &hash[..8]
            );
            
            // Binary search from 0 to our_height
            let mut low = 0u64;
            let mut high = our_height;
            let mut common = 0u64;
            
            while low <= high {
                let mid = (low + high) / 2;
                
                // Get our hash at mid
                let our_hash = {
                    let chain = crate::CHAIN.lock();
                    if mid == 0 || mid as usize > chain.blocks.len() {
                        break;
                    }
                    let idx = (mid - 1) as usize;
                    chain.blocks.get(idx)
                        .map(|b| crate::canon_hash(&b.header.pow_hash))
                };
                
                let Some(our_hash) = our_hash else { break; };
                
                // Request peer's hash at mid
                tracing::debug!(
                    "[SYNC-FORK] Binary search: checking height {} (range {}-{})",
                    mid, low, high
                );
                let msg = crate::p2p::connection::P2PMessage::GetBlockHash { height: mid };
                crate::P2P_MANAGER.send_to_peer(peer_addr, msg).await
                    .map_err(|e| anyhow::anyhow!("send_to_peer failed: {}", e))?;
                tokio::time::sleep(tokio::time::Duration::from_secs(SYNC_FORK_SEARCH_TIMEOUT_SECS)).await;
                
                let peer_hash = {
                    let cache = crate::SYNC_HASH_CACHE.lock();
                    cache.get(&(peer_addr.to_string(), mid)).cloned()
                };
                
                match peer_hash {
                    Some(hash) if hash == our_hash => {
                        // Agreement at mid, search higher
                        common = mid;
                        low = mid + 1;
                        tracing::debug!(
                            "[SYNC-FORK] Agreement at height {}, searching higher",
                            mid
                        );
                    }
                    Some(_) => {
                        // Disagreement at mid, search lower
                        if mid == 0 {
                            break;
                        }
                        high = mid - 1;
                        tracing::debug!(
                            "[SYNC-FORK] Disagreement at height {}, searching lower",
                            mid
                        );
                    }
                    None => {
                        // Timeout or peer doesn't have this height
                        tracing::warn!(
                            "[SYNC-FORK] No response from peer at height {}",
                            mid
                        );
                        break;
                    }
                }
            }
            
            tracing::info!(
                "[SYNC-FORK] Common ancestor with {} found at height {}",
                peer_addr,
                common
            );
            return Ok(common);
        }
        None => {
            // Timeout waiting for peer response
            return Err(anyhow::anyhow!(
                "timeout waiting for peer hash response"
            ));
        }
    }
}
