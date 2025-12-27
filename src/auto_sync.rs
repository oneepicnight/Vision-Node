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
            .unwrap_or(1);
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
    /// Requires: sync_status="ready", peers>=2, not too far ahead (desync<=1)
    pub fn exchange_ready(&self) -> bool {
        // Must not be syncing
        if self.is_syncing {
            return false;
        }

        // Must have at least 2 peers for consensus
        if self.connected_peers < 2 {
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
            .unwrap_or(1);
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
    tokio::spawn(async move {
        loop {
            if let Err(e) = auto_sync_step(&config).await {
                tracing::warn!("AUTO-SYNC error: {:?}", e);
            }

            sleep(Duration::from_secs(config.poll_interval_secs)).await;
        }
    });
}

/// Single auto-sync check: compare local vs remote height, sync if needed
async fn auto_sync_step(config: &AutoSyncConfig) -> anyhow::Result<()> {
    // 1) Get best remote height from peer manager
    let best_remote = crate::PEER_MANAGER.best_remote_height().await;

    let best_remote = match best_remote {
        Some(h) => h,
        None => {
            // No peers with known height yet
            tracing::debug!("AUTO-SYNC: no peers with known height");
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
    tracing::info!(
        "AUTO-SYNC: behind by {} blocks (local={}, remote={}), starting catch-up",
        lag,
        local_height,
        best_remote
    );

    // 7) Perform chain catchup via HTTP /sync/pull (uses existing sync logic)
    perform_chain_catchup(local_height, best_remote).await?;

    Ok(())
}

/// Trigger chain catchup by calling existing /sync/pull endpoint
async fn perform_chain_catchup(local_height: u64, target_height: u64) -> anyhow::Result<()> {
    let my_port = local_port();
    let url = format!("http://127.0.0.1:{}/sync/pull", my_port);

    // Get peers from P2P manager
    let peers_url = format!("http://127.0.0.1:{}/peers", my_port);
    let peers: Vec<String> = match HTTP.get(&peers_url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(v) => v
                .get("peers")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(|t| t.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => vec![],
        },
        Err(_) => vec![],
    };

    if peers.is_empty() {
        return Err(anyhow::anyhow!("no peers available for sync"));
    }

    // Try to sync from any peer
    for peer_url in peers {
        let body = json!({
            "src": peer_url,
            "from": local_height + 1,
            "to": target_height
        });

        match HTTP.post(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(result) = resp.json::<serde_json::Value>().await {
                        if let Some(pulled) = result.get("pulled").and_then(|p| p.as_u64()) {
                            if pulled > 0 {
                                tracing::info!(
                                    "AUTO-SYNC: pulled {} blocks from {}",
                                    pulled,
                                    peer_url
                                );
                                return Ok(());
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!("AUTO-SYNC: failed to sync from {}: {:?}", peer_url, e);
                continue;
            }
        }
    }

    Err(anyhow::anyhow!("failed to sync from any peer"))
}

/// Legacy function - kept for backward compatibility
/// Calls the new spawn_auto_sync_task() with default config
pub fn start_autosync() {
    spawn_auto_sync_task(AutoSyncConfig::default());
}
