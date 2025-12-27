#![allow(dead_code)]
//! Peer Gossip Protocol - Exponential peer discovery through friend list sharing
//!
//! When Node A connects to Node B:
//! 1. Node B shares its entire peer list (verified peers prioritized)
//! 2. Node A immediately attempts handshakes with C, D, E...
//! 3. Result: Exponential connectivity growth
//!
//! This transforms peer discovery from:
//!   find peer -> connect -> wait -> find more
//! Into:
//!   find 1 → connect → BOOM (instant peer explosion)

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::p2p::peer_store::{PeerStore, VisionPeer};
use crate::p2p::swarm_intelligence::ReputationTier;

/// Maximum peers to include in gossip message (prevent bandwidth abuse)
/// Fix 5: Reduced from 50 to 100 cap on processing (not sending)
pub const MAX_GOSSIP_PEERS: usize = 50;

/// Only gossip peers that meet these criteria
pub const MIN_GOSSIP_TIER: ReputationTier = ReputationTier::Fair;

/// Gossip message - exchange peer lists after handshake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerGossipMessage {
    /// Sender's node tag
    pub from_node: String,

    /// List of known peers
    pub peers: Vec<GossipPeerInfo>,

    /// Timestamp of gossip
    pub timestamp: u64,
}

/// Peer information shared in gossip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPeerInfo {
    /// Node identifier
    pub node_id: String,

    /// Node tag (human readable)
    pub node_tag: String,

    /// Vision address
    pub vision_address: String,

    /// IP address for connection (without port, deprecated - use ip_address for backwards compat)
    pub ip_address: Option<String>,

    /// P2P port for this peer
    #[serde(default = "default_gossip_port")]
    pub port: u16,

    /// Peer role (constellation/guardian)
    pub role: String,

    /// Last seen timestamp
    pub last_seen: i64,

    /// Reputation tier (elite, excellent, good, fair)
    pub reputation_tier: String,

    /// Is verified/stable peer
    pub is_verified: bool,

    // Phase 10: Reachability Advertisement
    /// Is this peer publicly reachable? (Only advertise if true)
    #[serde(default)]
    pub reachable: bool,

    /// NAT type ("Open", "Restricted", etc.)
    #[serde(default)]
    pub nat_type: Option<String>,

    /// Geographic region
    #[serde(default)]
    pub region: Option<String>,
}

fn default_gossip_port() -> u16 {
    7072 // Default P2P port
}

impl From<&VisionPeer> for GossipPeerInfo {
    fn from(peer: &VisionPeer) -> Self {
        // Extract port from ip_address string (format: "IP:PORT")
        let port = peer
            .ip_address
            .as_ref()
            .and_then(|addr| {
                addr.rsplit_once(':')
                    .and_then(|(_, port_str)| port_str.parse::<u16>().ok())
            })
            .unwrap_or(7072); // Default to P2P port if no port specified

        Self {
            node_id: peer.node_id.clone(),
            node_tag: peer.node_tag.clone(),
            vision_address: peer.vision_address.clone(),
            ip_address: peer.ip_address.clone(),
            port,
            role: peer.role.clone(),
            last_seen: peer.last_seen,
            reputation_tier: "fair".to_string(), // Will be updated with actual reputation
            is_verified: peer.trusted,           // Use trusted flag instead of lifecycle_state
            reachable: false,                    // TODO: Update from actual reachability test
            nat_type: None,                      // TODO: Populate from peer NAT detection
            region: None,                        // TODO: Populate from GeoIP
        }
    }
}

