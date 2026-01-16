#![allow(dead_code)]
//! Beacon-based P2P Bootstrap
//!
//! Fetches peer list from Guardian beacon and initiates P2P connections

use serde::Deserialize;

/// Get constellation config for guardian/beacon access
fn get_constellation_config() -> crate::config::constellation::ConstellationConfig {
    // Load from file or environment
    crate::config::constellation::ConstellationConfig::load_or_create("constellation.json")
        .unwrap_or_else(|e| {
            tracing::debug!(
                "Failed to load constellation.json: {}, using env/default",
                e
            );
            crate::config::constellation::ConstellationConfig::from_env_or_default()
        })
}

/// Get beacon base URL from constellation config
/// Returns None only if beacon is explicitly disabled
fn beacon_base_url() -> Option<String> {
    let config = get_constellation_config();

    if !config.enable_beacon {
        return None;
    }

    Some(config.guardian_base_url)
}

/// Flexible beacon peer struct - accepts multiple field name formats
/// Supports camelCase, snake_case, and legacy aliases to prevent parsing failures
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BeaconPeerRecord {
    // Support both camelCase and snake_case and some legacy aliases
    #[serde(rename = "nodeId", alias = "node_id", alias = "id", default)]
    pub node_id: Option<String>,

    #[serde(rename = "nodeTag", alias = "node_tag", alias = "tag")]
    pub node_tag: String,

    #[serde(rename = "statusTier", alias = "status_tier", default)]
    pub status_tier: Option<String>,

    #[serde(rename = "network_id", alias = "network", default)]
    pub network_id: Option<String>,

    #[serde(rename = "p2p_host", alias = "host", default)]
    pub p2p_host: Option<String>,

    #[serde(rename = "p2p_port", alias = "port", default)]
    pub p2p_port: Option<u16>,

    #[serde(rename = "http_host", default)]
    pub http_host: Option<String>,

    #[serde(rename = "role", default)]
    pub role: Option<String>,

    #[serde(rename = "vision_address", default)]
    pub vision_address: Option<String>,

    #[serde(rename = "fingerprint", default)]
    pub fingerprint: Option<String>,

    #[serde(rename = "lastSeen", alias = "last_seen", default)]
    pub last_seen: Option<u64>,
}

impl BeaconPeerRecord {
    /// Convert peer to SocketAddr - returns None if IPv6 or invalid
    pub fn to_socket_addr(&self) -> Option<std::net::SocketAddr> {
        use std::net::{IpAddr, SocketAddr};

        let host = self
            .p2p_host
            .as_ref()
            .or(self.http_host.as_ref())
            .map(|h| {
                h.trim_start_matches("http://")
                    .trim_start_matches("https://")
            })?;

        let port = self.p2p_port.unwrap_or(7072);

        // Try parse as IP address
        if let Ok(ip_addr) = host.parse::<IpAddr>() {
            // Reject IPv6
            if ip_addr.is_ipv6() {
                tracing::debug!(
                    target: "vision_node::p2p::beacon_bootstrap",
                    "[BEACON_BOOTSTRAP] Rejecting IPv6 peer: {} at [{}]:{}",
                    self.node_tag, host, port
                );
                return None;
            }
            return Some(SocketAddr::new(ip_addr, port));
        }

        // For hostnames, try DNS resolution and filter IPv4
        use std::net::ToSocketAddrs;
        let addr_str = format!("{}:{}", host, port);
        if let Ok(addrs) = addr_str.to_socket_addrs() {
            // Return first IPv4 address found
            for addr in addrs {
                if addr.ip().is_ipv4() {
                    return Some(addr);
                }
            }
            tracing::debug!(
                target: "vision_node::p2p::beacon_bootstrap",
                "[BEACON_BOOTSTRAP] No IPv4 address resolved for: {} at {}:{}",
                self.node_tag, host, port
            );
        }

        None
    }
}

