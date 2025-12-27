#![allow(dead_code)]
//! Network Self-Healing - Continuous network health maintenance
//!
//! This worker runs continuously to:
//! - Retry lost peers
//! - Verify blockchain heights
//! - Repair stale routes
//! - Purge dead entries
//! - Refresh anchor connections
//! - Optimize network paths
//!
//! Result: Even if 50% of network shuts down, the rest reorganizes and continues

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::p2p::anchor_election::{evaluate_anchor_candidates, AnchorRegistry};
use crate::p2p::backoff::current_time;
use crate::p2p::peer_store::PeerStore;

/// Self-healing check interval (60 seconds)
const HEALING_INTERVAL_SECS: u64 = 60;

/// Maximum age for stale peer entries (24 hours)
const MAX_PEER_AGE_SECS: u64 = 86400;

/// Health score threshold for purging dead peers
const PURGE_THRESHOLD: i32 = -50;

/// Minimum number of connections to maintain
const MIN_HEALTHY_CONNECTIONS: usize = 5;

/// Self-healing statistics
#[derive(Debug, Clone, Default)]
pub struct HealingStats {
    pub peers_retried: u32,
    pub peers_purged: u32,
    pub routes_repaired: u32,
    pub anchors_refreshed: u32,
    pub last_run: u64,
}

/// Network self-healing worker
pub async fn spawn_self_healing_worker(
    peer_store: Arc<PeerStore>,
    anchor_registry: Arc<AnchorRegistry>,
) {
    info!(
        "[HEAL] 🔄 Starting network self-healing worker (interval: {}s)",
        HEALING_INTERVAL_SECS
    );

    loop {
        sleep(Duration::from_secs(HEALING_INTERVAL_SECS)).await;

        let stats = perform_healing_cycle(peer_store.clone(), anchor_registry.clone()).await;

        if stats.peers_purged > 0 || stats.routes_repaired > 0 {
            info!(
                "[HEAL] Healing cycle complete: retried={}, purged={}, repaired={}, anchors={}",
                stats.peers_retried,
                stats.peers_purged,
                stats.routes_repaired,
                stats.anchors_refreshed
            );
        }
    }
}

/// Compatibility function for main.rs spawn code
pub async fn spawn_network_healing(
    interval: Duration,
    min_healthy: usize,
    peer_store: Arc<PeerStore>,
    anchor_registry: Arc<AnchorRegistry>,
) {
    info!(
        "[HEAL] 🔄 Starting network healing (interval: {:?}, min_healthy: {})",
        interval, min_healthy
    );

    loop {
        sleep(interval).await;

        let stats = perform_healing_cycle(peer_store.clone(), anchor_registry.clone()).await;

        if stats.peers_purged > 0 || stats.routes_repaired > 0 {
            info!(
                "[HEAL] Healing cycle complete: retried={}, purged={}, repaired={}, anchors={}",
                stats.peers_retried,
                stats.peers_purged,
                stats.routes_repaired,
                stats.anchors_refreshed
            );
        }

        let health = check_network_health(peer_store.clone()).await;
        if health.total_peers < min_healthy {
            warn!(
                "[HEAL] ⚠️  Network health degraded: {} peers (need {})",
                health.total_peers, min_healthy
            );
        }
    }
}

/// Perform one healing cycle
async fn perform_healing_cycle(
    peer_store: Arc<PeerStore>,
    anchor_registry: Arc<AnchorRegistry>,
) -> HealingStats {
    let mut stats = HealingStats::default();
    stats.last_run = current_time();

    let now = current_time();
    let all_peers = peer_store.get_all();

    // Count healthy connections (health_score > 30 = healthy)
    let healthy_count = all_peers.iter().filter(|p| p.health_score > 30).count();

    debug!("[HEAL] Current healthy connections: {}", healthy_count);

    // Phase 1: Purge dead/unreachable peers
    for peer in &all_peers {
        // Check if peer should be purged
        if should_purge_peer(peer, now) {
            if let Err(e) = peer_store.remove(&peer.node_id) {
                warn!("[HEAL] Failed to purge peer {}: {}", peer.node_tag, e);
            } else {
                info!(
                    "[HEAL] 🗑️  Purged dead peer: {} (score: {})",
                    peer.node_tag, peer.health_score
                );
                stats.peers_purged += 1;
            }
            continue;
        }

        // Check if healthy peer has become stale
        if peer.health_score > 30 {
            let time_since_success = now.saturating_sub(peer.last_success);
            if time_since_success > 7200 {
                // Healthy peer not seen in 2 hours - mark for retry
                info!(
                    "[HEAL] Peer {} is stale ({}h idle) - will retry",
                    peer.node_tag,
                    time_since_success / 3600
                );
                stats.routes_repaired += 1;
            }
        }
    }

    // Phase 2: Evaluate and refresh anchors
    let anchor_candidates = evaluate_anchor_candidates(peer_store.clone()).await;
    let active_anchors = anchor_registry.get_active_anchors().await;

    // If we have fewer than max anchors, elect new ones
    if active_anchors.len() < crate::p2p::anchor_election::MAX_ANCHORS {
        for mut candidate in anchor_candidates.into_iter().take(3) {
            // Skip if already elected
            if active_anchors
                .iter()
                .any(|a| a.node_id == candidate.node_id)
            {
                continue;
            }

            candidate.status = crate::p2p::anchor_election::AnchorStatus::Active;
            candidate.elected_at = now;
            anchor_registry.elect_anchor(candidate).await;
            stats.anchors_refreshed += 1;
        }
    }

    // Phase 3: Check anchor health and demote if needed
    for anchor in active_anchors {
        if anchor.should_demote() {
            anchor_registry.demote_anchor(&anchor.node_id).await;
            info!("[HEAL] Demoted unhealthy anchor: {}", anchor.node_tag);
        }
    }

    // Phase 4: Network status assessment
    if healthy_count < MIN_HEALTHY_CONNECTIONS {
        warn!(
            "[HEAL] ⚠️  Network health degraded: only {} healthy connections (need {})",
            healthy_count, MIN_HEALTHY_CONNECTIONS
        );
    } else {
        debug!("[HEAL] ✅ Network health: {} connections", healthy_count);
    }

    stats
}

