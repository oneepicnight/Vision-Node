#![allow(dead_code)]
//! Swarm-only bootstrap for testnet
//!
//! TESTNET SWARM-ONLY MODE BOOTSTRAP:
//! Change the bootstrap logic so we do NOT give up and go into isolated mode after one failed attempt.
//! In swarm-only mode, we loop forever: try peer_store, then seeds; if still 0 connected peers,
//! sleep according to p2p_config.retry_backoff (cycled) and try again.
//! Only leave bootstrap once we have at least one connected peer.

use super::p2p_config::{DiscoveryMode, SeedPeersConfig};
use super::peer_manager::{Peer, PeerBucket, PeerManager, PeerState};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

/// Bootstrap state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapState {
    /// Successfully joined network with at least one peer
    Joined,
    /// Isolated mode (no peers) - only used in GuardianAware/Hybrid modes
    Isolated,
}

/// Bootstrap the network with infinite retry for swarm-only mode
///
/// In SwarmOnly mode:
/// - Try peer_store (persisted peers from previous sessions)
/// - Try genesis seed peers
/// - If still 0 connected peers, sleep with backoff and retry forever
/// - Never give up and go isolated
///
/// In GuardianAware/Hybrid modes:
/// - Try once, fall back to isolated mode if no peers found
pub async fn bootstrap_swarm(
    cfg: &SeedPeersConfig,
    peer_manager: Arc<PeerManager>,
) -> BootstrapState {
    let mut attempt: u32 = 0;

    // Backoff strategy - use exponential backoff based on retry_backoff setting
    let base_backoff_secs = cfg.retry_backoff;
    let backoff = vec![
        base_backoff_secs / 12, // 5s if base is 60
        base_backoff_secs / 6,  // 10s
        base_backoff_secs / 2,  // 30s
        base_backoff_secs,      // 60s
        base_backoff_secs * 2,  // 120s
        base_backoff_secs * 5,  // 300s
    ];

    loop {
        attempt += 1;
        info!(
            target: "p2p::swarm_bootstrap",
            "🔄 Bootstrap attempt #{} (discovery_mode = {:?})",
            attempt,
            cfg.discovery_mode
        );

        // 1) Try to restore connections from peer_store
        let connected_from_store = try_connect_peer_store(cfg, &peer_manager).await;
        if connected_from_store > 0 {
            info!(
                target: "p2p::swarm_bootstrap",
                "✅ Connected {} peers from peer_store",
                connected_from_store
            );
        }

        // 2) If still no peers, try genesis seeds
        let _connected_from_seeds = if connected_peers_count(&peer_manager).await == 0 {
            let seeds = try_connect_seed_peers(cfg, &peer_manager).await;
            if seeds > 0 {
                info!(
                    target: "p2p::swarm_bootstrap",
                    "✅ Added {} seed peers to peer_store",
                    seeds
                );
            }
            seeds
        } else {
            0
        };

        // Check total connected peers
        let total_connected = connected_peers_count(&peer_manager).await;
        if total_connected > 0 {
            info!(
                target: "p2p::swarm_bootstrap",
                "🎉 Bootstrap successful: {} peer(s) connected",
                total_connected
            );
            return BootstrapState::Joined;
        }

        // No peers yet – behavior depends on discovery mode
        match cfg.discovery_mode {
            DiscoveryMode::Dynamic | DiscoveryMode::Hybrid => {
                // In guardian-aware modes, fall back to isolated mode after one attempt
                warn!(
                    target: "p2p::swarm_bootstrap",
                    "⚠️  No peers after bootstrap attempt #{}, entering isolated mode (GuardianAware/Hybrid)",
                    attempt
                );
                return BootstrapState::Isolated;
            }
            DiscoveryMode::Static => {
                // In swarm-only mode, NEVER give up – retry forever with backoff
                let delay_secs = backoff[(attempt as usize - 1) % backoff.len()];
                warn!(
                    target: "p2p::swarm_bootstrap",
                    "🔄 No peers after attempt #{} in SwarmOnly mode. Retrying in {} seconds... (seeds will NEVER be blacklisted)",
                    attempt,
                    delay_secs
                );
                sleep(Duration::from_secs(delay_secs)).await;
                continue;
            }
        }
    }
}

/// Try to connect to peers from peer_store (persisted from previous sessions)
///
/// In swarm mode, this function:
/// 1. Checks peer_store for previously connected peers
/// 2. Attempts to reconnect to them
/// 3. Returns the count of currently connected peers
async fn try_connect_peer_store(_cfg: &SeedPeersConfig, peer_manager: &PeerManager) -> usize {
    // Get all known peers
    let peers = peer_manager.get_all_peers().await;

    let connected = peers
        .iter()
        .filter(|p| p.state == PeerState::Connected)
        .count();

    info!(
        target: "p2p::swarm_bootstrap",
        "📚 Peer store has {} known peers, {} already connected",
        peers.len(),
        connected
    );

    // Note: Actual connection attempts are handled by the P2P subsystem
    // This function reports the current state
    // The peer manager maintains peer state and the P2P layer attempts connections

    connected
}

/// Try to connect to genesis seed peers
/// Seeds are added to peer_store with permanent flags in swarm mode
async fn try_connect_seed_peers(cfg: &SeedPeersConfig, peer_manager: &PeerManager) -> usize {
    let mut added = 0;

    for seed_addr in &cfg.seed_peers {
        info!(
            target: "p2p::swarm_bootstrap",
            "🌱 Adding seed peer to peer_store: {}",
            seed_addr
        );

        // Parse seed address
        if let Ok(socket_addr) = seed_addr.parse::<std::net::SocketAddr>() {
            // Generate EBID from address (deterministic)
            let ebid = format!("seed-{}", seed_addr.replace(':', "-"));

            // Create seed peer with permanent flags
            let mut seed_peer = Peer::new(
                socket_addr.ip().to_string(),
                socket_addr.port(),
                ebid.clone(),
            );

            // In Static mode with permanent_seeds, protect seed peers
            if cfg.discovery_mode == DiscoveryMode::Static && !cfg.permanent_seeds.is_empty() {
                // Mark as Hot bucket for protection from eviction
                seed_peer.bucket = PeerBucket::Hot;
                info!(
                    target: "p2p::swarm_bootstrap",
                    "🔒 Seed {} marked as PERMANENT (Hot bucket protected)",
                    seed_addr
                );
            }

            // Add to peer manager
            peer_manager.add_peer(seed_peer).await;
            added += 1;
        } else {
            warn!(
                target: "p2p::swarm_bootstrap",
                "❌ Invalid seed peer address: {}",
                seed_addr
            );
        }
    }

    added
}

/// Get count of currently connected peers
async fn connected_peers_count(peer_manager: &PeerManager) -> usize {
    peer_manager
        .get_all_peers()
        .await
        .iter()
        .filter(|p| p.state == PeerState::Connected)
        .count()
}
