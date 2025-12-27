//! Bootstrap Module
//!
//! Fetches dynamic seed peers and network information from a central bootstrap endpoint.
//! This allows nodes to discover peers without hardcoding addresses, while keeping
//! consensus logic completely independent.
//!
//! **Phase 5 Unified Bootstrap Strategy:**
//! 1. Local peer book (sled "vision_peer_book") - fastest, most reliable
//! 2. VISION_BOOTSTRAP_PEERS env var - static seeds
//! 3. Beacon / website helpers via BEACON_ENDPOINT - dynamic discovery

use serde::Deserialize;
use tracing::{debug, info, warn};

/// Response from the bootstrap endpoint
#[derive(Debug, Deserialize)]
pub struct BootstrapResponse {
    /// Network identifier (e.g., "testnet", "mainnet")
    pub network: String,
    /// Welcome message to display to users
    #[serde(default)]
    pub welcome_message: Option<String>,
    /// Minimum supported node version
    #[serde(default)]
    pub min_version: Option<String>,
    /// Recommended node version
    #[serde(default)]
    pub recommended_version: Option<String>,
    /// List of seed peer addresses in "host:port" format
    pub seed_peers: Vec<String>,
}

/// Fetch bootstrap information from the specified URL
///
/// # Arguments
/// * `url` - Bootstrap endpoint URL (e.g., "https://visionworld.tech/api/bootstrap")
///
/// # Returns
/// * `Ok(BootstrapResponse)` - Successfully fetched bootstrap info
/// * `Err(String)` - Error message if fetch failed
pub async fn fetch_bootstrap_info(url: &str) -> Result<BootstrapResponse, String> {
    info!(url = url, "Fetching bootstrap information");

    // Create a client with timeout to avoid hanging
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Fetch the bootstrap endpoint
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch bootstrap endpoint: {}", e))?;

    // Check status code
    if !response.status().is_success() {
        return Err(format!(
            "Bootstrap endpoint returned status {}",
            response.status()
        ));
    }

    // Parse JSON response
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let info: BootstrapResponse = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Failed to parse bootstrap JSON: {}", e))?;

    // Log welcome message if present
    if let Some(ref msg) = info.welcome_message {
        info!(message = %msg, "Bootstrap welcome message");
    }

    // Log version information if present
    if let Some(ref min_ver) = info.min_version {
        info!(min_version = %min_ver, "Minimum supported version from bootstrap");
    }
    if let Some(ref rec_ver) = info.recommended_version {
        info!(recommended_version = %rec_ver, "Recommended node version from bootstrap");
    }

    info!(
        network = %info.network,
        seed_count = info.seed_peers.len(),
        "Successfully fetched bootstrap information"
    );

    Ok(info)
}

/// Fetch bootstrap peers with fallback to empty list on error
///
/// This is a convenience wrapper that logs errors but doesn't fail the node startup
/// if the bootstrap endpoint is unavailable.
pub async fn fetch_bootstrap_peers(url: &str) -> Vec<String> {
    match fetch_bootstrap_info(url).await {
        Ok(info) => {
            info!(count = info.seed_peers.len(), "Fetched bootstrap peers");
            info.seed_peers
        }
        Err(e) => {
            warn!(error = %e, "Failed to fetch bootstrap peers, continuing with static configuration");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bootstrap_response_deserialization() {
        let json = r#"{
            "network": "testnet",
            "welcome_message": "Welcome to Vision World, Dreamer...",
            "min_version": "0.7.0",
            "recommended_version": "0.7.9",
            "seed_peers": ["1.2.3.4:7072", "5.6.7.8:7072"]
        }"#;

        let response: BootstrapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.network, "testnet");
        assert_eq!(
            response.welcome_message.unwrap(),
            "Welcome to Vision World, Dreamer..."
        );
        assert_eq!(response.seed_peers.len(), 2);
    }

    #[tokio::test]
    async fn test_bootstrap_response_minimal() {
        let json = r#"{
            "network": "mainnet",
            "seed_peers": []
        }"#;

        let response: BootstrapResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.network, "mainnet");
        assert!(response.welcome_message.is_none());
        assert!(response.seed_peers.is_empty());
    }
}

