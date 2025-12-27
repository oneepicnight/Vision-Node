//! Connection Maintainer - Ensures minimum peer connections are always maintained
//!
//! This module implements PATCH 2: Stronger connection maintainer to avoid the 1-peer trap
//!
//! Continuously monitors peer count and aggressively tries to reach min_outbound_connections:
//! - Tries all known seeds first
//! - Falls back to peer store if still low
//! - Runs every reconnection_interval_seconds
//! - Never gives up trying to connect

use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, info, warn};

use crate::p2p::p2p_config::SeedPeersConfig;
use crate::p2p::peer_store::PeerStore;

/// Connection maintainer statistics
#[derive(Debug, Clone, Default)]
pub struct MaintainerStats {
    pub checks_performed: u64,
    pub seeds_tried: u64,
    pub store_peers_tried: u64,
    pub connections_established: u64,
    pub last_check_time: u64,
}

/// Run the connection maintainer loop
///
/// This task runs continuously in the background and ensures we always
/// try to maintain at least min_outbound_connections active peers.
///
/// The maintainer will continue dialing until BOTH conditions are met:
/// 1. connected_peers >= max_outbound_connections
/// 2. peer_store.count() <= connected_peers (fully saturated)
///
/// This ensures aggressive peer discovery and connection expansion.
pub async fn run_connection_maintainer(
    cfg: Arc<SeedPeersConfig>,
    peer_store: Arc<PeerStore>,
    our_node_id: String,
) {
    // Hard cap to prevent dial storms from large peer books / handshake seeding.
    const MAX_NEW_DIALS_PER_CYCLE: usize = 3;

    // Wake periodically to try new unconnected peer candidates
    let check_interval = Duration::from_secs(cfg.reconnection_interval_seconds.max(1));
    let mut stats = MaintainerStats::default();

    info!(
        "[CONN_MAINTAINER] 🔧 Starting connection maintainer (interval: {}s, min_peers: {}, max_peers: {})",
        cfg.reconnection_interval_seconds,
        cfg.min_outbound_connections,
        cfg.max_outbound_connections
    );

    // PERIODIC GOSSIP LOOP (30s interval)
    // This replaces on-connect gossip to prevent thundering herd
    let peer_store_for_gossip = Arc::clone(&peer_store);
    let our_node_tag = if our_node_id.len() >= 8 {
        format!("VNODE-{}", &our_node_id[..8])
    } else {
        format!("VNODE-{}", our_node_id)
    };
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(30));
        info!("[GOSSIP] 🌐 Starting periodic gossip loop (30s interval)");

        loop {
            ticker.tick().await;

            // Only gossip to currently-connected peers (truth source: active connections map)
            let connected: Vec<String> = crate::P2P_MANAGER.get_peer_addresses().await;

            if connected.is_empty() {
                debug!("[GOSSIP] No connected peers to gossip to");
                continue;
            }

            // Build gossip message ONCE (reuse for all peers)
            let Some(gossip) = crate::p2p::peer_gossip::create_gossip_message(
                peer_store_for_gossip.clone(),
                &our_node_tag,
            )
            .await
            else {
                debug!("[GOSSIP] No peers to share in gossip message");
                continue;
            };

            info!(
                "[GOSSIP] Broadcasting to {} connected peers ({} peers in message)",
                connected.len(),
                gossip.peers.len()
            );

            // Send to all connected peers with 5-second timeout per send
            let mut success = 0;
            let mut failed = 0;
            let mut timed_out = 0;

            for addr in connected {
                let gossip_clone = gossip.clone();
                let send_fut = async {
                    crate::P2P_MANAGER
                        .send_to_peer(
                            &addr,
                            crate::p2p::connection::P2PMessage::PeerGossip(gossip_clone),
                        )
                        .await
                };

                match timeout(Duration::from_secs(5), send_fut).await {
                    Ok(Ok(())) => success += 1,
                    Ok(Err(e)) => {
                        failed += 1;
                        debug!("[GOSSIP] Send failed to {}: {}", addr, e);
                    }
                    Err(_) => {
                        timed_out += 1;
                        warn!("[GOSSIP] Send timeout to {} (>5s)", addr);
                    }
                }
            }

            if success > 0 {
                info!(
                    "[GOSSIP] Broadcast complete: {} success, {} failed, {} timeout",
                    success, failed, timed_out
                );
            }
        }
    });

    loop {
        sleep(check_interval).await;

        stats.checks_performed += 1;
        stats.last_check_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Count currently connected peers AND peer store size
        let connected = count_connected_peers(&peer_store).await;
        let peer_store_count = peer_store.get_all().len();

        // FULL MESH LOGIC: Only stop when we've connected to ALL known peers.
        // IMPORTANT: Do NOT short-circuit when the peer store is empty; we still
        // need to dial configured seed peers to bootstrap.
        if connected >= cfg.min_outbound_connections
            && peer_store_count > 0
            && peer_store_count <= connected
        {
            debug!(
                "[CONN_MAINTAINER] ✅ Full mesh achieved: connected={}, all {} known peers connected",
                connected,
                peer_store_count
            );
            continue;
        }

        // Warn if we hit the configured max but still have unconnected peers
        if connected >= cfg.max_outbound_connections {
            warn!(
                "[CONN_MAINTAINER] ⚠️ Hit max_outbound_connections ({}) but {} unconnected peers remain. Consider increasing VISION_MAX_PEERS.",
                cfg.max_outbound_connections,
                peer_store_count - connected
            );
            continue;
        }

        info!(
            target: "p2p::maintainer",
            "🔄 Connection expansion: connected={}/{} max, known_peers={}, attempting new connections",
            connected,
            cfg.max_outbound_connections,
            peer_store_count
        );

        // Calculate how many more connections we can make
        let slots_available = cfg.max_outbound_connections.saturating_sub(connected);
        let unconnected_peers = peer_store_count.saturating_sub(connected);

        // Gradual dialing: never attempt more than MAX_NEW_DIALS_PER_CYCLE per tick.
        let needed = slots_available
            .min(unconnected_peers)
            .min(MAX_NEW_DIALS_PER_CYCLE)
            .max(1);

        info!(
            "[CONN_MAINTAINER] 🌐 Full mesh target: {} unconnected peers, {} slots available",
            unconnected_peers, slots_available
        );

        // Phase 1: Try all seeds first
        info!(
            "[CONN_MAINTAINER] 🌱 Trying {} seed peers",
            cfg.seed_peers.len()
        );
        let mut dials_this_cycle = 0usize;
        for seed_addr in &cfg.seed_peers {
            if dials_this_cycle >= needed {
                break;
            }
            stats.seeds_tried += 1;

            // PATCH 3: Prefilter candidates before dialing
            if !should_dial(seed_addr).await {
                debug!("[DIAL] SKIP seed peer (prefilter): {}", seed_addr);
                continue;
            }

            info!(
                target: "p2p::connect",
                seed = %seed_addr,
                "Attempting seed connection (maintainer)"
            );

            let p2p_manager = std::sync::Arc::clone(&crate::P2P_MANAGER);
            dials_this_cycle += 1;
            match p2p_manager.connect_to_peer(seed_addr.clone()).await {
                Ok(_) => {
                    stats.connections_established += 1;
                    info!("[CONN_MAINTAINER] ✅ Connected to seed: {}", seed_addr);

                    // Check if we've reached max capacity
                    let new_count = count_connected_peers(&peer_store).await;
                    if new_count >= cfg.max_outbound_connections {
                        info!(
                            "[CONN_MAINTAINER] Max capacity reached: {} peers",
                            new_count
                        );
                        break;
                    }
                }
                Err(e) => {
                    // PATCH 3: Only record failures for real dial attempts (not prefiltered ones)
                    debug!(
                        target: "p2p::connect",
                        seed = %seed_addr,
                        reason = %e,
                        "Seed connection attempt failed"
                    );
                    crate::p2p::dial_tracker::record_dial_failure(
                        seed_addr.clone(),
                        e.to_string(),
                        "seed".to_string(),
                    );
                }
            }
        }

        // Check again after seed attempts - continue if not at max capacity
        let connected_after_seeds = count_connected_peers(&peer_store).await;
        if connected_after_seeds >= cfg.max_outbound_connections {
            info!(
                "[CONN_MAINTAINER] Max capacity reached after seed attempts: {} peers",
                connected_after_seeds
            );
            continue;
        }

        // Phase 2: Try peers from peer store (sorted by health)
        info!("[CONN_MAINTAINER] 📖 Trying peers from peer store");
        let store_peers = peer_store.get_all();

        // Filter candidates and shuffle for round-robin rotation
        let mut candidates: Vec<_> = store_peers
            .into_iter()
            .filter(|p| {
                // Only try peers with some health and a valid IP
                p.health_score > 0 && p.ip_address.is_some()
            })
            .collect();

        // Shuffle candidates for fair round-robin selection instead of always picking highest health
        // Keep the RNG in a tight scope so this future remains Send.
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            candidates.shuffle(&mut rng);
        }

        // Then sort by health score descending (shuffle breaks ties)
        candidates.sort_by(|a, b| b.health_score.cmp(&a.health_score));

        let to_try = needed.saturating_sub(dials_this_cycle).max(0);
        if to_try == 0 {
            continue;
        }

        for peer in candidates.into_iter().take(to_try) {
            stats.store_peers_tried += 1;

            let addr = match peer.ip_address {
                Some(ref a) => a.clone(),
                None => continue,
            };

            // PATCH 3: Prefilter candidates before dialing
            if !should_dial(&addr).await {
                debug!(
                    "[DIAL] SKIP peer book entry (prefilter): {} ({})",
                    peer.node_tag, addr
                );
                continue;
            }

            info!(
                target: "p2p::connect",
                peer = %peer.node_tag,
                addr = %addr,
                health = peer.health_score,
                "Attempting stored peer connection (maintainer)"
            );

            let p2p_manager = std::sync::Arc::clone(&crate::P2P_MANAGER);
            dials_this_cycle = dials_this_cycle.saturating_add(1);
            match p2p_manager.connect_to_peer(addr.clone()).await {
                Ok(_) => {
                    stats.connections_established += 1;
                    info!(
                        "[CONN_MAINTAINER] ✅ Connected to stored peer: {}",
                        peer.node_tag
                    );

                    let new_count = count_connected_peers(&peer_store).await;
                    if new_count >= cfg.max_outbound_connections {
                        info!(
                            "[CONN_MAINTAINER] Max capacity reached: {} peers",
                            new_count
                        );
                        break;
                    }
                }
                Err(e) => {
                    // PATCH 3: Only record failures for real dial attempts (not prefiltered ones)
                    debug!(
                        target: "p2p::connect",
                        peer = %peer.node_tag,
                        addr = %addr,
                        reason = %e,
                        "Stored peer connection attempt failed"
                    );
                    crate::p2p::dial_tracker::record_dial_failure(
                        addr.clone(),
                        e.to_string(),
                        "peer_book".to_string(),
                    );
                }
            }
        }

        let final_count = count_connected_peers(&peer_store).await;
        let final_store_count = peer_store.get_all().len();
        let unconnected = final_store_count.saturating_sub(final_count);

        if final_count < cfg.min_outbound_connections {
            warn!(
                "[CONN_MAINTAINER] ⚠️ Below minimum: connected={}/{} min, known_peers={}, unconnected={}",
                final_count,
                cfg.min_outbound_connections,
                final_store_count,
                unconnected
            );
        } else if unconnected > 0 {
            info!(
                "[CONN_MAINTAINER] 🌐 Building full mesh: connected={}, known_peers={}, unconnected={} (will retry in {}s)",
                final_count,
                final_store_count,
                unconnected,
                cfg.reconnection_interval_seconds
            );
        } else {
            info!(
                "[CONN_MAINTAINER] ✅ Full mesh achieved: connected={}, all {} known peers connected (everyone talks to everyone!)",
                final_count,
                final_store_count
            );
        }
    }
}