/// Internal bootstrap peer structure
#[derive(Debug, Clone)]
pub struct BeaconBootstrapPeer {
    pub node_id: String,
    pub node_tag: String,
    pub host: String,
    pub port: u16,
    pub role: Option<String>,
    pub status_tier: Option<String>,
    pub vision_address: Option<String>,
}

/// Phase 3: Sanitize beacon peer addresses - only allow valid IPv4, no localhost/loopback
fn is_valid_beacon_peer_addr(host: &str, port: u16) -> bool {
    use std::net::{SocketAddr, ToSocketAddrs};

    // Try to parse as SocketAddr first
    if let Ok(sock_addr) = format!("{}:{}", host, port).parse::<SocketAddr>() {
        return crate::p2p::is_valid_ipv4_endpoint(&sock_addr);
    }

    // Try DNS resolution for hostnames
    if let Ok(mut addrs) = format!("{}:{}", host, port).to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return crate::p2p::is_valid_ipv4_endpoint(&addr);
        }
    }

    false
}

/// Parse beacon peers from website JSON format with proper field mapping
/// Accepts multiple response shapes: array, {peers: []}, or {nodes: []}
pub fn parse_beacon_peers(
    body: &str,
    self_node_id: Option<&str>,
) -> Result<Vec<BeaconBootstrapPeer>, anyhow::Error> {
    // Tolerant parsing - accept multiple JSON shapes
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| anyhow::anyhow!("invalid JSON: {}", e))?;

    // Support three shapes:
    // 1) [ { ...peer... }, ... ]
    // 2) { "peers": [ { ...peer... } ], ... }
    // 3) { "nodes": [ { ...peer... } ], ... }
    let peers_value = if value.is_array() {
        value
    } else if let Some(peers) = value.get("peers") {
        peers.clone()
    } else if let Some(nodes) = value.get("nodes") {
        nodes.clone()
    } else {
        return Err(anyhow::anyhow!(
            "unexpected beacon response shape (no array/peers/nodes)"
        ));
    };

    let records: Vec<BeaconPeerRecord> = serde_json::from_value(peers_value)
        .map_err(|e| anyhow::anyhow!("failed to parse peers: {}", e))?;

    let total = records.len();
    tracing::info!(
        target: "vision_node::p2p::beacon_bootstrap",
        "[BEACON_BOOTSTRAP] Received {} peer records from beacon",
        total
    );

    // Filter and map into usable peers
    let mut usable: Vec<BeaconBootstrapPeer> = records
        .into_iter()
        .filter_map(|peer| {
            // Synthesize node_id from nodeTag if missing
            let node_id = if let Some(id) = peer.node_id.as_ref() {
                if id.trim().is_empty() {
                    // Empty node_id, synthesize from tag
                    tracing::warn!(
                        target: "vision_node::p2p::beacon_bootstrap",
                        "[BEACON_BOOTSTRAP] Beacon peer has empty nodeId, synthesizing from nodeTag={}",
                        peer.node_tag
                    );
                    format!("beacon-{}", peer.node_tag)
                } else {
                    id.clone()
                }
            } else {
                // Missing node_id, synthesize from tag
                tracing::warn!(
                    target: "vision_node::p2p::beacon_bootstrap",
                    "[BEACON_BOOTSTRAP] Beacon peer missing nodeId, synthesizing from nodeTag={}",
                    peer.node_tag
                );
                format!("beacon-{}", peer.node_tag)
            };

            // Skip only if it's actually us
            if let Some(self_id) = self_node_id {
                if node_id == self_id {
                    tracing::debug!(
                        target: "vision_node::p2p::beacon_bootstrap",
                        "[BEACON_BOOTSTRAP] Filtering out ourselves: {}",
                        self_id
                    );
                    return None;
                }
            }

            // Optional: enforce same network_id = "mainnet"
            if let Some(net) = &peer.network_id {
                if net != "mainnet" {
                    tracing::debug!(
                        target: "vision_node::p2p::beacon_bootstrap",
                        "[BEACON_BOOTSTRAP] Skipping peer {} from network: {}",
                        node_id,
                        net
                    );
                    return None;
                }
            }

            // Require a valid host - skip peers without one
            let host = match peer.p2p_host.clone() {
                Some(h) if !h.is_empty() && !h.eq_ignore_ascii_case("unknown") => h,
                _ => {
                    // Try http_host as fallback
                    match peer.http_host.as_ref() {
                        Some(http) if !http.is_empty() => {
                            // Strip protocol if present
                            http.trim_start_matches("http://")
                                .trim_start_matches("https://")
                                .split('/')
                                .next()
                                .unwrap_or("")
                                .to_string()
                        }
                        _ => String::new()
                    }
                }
            };

            // Skip if no valid host
            if host.is_empty() || host.eq_ignore_ascii_case("unknown") {
                tracing::info!(
                    target: "vision_node::p2p::beacon_bootstrap",
                    "[BEACON_BOOTSTRAP] Skipping peer {} ({}) - no reachable host from beacon",
                    peer.node_tag,
                    node_id,
                );
                return None;
            }

            let port = peer.p2p_port.unwrap_or(7072);

            // Skip invalid ports
            if port == 0 {
                tracing::info!(
                    target: "vision_node::p2p::beacon_bootstrap",
                    "[BEACON_BOOTSTRAP] Skipping peer {} ({}) - invalid port 0",
                    peer.node_tag,
                    node_id,
                );
                return None;
            }

            // Phase 3: IPv4-only validation - reject IPv6/localhost/loopback
            if !is_valid_beacon_peer_addr(&host, port) {
                tracing::debug!(
                    target: "vision_node::p2p::beacon_bootstrap",
                    "[BEACON_BOOTSTRAP] Dropping unusable beacon peer (IPv6/localhost): {}:{}",
                    host,
                    port
                );
                return None;
            }

            let status_tier = peer.status_tier.clone().unwrap_or_else(|| "dreamer".to_string());

            Some(BeaconBootstrapPeer {
                node_id,
                node_tag: peer.node_tag.clone(),
                host,
                port,
                role: peer.role.clone(),
                status_tier: Some(status_tier),
                vision_address: peer.vision_address.clone(),
            })
        })
        .collect();

    let usable_count = usable.len();

    // Sort peers to prefer IPv4 addresses first (more stable connectivity)
    usable.sort_by(|a, b| {
        let a_v4 = a
            .host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_ipv4())
            .unwrap_or(false);
        let b_v4 = b
            .host
            .parse::<std::net::IpAddr>()
            .map(|ip| ip.is_ipv4())
            .unwrap_or(false);

        // IPv4 peers come first
        b_v4.cmp(&a_v4)
    });

    tracing::info!(
        target: "vision_node::p2p::beacon_bootstrap",
        "[BEACON_BOOTSTRAP] Beacon returned {} peers, {} usable after filtering (sorted IPv4-first)",
        total,
        usable_count
    );

    Ok(usable)
}

