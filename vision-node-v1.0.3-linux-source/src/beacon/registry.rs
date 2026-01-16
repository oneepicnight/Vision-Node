//! Beacon Peer Registry
//!
//! In-memory registry of constellation nodes that have registered with the Guardian beacon.
//! This enables P2P peer discovery and network bootstrapping.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use url::{Host, Url};

/// Check if a URL string contains an IPv4 address
pub fn is_ipv4_url(url_str: &str) -> bool {
    if let Ok(parsed) = Url::parse(url_str) {
        match parsed.host() {
            Some(Host::Ipv4(_)) => true,
            _ => false,
        }
    } else {
        false
    }
}

/// Check if a host string is an IPv4 address
pub fn is_ipv4_host(host: &str) -> bool {
    // Quick path: IPv6 hostnames always contain ':'
    if host.contains(':') {
        return false;
    }
    host.parse::<std::net::Ipv4Addr>().is_ok()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BeaconPeer {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "nodeTag")]
    pub node_tag: Option<String>,
    #[serde(rename = "p2p_host")]
    pub ip: String,
    #[serde(rename = "p2p_port")]
    pub p2p_port: u16,
    #[serde(rename = "http_host")]
    pub http_host: Option<String>,
    pub region: Option<String>,
    pub capabilities: Vec<String>,
    #[serde(rename = "lastSeen")]
    pub last_seen: i64, // unix timestamp
    #[serde(rename = "network_id")]
    pub network_id: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "statusTier")]
    pub status_tier: Option<String>,
    #[serde(rename = "vision_address")]
    pub vision_address: Option<String>,
    /// Whether this node is an anchor (accepts inbound connections)
    #[serde(rename = "isAnchor", default)]
    pub is_anchor: bool,
    /// Whether this node is publicly reachable (verified by Guardian)
    #[serde(rename = "publicReachable", default)]
    pub public_reachable: bool,
    /// Last time reachability was tested (unix timestamp)
    #[serde(
        rename = "lastReachabilityCheck",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_reachability_check: Option<i64>,
}

static BEACON_PEERS: Lazy<RwLock<HashMap<String, BeaconPeer>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register or update a peer in the beacon registry
pub fn register_peer(peer: BeaconPeer) -> usize {
    let mut map = BEACON_PEERS.write().expect("BEACON_PEERS poisoned");
    map.insert(peer.node_id.clone(), peer);
    map.len()
}

/// Get all registered peers, filtered to IPv4-only
/// NOTE: IPv4-only P2P for initial testnet ignition
pub fn get_peers() -> Vec<BeaconPeer> {
    let map = BEACON_PEERS.read().expect("BEACON_PEERS poisoned");

    // Filter to IPv4 only, then sort by last_seen (most recent first)
    let mut peers: Vec<BeaconPeer> = map
        .values()
        .filter(|peer| {
            // Only include IPv4 peers
            peer.ip
                .parse::<std::net::IpAddr>()
                .map(|ip| ip.is_ipv4())
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    // Sort by most recent heartbeat
    peers.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

    peers
}

/// Get peer count
pub fn peer_count() -> usize {
    let map = BEACON_PEERS.read().expect("BEACON_PEERS poisoned");
    map.len()
}

/// Remove stale peers (last_seen > threshold)
pub fn prune_stale_peers(max_age_secs: i64) -> usize {
    use chrono::Utc;
    let now = Utc::now().timestamp();

    let mut map = BEACON_PEERS.write().expect("BEACON_PEERS poisoned");
    let before_count = map.len();

    map.retain(|_, peer| {
        let age = now - peer.last_seen;
        age < max_age_secs
    });

    let removed = before_count - map.len();
    if removed > 0 {
        tracing::info!(
            "[BEACON] Pruned {} stale peers (older than {}s)",
            removed,
            max_age_secs
        );
    }
    removed
}

/// Purge all IPv6 peers from the registry (IPv4-only enforcement)
pub fn purge_ipv6_peers() -> usize {
    let mut map = BEACON_PEERS.write().expect("BEACON_PEERS poisoned");
    let before_count = map.len();

    map.retain(|_, peer| {
        if let Ok(ip_addr) = peer.ip.parse::<std::net::IpAddr>() {
            if ip_addr.is_ipv6() {
                tracing::info!(
                    "[BEACON] 🚫 Purging IPv6 peer: {} at [{}]:{}",
                    peer.node_tag.as_deref().unwrap_or(&peer.node_id),
                    peer.ip,
                    peer.p2p_port
                );
                return false; // Remove this peer
            }
        }
        true // Keep non-IPv6 peers
    });

    let removed = before_count - map.len();
    if removed > 0 {
        tracing::info!(
            "[BEACON] ✅ Purged {} IPv6 peers (IPv4-only policy)",
            removed
        );
    }
    removed
}

/// Test if an anchor node is publicly reachable via TCP connection
pub async fn test_peer_reachability(node_id: &str, timeout_ms: u64) {
    use chrono::Utc;
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let now = Utc::now().timestamp();

    // Get peer info
    let (ip, port) = {
        let map = BEACON_PEERS.read().expect("BEACON_PEERS poisoned");
        match map.get(node_id) {
            Some(peer) if peer.is_anchor => (peer.ip.clone(), peer.p2p_port),
            Some(_peer) => {
                // Not an anchor, skip test
                return;
            }
            None => {
                tracing::warn!(
                    "[BEACON] Cannot test reachability for unknown node: {}",
                    node_id
                );
                return;
            }
        }
    };

    // Parse address
    let addr_str = format!("{}:{}", ip, port);
    let addr = match addr_str.parse::<std::net::SocketAddr>() {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!(
                "[BEACON] Invalid address for {}: {} - {}",
                node_id,
                addr_str,
                e
            );
            // Update as unreachable
            let mut map = BEACON_PEERS.write().expect("BEACON_PEERS poisoned");
            if let Some(peer) = map.get_mut(node_id) {
                peer.public_reachable = false;
                peer.last_reachability_check = Some(now);
            }
            return;
        }
    };

    // Attempt TCP connection
    let result = timeout(Duration::from_millis(timeout_ms), TcpStream::connect(addr)).await;

    let reachable = matches!(result, Ok(Ok(_)));

    // Update registry
    let mut map = BEACON_PEERS.write().expect("BEACON_PEERS poisoned");
    if let Some(peer) = map.get_mut(node_id) {
        peer.public_reachable = reachable;
        peer.last_reachability_check = Some(now);

        if reachable {
            tracing::info!(
                "[BEACON] ✅ Anchor {} at {}:{} is publicly reachable",
                node_id,
                ip,
                port
            );
        } else {
            tracing::warn!(
                "[BEACON] ❌ Anchor {} at {}:{} is NOT reachable",
                node_id,
                ip,
                port
            );
        }
    }
}
