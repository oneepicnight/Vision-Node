//! P2P Routing Helpers
//!
//! Smart peer selection for broadcasting and routing with anchor preference

use crate::p2p::peer_store::VisionPeer;

/// Choose peers for broadcasting, prioritizing anchors
///
/// Strategy:
/// 1. Always choose anchors first (is_anchor=true)
/// 2. Fill remaining slots with regular peers
/// 3. This ensures outbound-only nodes get their view from backbone
///
/// # Arguments
/// * `peers` - List of all available peers
/// * `max` - Maximum number of peers to select
///
/// # Returns
/// * Vec of selected peers (anchors first, then regular peers)
pub fn choose_broadcast_peers(peers: &[VisionPeer], max: usize) -> Vec<VisionPeer> {
    let mut anchors: Vec<_> = peers.iter().filter(|p| p.is_anchor).cloned().collect();

    let mut normals: Vec<_> = peers.iter().filter(|p| !p.is_anchor).cloned().collect();

    // Take up to max anchors
    anchors.truncate(max);

    // Fill remaining slots with normal peers
    let remaining = max.saturating_sub(anchors.len());
    normals.truncate(remaining);

    // Combine: anchors first, then normals
    anchors.extend(normals);
    anchors
}

/// Choose best peers for sync operations
///
/// Prefers anchors for sync since they're most likely to have correct chain
pub fn choose_sync_peers(peers: &[VisionPeer], max: usize) -> Vec<VisionPeer> {
    // Same strategy as broadcast: anchors first
    choose_broadcast_peers(peers, max)
}

/// Get anchor-only peers
pub fn get_anchor_peers(peers: &[VisionPeer]) -> Vec<VisionPeer> {
    peers.iter().filter(|p| p.is_anchor).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p2p::peer_store::{PeerTier, PeerTrustLevel, VisionPeer};

    fn make_test_peer(id: &str, is_anchor: bool) -> VisionPeer {
        let now = chrono::Utc::now().timestamp() as u64;
        VisionPeer {
            node_id: id.to_string(),
            node_tag: format!("TEST-{}", id),
            public_key: String::new(),
            vision_address: format!("vision://{}@test", id),
            admission_ticket_fingerprint: String::new(),
            role: if is_anchor {
                "anchor".to_string()
            } else {
                "miner".to_string()
            },
            last_seen: now as i64,
            trusted: false,
            mood: None,
            ip_address: Some(format!("192.168.1.{}", id.parse::<u8>().unwrap_or(1))),
            health_score: 80,
            last_success: now,
            last_failure: 0,
            fail_count: 0,
            is_seed: false,
            is_anchor,
            connection_status: "connected".to_string(),
            last_rtt_ms: Some(50),
            avg_rtt_ms: Some(50),
            latency_bucket: None,
            reliability_score: 0.9,
            success_count: 100,
            region: None,
            trust_level: PeerTrustLevel::Normal,
            reputation: 75.0,
            misbehavior_score: 0.0,
            graylisted_until: None,
            banned_until: None,
            total_invalid_msgs: 0,
            total_protocol_violations: 0,
            total_spam_events: 0,
            route_uses: 0,
            route_successes: 0,
            route_failures: 0,
            avg_delivery_ms: None,
            peer_tier: PeerTier::Hot,
            last_promotion: None,
            public_reachable: is_anchor,
        }
    }

    #[test]
    fn test_choose_broadcast_peers_prioritizes_anchors() {
        let peers = vec![
            make_test_peer("1", false), // regular miner
            make_test_peer("2", true),  // anchor
            make_test_peer("3", false), // regular miner
            make_test_peer("4", true),  // anchor
            make_test_peer("5", false), // regular miner
        ];

        let selected = choose_broadcast_peers(&peers, 3);

        assert_eq!(selected.len(), 3);
        // First two should be anchors
        assert!(selected[0].is_anchor);
        assert!(selected[1].is_anchor);
        // Third should be a regular miner
        assert!(!selected[2].is_anchor);
    }

    #[test]
    fn test_choose_broadcast_all_anchors() {
        let peers = vec![
            make_test_peer("1", true),
            make_test_peer("2", true),
            make_test_peer("3", true),
        ];

        let selected = choose_broadcast_peers(&peers, 5);

        assert_eq!(selected.len(), 3);
        assert!(selected.iter().all(|p| p.is_anchor));
    }

    #[test]
    fn test_choose_broadcast_no_anchors() {
        let peers = vec![
            make_test_peer("1", false),
            make_test_peer("2", false),
            make_test_peer("3", false),
        ];

        let selected = choose_broadcast_peers(&peers, 2);

        assert_eq!(selected.len(), 2);
        assert!(selected.iter().all(|p| !p.is_anchor));
    }

    #[test]
    fn test_get_anchor_peers_filters_correctly() {
        let peers = vec![
            make_test_peer("1", false),
            make_test_peer("2", true),
            make_test_peer("3", false),
            make_test_peer("4", true),
        ];

        let anchors = get_anchor_peers(&peers);

        assert_eq!(anchors.len(), 2);
        assert!(anchors.iter().all(|p| p.is_anchor));
    }
}