/// Fetch peer list from Guardian beacon
pub async fn fetch_beacon_peers(
    _beacon_url: &str,
    self_node_id: Option<&str>,
) -> Result<Vec<BeaconBootstrapPeer>, String> {
    // Use constellation config to build proper guardian endpoint
    let config = get_constellation_config();
    let peers_url =
        config.build_guardian_url("api/beacon/peers?role=constellation&limit=32&format=node");

    tracing::info!(
        target: "vision_node::p2p::beacon_bootstrap",
        "[BEACON_BOOTSTRAP] Fetching constellation peers from Guardian beacon..."
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&peers_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("failed to call beacon: {}", e))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("failed to read response body: {}", e))?;

    if !status.is_success() {
        tracing::warn!(
            target: "vision_node::p2p::beacon_bootstrap",
            "[BEACON_BOOTSTRAP] peers endpoint returned non-success status: {} body={}",
            status,
            body
        );
        return Ok(Vec::new());
    }

    // Parse with proper field mapping and filtering
    match parse_beacon_peers(&body, self_node_id) {
        Ok(mut peers) if !peers.is_empty() => {
            // Prioritize IPv4 addresses before IPv6
            peers.sort_by(|a, b| {
                let a_v4 = a
                    .host
                    .parse::<std::net::IpAddr>()
                    .map(|ip| ip.is_ipv4())
                    .unwrap_or(false);
                let b_v4 = b
                    .host
                    .parse::<std::net::IpAddr>()
                    .map(|ip| ip.is_ipv4())
                    .unwrap_or(false);
                match (a_v4, b_v4) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            });
            Ok(peers)
        }
        Ok(_) => {
            tracing::warn!(
                target: "vision_node::p2p::beacon_bootstrap",
                "[BEACON_BOOTSTRAP] No usable peers from beacon – nothing to connect to"
            );
            Ok(Vec::new())
        }
        Err(e) => {
            tracing::warn!(
                target: "vision_node::p2p::beacon_bootstrap",
                "[BEACON_BOOTSTRAP] Failed to parse peers from beacon: {}",
                e
            );
            Ok(Vec::new())
        }
    }
}

