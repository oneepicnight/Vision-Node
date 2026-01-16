//! Anchor Node Election System - Automatic election of stable relay points
//!
//! Anchors are peers that naturally emerge with:
//! - Static/public IPs
//! - Low latency
//! - High uptime
//! - Strong reputation
//!
//! They serve as:
//! - Fallback bootstrap points
//! - Relay candidates for NAT traversal
//! - Network routing memory
//!
//! This removes Guardian dependency while maintaining authority
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::p2p::peer_store::PeerStore;

/// Minimum reputation score to become anchor
pub const ANCHOR_MIN_REPUTATION: i32 = 750;

/// Minimum uptime (in seconds) to become anchor (12 hours)
pub const ANCHOR_MIN_UPTIME_SECS: u64 = 43200;

/// Maximum latency (ms) for anchor candidate
pub const ANCHOR_MAX_LATENCY_MS: u32 = 150;

/// Maximum number of active anchors
pub const MAX_ANCHORS: usize = 10;

/// Anchor status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorStatus {
    Candidate, // Eligible but not elected
    Active,    // Currently serving as anchor
    Demoted,   // Was anchor, now demoted due to poor performance
}

/// Anchor node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorNode {
    /// Node identifier
    pub node_id: String,

    /// Node tag
    pub node_tag: String,

    /// Vision address
    pub vision_address: String,

    /// Public IP address
    pub ip_address: String,

    /// Current anchor status
    pub status: AnchorStatus,

    /// Reputation score at election
    pub reputation_score: i32,

    /// Average latency (ms)
    pub avg_latency_ms: u32,

    /// Estimated uptime (seconds)
    pub uptime_secs: u64,

    /// Number of successful relays performed
    pub relay_count: u64,

    /// When this node became anchor
    pub elected_at: u64,

    /// Last anchor health check
    pub last_checked: u64,
}

impl AnchorNode {
    /// Check if anchor should be demoted
    pub fn should_demote(&self) -> bool {
        // Demote if latency increased significantly
        if self.avg_latency_ms > ANCHOR_MAX_LATENCY_MS * 2 {
            return true;
        }

        // Demote if not checked recently (offline?)
        let now = current_time();
        if now - self.last_checked > 3600 {
            return true;
        }

        false
    }
}

/// Anchor registry - manages elected anchor nodes
pub struct AnchorRegistry {
    /// Active anchor nodes
    anchors: Arc<RwLock<HashMap<String, AnchorNode>>>,
}