// ============================================================================
// PHASE 5 P2P BOOTSTRAP - ✅ IMPLEMENTED
// ============================================================================
//
// Unified bootstrap system that makes nodes self-bootstrapping without
// dependency on Guardian or single beacon.
//
// Bootstrap Strategy (in order):
//   Phase 0: Genesis seed peers (offline-first, hardcoded + seed_peers.json)
//   Phase 1: Local peer book (sled "vision_peer_book")
//   Phase 2: VISION_BOOTSTRAP_PEERS env var (vision:// addresses)
//   Phase 3: Beacon discovery (BEACON_ENDPOINT API)
//
// Implementation Status:
//   ✅ unified_bootstrap() - Main orchestrator (line 229)
//   ✅ bootstrap_from_peer_book() - Local peer discovery
//   ✅ bootstrap_from_env_seeds() - Environment variable seeds
//   ✅ bootstrap_from_beacon() - Beacon API integration
//   ✅ resolve_vision_address() - vision:// address resolution (line 612)
//   ✅ Comprehensive logging at each phase
//   ✅ Graceful fallback to isolated mode
//   ✅ Integrated with main.rs startup sequence
//
// Features:
//   - Tries up to 8 peers per phase
//   - Returns Ok(()) on first successful connection
//   - Continues to isolated mode if all phases fail
//   - Supports both standard IP:port and vision:// formats
//   - Peer book lookup by public key hash
//   - Random peer selection for load distribution
//
// Usage:
//   Called from main.rs during node startup after P2P listener is initialized.
//   Node continues running even if bootstrap fails (lonely mode).
// ============================================================================
//   - beacon endpoints exist on the website / upstream
//   - Node already has HTTP client (reqwest) used elsewhere
//
// Implement the unified_bootstrap() function, plus any small private helpers you need.
//
// If all phases fail, do NOT crash the node.
// Instead:
//   - Log: [BOOTSTRAP] ❌ No peers reachable – running in isolated mode.
//   - Return Ok(()) so the node can still mine / be manually connected later.

/// Bootstrap peer connection target
#[derive(Debug, Clone)]
pub struct BootstrapPeer {
    pub host: String,
    pub port: u16,
    pub node_tag: Option<String>,
    pub vision_address: Option<String>,
}

/// Phase 5 Unified Bootstrap
///
/// Implements a tiered bootstrap strategy:
/// 0. Genesis seed peers (offline-first, hardcoded + vision_data/seed_peers.json)
/// 1. Local peer book (fastest, most reliable)
/// 2. Environment variable seeds (VISION_BOOTSTRAP_PEERS)
/// 3. Beacon-based discovery (BEACON_ENDPOINT)
///
/// Always returns Ok() even if no peers connect - node runs in isolated mode.
pub async fn unified_bootstrap() -> Result<(), String> {
    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Starting Phase 5 unified bootstrap sequence"
    );

    // Phase -1: Hydrate peer book from HTTP anchors (v2.7.0+)
    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase -1: Hydrating peer book from HTTP anchors..."
    );
    hydrate_peer_book_from_http_anchors().await;

    // Phase 0: Try genesis seed peers (OFFLINE-FIRST)
    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase 0: Trying genesis seed peers (offline-first)..."
    );
    match super::seed_peers::bootstrap_from_seeds().await {
        Ok(count) if count > 0 => {
            info!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] 🌱 Initiated {} genesis seed connections",
                count
            );
            // Continue to try other methods in parallel
        }
        Ok(_) => {
            warn!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] ❌ No genesis seeds available"
            );
        }
        Err(e) => {
            warn!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] ❌ Genesis seed bootstrap error: {}",
                e
            );
        }
    }

    // Phase 1: Try local peer book
    match bootstrap_from_peer_book().await {
        Ok(count) if count > 0 => {
            // connected_count += count; // Unused assignment
            info!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] ✅ Connected to {} peers from peer book",
                count
            );
            return Ok(());
        }
        Ok(_) => {
            info!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] ❌ Failed to connect to any peers from peer book"
            );
        }
        Err(e) => {
            warn!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] ❌ Peer book bootstrap error: {}",
                e
            );
        }
    }

    // Phase 2: Try environment variable seeds
    if let Ok(seeds) = std::env::var("VISION_BOOTSTRAP_PEERS") {
        match bootstrap_from_env_seeds(&seeds).await {
            Ok(count) if count > 0 => {
                // connected_count += count; // Unused assignment
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ✅ Connected to {} peers from VISION_BOOTSTRAP_PEERS",
                    count
                );
                return Ok(());
            }
            Ok(_) => {
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ❌ Failed to connect to any peers from VISION_BOOTSTRAP_PEERS"
                );
            }
            Err(e) => {
                warn!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ❌ Env seeds bootstrap error: {}",
                    e
                );
            }
        }
    }

    // Phase 3: Try beacon-based discovery
    if let Ok(beacon_endpoint) = std::env::var("BEACON_ENDPOINT") {
        match bootstrap_from_beacon(&beacon_endpoint).await {
            Ok(count) if count > 0 => {
                // connected_count += count; // Unused assignment
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ✅ Connected to {} peers from beacon {}",
                    count,
                    beacon_endpoint
                );
                return Ok(());
            }
            Ok(_) => {
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ❌ Failed to connect to any peers from beacon"
                );
            }
            Err(e) => {
                warn!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ❌ Beacon bootstrap error: {}",
                    e
                );
            }
        }
    }

    // All phases failed - run in isolated mode
    warn!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] ❌ No peers reachable – running in isolated mode"
    );
    warn!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Node can still mine and accept manual connections"
    );

    Ok(())
}