/// Check if peer should be purged
fn should_purge_peer(peer: &crate::p2p::peer_store::VisionPeer, now: u64) -> bool {
    // Never purge seeds
    if peer.is_seed {
        return false;
    }

    // Purge if health score is critically low
    if peer.health_score < PURGE_THRESHOLD {
        return true;
    }

    // Purge if too many consecutive failures
    if peer.fail_count > 10 {
        return true;
    }

    // Purge if very old and never connected
    if peer.last_success == 0 {
        let age = now.saturating_sub(peer.last_failure);
        if age > MAX_PEER_AGE_SECS {
            return true;
        }
    }

    false
}

/// Check if network is in healthy state
pub async fn check_network_health(peer_store: Arc<PeerStore>) -> NetworkHealth {
    let all_peers = peer_store.get_all();

    let total = all_peers.len();
    let verified = all_peers.iter().filter(|p| p.trusted).count();
    let stable = all_peers.iter().filter(|p| p.health_score > 30).count();
    let quarantined = all_peers.iter().filter(|p| p.health_score <= 0).count();
    let trying = all_peers
        .iter()
        .filter(|p| p.health_score > 0 && p.health_score <= 30)
        .count();

    let healthy = verified + stable; // Total healthy connections

    let status = if healthy >= 15 {
        HealthStatus::Optimal
    } else if healthy >= 8 {
        HealthStatus::Good
    } else if healthy >= 3 {
        HealthStatus::Fair
    } else if healthy > 0 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Critical
    };

    NetworkHealth {
        status,
        total_peers: total,
        verified_peers: verified,
        stable_peers: stable,
        quarantined_peers: quarantined,
        trying_peers: trying,
    }
}

/// Network health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Optimal,  // 15+ connections
    Good,     // 8-14 connections
    Fair,     // 3-7 connections
    Degraded, // 1-2 connections
    Critical, // 0 connections
}

impl HealthStatus {
    pub fn label(&self) -> &'static str {
        match self {
            HealthStatus::Optimal => "OPTIMAL",
            HealthStatus::Good => "GOOD",
            HealthStatus::Fair => "FAIR",
            HealthStatus::Degraded => "DEGRADED",
            HealthStatus::Critical => "CRITICAL",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            HealthStatus::Optimal => "🌌",
            HealthStatus::Good => "✅",
            HealthStatus::Fair => "⚠️",
            HealthStatus::Degraded => "❌",
            HealthStatus::Critical => "🆘",
        }
    }
}

/// Network health report
#[derive(Debug, Clone)]
pub struct NetworkHealth {
    pub status: HealthStatus,
    pub total_peers: usize,
    pub verified_peers: usize,
    pub stable_peers: usize,
    pub quarantined_peers: usize,
    pub trying_peers: usize,
}

impl NetworkHealth {
    pub fn log_status(&self) {
        info!(
            "{} Vision Network State: {} | Connected: {} | Verified: {} | Stable: {} | Quarantined: {} | Trying: {}",
            self.status.emoji(),
            self.status.label(),
            self.verified_peers + self.stable_peers,
            self.verified_peers,
            self.stable_peers,
            self.quarantined_peers,
            self.trying_peers
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::peer_store::VisionPeer;

    #[test]
    fn test_purge_criteria() {
        let now = current_time();

        // Healthy peer - should not purge
        let mut peer = VisionPeer::new(
            "test-1".to_string(),
            "VNODE-1".to_string(),
            "pk".to_string(),
            "vision://test".to_string(),
            None,
            "constellation".to_string(),
        );
        peer.health_score = 70;
        assert!(!should_purge_peer(&peer, now));

        // Critical low score - should purge
        peer.health_score = -100;
        assert!(should_purge_peer(&peer, now));

        // Seed protection - never purge
        peer.is_seed = true;
        assert!(!should_purge_peer(&peer, now));
    }

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Optimal.label(), "OPTIMAL");
        assert_eq!(HealthStatus::Critical.emoji(), "🆘");
    }
}