impl AnchorRegistry {
    pub fn new() -> Self {
        Self {
            anchors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get all active anchors
    pub async fn get_active_anchors(&self) -> Vec<AnchorNode> {
        let anchors = self.anchors.read().await;
        anchors
            .values()
            .filter(|a| a.status == AnchorStatus::Active)
            .cloned()
            .collect()
    }

    /// Add new anchor
    pub async fn elect_anchor(&self, anchor: AnchorNode) {
        let mut anchors = self.anchors.write().await;

        // Check if we're at capacity
        let active_count = anchors
            .values()
            .filter(|a| a.status == AnchorStatus::Active)
            .count();

        if active_count >= MAX_ANCHORS {
            warn!(
                "[ANCHOR] Cannot elect {} - at max capacity ({})",
                anchor.node_tag, MAX_ANCHORS
            );
            return;
        }

        info!(
            "[ANCHOR] 🔗 Elected stable anchor node: {} ({}) - latency: {}ms, score: {}",
            anchor.node_tag, anchor.ip_address, anchor.avg_latency_ms, anchor.reputation_score
        );

        anchors.insert(anchor.node_id.clone(), anchor);
    }

    /// Remove anchor
    pub async fn demote_anchor(&self, node_id: &str) {
        let mut anchors = self.anchors.write().await;

        if let Some(anchor) = anchors.get_mut(node_id) {
            info!(
                "[ANCHOR] Demoting anchor: {} (poor performance)",
                anchor.node_tag
            );
            anchor.status = AnchorStatus::Demoted;
        }
    }

    /// Update anchor health check
    pub async fn update_anchor_health(&self, node_id: &str, latency_ms: u32) {
        let mut anchors = self.anchors.write().await;

        if let Some(anchor) = anchors.get_mut(node_id) {
            anchor.avg_latency_ms =
                ((anchor.avg_latency_ms as f32 * 0.7) + (latency_ms as f32 * 0.3)) as u32;
            anchor.last_checked = current_time();

            if anchor.should_demote() {
                anchor.status = AnchorStatus::Demoted;
                info!(
                    "[ANCHOR] Auto-demoted anchor {} due to degraded performance",
                    anchor.node_tag
                );
            }
        }
    }

    /// Get random active anchor for relay
    pub async fn get_random_anchor(&self) -> Option<AnchorNode> {
        let anchors = self.anchors.read().await;
        let active: Vec<_> = anchors
            .values()
            .filter(|a| a.status == AnchorStatus::Active)
            .collect();

        if active.is_empty() {
            return None;
        }

        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        active.choose(&mut rng).map(|a| (*a).clone())
    }
}

/// Evaluate peers for anchor candidacy
pub async fn evaluate_anchor_candidates(peer_store: Arc<PeerStore>) -> Vec<AnchorNode> {
    let all_peers = peer_store.get_all();
    let mut candidates = Vec::new();

    for peer in all_peers {
        // Must be healthy (health_score > 50)
        if peer.health_score <= 50 {
            continue;
        }

        // Must have public IP
        let ip_addr = match &peer.ip_address {
            Some(ip) => ip,
            None => continue,
        };

        // Handle legacy entries without port
        let ip_with_port = if ip_addr.contains(':') {
            ip_addr.clone()
        } else {
            format!("{}:7072", ip_addr)
        };

        // Check if IP is public (not private/loopback)
        if let Ok(sock_addr) = ip_with_port.parse::<SocketAddr>() {
            let ip = sock_addr.ip();
            if !is_public_ip(&ip) {
                continue;
            }
        } else {
            continue;
        }

        // Must have good health score (proxy for reputation)
        if peer.health_score < 75 {
            continue;
        }

        // Must not have recent failures
        if peer.fail_count > 2 {
            continue;
        }

        // Calculate uptime estimate
        let now = current_time();
        let uptime = if peer.last_success > 0 {
            now - peer.last_success
        } else {
            0
        };

        // Must have sufficient uptime
        if uptime < ANCHOR_MIN_UPTIME_SECS {
            continue;
        }

        // Create anchor candidate
        let anchor = AnchorNode {
            node_id: peer.node_id.clone(),
            node_tag: peer.node_tag.clone(),
            vision_address: peer.vision_address.clone(),
            ip_address: ip_with_port,
            status: AnchorStatus::Candidate,
            reputation_score: peer.health_score,
            avg_latency_ms: 100, // Will be measured
            uptime_secs: uptime,
            relay_count: 0,
            elected_at: 0,
            last_checked: now,
        };

        candidates.push(anchor);
    }

    // Sort by reputation score (descending)
    candidates.sort_by(|a, b| b.reputation_score.cmp(&a.reputation_score));

    if !candidates.is_empty() {
        info!("[ANCHOR] Found {} anchor candidates", candidates.len());
    }

    candidates
}

/// Check if IP address is publicly routable
fn is_public_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            !ipv4.is_private()
                && !ipv4.is_loopback()
                && !ipv4.is_link_local()
                && !ipv4.is_unspecified()
        }
        std::net::IpAddr::V6(_) => false, // IPv4 only for now
    }
}

fn current_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_ip_detection() {
        use std::net::IpAddr;

        let public: IpAddr = "8.8.8.8".parse().unwrap();
        let private: IpAddr = "192.168.1.1".parse().unwrap();
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();

        assert!(is_public_ip(&public));
        assert!(!is_public_ip(&private));
        assert!(!is_public_ip(&loopback));
    }

    #[tokio::test]
    async fn test_anchor_registry() {
        let registry = AnchorRegistry::new();

        let anchor = AnchorNode {
            node_id: "test-1".to_string(),
            node_tag: "ANCHOR-1".to_string(),
            vision_address: "vision://test".to_string(),
            ip_address: "1.2.3.4:7072".to_string(),
            status: AnchorStatus::Active,
            reputation_score: 800,
            avg_latency_ms: 50,
            uptime_secs: 86400,
            relay_count: 0,
            elected_at: current_time(),
            last_checked: current_time(),
        };

        registry.elect_anchor(anchor).await;

        let active = registry.get_active_anchors().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].node_tag, "ANCHOR-1");
    }
}