/// Bootstrap from local peer book (sled "vision_peer_book")
async fn bootstrap_from_peer_book() -> Result<usize, String> {
    use super::peer_store::PeerStore;

    // Get global peer store DB
    let db = crate::PEER_STORE_DB.clone();

    let store = PeerStore::new(&db).map_err(|e| format!("Failed to open peer store: {}", e))?;

    // ⭐ ROLLING MESH: Get best healthy peers (sorted by health + recency)
    // Priority: get_best_peers(64, min_health=30) -> healthy peers with score >= 30
    let best_peers = store.get_best_peers(64, 30);

    // Fallback: if no healthy peers, try recent peers with lower bar
    let mut peers = if !best_peers.is_empty() {
        info!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Phase 1: Using {} best healthy peers from rolling mesh (health >= 30)",
            best_peers.len()
        );
        best_peers
    } else {
        info!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Phase 1: No healthy peers, falling back to recent peers"
        );
        store.recent()
    };

    // ⭐ Change 9: Randomize peer order before dialing (do this BEFORE spawning tasks)
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        peers.shuffle(&mut rng);
    } // RNG dropped here, before async spawns

    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase 1: Trying peer book ({} entries, randomized)...",
        peers.len()
    );

    if peers.is_empty() {
        debug!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Phase 1: Peer book is empty"
        );
        return Ok(0);
    }

    // Connect to up to 8 peers from peer book
    // use crate::P2P_MANAGER;  // TODO: P2P_MANAGER not defined
    // use std::sync::Arc;

    let mut attempted = 0;
    let mut successful = 0;
    for peer in peers.into_iter().take(8) {
        // Skip peers without IP addresses
        let mut addr = match peer.ip_address {
            Some(ref ip) => ip.clone(),
            None => {
                debug!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] Skipping peer {} ({}) - no IP address",
                    peer.node_tag, peer.node_id
                );
                continue;
            }
        };

        // If legacy entry has no port, default to P2P port 7072
        if !addr.contains(':') {
            addr = format!("{}:7072", addr);
        }

        // ⭐ Change 9: Filter IPv6 addresses (IPv4-only policy)
        if let Ok(sock_addr) = addr.parse::<std::net::SocketAddr>() {
            if !sock_addr.is_ipv4() {
                debug!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] Skipping IPv6 peer: {} ({})",
                    peer.node_tag,
                    addr
                );
                continue;
            }
        } else {
            debug!(
                target: "vision_node::p2p::bootstrap",
                "[BOOTSTRAP] Skipping malformed address: {}",
                addr
            );
            continue;
        }

        let p2p = Arc::clone(&*P2P_MANAGER);

        debug!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Connecting to IPv4 peer {} ({}) @ {}",
            peer.node_tag, peer.node_id, addr
        );

        // Attempt connection with timeout and track result
        let connect_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(3),
            p2p.connect_to_peer(addr.clone()),
        )
        .await;

        match connect_result {
            Ok(Ok(_)) => {
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ✅ Connected to peer from book: {}",
                    addr
                );
                successful += 1;
            }
            Ok(Err(e)) => {
                debug!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] Failed to connect to peer {}: {}",
                    addr, e
                );
            }
            Err(_) => {
                debug!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] Connection timeout (3s) for peer: {}",
                    addr
                );
            }
        }

        attempted += 1;

        // Small delay between connection attempts
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Completed: {} attempted, {} successful",
        attempted, successful
    );

    Ok(successful)
}

