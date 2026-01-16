#![allow(dead_code)]
//! Routing Intelligence - Phase 3.5 Auto-Clustering
//!
//! Implements intelligent peer selection and cluster balancing for optimal
//! network topology. Maintains a balanced mix of local, regional, and global
//! connections for efficient gossip and resilience.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::peer_store::{ClassifiedPeer, PeerRing, PeerStore, VisionPeer};

/// Cluster balance targets
#[derive(Debug, Clone)]
pub struct ClusterTargets {
    /// Inner ring target (local cluster, low latency)
    pub inner: usize,

    /// Middle ring target (regional backup)
    pub middle: usize,

    /// Outer ring target (global backbone)
    pub outer: usize,
}

impl Default for ClusterTargets {
    fn default() -> Self {
        Self {
            inner: 8,
            middle: 6,
            outer: 4,
        }
    }
}

/// Select relay targets for transaction/block gossip
///
/// Uses intelligent peer selection based on routing rings:
/// - 60% from inner ring (local, low latency)
/// - 25% from middle ring (regional backup)
/// - 15% from outer ring (global reach)
pub fn select_relay_targets(
    peer_store: &PeerStore,
    local_region: Option<&str>,
    max_total: usize,
) -> Vec<VisionPeer> {
    let classified = peer_store.classify_peers_for_routing(local_region);

    let mut inner = Vec::new();
    let mut middle = Vec::new();
    let mut outer = Vec::new();

    // Separate peers by ring
    for cp in classified {
        match cp.ring {
            PeerRing::Inner => inner.push(cp.peer),
            PeerRing::Middle => middle.push(cp.peer),
            PeerRing::Outer => outer.push(cp.peer),
        }
    }

    let mut result = Vec::new();

    // Calculate targets per ring (60% inner, 25% middle, 15% outer)
    let inner_target = (max_total * 60 / 100).max(1);
    let middle_target = (max_total * 25 / 100).max(1);
    let outer_target = (max_total * 15 / 100).max(1);

    // Take best peers from each ring (already sorted by score)
    result.extend(inner.into_iter().take(inner_target));
    result.extend(middle.into_iter().take(middle_target));
    result.extend(outer.into_iter().take(outer_target));

    // Truncate if we went over
    if result.len() > max_total {
        result.truncate(max_total);
    }

    debug!(
        target: "p2p::routing",
        "Selected {} relay targets (inner={}, middle={}, outer={})",
        result.len(),
        result.iter().filter(|p| classify_ring_simple(p, local_region) == PeerRing::Inner).count(),
        result.iter().filter(|p| classify_ring_simple(p, local_region) == PeerRing::Middle).count(),
        result.iter().filter(|p| classify_ring_simple(p, local_region) == PeerRing::Outer).count(),
    );

    result
}

/// Maintain cluster balance background task
///
/// Periodically checks the distribution of connected peers across rings
/// and attempts to maintain target distribution by connecting/disconnecting
/// peers as needed.
pub async fn maintain_cluster_balance(
    peer_store: Arc<PeerStore>,
    local_region: Option<String>,
    targets: ClusterTargets,
) {
    info!(
        target: "p2p::clustering",
        "Starting cluster balance maintenance: inner={}, middle={}, outer={}",
        targets.inner,
        targets.middle,
        targets.outer
    );

    let interval = Duration::from_secs(30);
    let mut round = 0_u64;

    loop {
        sleep(interval).await;
        round += 1;

        let classified = peer_store.classify_peers_for_routing(local_region.as_deref());

        // Count connected peers per ring (for now, we'll consider "recent" peers as connected)
        let mut inner_count = 0;
        let mut middle_count = 0;
        let mut outer_count = 0;

        for cp in &classified {
            if cp.peer.is_recent() {
                match cp.ring {
                    PeerRing::Inner => inner_count += 1,
                    PeerRing::Middle => middle_count += 1,
                    PeerRing::Outer => outer_count += 1,
                }
            }
        }

        debug!(
            target: "p2p::clustering",
            "Round {}: Current distribution - Inner: {}/{}, Middle: {}/{}, Outer: {}/{}",
            round,
            inner_count, targets.inner,
            middle_count, targets.middle,
            outer_count, targets.outer
        );

        // Analyze and report imbalances
        if inner_count < targets.inner {
            info!(
                target: "p2p::clustering",
                "Inner ring under target ({} < {}), need more local peers",
                inner_count,
                targets.inner
            );
        }

        if middle_count < targets.middle {
            info!(
                target: "p2p::clustering",
                "Middle ring under target ({} < {}), need more regional peers",
                middle_count,
                targets.middle
            );
        }

        if outer_count < targets.outer {
            info!(
                target: "p2p::clustering",
                "Outer ring under target ({} < {}), need more global peers",
                outer_count,
                targets.outer
            );
        }

        // TODO: Implement active connection management
        // For now, this serves as monitoring and logging
        // In full implementation, would:
        // 1. Dial additional peers from under-represented rings
        // 2. Gracefully disconnect lowest-scoring peers from over-represented rings
        // 3. Use PeerManager to open/close connections
    }
}

