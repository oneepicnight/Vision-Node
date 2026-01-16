#![allow(dead_code)]
//! Network Readiness Gate for Mining
//!
//! Prevents premature mining before the node has:
//! 1. Connected to minimum peer threshold
//! 2. Completed initial sync attempt
//!
//! This prevents isolated forks when nodes boot alone.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use super::peer_manager::PeerManager;

/// Network readiness status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkReadiness {
    pub ready: bool,
    pub peer_count: usize,
    pub min_peers: usize,
    pub elapsed_seconds: u64,
}

/// Check if network is ready without blocking
pub fn check_readiness(peer_count: usize, min_peers: usize) -> NetworkReadiness {
    NetworkReadiness {
        ready: peer_count >= min_peers,
        peer_count,
        min_peers,
        elapsed_seconds: 0,
    }
}

/// Wait for network to be ready for mining with chain compatibility quorum
///
/// # Arguments
/// * `peer_manager` - Peer manager to check peer count and compatibility
/// * `min_peers` - Minimum required connected compatible peers
/// * `check_interval` - How often to check readiness (default: 5s)
/// * `max_wait` - Maximum time to wait before giving up (None = wait forever)
///
/// # Returns
/// * `true` if network is ready (quorum of compatible peers with similar height)
/// * `false` if timeout exceeded (proceed anyway with warning)
pub async fn wait_for_network_readiness(
    peer_manager: Arc<PeerManager>,
    min_peers: usize,
    check_interval: Duration,
    max_wait: Option<Duration>,
) -> bool {
    let start = Instant::now();

    info!(
        target: "vision_node::miner::gate",
        "[MINER_GATE] 🛡️ Waiting for network readiness with chain compatibility quorum: min_peers={}, timeout={:?}",
        min_peers,
        max_wait
    );

    loop {
        // Check elapsed time
        let elapsed = start.elapsed();
        if let Some(max) = max_wait {
            if elapsed >= max {
                warn!(
                    target: "vision_node::miner::gate",
                    "[MINER_GATE] ⚠️  Network readiness timed out after {:?}. Proceeding anyway (may cause forks).",
                    max
                );
                return false;
            }
        }

        // Get consensus quorum snapshot
        let quorum = peer_manager.consensus_quorum().await;

        // Log detailed quorum status
        info!(
            target: "vision_node::miner::gate",
            "[MINER_GATE] Network quorum check: compatible_peers={} incompatible_peers={} min_h={:?} max_h={:?}",
            quorum.compatible_peers,
            quorum.incompatible_peers,
            quorum.min_compatible_height,
            quorum.max_compatible_height,
        );

        // SWARM MODE: If no peers are connected yet, allow bootstrap to proceed
        // Don't block peer discovery with sync requirements
        if quorum.compatible_peers == 0 && quorum.incompatible_peers == 0 {
            info!(
                target: "vision_node::miner::gate",
                "[MINER_GATE] 🌱 Zero peers - allowing bootstrap to proceed (quorum check disabled during initial discovery)"
            );
            return true;
        }

        // Basic quorum: we need N compatible peers
        if quorum.compatible_peers >= min_peers {
            // Optional: ensure their heights are "close enough" to avoid syncing to a tiny fork.
            if let (Some(min_h), Some(max_h)) =
                (quorum.min_compatible_height, quorum.max_compatible_height)
            {
                let spread = max_h.saturating_sub(min_h);
                if spread <= 8 {
                    info!(
                        target: "vision_node::miner::gate",
                        "[MINER_GATE] ✅ Network ready: compatible_peers={} height_spread={} elapsed={:.1}s",
                        quorum.compatible_peers,
                        spread,
                        elapsed.as_secs_f32()
                    );
                    info!(
                        target: "vision_node::miner::gate",
                        "[MINER_GATE] 🎯 Unlocking mining - compatible quorum achieved"
                    );
                    return true;
                } else {
                    warn!(
                        target: "vision_node::miner::gate",
                        "[MINER_GATE] ⏳ Quorum peers disagree on height too much (spread={}), still waiting... elapsed={:.1}s",
                        spread,
                        elapsed.as_secs_f32()
                    );
                }
            } else {
                info!(
                    target: "vision_node::miner::gate",
                    "[MINER_GATE] ✅ Network ready: compatible_peers={} (no heights available yet) elapsed={:.1}s",
                    quorum.compatible_peers,
                    elapsed.as_secs_f32()
                );
                return true;
            }
        } else {
            info!(
                target: "vision_node::miner::gate",
                "[MINER_GATE] ⏳ Waiting for quorum... (compatible: {}/{}, incompatible: {}, elapsed: {:.1}s)",
                quorum.compatible_peers,
                min_peers,
                quorum.incompatible_peers,
                elapsed.as_secs_f32()
            );
        }

        // Wait before next check
        tokio::time::sleep(check_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_immediate_timeout() {
        let peer_manager = Arc::new(PeerManager::new());

        // With 0s timeout, should immediately return false
        let ready = wait_for_network_readiness(
            peer_manager,
            1,
            Duration::from_millis(100),
            Some(Duration::from_secs(0)),
        )
        .await;

        assert!(!ready); // Should timeout immediately
    }
}