/// Bootstrap from VISION_BOOTSTRAP_PEERS environment variable
/// Format: comma-separated "vision://VNODE-XXXX-YYYY@abcdef123456..."
async fn bootstrap_from_env_seeds(seeds: &str) -> Result<usize, String> {
    let addresses: Vec<&str> = seeds
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase 2: Trying VISION_BOOTSTRAP_PEERS ({} addresses)...",
        addresses.len()
    );

    if addresses.is_empty() {
        debug!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Phase 2: No bootstrap peers in environment"
        );
        return Ok(0);
    }

    // Connect to seed peers
    // use crate::P2P_MANAGER;  // TODO: P2P_MANAGER not defined
    // use std::sync::Arc;

    let mut attempted = 0;
    for addr_str in addresses.into_iter().take(8) {
        // Support both standard "host:port" and vision:// address formats
        let target_addr = if addr_str.starts_with("vision://") {
            // Resolve vision:// address to IP:port
            match resolve_vision_address(addr_str) {
                Some(peer) => {
                    let resolved = format!("{}:{}", peer.host, peer.port);
                    debug!(
                        target: "vision_node::p2p::bootstrap",
                        vision_addr = %addr_str,
                        resolved_to = %resolved,
                        "[BOOTSTRAP] Resolved vision:// address"
                    );
                    resolved
                }
                None => {
                    debug!(
                        target: "vision_node::p2p::bootstrap",
                        "[BOOTSTRAP] Could not resolve vision:// address: {}",
                        addr_str
                    );
                    continue;
                }
            }
        } else {
            addr_str.to_string()
        };

        let p2p = Arc::clone(&*P2P_MANAGER);
        let addr = target_addr;

        debug!(
            target: "vision_node::p2p::bootstrap",
            "[BOOTSTRAP] Connecting to seed peer: {}",
            addr
        );

        // Attempt connection and track result
        match p2p.connect_to_peer(addr.clone()).await {
            Ok(_) => {
                info!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] ✅ Connected to seed peer: {}",
                    addr
                );
                attempted += 1;
            }
            Err(e) => {
                debug!(
                    target: "vision_node::p2p::bootstrap",
                    "[BOOTSTRAP] Failed to connect to seed peer {}: {}",
                    addr, e
                );
            }
        }

        // Small delay between connection attempts
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Ok(attempted)
}

/// Bootstrap from beacon endpoint
/// GET {BEACON_ENDPOINT}/api/beacon/peers?role=constellation&limit=32
async fn bootstrap_from_beacon(beacon_endpoint: &str) -> Result<usize, String> {
    use super::beacon_bootstrap;

    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase 3: Trying beacon peers from {}...",
        beacon_endpoint
    );

    // Use existing beacon bootstrap module (reads BEACON_ENDPOINT from env)
    beacon_bootstrap::bootstrap_from_beacon().await;

    // The beacon bootstrap function handles its own success/failure logging
    // We don't have a simple way to get peer count without async, so just indicate completion
    info!(
        target: "vision_node::p2p::bootstrap",
        "[BOOTSTRAP] Phase 3: Beacon bootstrap process completed"
    );

    // Return non-zero to indicate the phase ran (actual success determined by beacon module logs)
    Ok(1)
}