/// Bootstrap P2P connections from Guardian beacon
/// This should be called during node startup if BEACON_ENDPOINT is configured
///
/// Constellation-Only Bootstrap Behavior:
/// 1. Fetches peer list from Guardian beacon (OTHER constellation nodes)
/// 2. Filters out ourselves (to avoid self-connection)
/// 3. Connects to up to 8 constellation peers concurrently
/// 4. Adds successful connections to both P2P_MANAGER and legacy CHAIN.peers
///
/// Note: Guardian is NOT in the peer list - it's HTTP beacon only, not a P2P peer
pub async fn bootstrap_from_beacon() {
    use std::sync::Arc;

    // Get beacon base URL (defaults to public beacon unless explicitly disabled)
    let beacon_endpoint = match beacon_base_url() {
        Some(url) => url,
        None => {
            tracing::info!("[BEACON] Standalone mode (BEACON_ENDPOINT=standalone/off) – skipping beacon bootstrap");
            return;
        }
    };

    tracing::info!(
        "[BEACON] 🚀 Starting smart bootstrap from: {}",
        beacon_endpoint
    );

    // Access the global systems from main.rs
    use crate::{CHAIN, P2P_MANAGER, PEER_PREFIX};
    use sled::IVec;

    // Get our node_id from P2P manager for proper filtering
    let self_node_id = Some(crate::P2P_MANAGER.get_node_id().to_string());

    let self_node_id_ref = self_node_id.as_deref();

    // Fetch peer list from beacon (filtering happens inside parse_beacon_peers)
    match fetch_beacon_peers(&beacon_endpoint, self_node_id_ref).await {
        Ok(peers) => {
            if peers.is_empty() {
                // Already logged inside fetch_beacon_peers
                return;
            }

            let connect_count = peers.len().min(8);
            tracing::info!(
                target: "vision_node::p2p::beacon_bootstrap",
                "[BEACON_BOOTSTRAP] 🌐 Discovered {} constellation peers, connecting to {}",
                peers.len(),
                connect_count
            );

            // ⭐ ROLLING MESH: Add beacon-discovered peers to peer store with upsert (capacity enforcement)
            {
                let chain = crate::CHAIN.lock();
                if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                    for peer in &peers {
                        let now = chrono::Utc::now().timestamp();
                        let vision_peer = crate::p2p::peer_store::VisionPeer {
                            node_id: peer.node_id.clone(),
                            node_tag: peer.node_tag.clone(),
                            public_key: String::new(), // Will be filled during handshake
                            vision_address: peer.vision_address.clone().unwrap_or_default(),
                            ip_address: Some(format!("{}:{}", peer.host, peer.port)),
                            role: peer
                                .role
                                .clone()
                                .unwrap_or_else(|| "constellation".to_string()),
                            last_seen: now,
                            trusted: false,
                            admission_ticket_fingerprint: String::new(),
                            mood: None, // Will be computed during handshake
                            // Rolling mesh health fields (default for new peers)
                            health_score: 50,
                            last_success: 0,
                            last_failure: 0,
                            fail_count: 0,
                            is_seed: false,   // Beacon peers are not protected seeds
                            is_anchor: false, // Will be marked true if from VISION_ANCHOR_SEEDS
                            connection_status: "disconnected".to_string(),
                            // Phase 3.5: Latency & routing defaults
                            last_rtt_ms: None,
                            avg_rtt_ms: None,
                            latency_bucket: None,
                            reliability_score: 0.5,
                            success_count: 0,
                            region: None,
                            // Phase 4: Reputation defaults
                            trust_level: crate::p2p::peer_store::PeerTrustLevel::Normal,
                            reputation: 50.0,
                            misbehavior_score: 0.0,
                            graylisted_until: None,
                            banned_until: None,
                            total_invalid_msgs: 0,
                            total_protocol_violations: 0,
                            total_spam_events: 0,
                            // Phase 4: Route learning defaults
                            route_uses: 0,
                            route_successes: 0,
                            route_failures: 0,
                            avg_delivery_ms: None,
                            // Phase 5: Peer hierarchy defaults
                            peer_tier: crate::p2p::peer_store::PeerTier::Hot,
                            last_promotion: None,
                            public_reachable: false,
                        };

                        if let Err(e) = peer_store.upsert_peer(vision_peer) {
                            tracing::debug!(
                                "[BEACON_BOOTSTRAP] Failed to upsert peer {}: {}",
                                peer.node_tag,
                                e
                            );
                        }
                    }
                    tracing::info!(
                        "[BEACON_BOOTSTRAP] 📚 Added {} peers to unlimited peer book (no capacity limit)",
                        peers.len().min(8)
                    );
                }
            }

            // Connect to up to 8 constellation peers concurrently
            // Note: peers list already filtered - only contains valid host+port
            for peer in peers.into_iter().take(8) {
                let p2p = Arc::clone(&*P2P_MANAGER);
                let addr = format!("{}:{}", peer.host, peer.port);
                let addr_clone = addr.clone();
                let node_tag = peer.node_tag.clone();

                tracing::info!(
                    "[BEACON_BOOTSTRAP] Connecting to peer {} @ {}",
                    node_tag,
                    addr
                );

                // Spawn concurrent connection attempts
                tokio::spawn(async move {
                    tracing::info!(
                        "[BEACON] 🔌 Connecting to constellation peer {}...",
                        addr_clone
                    );

                    match p2p.connect_to_peer(addr_clone.clone()).await {
                        Ok(_) => {
                            tracing::info!("[BEACON] ✅ P2P handshake completed: {}", addr_clone);

                            // Add to legacy CHAIN.peers system for /api/peers endpoint
                            // NOTE: P2P uses :7072 but HTTP API uses :7070 by default.
                            let host = addr_clone.split(':').next().unwrap_or(addr_clone.as_str());
                            let peer_url = format!("http://{}:7070", host);
                            let mut g = CHAIN.lock();
                            if g.peers.insert(peer_url.clone()) {
                                let key = format!("{}{}", PEER_PREFIX, peer_url);
                                let _ = g.db.insert(key.as_bytes(), IVec::from(&b"1"[..]));
                                let _ = g.db.flush();
                                tracing::info!("[BEACON] 📝 Peer added to registry: {}", peer_url);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[BEACON] ❌ Failed to connect to peer {}: {}",
                                addr_clone,
                                e
                            );
                        }
                    }
                });

                // Small delay between spawns to avoid overwhelming the network
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            tracing::info!("[BEACON] ✨ Bootstrap initiated – P2P connections establishing...");
        }
        Err(e) => {
            tracing::warn!(
                "[BEACON] ❌ Failed to fetch peers from beacon {}: {}",
                beacon_endpoint,
                e
            );
        }
    }
}