/// Count currently connected peers
async fn count_connected_peers(_peer_store: &Arc<PeerStore>) -> usize {
    // Truth source for live connections is the TCP connection manager.
    // `peer_store.last_seen` can go stale during quiet periods and cause
    // periodic re-dial storms (duplicates every ~60s).
    crate::P2P_MANAGER.connected_peer_count().await
}

/// Check if we're already connected to a specific address
async fn is_already_connected(_peer_store: &Arc<PeerStore>, addr: &str) -> bool {
    // Match against the live connected peer keys.
    // Keys are normalized socket strings like "ip:port".
    let connected = crate::P2P_MANAGER.get_peer_addresses().await;

    if connected.iter().any(|c| c == addr) {
        return true;
    }

    // Canonicalize common forms (ip-only => default port) for safety.
    let normalized = if let Ok(sock) = addr.parse::<std::net::SocketAddr>() {
        sock.to_string()
    } else if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
        format!("{}:7072", ip)
    } else {
        addr.to_string()
    };

    connected.iter().any(|c| c == &normalized)
}

/// Prefilter: Should we attempt to dial this peer?
///
/// PATCH 3: Dial candidate prefilter to prevent wasted dial attempts
///
/// Returns false if:
/// - Peer is self (loopback)
/// - Already connected
/// - Port is invalid (0)
/// - IP failed validation for dialing
async fn should_dial(addr: &str) -> bool {
    use std::net::SocketAddr;

    // Parse the address
    let socket_addr: SocketAddr = match addr.parse() {
        Ok(sa) => sa,
        Err(_) => {
            // If we can't parse it as a full socket address, try as IP + default port
            match addr.parse::<std::net::IpAddr>() {
                Ok(ip) => match format!("{}:7072", ip).parse() {
                    Ok(sa) => sa,
                    Err(_) => return false,
                },
                Err(_) => return false,
            }
        }
    };

    // Skip invalid port
    if socket_addr.port() == 0 {
        return false;
    }

    // Skip loopback (self)
    if socket_addr.ip().is_loopback() {
        return false;
    }

    // Skip if already connected
    let connected = crate::P2P_MANAGER.get_peer_addresses().await;
    if connected.iter().any(|c| c == addr) {
        return false;
    }

    // Also check normalized form
    let normalized = if let Ok(sock) = addr.parse::<SocketAddr>() {
        sock.to_string()
    } else if let Ok(ip) = addr.parse::<std::net::IpAddr>() {
        format!("{}:7072", ip)
    } else {
        addr.to_string()
    };

    if connected.iter().any(|c| c == &normalized) {
        return false;
    }

    // Skip if IP validation fails for dialing
    let local_ips = crate::p2p::ip_filter::get_local_ips();
    if crate::p2p::ip_filter::validate_ip_for_dial(addr, &local_ips).is_some() {
        return false;
    }

    true
}