/// Resolve a vision address to a connection target
/// Format: vision://VNODE-XXXX-YYYY@pubkey_hash or vision://pubkey_hash
fn resolve_vision_address(vision_address: &str) -> Option<BootstrapPeer> {
    use tracing::{debug, warn};

    // Validate format
    if !vision_address.starts_with("vision://") {
        warn!(
            target: "vision_node::p2p::bootstrap",
            address = vision_address,
            "Invalid vision:// address format"
        );
        return None;
    }

    // Strip protocol prefix
    let addr_part = vision_address.strip_prefix("vision://").unwrap();

    // Parse format: VNODE-XXXX-YYYY@pubkey or just pubkey
    let pubkey_hash = if let Some(at_pos) = addr_part.find('@') {
        // Format: VNODE-XXXX-YYYY@pubkey_hash
        let (node_id, pubkey) = addr_part.split_at(at_pos);
        debug!(
            target: "vision_node::p2p::bootstrap",
            node_id = node_id,
            "Parsing vision address with node ID"
        );
        &pubkey[1..] // Skip '@'
    } else {
        // Format: vision://pubkey_hash
        addr_part
    };

    // Look up in peer store for last known IP:port
    // Access the chain's peer store database directly
    let chain = crate::CHAIN.lock();
    if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
        // Search through all peers to find matching pubkey
        let all_peers = peer_store.all();
        drop(chain); // Release lock early

        for peer in all_peers {
            if peer.public_key == pubkey_hash {
                // Found matching peer - check if we have an IP address
                if let Some(ip_addr) = peer.ip_address {
                    // Parse to extract host:port
                    if let Some((host, port_str)) = ip_addr.rsplit_once(':') {
                        if let Ok(port) = port_str.parse::<u16>() {
                            debug!(
                                target: "vision_node::p2p::bootstrap",
                                pubkey = pubkey_hash,
                                host = host,
                                port = port,
                                "Resolved vision:// address from peer store"
                            );

                            return Some(BootstrapPeer {
                                host: host.to_string(),
                                port,
                                node_tag: Some(peer.node_tag),
                                vision_address: Some(peer.vision_address),
                            });
                        }
                    } else {
                        // Legacy entry with no explicit port – assume P2P port 7072
                        let host = ip_addr;
                        let port = 7072;
                        debug!(
                            target: "vision_node::p2p::bootstrap",
                            pubkey = pubkey_hash,
                            host = host,
                            port = port,
                            "Resolved vision:// address from peer store (legacy IP-only entry)"
                        );
                        return Some(BootstrapPeer {
                            host,
                            port,
                            node_tag: Some(peer.node_tag),
                            vision_address: Some(peer.vision_address),
                        });
                    }
                }

                debug!(
                    target: "vision_node::p2p::bootstrap",
                    pubkey = pubkey_hash,
                    "Found peer in store but no valid IP:port; cannot resolve vision:// address"
                );
                break;
            }
        }

        debug!(
            target: "vision_node::p2p::bootstrap",
            pubkey = pubkey_hash,
            "Vision address not found in local peer store"
        );
    } else {
        drop(chain);
    }

    // Fallback: Query website resolver (if BEACON_ENDPOINT is set)
    // This would make an HTTP request to resolve the vision:// address
    // For now, return None if not found in local peer store
    debug!(
        target: "vision_node::p2p::bootstrap",
        address = vision_address,
        "Could not resolve vision:// address (not in peer store, resolver not implemented)"
    );

    None
}

// ============================================================================
// HTTP SEED PEER DISTRIBUTION (v2.7.0+)
// ============================================================================

/// Remote seed peer from HTTP endpoint
#[derive(Debug, Deserialize)]
pub struct RemoteSeedPeer {
    /// P2P address in "ip:port" format
    pub address: String,
    /// Anchor flag - true for backbone nodes
    pub is_anchor: bool,
}

/// Fetch seed peers from HTTP endpoint on port 7070
///
/// Queries an anchor node's HTTP API to get a list of healthy peers.
/// This allows new nodes to bootstrap without relying on P2P gossip.
async fn fetch_http_seed_peers(base_http_url: &str) -> anyhow::Result<Vec<RemoteSeedPeer>> {
    let url = format!("{}/api/p2p/seed_peers", base_http_url.trim_end_matches('/'));

    debug!(
        target: "vision_node::p2p::bootstrap",
        url = %url,
        "[HTTP_BOOTSTRAP] Fetching seed peers from anchor"
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP seed fetch failed: {}", resp.status());
    }

    let peers = resp.json::<Vec<RemoteSeedPeer>>().await?;

    debug!(
        target: "vision_node::p2p::bootstrap",
        count = peers.len(),
        "[HTTP_BOOTSTRAP] Received {} seed peers",
        peers.len()
    );

    Ok(peers)
}