/// Simple ring classification helper (doesn't need full ClassifiedPeer)
fn classify_ring_simple(peer: &VisionPeer, local_region: Option<&str>) -> PeerRing {
    let same_region = match (local_region, &peer.region) {
        (Some(l), Some(r)) => r.starts_with(l),
        _ => false,
    };

    let avg = peer.avg_rtt_ms.unwrap_or(200);

    if same_region && avg <= 100 {
        PeerRing::Inner
    } else if same_region {
        PeerRing::Middle
    } else {
        PeerRing::Outer
    }
}

/// Get guardian and anchor peers for backbone connections
///
/// Prioritizes guardians and anchors for outer ring (global backbone) connections.
/// These are trusted, high-availability nodes ideal for cross-region connectivity.
pub fn select_backbone_peers(peer_store: &PeerStore, max: usize) -> Vec<VisionPeer> {
    let mut peers = peer_store.all();

    // Filter to guardians and anchors only
    peers.retain(|p| p.role == "guardian" || p.role == "anchor");

    // Sort by routing score (highest first)
    peers.sort_by(|a, b| {
        let sa = peer_store.routing_score(a, None);
        let sb = peer_store.routing_score(b, None);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    peers.into_iter().take(max).collect()
}

/// Get cluster statistics for monitoring
pub struct ClusterStats {
    pub inner_count: usize,
    pub middle_count: usize,
    pub outer_count: usize,
    pub guardians: usize,
    pub anchors: usize,
    pub avg_inner_latency: u32,
    pub avg_middle_latency: u32,
    pub avg_outer_latency: u32,
}

pub fn get_cluster_stats(peer_store: &PeerStore, local_region: Option<&str>) -> ClusterStats {
    let classified = peer_store.classify_peers_for_routing(local_region);

    let mut inner_count = 0;
    let mut middle_count = 0;
    let mut outer_count = 0;
    let mut guardians = 0;
    let mut anchors = 0;

    let mut inner_latencies = Vec::new();
    let mut middle_latencies = Vec::new();
    let mut outer_latencies = Vec::new();

    for cp in classified {
        if !cp.peer.is_recent() {
            continue;
        }

        match cp.ring {
            PeerRing::Inner => {
                inner_count += 1;
                if let Some(lat) = cp.peer.avg_rtt_ms {
                    inner_latencies.push(lat);
                }
            }
            PeerRing::Middle => {
                middle_count += 1;
                if let Some(lat) = cp.peer.avg_rtt_ms {
                    middle_latencies.push(lat);
                }
            }
            PeerRing::Outer => {
                outer_count += 1;
                if let Some(lat) = cp.peer.avg_rtt_ms {
                    outer_latencies.push(lat);
                }
            }
        }

        if cp.peer.role == "guardian" {
            guardians += 1;
        } else if cp.peer.role == "anchor" {
            anchors += 1;
        }
    }

    let avg_inner_latency = if inner_latencies.is_empty() {
        0
    } else {
        inner_latencies.iter().sum::<u32>() / inner_latencies.len() as u32
    };

    let avg_middle_latency = if middle_latencies.is_empty() {
        0
    } else {
        middle_latencies.iter().sum::<u32>() / middle_latencies.len() as u32
    };

    let avg_outer_latency = if outer_latencies.is_empty() {
        0
    } else {
        outer_latencies.iter().sum::<u32>() / outer_latencies.len() as u32
    };

    ClusterStats {
        inner_count,
        middle_count,
        outer_count,
        guardians,
        anchors,
        avg_inner_latency,
        avg_middle_latency,
        avg_outer_latency,
    }
}

// ============================================================================
// Phase 4: Learning-Based Routing with Epsilon-Greedy Exploration
// ============================================================================

/// Select relay targets with learning-based routing and epsilon-greedy exploration
///
/// **Epsilon-Greedy Strategy:**
/// - 90% exploitation: Choose highest-scoring peers (proven performance)
/// - 10% exploration: Choose random peers (discover better routes)
///
/// This allows the network to:
/// 1. Favor peers with proven success rates
/// 2. Discover new high-performance routes
/// 3. Adapt to changing network conditions
/// 4. Avoid getting stuck in local optima
pub fn select_relay_targets_with_learning(
    peer_store: &PeerStore,
    local_region: Option<&str>,
    max_total: usize,
    epsilon: f32, // Exploration rate (0.0 = pure exploitation, 1.0 = pure exploration)
) -> Vec<VisionPeer> {
    use crate::p2p::reputation::is_excluded_from_routing;
    use rand::Rng;

    let classified = peer_store.classify_peers_for_routing(local_region);

    // Filter out banned/graylisted peers
    let available: Vec<ClassifiedPeer> = classified
        .into_iter()
        .filter(|cp| !is_excluded_from_routing(&cp.peer))
        .collect();

    if available.is_empty() {
        warn!(target: "p2p::routing", "No available peers for relay");
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut rng = rand::thread_rng();

    // Calculate how many to explore vs exploit
    let explore_count = ((max_total as f32) * epsilon) as usize;
    let exploit_count = max_total.saturating_sub(explore_count);

    // 1) Exploitation: Take best-scoring peers (already sorted by score)
    let exploit_peers = available
        .iter()
        .take(exploit_count)
        .map(|cp| cp.peer.clone())
        .collect::<Vec<_>>();

    result.extend(exploit_peers);

    // 2) Exploration: Random selection from remaining peers
    let remaining: Vec<VisionPeer> = available
        .into_iter()
        .skip(exploit_count)
        .map(|cp| cp.peer)
        .collect();

    if !remaining.is_empty() {
        let mut explore_peers = Vec::new();
        let mut used_indices = std::collections::HashSet::new();

        for _ in 0..explore_count.min(remaining.len()) {
            // rand 0.8 API: gen_range(low..high)
            let mut idx = rng.gen_range(0..remaining.len());
            while used_indices.contains(&idx) && used_indices.len() < remaining.len() {
                idx = rng.gen_range(0..remaining.len());
            }
            used_indices.insert(idx);
            explore_peers.push(remaining[idx].clone());
        }

        result.extend(explore_peers);
    }

    debug!(
        target: "p2p::routing",
        "Selected {} relay targets with learning (exploit: {}, explore: {}, epsilon: {:.2})",
        result.len(),
        exploit_count,
        result.len().saturating_sub(exploit_count),
        epsilon
    );

    result
}

/// Mark successful route delivery (called when message confirms delivery)
///
/// Updates peer effectiveness metrics:
/// - Increments route_uses and route_successes
/// - Updates avg_delivery_ms with EMA
/// - Persists to peer store
pub fn mark_route_success(
    peer_store: &mut PeerStore,
    peer_id: &str,
    delivery_time_ms: u32,
) -> Result<(), String> {
    use crate::p2p::reputation::mark_route_success as reputation_mark_success;

    if let Some(mut peer) = peer_store.get(peer_id) {
        reputation_mark_success(&mut peer, delivery_time_ms);

        // Log peer promotion event if performing exceptionally well
        if peer.route_uses > 100 {
            let success_rate = (peer.route_successes as f32 / peer.route_uses as f32) * 100.0;
            if success_rate >= 95.0 {
                if let Some(avg_ms) = peer.avg_delivery_ms {
                    crate::api::routing_api::log_peer_promotion_event(
                        &peer.node_tag,
                        success_rate,
                        avg_ms,
                    );
                }
            }
        }

        peer_store
            .upsert(peer)
            .map_err(|e| format!("Failed to update peer: {}", e))?;
        Ok(())
    } else {
        Err(format!("Peer {} not found", peer_id))
    }
}

/// Mark failed route delivery (called when message times out or fails)
///
/// Updates peer effectiveness metrics:
/// - Increments route_uses and route_failures
/// - Lowers routing score for future selection
/// - Persists to peer store
pub fn mark_route_failure(peer_store: &mut PeerStore, peer_id: &str) -> Result<(), String> {
    use crate::p2p::reputation::mark_route_failure as reputation_mark_failure;

    if let Some(mut peer) = peer_store.get(peer_id) {
        reputation_mark_failure(&mut peer);
        peer_store
            .upsert(peer)
            .map_err(|e| format!("Failed to update peer: {}", e))?;
        Ok(())
    } else {
        Err(format!("Peer {} not found", peer_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::peer_store::VisionPeer;

    #[test]
    fn test_cluster_targets_default() {
        let targets = ClusterTargets::default();
        assert_eq!(targets.inner, 8);
        assert_eq!(targets.middle, 6);
        assert_eq!(targets.outer, 4);
        assert_eq!(targets.inner + targets.middle + targets.outer, 18);
    }

    #[test]
    fn test_epsilon_range() {
        let epsilon = 0.1;
        assert!(epsilon >= 0.0 && epsilon <= 1.0);

        let max_total = 10;
        let explore_count = ((max_total as f32) * epsilon) as usize;
        let exploit_count = max_total - explore_count;

        assert_eq!(explore_count, 1); // 10% of 10 = 1
        assert_eq!(exploit_count, 9); // 90% of 10 = 9
    }
}
