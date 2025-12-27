//! Autonomous Peer Recovery System
//!
//! Automatically reconnects to known peers when connections are lost,
//! enabling the Hydra effect: kill one connection, the node finds another.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

/// ⭐ Change 8: Target minimum peers for healthy network
const MIN_TARGET_PEERS: usize = 3;

/// ⭐ Change 8: Maximum target peers to avoid overwhelming network
const MAX_TARGET_PEERS: usize = 8;

/// Tracks failed connection attempts per peer
pub struct FailureTracker {
    failures: HashMap<String, (u32, u64)>, // peer_id -> (count, last_attempt_time)
    max_failures: u32,
    reset_window_secs: u64,
}

impl FailureTracker {
    pub fn new(max_failures: u32, reset_window_secs: u64) -> Self {
        Self {
            failures: HashMap::new(),
            max_failures,
            reset_window_secs,
        }
    }

    /// Record a failed connection attempt
    pub fn record_failure(&mut self, peer_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = self.failures.entry(peer_id.to_string()).or_insert((0, now));
        entry.0 += 1;
        entry.1 = now;
    }

    /// Check if peer should be skipped due to recent failures
    pub fn should_skip(&self, peer_id: &str) -> bool {
        if let Some((count, _)) = self.failures.get(peer_id) {
            *count >= self.max_failures
        } else {
            false
        }
    }

    /// Reset failure counters for peers outside the window
    pub fn cleanup(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.failures
            .retain(|_, (_, last_attempt)| now - *last_attempt < self.reset_window_secs);
    }
}

/// Spawn the autonomous peer recovery loop
///
/// This background task monitors peer connectivity and automatically
/// attempts to reconnect to known peers when connections drop below threshold.
pub async fn spawn_peer_recovery_loop(recovery_interval_secs: u64, min_peer_count: usize) {
    info!(
        target: "vision_node::p2p::recovery",
        "[PEER_RECOVERY] Starting autonomous recovery loop (interval: {}s, min_peers: {})",
        recovery_interval_secs,
        min_peer_count
    );

    let mut failure_tracker = FailureTracker::new(5, 600); // Max 5 failures in 10 minutes

    loop {
        sleep(Duration::from_secs(recovery_interval_secs)).await;

        // Cleanup old failure records periodically
        failure_tracker.cleanup();

        // Decay fail_count for all peers (P2P Robustness #2)
        {
            let mut memory = crate::CONSTELLATION_MEMORY.lock();
            memory.decay_all_fail_counts();
        }

        // Check current peer count (validated connected peers)
        let live_peers = crate::PEER_MANAGER.connected_validated_count().await;

        // ⭐ Change 8: Use MIN_TARGET_PEERS threshold
        if live_peers < MIN_TARGET_PEERS {
            warn!(
                target: "vision_node::p2p::recovery",
                "[PEER_RECOVERY] Peer count low ({}). Requesting additional peers from beacon and peer book...",
                live_peers
            );

            // Get best peers from constellation memory (request up to MAX_TARGET_PEERS)
            let candidates = {
                let memory = crate::CONSTELLATION_MEMORY.lock();
                memory.get_best_peers(MAX_TARGET_PEERS * 2) // Get extra candidates for filtering
            };

            if candidates.is_empty() {
                if live_peers == 0 {
                    warn!(
                        target: "vision_node::p2p::recovery",
                        "[PEER_RECOVERY] No peers in constellation memory and 0 connected. Running in isolated mode."
                    );
                } else {
                    info!(
                        target: "vision_node::p2p::recovery",
                        "[HEALTH] Not isolated: at least {} peer(s) connected. Continuing recovery in background.",
                        live_peers
                    );
                }
                continue;
            }

            info!(
                target: "vision_node::p2p::recovery",
                "[PEER_RECOVERY] Found {} candidates in memory. Attempting reconnection...",
                candidates.len()
            );

            // Try to reconnect to peers
            let mut attempted = 0;
            let mut connected = 0;

            for peer in candidates {
                // Skip if recently failed too many times
                if failure_tracker.should_skip(&peer.peer_id) {
                    debug!(
                        target: "vision_node::p2p::recovery",
                        "[PEER_RECOVERY] Skipping peer {} (too many recent failures)",
                        peer.ebid
                    );
                    continue;
                }

                // Skip if no IP address
                if peer.last_ip.is_empty() {
                    continue;
                }

                let address = format!("{}:{}", peer.last_ip, peer.last_port);

                debug!(
                    target: "vision_node::p2p::recovery",
                    "[PEER_RECOVERY] Attempting connection to EBID: {} @ {}",
                    peer.ebid,
                    address
                );

                attempted += 1;

                // Attempt connection
                let p2p = Arc::clone(&*crate::P2P_MANAGER);
                match p2p.connect_to_peer(address.clone()).await {
                    Ok(_) => {
                        connected += 1;
                        info!(
                            target: "vision_node::p2p::recovery",
                            "[PEER_RECOVERY] ✅ Reconnected to EBID: {} (uptime: {:.2})",
                            peer.ebid,
                            peer.uptime_score
                        );

                        // Update constellation memory with successful connection
                        {
                            let mut memory = crate::CONSTELLATION_MEMORY.lock();
                            let _now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();

                            memory.update_from_handshake(
                                peer.peer_id.clone(),
                                peer.ebid.clone(),
                                peer.last_ip.clone(),
                                peer.last_port,
                                peer.http_api_port,
                                peer.is_guardian_candidate,
                                peer.is_guardian_candidate,
                            );
                        }
                    }
                    Err(e) => {
                        debug!(
                            target: "vision_node::p2p::recovery",
                            "[PEER_RECOVERY] Failed to connect to EBID: {} - {}",
                            peer.ebid,
                            e
                        );

                        // Record failure
                        failure_tracker.record_failure(&peer.peer_id);

                        // Update constellation memory with timestamp
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        {
                            let mut memory = crate::CONSTELLATION_MEMORY.lock();
                            memory.record_failure(&peer.peer_id, now);
                        }
                    }
                }

                // Small delay between attempts
                sleep(Duration::from_millis(200)).await;

                // ⭐ Change 8: Stop if we've reached target peer count
                let current_peers = crate::PEER_MANAGER.connected_validated_count().await;
                if current_peers >= MAX_TARGET_PEERS {
                    info!(
                        target: "vision_node::p2p::recovery",
                        "[PEER_RECOVERY] Target peer count reached: {}",
                        current_peers
                    );
                    break;
                }
            }

            info!(
                target: "vision_node::p2p::recovery",
                "[PEER_RECOVERY] Recovery attempt complete: {}/{} successful reconnections (total peers: {})",
                connected,
                attempted,
                crate::PEER_MANAGER.connected_validated_count().await
            );
        } else {
            debug!(
                target: "vision_node::p2p::recovery",
                "[PEER_RECOVERY] Peer count healthy: {} (threshold: {})",
                live_peers,
                min_peer_count
            );
        }
    }
}
