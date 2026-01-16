#![allow(dead_code)]
//! Latency Monitoring Engine - Phase 3.5 Routing Intelligence
//!
//! Periodically probes peers to measure round-trip time (RTT) and update
//! routing scores for intelligent peer selection.

use rand::seq::SliceRandom;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

use super::peer_store::{PeerStore, VisionPeer};

/// Latency monitoring configuration
#[derive(Debug, Clone)]
pub struct LatencyConfig {
    /// How often to probe peers (seconds)
    pub probe_interval_secs: u64,

    /// Maximum number of peers to probe per round
    pub max_peers_per_round: usize,

    /// Timeout for ping operations (milliseconds)
    pub ping_timeout_ms: u64,
}

impl Default for LatencyConfig {
    fn default() -> Self {
        Self {
            probe_interval_secs: 30,
            max_peers_per_round: 16,
            ping_timeout_ms: 2000,
        }
    }
}

/// Start latency monitoring background task
///
/// Periodically samples connected peers and measures their RTT.
/// Updates PeerStore with latency measurements for routing decisions.
pub async fn start_latency_monitor(peer_store: Arc<PeerStore>, config: LatencyConfig) {
    info!(
        target: "p2p::latency",
        "Starting latency monitor: probe_interval={}s, max_peers_per_round={}",
        config.probe_interval_secs,
        config.max_peers_per_round
    );

    let interval = Duration::from_secs(config.probe_interval_secs);
    let mut round = 0_u64;

    loop {
        sleep(interval).await;
        round += 1;

        // Get all recently seen peers
        let peers = peer_store.recent();
        if peers.is_empty() {
            debug!(target: "p2p::latency", "No recent peers to probe (round {})", round);
            continue;
        }

        // Select random subset for this round
        let sample = select_random_subset(peers, config.max_peers_per_round);

        debug!(
            target: "p2p::latency",
            "Round {}: Probing {} peers",
            round,
            sample.len()
        );

        // Probe each peer in parallel
        let mut tasks = Vec::new();
        for peer in sample {
            let peer_store = peer_store.clone();
            let timeout = config.ping_timeout_ms;

            let task = tokio::spawn(async move { probe_peer(peer, peer_store, timeout).await });

            tasks.push(task);
        }

        // Wait for all probes to complete
        let mut probed = 0;
        let mut successful = 0;

        for task in tasks {
            if let Ok(ok) = task.await {
                probed += 1;
                if ok {
                    successful += 1;
                }
            }
        }

        debug!(
            target: "p2p::latency",
            "Round {} complete: {}/{} probes successful",
            round,
            successful,
            probed
        );
    }
}

/// Probe a single peer and update latency metrics
async fn probe_peer(peer: VisionPeer, peer_store: Arc<PeerStore>, timeout_ms: u64) -> bool {
    let peer_id = peer.node_id.clone();
    let node_tag = peer.node_tag.clone();

    // Attempt ping with timeout
    let start = Instant::now();

    // Use tokio timeout for ping operation
    let result = tokio::time::timeout(Duration::from_millis(timeout_ms), send_ping(&peer)).await;

    match result {
        Ok(Ok(())) => {
            // Successful ping
            let rtt = start.elapsed().as_millis() as u32;

            if let Err(e) = peer_store.update_peer_latency(&peer_id, rtt, true) {
                warn!(
                    target: "p2p::latency",
                    "Failed to update latency for {}: {}",
                    node_tag,
                    e
                );
            }

            true
        }
        Ok(Err(e)) => {
            // Ping failed
            let elapsed = start.elapsed().as_millis() as u32;
            let penalty_rtt = elapsed.max(500); // Penalize with at least 500ms

            if let Err(update_err) = peer_store.update_peer_latency(&peer_id, penalty_rtt, false) {
                warn!(
                    target: "p2p::latency",
                    "Failed to update latency for {}: {}",
                    node_tag,
                    update_err
                );
            }

            debug!(
                target: "p2p::latency",
                "Ping failed for {}: {}",
                node_tag,
                e
            );

            false
        }
        Err(_) => {
            // Timeout
            let penalty_rtt = timeout_ms as u32;

            if let Err(e) = peer_store.update_peer_latency(&peer_id, penalty_rtt, false) {
                warn!(
                    target: "p2p::latency",
                    "Failed to update latency for {}: {}",
                    node_tag,
                    e
                );
            }

            debug!(
                target: "p2p::latency",
                "Ping timeout for {} (>{}ms)",
                node_tag,
                timeout_ms
            );

            false
        }
    }
}

/// Send a ping message to a peer
///
/// This is a lightweight health check. In a real implementation, this would
/// send a P2P ping message and wait for pong response.
async fn send_ping(peer: &VisionPeer) -> Result<(), String> {
    // For now, we'll simulate a ping with a small delay
    // In production, this would use the actual P2P messaging layer

    // Simulate network delay based on existing latency bucket
    let simulated_delay_ms = match peer.latency_bucket {
        Some(super::peer_store::LatencyBucket::UltraLow) => 10,
        Some(super::peer_store::LatencyBucket::Low) => 50,
        Some(super::peer_store::LatencyBucket::Medium) => 100,
        Some(super::peer_store::LatencyBucket::High) => 200,
        Some(super::peer_store::LatencyBucket::Extreme) => 400,
        None => 100,
    };

    // Add some jitter
    let jitter = (rand::random::<u64>() % 50) as u64;
    tokio::time::sleep(Duration::from_millis(simulated_delay_ms + jitter)).await;

    // Simulate 95% success rate (5% packet loss)
    if rand::random::<u8>() < 242 {
        // 95% of 256
        Ok(())
    } else {
        Err("Simulated packet loss".to_string())
    }

    // TODO: Replace with actual P2P ping implementation
    // Example:
    // let addr = peer.ip_address.as_ref().ok_or("No IP address")?;
    // p2p_manager.send_ping(addr).await?;
    // Ok(())
}

/// Select random subset of peers for probing
fn select_random_subset(mut peers: Vec<VisionPeer>, max: usize) -> Vec<VisionPeer> {
    if peers.len() <= max {
        return peers;
    }

    let mut rng = rand::thread_rng();
    peers.shuffle(&mut rng);
    peers.into_iter().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_random_subset() {
        let peers: Vec<VisionPeer> = (0..20)
            .map(|i| {
                VisionPeer::new(
                    format!("node-{}", i),
                    format!("TAG-{}", i),
                    format!("pubkey-{}", i),
                    format!("vision://tag-{}@hash", i),
                    None,
                    "constellation".to_string(),
                )
            })
            .collect();

        let sample = select_random_subset(peers.clone(), 10);
        assert_eq!(sample.len(), 10);

        let sample_all = select_random_subset(peers.clone(), 50);
        assert_eq!(sample_all.len(), 20);
    }
}
