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
use crate::p2p::peer_connect_report::{PeerConnectReport, PeerConnectReason};

/// Connection maintainer statistics
#[derive(Debug, Clone, Default)]
pub struct MaintainerStats {
    pub checks_performed: u64,
    pub seeds_tried: u64,
    pub store_peers_tried: u64,
    pub connections_established: u64,
    pub last_check_time: u64,
    pub seed_cursor: usize, // Round-robin position in seed list
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

        // Initialize connection report for this cycle
        // Use peer store scope (matches chain scope)
        let scope = peer_store.get_scope().to_string();
        let mut report = PeerConnectReport::new(scope, cfg.max_outbound_connections);

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
            
            // Write report even when fully meshed
            report.set_final_connected(connected);
            if let Ok(data_dir) = std::path::PathBuf::from("./vision_data_7070").canonicalize() {
                let public_dir = data_dir.join("public");
                if let Err(e) = report.write_all(&public_dir) {
                    warn!("[CONN_REPORT] Failed to write diagnostics: {}", e);
                }
            }
            
            continue;
        }

        // Warn if we hit the configured max but still have unconnected peers
        if connected >= cfg.max_outbound_connections {
            warn!(
                "[CONN_MAINTAINER] ⚠️ Hit max_outbound_connections ({}) but {} unconnected peers remain. Consider increasing VISION_MAX_PEERS.",
                cfg.max_outbound_connections,
                peer_store_count - connected
            );
            
            // Write report
            report.set_final_connected(connected);
            if let Ok(data_dir) = std::path::PathBuf::from("./vision_data_7070").canonicalize() {
                let public_dir = data_dir.join("public");
                if let Err(e) = report.write_all(&public_dir) {
                    warn!("[CONN_REPORT] Failed to write diagnostics: {}", e);
                }
            }
            
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

        // === SMART DIAL SELECTION WITH BACKOFF & ROTATION ===
        
        // Prepare seed candidates with round-robin rotation
        let mut seeds = cfg.seed_peers.clone();
        
        // Start from seed_cursor (round-robin fairness)
        if stats.seed_cursor >= seeds.len() && !seeds.is_empty() {
            stats.seed_cursor = 0; // Wrap around
        }
        
        // Rotate seeds so we start from cursor position
        if !seeds.is_empty() && stats.seed_cursor < seeds.len() {
            seeds.rotate_left(stats.seed_cursor);
        }
        
        // Prepare peer store candidates (filtered and sorted by health)
        let store_peers = peer_store.get_all();
        let connected_peers = crate::P2P_MANAGER.get_peer_addresses().await;
        
        // Track already-connected peers in report
        for connected_addr in &connected_peers {
            if let Ok(sock_addr) = connected_addr.parse() {
                report.add_reason(PeerConnectReason::AlreadyConnected, &sock_addr);
            }
        }
        
        let mut candidates: Vec<_> = store_peers
            .into_iter()
            .filter(|p| {
                // Only try peers with some health and a valid IP
                if p.health_score == 0 || p.ip_address.is_none() {
                    // Track peers with zero health
                    if p.health_score == 0 {
                        if let Some(ref addr) = p.ip_address {
                            if let Ok(sock_addr) = addr.parse() {
                                report.add_reason(PeerConnectReason::PeerUnhealthy, &sock_addr);
                            }
                        }
                    }
                    // Track peers with invalid addresses
                    if p.ip_address.is_none() {
                        // Can't add to report without address
                    }
                    return false;
                }
                // ✅ CRITICAL: Don't include already-connected peers in candidates
                // This was wasting dial slots on "recent_success" peers we're already talking to
                if let Some(ref addr) = p.ip_address {
                    if connected_peers.iter().any(|c| c == addr) {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Shuffle candidates for fair round-robin selection
        {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            candidates.shuffle(&mut rng);
        }

        // Sort by health score descending (shuffle breaks ties)
        candidates.sort_by(|a, b| b.health_score.cmp(&a.health_score));

        // === BACKOFF FILTERING ===
        // Remove peers that are in cooldown or quarantined
        let seeds_before_filter = seeds.len();
        let peers_before_filter = candidates.len();
        
        seeds.retain(|addr| {
            if crate::p2p::dial_tracker::is_peer_in_cooldown(addr) {
                if let Some(backoff) = crate::p2p::dial_tracker::get_peer_backoff(addr) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    debug!(
                        "[BACKOFF] Skipping seed {} (cooldown: {}s remaining, streak: {})",
                        addr,
                        backoff.cooldown_remaining(now),
                        backoff.fail_streak
                    );
                    
                    // Record in report
                    if let Ok(sock_addr) = addr.parse() {
                        report.add_reason(PeerConnectReason::CooldownActive, &sock_addr);
                    }
                }
                return false;
            }
            if crate::p2p::dial_tracker::should_quarantine_peer(addr) {
                debug!("[QUARANTINE] Skipping seed {} (quarantined)", addr);
                if let Ok(sock_addr) = addr.parse() {
                    report.add_reason(PeerConnectReason::PeerBanned, &sock_addr);
                }
                return false;
            }
            true
        });
        
        candidates.retain(|p| {
            if let Some(ref addr) = p.ip_address {
                if crate::p2p::dial_tracker::is_peer_in_cooldown(addr) {
                    if let Ok(sock_addr) = addr.parse() {
                        report.add_reason(PeerConnectReason::CooldownActive, &sock_addr);
                    }
                    return false;
                }
                if crate::p2p::dial_tracker::should_quarantine_peer(addr) {
                    if let Ok(sock_addr) = addr.parse() {
                        report.add_reason(PeerConnectReason::PeerBanned, &sock_addr);
                    }
                    return false;
                }
            }
            true
        });
        
        let seeds_skipped = seeds_before_filter.saturating_sub(seeds.len());
        let peers_skipped = peers_before_filter.saturating_sub(candidates.len());
        
        // Calculate dial limit for this cycle
        let dial_limit = slots_available.min(MAX_NEW_DIALS_PER_CYCLE).max(1);
        
        // === COMPREHENSIVE CYCLE LOGGING ===
        info!(
            "[DIAL_CYCLE] 📊 Seeds: {}/{} available ({} in cooldown), PeerStore: {}/{} available ({} in cooldown), Slots: {}, Will attempt: {}",
            seeds.len(),
            seeds_before_filter,
            seeds_skipped,
            candidates.len(),
            peers_before_filter,
            peers_skipped,
            slots_available,
            dial_limit
        );
        
        // Log first 5 chosen peers with selection reason
        let mut chosen_preview: Vec<String> = Vec::new();
        for (idx, seed) in seeds.iter().take(5).enumerate() {
            chosen_preview.push(format!("{}. {} (seed_rotation)", idx + 1, seed));
        }
        for (idx, peer) in candidates.iter().take(5 - chosen_preview.len().min(5)).enumerate() {
            if let Some(ref addr) = peer.ip_address {
                let reason = if peer.health_score >= 80 {
                    "high_health"
                } else if crate::p2p::dial_tracker::get_peer_backoff(addr)
                    .map(|b| b.total_successes > 0)
                    .unwrap_or(false)
                {
                    "recent_success"
                } else {
                    "new_peer"
                };
                chosen_preview.push(format!(
                    "{}. {} ({})",
                    chosen_preview.len() + 1,
                    peer.node_tag,
                    reason
                ));
            }
        }
        
        if !chosen_preview.is_empty() {
            info!("[DIAL_CYCLE] 🎯 Top candidates:\n{}", chosen_preview.join("\n"));
        }

        // Interleave: Try 2 seeds, then 2 peer store peers, repeat
        let mut seed_idx = 0;
        let mut peer_idx = 0;
        let mut dials_this_cycle = 0usize;
        let mut successful_connections = 0;
        
        while dials_this_cycle < dial_limit {
            let mut made_attempt = false;
            
            // Try 2 seeds
            for _ in 0..2 {
                if seed_idx >= seeds.len() || dials_this_cycle >= dial_limit {
                    break;
                }
                
                let seed_addr = &seeds[seed_idx];
                seed_idx += 1;
                stats.seeds_tried += 1;

                // Skip seeds in cooldown
                if is_seed_in_cooldown(seed_addr) {
                    debug!("[BOOTSTRAP] ⏳ Skipping seed in cooldown: {}", seed_addr);
                    continue;
                }

                // Prefilter candidates
                if !should_dial(seed_addr).await {
                    debug!("[DIAL] SKIP seed peer (prefilter): {}", seed_addr);
                    continue;
                }

                info!(
                    "[BOOTSTRAP] 🔁 Trying seed {}/{}: {}",
                    seed_idx,
                    seeds.len(),
                    seed_addr
                );

                let p2p_manager = std::sync::Arc::clone(&crate::P2P_MANAGER);
                dials_this_cycle += 1;
                made_attempt = true;
                report.record_attempt();
                
                match p2p_manager.connect_to_peer(seed_addr.clone()).await {
                    Ok(_) => {
                        stats.connections_established += 1;
                        successful_connections += 1;
                        report.record_connected();
                        info!("[CONN_MAINTAINER] ✅ Connected to seed: {}", seed_addr);
                        
                        // Record success in backoff tracker (resets fail streak)
                        crate::p2p::dial_tracker::record_dial_success(seed_addr.clone());
                        
                        // Advance seed cursor for round-robin fairness
                        stats.seed_cursor = (stats.seed_cursor + 1) % seeds_before_filter.max(1);

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
                        debug!(
                            target: "p2p::connect",
                            seed = %seed_addr,
                            reason = %e,
                            "Seed connection attempt failed"
                        );
                        
                        // Record failure reason in report
                        if let Ok(sock_addr) = seed_addr.parse() {
                            let reason = classify_dial_error(&e.to_string());
                            report.add_reason(reason, &sock_addr);
                        }
                        
                        crate::p2p::dial_tracker::record_dial_failure(
                            seed_addr.clone(),
                            e.to_string(),
                            "seed".to_string(),
                        );
                    }
                }
            }
            
            // Try 2 peer store peers
            for _ in 0..2 {
                if peer_idx >= candidates.len() || dials_this_cycle >= dial_limit {
                    break;
                }
                
                let peer = &candidates[peer_idx];
                peer_idx += 1;
                stats.store_peers_tried += 1;

                let addr = match peer.ip_address {
                    Some(ref a) => a.clone(),
                    None => continue,
                };

                // Prefilter candidates
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
                dials_this_cycle += 1;
                made_attempt = true;
                report.record_attempt();
                
                match p2p_manager.connect_to_peer(addr.clone()).await {
                    Ok(_) => {
                        stats.connections_established += 1;
                        successful_connections += 1;
                        report.record_connected();
                        info!(
                            "[CONN_MAINTAINER] ✅ Connected to stored peer: {}",
                            peer.node_tag
                        );
                        
                        // Record success in backoff tracker (resets fail streak)
                        crate::p2p::dial_tracker::record_dial_success(addr.clone());

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
                        debug!(
                            target: "p2p::connect",
                            peer = %peer.node_tag,
                            addr = %addr,
                            reason = %e,
                            "Stored peer connection attempt failed"
                        );
                        
                        // Record failure reason in report
                        if let Ok(sock_addr) = addr.parse() {
                            let reason = classify_dial_error(&e.to_string());
                            report.add_reason(reason, &sock_addr);
                        }
                        
                        crate::p2p::dial_tracker::record_dial_failure(
                            addr.clone(),
                            e.to_string(),
                            "peer_book".to_string(),
                        );
                    }
                }
            }
            
            // If we've exhausted both seeds and peers, break
            if !made_attempt && seed_idx >= seeds.len() && peer_idx >= candidates.len() {
                break;
            }
        }

        // Check if we've reached max capacity after all attempts
        let connected_after_attempts = count_connected_peers(&peer_store).await;
        if connected_after_attempts >= cfg.max_outbound_connections {
            info!(
                "[CONN_MAINTAINER] Max capacity reached: {} peers",
                connected_after_attempts
            );
            continue;
        }

        // Phase 2 is now integrated above with seeds (interleaved)
        
        let final_count = count_connected_peers(&peer_store).await;
        let final_store_count = peer_store.get_all().len();
        let unconnected = final_store_count.saturating_sub(final_count);
        
        // Update report with final counts
        report.set_final_connected(final_count);
        
        // === CYCLE SUMMARY ===
        info!(
            "[CYCLE_SUMMARY] 📊 Attempted: {}, Connected: {}, Skipped cooldown: {} seeds + {} peers, Final: {}/{} peers",
            dials_this_cycle,
            successful_connections,
            seeds_skipped,
            peers_skipped,
            final_count,
            cfg.max_outbound_connections
        );

        // Write connection diagnostics report
        if let Ok(data_dir) = std::path::PathBuf::from("./vision_data_7070").canonicalize() {
            let public_dir = data_dir.join("public");
            match report.write_all(&public_dir) {
                Ok(()) => {
                    debug!(
                        "[CONN_REPORT] ✅ Wrote diagnostics to {}",
                        public_dir.display()
                    );
                }
                Err(e) => {
                    warn!("[CONN_REPORT] Failed to write diagnostics: {}", e);
                }
            }
        } else {
            debug!("[CONN_REPORT] Could not resolve vision_data_7070 path for diagnostics");
        }

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
/// Check if a seed is in cooldown period (recently failed)
fn is_seed_in_cooldown(addr: &str) -> bool {
    let failures = crate::p2p::dial_tracker::get_dial_failures();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // 60-second cooldown for failed seeds
    const COOLDOWN_SECONDS: u64 = 60;
    
    failures.iter().any(|f| {
        f.addr == addr && (now - f.timestamp_unix < COOLDOWN_SECONDS)
    })
}

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

    // Skip loopback (self) unless in local test mode
    if socket_addr.ip().is_loopback() {
        // Allow loopback in local test mode for testing
        let allow_loopback = std::env::var("VISION_LOCAL_TEST")
            .ok()
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false)
            || std::env::var("VISION_ALLOW_PRIVATE_PEERS")
                .ok()
                .map(|v| v.to_lowercase() == "true")
                .unwrap_or(false);
        
        if !allow_loopback {
            return false;
        }
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

/// Classify dial error into a reason category
fn classify_dial_error(error: &str) -> PeerConnectReason {
    let lower = error.to_lowercase();
    
    if lower.contains("timeout") || lower.contains("timed out") {
        PeerConnectReason::DialTimeout
    } else if lower.contains("refused") || lower.contains("connection refused") {
        PeerConnectReason::DialRefused
    } else if lower.contains("no route") || lower.contains("unreachable") {
        PeerConnectReason::NoRouteToHost
    } else if lower.contains("handshake") && lower.contains("timeout") {
        PeerConnectReason::HandshakeTimeout
    } else if lower.contains("incompatible") || lower.contains("chain") {
        PeerConnectReason::HandshakeFailed_IncompatibleChain
    } else if lower.contains("version") {
        PeerConnectReason::HandshakeFailed_Version
    } else if lower.contains("chain_id") || lower.contains("chainid") {
        PeerConnectReason::HandshakeFailed_ChainId
    } else if lower.contains("handshake") {
        PeerConnectReason::HandshakeFailed_Other
    } else {
        PeerConnectReason::DialError
    }
}