/// Create gossip message from current peer store
///
/// PATCH 1: Always gossip some peers, even "leaf" ones
///
/// Prioritizes:
/// 1. All seed peers (always included, never filtered)
/// 2. All currently connected peers (even if leaf/not public)
/// 3. High reputation peers
/// 4. Recently seen peers
///
/// This ensures new nodes always learn about multiple peers, not just public anchors.
pub async fn create_gossip_message(
    peer_store: Arc<PeerStore>,
    our_node_tag: &str,
) -> Option<PeerGossipMessage> {
    let all_peers = peer_store.get_all();

    if all_peers.is_empty() {
        debug!("[GOSSIP] No peers to share in gossip");
        return None;
    }

    // PATCH 1: Include ALL connected peers and seeds, don't filter on public_reachable
    // This helps leaf nodes (miners behind NAT) still get gossiped
    let mut gossip_candidates: Vec<_> = all_peers
        .into_iter()
        .filter(|p| {
            // Only require that peer has a valid IP - don't filter on health or reachability
            // Seeds and connected peers are valuable regardless of metrics
            p.ip_address.is_some()
        })
        .collect();

    // Sort by priority: Seeds > Connected > Verified > Recently seen
    // This ensures seeds and active connections appear first
    gossip_candidates.sort_by(|a, b| {
        let a_priority = peer_priority(a);
        let b_priority = peer_priority(b);
        b_priority.cmp(&a_priority) // Descending
    });

    // Take a reasonable number (16+ peers to share)
    // Increased from MAX_GOSSIP_PEERS if needed to include more candidates
    let target_count = MAX_GOSSIP_PEERS.max(16);

    // Phase 10: Enrich gossip info with reachability data from PeerManager
    let mut peers_to_share: Vec<GossipPeerInfo> = Vec::new();
    for peer in gossip_candidates.into_iter().take(target_count) {
        // Fix 2: Never save/gossip private IPs
        if let Some(ref ip_addr) = peer.ip_address {
            if !crate::p2p::ip_filter::validate_ip_for_storage(ip_addr) {
                debug!(
                    "[GOSSIP] Filtering out peer {} with private IP: {}",
                    peer.node_tag, ip_addr
                );
                continue;
            }
        }

        let mut gossip_info = GossipPeerInfo::from(&peer);

        // Try to get reachability data from PeerManager if available
        // Extract EBID from node_id (format varies, could be "peer-xxx" or direct EBID)
        let ebid_to_check = if peer.node_id.starts_with("peer-") {
            peer.node_id.strip_prefix("peer-").unwrap_or(&peer.node_id)
        } else {
            &peer.node_id
        };

        if let Some((reachable, nat_type)) = crate::PEER_MANAGER
            .get_peer_reachability(ebid_to_check)
            .await
        {
            gossip_info.reachable = reachable;
            gossip_info.nat_type = Some(nat_type.clone());

            // Phase 10: Only advertise peers that are publicly reachable OR are seed peers
            // Seeds get exemption because they're bootstrap nodes
            if !reachable && peer.role != "seed" {
                debug!(
                    "[GOSSIP] Skipping unreachable peer {} in gossip (NAT: {})",
                    peer.node_tag, nat_type
                );
                continue;
            }
        }

        peers_to_share.push(gossip_info);
    }

    if peers_to_share.is_empty() {
        debug!("[GOSSIP] No eligible peers to share");
        return None;
    }

    info!(
        "[GOSSIP] Prepared gossip message with {} peers",
        peers_to_share.len()
    );

    Some(PeerGossipMessage {
        from_node: our_node_tag.to_string(),
        peers: peers_to_share,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// Calculate peer priority for gossip ranking
///
/// PATCH 1: Seeds and connected peers get ultra-high priority
fn peer_priority(peer: &VisionPeer) -> i32 {
    let mut priority = 0;

    // PATCH 1: Seed peers ALWAYS get highest priority (2000+ points)
    // We detect seeds by checking if they're marked as seed_peer or have special role
    if peer.role == "seed" || peer.node_tag.contains("SEED") {
        priority += 2000;
        debug!(
            target: "p2p::gossip_priority",
            peer = %peer.node_tag,
            "Seed peer gets ultra-high gossip priority (+2000)"
        );
    }

    // Currently connected peers get very high priority (1500+ points)
    if peer.connection_status == "connected" {
        priority += 1500;
        debug!(
            target: "p2p::gossip_priority",
            peer = %peer.node_tag,
            "Connected peer gets high gossip priority (+1500)"
        );
    }

    // Trusted peers get highest priority
    if peer.trusted {
        priority += 1000;
    }

    // Health score is secondary ranking metric
    priority += peer.health_score * 10;

    // Recently active peers get bonus
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let age = now.saturating_sub(peer.last_success);
    if age < 300 {
        // Active in last 5 minutes
        priority += 500;
    }

    // Recently seen bonus (within last hour)
    if peer.last_seen > 0 {
        let time_since_seen = now.saturating_sub(peer.last_seen as u64);
        if time_since_seen < 3600 {
            priority += 100;
        }
    }

    priority
}

/// Process received gossip message and extract new peer candidates
///
/// Returns: List of new peer addresses to attempt connection
pub async fn process_gossip_message(
    gossip: PeerGossipMessage,
    peer_store: Arc<PeerStore>,
    our_node_id: &str,
) -> Vec<String> {
    let mut new_peers = Vec::new();
    let mut existing_ids: HashSet<String> = peer_store
        .get_all()
        .into_iter()
        .map(|p| p.node_id)
        .collect();

    // Fix 5: Safety cap reduced from 256 to 100 to prevent peer explosion
    let received_len = gossip.peers.len();
    let capped_peers = if received_len > 100 {
        warn!(
            "[GOSSIP] Peer {} sent {} peers, capping to 100 for safety (Fix 5)",
            gossip.from_node, received_len
        );
        gossip.peers.into_iter().take(100).collect::<Vec<_>>()
    } else {
        gossip.peers
    };

    info!(
        "[GOSSIP] Received peer list from {} - received_len={} processing_count={}",
        gossip.from_node,
        received_len,
        capped_peers.len()
    );

    // Deduplicate by socket address before processing
    let mut seen_addrs: HashSet<String> = HashSet::new();
    let mut _skipped_dupe = 0;

    for peer_info in capped_peers {
        // Skip ourselves
        if peer_info.node_id == our_node_id {
            continue;
        }

        // Skip if we already know this peer
        if existing_ids.contains(&peer_info.node_id) {
            debug!("[GOSSIP] Skipping known peer: {}", peer_info.node_tag);
            continue;
        }

        // Extract IP address and construct full socket address with port
        let ip_only = match peer_info.ip_address {
            Some(addr) => {
                // If addr already has :port, strip it to get just IP
                addr.split(':').next().unwrap_or(&addr).to_string()
            }
            None => {
                debug!("[GOSSIP] Skipping peer without IP: {}", peer_info.node_tag);
                continue;
            }
        };

        // Fix 2: Never save private IPs from gossip
        if !crate::p2p::ip_filter::validate_ip_for_storage(&ip_only) {
            debug!(
                "[GOSSIP] Filtering out peer {} with private IP: {}",
                peer_info.node_tag, ip_only
            );
            continue;
        }

        // Construct full socket address with port from gossip
        let socket_addr = format!("{}:{}", ip_only, peer_info.port);

        // Deduplicate by socket address
        if !seen_addrs.insert(socket_addr.clone()) {
            _skipped_dupe += 1;
            continue;
        }

        // Add to peer store as New peer
        let mut new_peer = VisionPeer::new(
            peer_info.node_id.clone(),
            peer_info.node_tag.clone(),
            String::new(),
            peer_info.vision_address.clone(),
            None,
            peer_info.role.clone(),
        );
        new_peer.last_seen = peer_info.last_seen;
        new_peer.ip_address = Some(socket_addr.clone());

        if let Err(e) = peer_store.save(&new_peer) {
            warn!("[GOSSIP] Failed to save peer {}: {}", peer_info.node_tag, e);
            continue;
        }

        existing_ids.insert(peer_info.node_id.clone());
        new_peers.push(socket_addr.clone());

        info!(
            "[GOSSIP] Discovered new peer from gossip: {} ({}) at {}",
            peer_info.node_tag, peer_info.node_id, socket_addr
        );
    }

    if !new_peers.is_empty() {
        info!(
            "[SWARM] 💥 BOOM! Discovered {} new peers from gossip - attempting connections",
            new_peers.len()
        );

        // 🌱 PURE SWARM: Auto-save discovered peers to seed_peers.json
        // This builds a self-growing peer book that persists across restarts
        tokio::task::spawn_blocking({
            let peers_to_save = new_peers.clone();
            move || {
                let mut config = crate::p2p::seed_peers::SeedPeerConfig::load();
                let added = config.add_peers(peers_to_save);

                if added > 0 {
                    if let Err(e) = config.save() {
                        tracing::warn!("[SWARM] Failed to save seed peers: {}", e);
                    } else {
                        tracing::info!(
                            "[SWARM] 💾 Saved {} new peers to seed_peers.json (total: {})",
                            added,
                            config.peers.len()
                        );
                    }
                }
            }
        });
    }

    new_peers
}

/// Spawn peer gossip loop - compatibility function for main.rs
pub async fn spawn_peer_gossip_loop(
    interval: std::time::Duration,
    peer_store: Arc<PeerStore>,
    our_node_tag: String,
) {
    use tokio::time::sleep;

    info!(
        "[GOSSIP] 🌐 Starting peer gossip loop (interval: {:?})",
        interval
    );

    loop {
        sleep(interval).await;

        // Create gossip message from current peer state
        if let Some(gossip_msg) = create_gossip_message(peer_store.clone(), &our_node_tag).await {
            debug!(
                "[GOSSIP] Created gossip message with {} peers to share",
                gossip_msg.peers.len()
            );
            // Note: Actual transmission happens via connection manager
            // This prepares the gossip data that will be sent during handshakes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gossip_peer_conversion() {
        let mut peer = VisionPeer::new(
            "test-1".to_string(),
            "VNODE-TEST-1".to_string(),
            "pubkey".to_string(),
            "vision://test".to_string(),
            None,
            "constellation".to_string(),
        );
        peer.trusted = true; // Verified = trusted

        let gossip_info = GossipPeerInfo::from(&peer);

        assert_eq!(gossip_info.node_tag, "VNODE-TEST-1");
        assert!(gossip_info.is_verified);
    }

    #[test]
    fn test_peer_priority() {
        let mut peer1 = VisionPeer::new(
            "test-1".to_string(),
            "VNODE-1".to_string(),
            "pk1".to_string(),
            "vision://test1".to_string(),
            None,
            "constellation".to_string(),
        );
        peer1.trusted = true; // Verified
        peer1.health_score = 80;

        let mut peer2 = VisionPeer::new(
            "test-2".to_string(),
            "VNODE-2".to_string(),
            "pk2".to_string(),
            "vision://test2".to_string(),
            None,
            "constellation".to_string(),
        );
        peer2.health_score = 90;

        assert!(peer_priority(&peer1) > peer_priority(&peer2)); // peer1 wins due to trusted
    }
}
