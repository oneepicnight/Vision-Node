#![allow(dead_code)]
//! Retry Worker - Background task to retry unhealthy peers
//!
//! This module implements the automatic retry logic that periodically checks
//! for peers with low health scores (health_score <= 0), and attempts
//! to reconnect to them.
//!
//! The worker runs every 30 seconds and processes all peers with negative health.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

use crate::p2p::backoff::current_time;
use crate::p2p::peer_store::PeerStore;

/// Retry interval - how often to check for unhealthy peers ready for retry
const RETRY_CHECK_INTERVAL_SECS: u64 = 30;

/// Retry worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryWorkerConfig {
    pub check_interval_secs: u64,
    pub max_retries_per_cycle: usize,
}

impl Default for RetryWorkerConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: RETRY_CHECK_INTERVAL_SECS,
            max_retries_per_cycle: 10,
        }
    }
}

/// Retry policy - determines when to retry failed connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub base_backoff_secs: u64,
    pub max_backoff_secs: u64,
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_backoff_secs: 10,
            max_backoff_secs: 3600,
            max_attempts: 10,
        }
    }
}

/// Reset retry counters for a peer when valid advertised IP/port is received
///
/// Called after handshake when peer provides advertised_ip and advertised_port.
/// This gives the peer a fresh chance to reconnect via their advertised address.
pub async fn reset_peer_retry(peer_store: Arc<PeerStore>, peer_id: &str) {
    if let Some(mut peer) = peer_store.get(peer_id) {
        info!(
            "[P2P RETRY] Resetting retry counters for peer {} (received valid advertised address)",
            peer.node_tag
        );
        peer.fail_count = 0;
        peer.last_failure = 0;
        let _ = peer_store.save(&peer);
    }
}

/// Retry unhealthy peers (health_score <= 0) that haven't been attempted recently
///
/// This function:
/// 1. Loads all peers from the peer book
/// 2. Filters for peers with health_score <= 0
/// 3. Checks if enough time has passed since last_failure (uses exponential backoff)
/// 4. Returns the list of peers to attempt connection to
///
/// # Arguments
/// * `peer_store` - The peer book containing all known peers
///
/// # Returns
/// Vector of peer addresses (vision addresses or IP addresses) to attempt connection
pub async fn retry_unhealthy_peers(peer_store: Arc<PeerStore>) -> Vec<String> {
    let now = current_time();
    let mut retry_peers = Vec::new();

    // Load all peers
    let peers = peer_store.get_all();

    for peer in peers {
        // Check if peer is unhealthy (negative health)
        if peer.health_score <= 0 {
            // Calculate backoff time based on fail count
            let backoff_secs = crate::p2p::backoff::backoff(peer.fail_count);
            let retry_after = peer.last_failure + backoff_secs;

            if now >= retry_after {
                let addr = peer
                    .ip_address
                    .clone()
                    .unwrap_or_else(|| peer.vision_address.clone());

                info!(
                    "[P2P RETRY] Retrying unhealthy peer {} (health={}, failures={})",
                    peer.node_tag, peer.health_score, peer.fail_count
                );

                retry_peers.push(addr);
            } else {
                let wait_time = retry_after.saturating_sub(now);
                debug!(
                    "[P2P RETRY] Peer {} waiting {}s before retry (health={})",
                    peer.node_tag, wait_time, peer.health_score
                );
            }
        }
    }

    if !retry_peers.is_empty() {
        info!(
            "[P2P RETRY] Found {} unhealthy peers ready for retry",
            retry_peers.len()
        );
    }

    retry_peers
}

/// Spawn retry worker - compatibility function for main.rs
///
/// This spawns a tokio task that runs indefinitely, checking every interval
/// for unhealthy peers that need to be retried.
///
/// # Arguments
/// * `interval` - How often to check for peers to retry
/// * `peer_store` - The peer store to monitor for unhealthy peers
pub async fn spawn_retry_worker(interval: Duration, peer_store: Arc<PeerStore>) {
    info!(
        "[P2P RETRY] Starting retry worker (checks every {:?})",
        interval
    );

    loop {
        sleep(interval).await;

        let retry_addrs = retry_unhealthy_peers(peer_store.clone()).await;

        if !retry_addrs.is_empty() {
            info!(
                "[P2P RETRY] Found {} peers ready for retry",
                retry_addrs.len()
            );
            // Note: Connection attempts are handled by bootstrap/connection manager
            // This just identifies which peers are ready to retry
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::peer_store::VisionPeer;

    #[tokio::test]
    async fn test_retry_logic() {
        // Create temporary test database
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_db = sled::open(temp_dir.path()).unwrap();
        let peer_store = Arc::new(PeerStore::new(&temp_db).unwrap());

        // Create an unhealthy peer ready for retry
        let mut peer = VisionPeer::new(
            "test-node-1".to_string(),
            "VNODE-TEST-1".to_string(),
            "pubkey123".to_string(),
            "vision://VNODE-TEST-1@abc123".to_string(),
            None,
            "constellation".to_string(),
        );

        let now = current_time();
        peer.health_score = -10; // Unhealthy
        peer.fail_count = 2;
        peer.last_failure = now.saturating_sub(100); // Failed 100s ago
        peer_store.save(&peer).unwrap();

        // Run retry logic
        let retry_addrs = retry_unhealthy_peers(peer_store.clone()).await;

        // Should find 1 peer to retry
        assert_eq!(retry_addrs.len(), 1);
    }
}