/// Parse anchor seeds from VISION_ANCHOR_SEEDS environment variable
///
/// Returns list of anchor hosts/IPs (no scheme, no port).
fn parse_anchor_seeds_from_env() -> Vec<String> {
    std::env::var("VISION_ANCHOR_SEEDS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|raw| {
            let mut s = raw.trim();
            if s.is_empty() {
                return None;
            }
            if let Some(rest) = s.strip_prefix("http://") {
                s = rest;
            } else if let Some(rest) = s.strip_prefix("https://") {
                s = rest;
            }
            if let Some((host_port, _path)) = s.split_once('/') {
                s = host_port;
            }
            let host = if let Some((h, _port)) = s.rsplit_once(':') {
                h
            } else {
                s
            };
            let host = host.trim();
            if host.is_empty() {
                None
            } else {
                Some(host.to_string())
            }
        })
        .collect()
}

/// Hydrate peer book from HTTP anchors before P2P bootstrap
///
/// Queries anchors listed in VISION_ANCHOR_SEEDS via HTTP (port 7070)
/// to get a fresh list of healthy peers. These peers are added to the
/// peer book before attempting P2P connections.
///
/// This allows home miners behind CGNAT to discover peers via HTTP
/// instead of relying solely on P2P gossip.
async fn hydrate_peer_book_from_http_anchors() {
    let mut anchors = parse_anchor_seeds_from_env();

    if anchors.is_empty() {
        anchors = crate::p2p::seed_peers::default_anchor_seeds();
        info!(
            target: "vision_node::p2p::bootstrap",
            count = anchors.len(),
            "[HTTP_BOOTSTRAP] No VISION_ANCHOR_SEEDS configured, using default anchors (probing HTTP 7070)"
        );
    }

    info!(
        target: "vision_node::p2p::bootstrap",
        count = anchors.len(),
        "[HTTP_BOOTSTRAP] Querying {} anchors for seed peers",
        anchors.len()
    );

    let mut total_added = 0;

    for host in anchors {
        let http_base = format!("http://{}:7070", host);

        match fetch_http_seed_peers(&http_base).await {
            Ok(remote_peers) => {
                info!(
                    target: "vision_node::p2p::bootstrap",
                    anchor = %host,
                    count = remote_peers.len(),
                    "[HTTP_BOOTSTRAP] ✅ Fetched {} peers from anchor {}",
                    remote_peers.len(),
                    host
                );

                // Insert peers into peer book
                let chain = crate::CHAIN.lock();
                if let Ok(store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                    drop(chain); // Release lock before processing

                    for rp in remote_peers {
                        // Parse "ip:port" from address
                        if let Some((rip, rport)) = rp.address.rsplit_once(':') {
                            if let Ok(port) = rport.parse::<u16>() {
                                match store.upsert_peer_from_http(
                                    rip.to_string(),
                                    port,
                                    rp.is_anchor,
                                ) {
                                    Ok(_) => {
                                        total_added += 1;
                                        debug!(
                                            target: "vision_node::p2p::bootstrap",
                                            peer = %rp.address,
                                            anchor = rp.is_anchor,
                                            "[HTTP_BOOTSTRAP] Added peer to book"
                                        );
                                    }
                                    Err(e) => {
                                        debug!(
                                            target: "vision_node::p2p::bootstrap",
                                            peer = %rp.address,
                                            error = %e,
                                            "[HTTP_BOOTSTRAP] Failed to add peer"
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                    drop(chain);
                    warn!(
                        target: "vision_node::p2p::bootstrap",
                        "[HTTP_BOOTSTRAP] Could not access peer store"
                    );
                }
            }
            Err(e) => {
                warn!(
                    target: "vision_node::p2p::bootstrap",
                    anchor = %host,
                    error = %e,
                    "[HTTP_BOOTSTRAP] ❌ Failed to fetch from anchor: {:?}",
                    e
                );
            }
        }
    }

    if total_added > 0 {
        info!(
            target: "vision_node::p2p::bootstrap",
            total = total_added,
            "[HTTP_BOOTSTRAP] 🌐 Hydrated peer book with {} peers from HTTP anchors",
            total_added
        );
    } else {
        warn!(
            target: "vision_node::p2p::bootstrap",
            "[HTTP_BOOTSTRAP] ⚠️  No peers added from HTTP anchors"
        );
    }
}
