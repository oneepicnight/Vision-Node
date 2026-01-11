//! P2P API endpoints for peer management and constellation status
//!
//! Provides REST endpoints to query peer state, connection quality,
//! and network health. Used by Vision Guard UI and node operators.

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::p2p::health_monitor::{HealthAlert, HealthMonitor, HealthScore};
use crate::p2p::peer_manager::{PeerBucket, PeerManager, PeerState};
use crate::p2p::upnp::UPNP_SUCCESS;
use crate::CHAIN;
use std::sync::atomic::Ordering;
use crate::globals::EBID_MANAGER; // Import EBID_MANAGER from globals

/// P2P API state with health monitoring
#[derive(Clone)]
pub struct P2PApiState {
    pub peer_manager: Arc<PeerManager>,
    pub health_monitor: Arc<HealthMonitor>,
}

/// Peer state enum for API responses
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ApiPeerState {
    Connected,
    Connecting,
    Disconnected,
    Failed,
    KnownOnly, // in memory/db, not currently trying
}

impl From<PeerState> for ApiPeerState {
    fn from(state: PeerState) -> Self {
        match state {
            PeerState::Connected => ApiPeerState::Connected,
            PeerState::Connecting => ApiPeerState::Connecting,
            PeerState::Disconnected => ApiPeerState::Disconnected,
            PeerState::Failed => ApiPeerState::Failed,
            PeerState::KnownOnly => ApiPeerState::KnownOnly,
        }
    }
}

/// Peer bucket classification for API responses
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ApiPeerBucket {
    Hot,
    Warm,
    Cold,
}

impl From<PeerBucket> for ApiPeerBucket {
    fn from(bucket: PeerBucket) -> Self {
        match bucket {
            PeerBucket::Hot => ApiPeerBucket::Hot,
            PeerBucket::Warm => ApiPeerBucket::Warm,
            PeerBucket::Cold => ApiPeerBucket::Cold,
        }
    }
}

/// Single peer information for API response
#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub ip: String,
    pub port: u16,
    pub ebid: String,
    pub state: ApiPeerState,
    pub bucket: ApiPeerBucket,
    pub last_seen: Option<u64>,  // unix timestamp
    pub latency_ms: Option<u32>, // last RTT if known
    pub failure_count: u32,
    pub score: f32, // 0.0 - 1.0, higher = better
}

/// Response for GET /p2p/peers endpoint
#[derive(Debug, Serialize)]
pub struct PeersResponse {
    pub peers: Vec<PeerInfo>,
    pub total_peers: usize,
    pub connected_peers: usize,
    pub hot_peers: usize,
    pub warm_peers: usize,
    pub cold_peers: usize,
}

/// Response for GET /constellation/status endpoint
#[derive(Debug, Serialize)]
pub struct ConstellationStatusResponse {
    pub local_ebid: String,

    pub total_known_peers: usize,
    pub connected_peers: usize,
    pub hot_peers: usize,
    pub warm_peers: usize,
    pub cold_peers: usize,

    pub last_successful_guardian_check: Option<u64>, // unix timestamp
    pub guardian_reachable: bool,
    pub guardian_ebid: Option<String>,

    pub avg_peer_latency_ms: Option<u32>,
    pub max_peer_latency_ms: Option<u32>,

    pub sync_height: u64, // local height
    pub network_estimated_height: u64,
    pub is_syncing: bool,

    pub last_peer_event_at: Option<u64>, // connect/disconnect/etc
    pub p2p_debug_mode: bool,            // from config

    // v1 anchor/leaf mode
    pub is_anchor: bool,
    pub public_reachable: bool,
    pub mode: String, // "ANCHOR", "ANCHOR_UNREACHABLE", or "LEAF"

    // P2P Health Status (P2P Robustness #7 UI)
    pub p2p_health: String, // "isolated" | "weak" | "ok" | "stable" | "immortal"

    // IPv4-only mode status
    pub ipv4_only_mode: bool,

    // Node identity (Fix D: Display in miner panel)
    pub node_id: Option<String>,
    pub node_pubkey: Option<String>,
    pub node_pubkey_fingerprint: Option<String>,
    // Consensus quorum fields (for ops debugging)
    pub compatible_peers: usize,
    pub incompatible_peers: usize,
    pub quorum_ok: bool,
    pub quorum_block_reason: Option<String>,
}

/// Response for GET /api/constellation/peers endpoint
#[derive(Debug, Serialize)]
pub struct ConstellationPeersResponse {
    pub active: usize,
    pub inbound: usize,
    pub outbound: usize,
    pub peers: Vec<ConstellationPeerInfo>,
}

/// Single constellation peer info for API response
#[derive(Debug, Serialize)]
pub struct ConstellationPeerInfo {
    pub vnode_tag: String,
    pub addr: String,
    pub direction: String, // "inbound" | "outbound"
}

/// GET /p2p/peers
///
/// Returns detailed information about all known peers including their
/// state, bucket classification, and connection quality metrics.
pub async fn get_peers(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    debug!("[P2P API] GET /p2p/peers");

    let peers = state.peer_manager.get_all_peers().await;

    let mut total_peers = 0;
    let mut connected_peers = 0;
    let mut hot_peers = 0;
    let mut warm_peers = 0;
    let mut cold_peers = 0;

    let peer_infos: Vec<PeerInfo> = peers
        .into_iter()
        .map(|peer| {
            total_peers += 1;

            if peer.state == PeerState::Connected {
                connected_peers += 1;
            }

            match peer.bucket {
                PeerBucket::Hot => hot_peers += 1,
                PeerBucket::Warm => warm_peers += 1,
                PeerBucket::Cold => cold_peers += 1,
            }

            PeerInfo {
                ip: peer.ip.clone(),
                port: peer.port,
                ebid: peer.ebid.clone(),
                state: peer.state.into(),
                bucket: peer.bucket.into(),
                last_seen: peer.metrics.last_seen,
                latency_ms: peer.metrics.latency_ms,
                failure_count: peer.metrics.failure_count,
                score: peer.metrics.score,
            }
        })
        .collect();

    let response = PeersResponse {
        peers: peer_infos,
        total_peers,
        connected_peers,
        hot_peers,
        warm_peers,
        cold_peers,
    };

    Json(response)
}

/// GET /constellation/status
///
/// Returns high-level constellation network health metrics including
/// peer distribution, sync status, and guardian connectivity.
pub async fn get_constellation_status(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    debug!("[P2P API] GET /constellation/status");

    // Get local EBID from EBID manager
    let local_ebid = {
        let ebid_mgr = EBID_MANAGER.lock();
        ebid_mgr.get_ebid().to_string()
    };

    // Get peer counts by bucket
    let peers = state.peer_manager.get_all_peers().await;
    let total_known_peers = peers.len();
    let connected_peers = peers
        .iter()
        .filter(|p| p.state == PeerState::Connected)
        .count();
    let hot_peers = peers.iter().filter(|p| p.bucket == PeerBucket::Hot).count();
    let warm_peers = peers
        .iter()
        .filter(|p| p.bucket == PeerBucket::Warm)
        .count();
    let cold_peers = peers
        .iter()
        .filter(|p| p.bucket == PeerBucket::Cold)
        .count();

    // Check guardian connectivity
    let (guardian_reachable, guardian_ebid) = state
        .peer_manager
        .get_guardian_status()
        .await
        .unwrap_or((false, None));

    let last_successful_guardian_check = state.peer_manager.get_last_guardian_check().await;

    // Calculate latency stats from connected peers
    let connected: Vec<_> = peers
        .iter()
        .filter(|p| p.state == PeerState::Connected && p.metrics.latency_ms.is_some())
        .collect();

    let avg_peer_latency_ms = if !connected.is_empty() {
        let sum: u32 = connected.iter().filter_map(|p| p.metrics.latency_ms).sum();
        Some(sum / connected.len() as u32)
    } else {
        None
    };

    let max_peer_latency_ms = connected.iter().filter_map(|p| p.metrics.latency_ms).max();

    // Get local chain height
    let sync_height = {
        let chain = CHAIN.lock();
        chain.blocks.len().saturating_sub(1) as u64
    };

    // Estimate network height from peer heights (simplified)
    let network_estimated_height = peers
        .iter()
        .filter_map(|p| p.height)
        .max()
        .unwrap_or(sync_height);

    // Consider syncing if we're more than 10 blocks behind
    let is_syncing = network_estimated_height.saturating_sub(sync_height) > 10;

    // Get last peer event timestamp
    let last_peer_event_at = state.peer_manager.get_last_peer_event().await;

    // Check if debug mode is enabled
    let p2p_debug_mode = std::env::var("P2P_DEBUG")
        .unwrap_or_default()
        .to_lowercase()
        == "true";

    // Determine anchor/leaf status dynamically
    // 1. Check if manually set via env var (overrides auto-detection)
    let manual_is_anchor = std::env::var("P2P_IS_ANCHOR")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .ok();

    // 2. Check if publicly reachable (UPnP success or manual port forward)
    let upnp_successful = UPNP_SUCCESS.load(Ordering::SeqCst);
    let manual_public = std::env::var("P2P_PUBLIC_REACHABLE")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .ok();

    // Use manual override if set, otherwise use UPnP status
    let public_reachable = manual_public.unwrap_or(upnp_successful);

    // 3. Auto-detect anchor candidacy based on:
    //    - Good peer connections (8+ peers = stable network presence)
    //    - Public reachability (can accept incoming connections)
    let auto_promoted_anchor = connected_peers >= 8 && public_reachable;

    // Final anchor determination: manual override OR auto-promotion
    let is_anchor = manual_is_anchor.unwrap_or(auto_promoted_anchor);

    let mode = if is_anchor && public_reachable {
        "ANCHOR".to_string()
    } else if is_anchor {
        "ANCHOR_UNREACHABLE".to_string()
    } else {
        "LEAF".to_string()
    };

    // Calculate P2P health status (P2P Robustness #7 UI)
    let p2p_health = if connected_peers == 0 {
        "isolated".to_string()
    } else if connected_peers == 1 {
        "weak".to_string()
    } else if (2..8).contains(&connected_peers) {
        "ok".to_string()
    } else if (8..32).contains(&connected_peers) {
        "stable".to_string()
    } else {
        // 32+ peers = IMMORTAL mode 🎉
        "immortal".to_string()
    };

    // Load P2P config to check IPv4-only mode
    let p2p_config = crate::config::p2p::P2pConfig::load_or_create("p2p.json").unwrap_or_default();

    // Fix D: Get node identity for miner panel display
    let (node_id, node_pubkey, node_pubkey_fingerprint) =
        if let Some(identity_arc) = crate::identity::node_id::NODE_IDENTITY.get() {
            let guard = identity_arc.read();
            let node_id = Some(guard.node_id.clone());
            let node_pubkey = Some(guard.pubkey_b64.clone());
            let fingerprint = Some(guard.fingerprint());
            (node_id, node_pubkey, fingerprint)
        } else {
            (None, None, None)
        };

    // Get consensus quorum for ops debugging
    let quorum = state.peer_manager.consensus_quorum().await;
    
    // Determine quorum status for operators
    const MIN_PEERS_FOR_SYNC: usize = 3;
    let quorum_ok = quorum.compatible_peers >= MIN_PEERS_FOR_SYNC;
    
    let quorum_block_reason = if !quorum_ok {
        if connected_peers == 0 {
            Some("no_peers".to_string())
        } else if quorum.compatible_peers < MIN_PEERS_FOR_SYNC {
            Some(format!("need_{}_compatible (have {} connected, {} compatible, {} incompatible)", 
                MIN_PEERS_FOR_SYNC, connected_peers, quorum.compatible_peers, quorum.incompatible_peers))
        } else {
            Some("unknown".to_string())
        }
    } else {
        // Check height spread
        if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
            const MAX_HEIGHT_SPREAD: u64 = 100;
            let spread = max_h.saturating_sub(min_h);
            if spread > MAX_HEIGHT_SPREAD {
                Some(format!("height_spread_too_large ({} blocks)", spread))
            } else {
                None // All good!
            }
        } else {
            None
        }
    };

    let response = ConstellationStatusResponse {
        local_ebid,
        total_known_peers,
        connected_peers,
        hot_peers,
        warm_peers,
        cold_peers,
        last_successful_guardian_check,
        guardian_reachable,
        guardian_ebid,
        avg_peer_latency_ms,
        max_peer_latency_ms,
        sync_height,
        network_estimated_height,
        is_syncing,
        last_peer_event_at,
        p2p_debug_mode,
        is_anchor,
        public_reachable,
        mode,
        p2p_health,
        ipv4_only_mode: p2p_config.force_ipv4,
        node_id,
        node_pubkey,
        node_pubkey_fingerprint,
        compatible_peers: quorum.compatible_peers,
        incompatible_peers: quorum.incompatible_peers,
        quorum_ok,
        quorum_block_reason,
    };

    Json(response)
}

/// GET /p2p/health
///
/// Returns network health score and active alerts
pub async fn get_health(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    debug!("[P2P API] GET /p2p/health");

    // Get health score
    let score = state.health_monitor.calculate_health_score().await;

    // Get recent alerts (last 10)
    let alerts = state.health_monitor.get_alert_history(10).await;

    #[derive(Serialize)]
    struct HealthResponse {
        score: HealthScore,
        alerts: Vec<HealthAlert>,
        status: String,
    }

    let status = if score.overall >= 80 {
        "healthy"
    } else if score.overall >= 60 {
        "degraded"
    } else if score.overall >= 40 {
        "unhealthy"
    } else {
        "critical"
    };

    let response = HealthResponse {
        score,
        alerts,
        status: status.to_string(),
    };

    Json(response)
}

/// GET /p2p/alerts
///
/// Returns recent health alerts
pub async fn get_alerts(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    debug!("[P2P API] GET /p2p/alerts");

    let alerts = state.health_monitor.get_alert_history(50).await;

    #[derive(Serialize)]
    struct AlertsResponse {
        alerts: Vec<HealthAlert>,
        count: usize,
    }

    let response = AlertsResponse {
        count: alerts.len(),
        alerts,
    };

    Json(response)
}

/// GET /api/constellation/peers
///
/// Returns list of actively connected constellation peers with their
/// node tags, addresses, and connection direction.
pub async fn get_constellation_peers(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    debug!("[P2P API] GET /api/constellation/peers");

    let snap = state.peer_manager.snapshot().await;

    let peers: Vec<ConstellationPeerInfo> = snap
        .active_peers
        .into_iter()
        .map(|p| ConstellationPeerInfo {
            vnode_tag: p.vnode_tag,
            addr: p.addr.to_string(),
            direction: if p.is_inbound {
                "inbound".to_string()
            } else {
                "outbound".to_string()
            },
        })
        .collect();

    Json(ConstellationPeersResponse {
        active: peers.len(),
        inbound: snap.inbound_count,
        outbound: snap.outbound_count,
        peers,
    })
}

/// Create P2P API router with health monitoring
pub fn p2p_api_router(peer_manager: Arc<PeerManager>) -> Router {
    // Create health monitor
    let health_monitor = Arc::new(HealthMonitor::new(peer_manager.clone()));

    // Start background monitoring
    health_monitor.clone().start_monitoring();

    let state = Arc::new(P2PApiState {
        peer_manager,
        health_monitor,
    });

    Router::new()
        .route("/p2p/peers", get(get_peers))
        .route("/constellation/status", get(get_constellation_status))
        .route("/constellation/peers", get(get_constellation_peers))
        .route("/p2p/health", get(get_health))
        .route("/p2p/alerts", get(get_alerts))
        .route("/p2p/peers/status", get(get_peer_book_status))
        .route("/p2p/seed_peers", get(get_seed_peers))
        .route("/p2p/debug", get(get_p2p_debug_info)) // Add debug endpoint with state
        .with_state(state)
}

/// GET /p2p/peers/status
///
/// Returns rolling 1000-peer mesh statistics including total peers,
/// seeds, average health, and top performing peers.
pub async fn get_peer_book_status() -> impl IntoResponse {
    use crate::p2p::peer_store::PeerStore;

    debug!("[P2P API] GET /p2p/peers/status");

    // Access peer book from global CHAIN
    let chain = CHAIN.lock();
    let peer_store = PeerStore::new(&chain.db).ok();
    drop(chain);

    if let Some(store) = peer_store {
        let stats = store.get_stats();
        Json(serde_json::json!({
            "total": stats.total,
            "seeds": stats.seeds,
            "avg_health": format!("{:.1}", stats.avg_health),
            "top_sample": stats.top_sample
        }))
    } else {
        Json(serde_json::json!({
            "error": "Peer book not available"
        }))
    }
}

/// GET /p2p/seed_peers
///
/// Returns a list of healthy peers for HTTP-based seed distribution.
/// Used by new nodes to bootstrap without relying on P2P gossip.
///
/// Returns up to 64 peers prioritizing anchors first, then by health score.
/// Each peer includes P2P address (ip:port) and anchor flag.
pub async fn get_seed_peers() -> impl IntoResponse {
    use crate::p2p::peer_store::PeerStore;

    debug!("[P2P API] GET /p2p/seed_peers");

    // Access peer book from global CHAIN
    let chain = CHAIN.lock();
    let peer_store = PeerStore::new(&chain.db).ok();
    drop(chain);

    if let Some(store) = peer_store {
        let stats = store.get_stats();
        let seed_peers = store.export_seed_peers(64); // Cap at 64 peers

        info!(
            "[P2P API] seed_peers: store_total={} returning={} anchors={}",
            stats.total,
            seed_peers.len(),
            seed_peers.iter().filter(|p| p.is_anchor).count()
        );

        Json(seed_peers)
    } else {
        warn!("[P2P API] Peer book not available for seed export");
        Json(vec![])
    }
}

/// P2P debug information for diagnosing connection issues
#[derive(Debug, Serialize)]
pub struct P2pDebugInfo {
    // Configuration
    pub node_build: String,
    pub node_version: u32,
    pub protocol_version: u32,
    pub chain_id: String,
    pub bootstrap_prefix: String,
    pub bootstrap_checkpoint_height: u64,
    pub bootstrap_checkpoint_hash: String,
    pub min_protocol: u32,
    pub max_protocol: u32,
    pub min_node_version: String,
    pub advertised_p2p: Option<String>,
    pub p2p_port: u16,
    pub debug_allow_all: bool,

    // Runtime state (new)
    pub connected_peers: Vec<DebugConnectedPeer>,
    pub peer_book_counts: DebugPeerBookCounts,
    pub dial_failures: Vec<DebugDialFailure>,
}

/// Debug info for a connected peer
#[derive(Debug, Serialize, Clone)]
pub struct DebugConnectedPeer {
    pub addr: String, // ip:port (P2P port, not HTTP)
    pub ebid: String,
    pub validated: bool,
    pub last_handshake_ok: bool,
    pub last_block_relay_unix: Option<u64>,
    pub last_compact_fetch_result: Option<String>,
    pub latency_ms: Option<u32>,
    pub connection_age_secs: u64,
}

/// Peer book statistics
#[derive(Debug, Serialize, Clone)]
pub struct DebugPeerBookCounts {
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
    pub total: usize,
}

/// Recent dial failure with reason
#[derive(Debug, Serialize, Clone)]
pub struct DebugDialFailure {
    pub addr: String,
    pub reason: String,
    pub timestamp_unix: u64,
    pub source: String, // "seed", "gossip", "handshake"
}

/// GET /api/p2p/debug - Get P2P configuration and runtime state for debugging
///
/// Returns all the values used in handshake validation so operators can
/// quickly compare two nodes and identify mismatches. Also includes runtime
/// connection state for debugging P2P issues.
pub async fn get_p2p_debug_info(State(state): State<Arc<P2PApiState>>) -> impl IntoResponse {
    use crate::vision_constants::{
        BOOTSTRAP_CHECKPOINT_HASH, BOOTSTRAP_CHECKPOINT_HEIGHT, VISION_BOOTSTRAP_PREFIX,
        VISION_MAX_PROTOCOL_VERSION, VISION_MIN_NODE_VERSION, VISION_MIN_PROTOCOL_VERSION,
    };

    // Canonical chain ID (deterministic, no runtime network selection)
    let chain_id = crate::vision_constants::expected_chain_id();

    // Get advertised P2P address
    let advertised = crate::ADVERTISED_P2P_ADDRESS.lock().clone();

    // Get P2P port
    let p2p_port = std::env::var("VISION_P2P_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7072);

    // Check debug flag
    let debug_allow_all = std::env::var("VISION_P2P_DEBUG_ALLOW_ALL").is_ok();

    // RUNTIME STATE: Get connected peers
    let peers = state.peer_manager.connected_peers().await;
    let connected_peers: Vec<DebugConnectedPeer> = peers
        .iter()
        .map(|p| {
            let connection_age_secs = p
                .metrics
                .last_seen
                .map(|t| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .saturating_sub(t)
                })
                .unwrap_or(0);

            DebugConnectedPeer {
                addr: format!("{}:{}", p.ip, p.port),
                ebid: p.ebid.clone(),
                validated: p.chain_id.is_some() && p.bootstrap_prefix.is_some(), // Has chain identity
                last_handshake_ok: p.state == crate::p2p::peer_manager::PeerState::Connected,
                last_block_relay_unix: p.metrics.last_seen,
                last_compact_fetch_result: None, // TODO: track this
                latency_ms: p.metrics.latency_ms,
                connection_age_secs,
            }
        })
        .collect();

    // RUNTIME STATE: Get peer book counts
    let all_peers = state.peer_manager.get_all_peers().await;
    let hot = all_peers
        .iter()
        .filter(|p| matches!(p.bucket, crate::p2p::peer_manager::PeerBucket::Hot))
        .count();
    let warm = all_peers
        .iter()
        .filter(|p| matches!(p.bucket, crate::p2p::peer_manager::PeerBucket::Warm))
        .count();
    let cold = all_peers
        .iter()
        .filter(|p| matches!(p.bucket, crate::p2p::peer_manager::PeerBucket::Cold))
        .count();

    let peer_book_counts = DebugPeerBookCounts {
        hot,
        warm,
        cold,
        total: all_peers.len(),
    };

    // RUNTIME STATE: Get recent dial failures from tracker
    let dial_failures = crate::p2p::dial_tracker::get_dial_failures()
        .into_iter()
        .map(|f| DebugDialFailure {
            addr: f.addr,
            reason: f.reason,
            timestamp_unix: f.timestamp_unix,
            source: f.source,
        })
        .collect();

    let info = P2pDebugInfo {
        node_build: crate::p2p::connection::NODE_BUILD_TAG.to_string(),
        node_version: crate::p2p::connection::VISION_NODE_VERSION,
        protocol_version: crate::p2p::connection::VISION_P2P_PROTOCOL_VERSION,
        chain_id,
        bootstrap_prefix: VISION_BOOTSTRAP_PREFIX.to_string(),
        bootstrap_checkpoint_height: BOOTSTRAP_CHECKPOINT_HEIGHT,
        bootstrap_checkpoint_hash: BOOTSTRAP_CHECKPOINT_HASH.to_string(),
        min_protocol: VISION_MIN_PROTOCOL_VERSION,
        max_protocol: VISION_MAX_PROTOCOL_VERSION,
        min_node_version: VISION_MIN_NODE_VERSION.to_string(),
        advertised_p2p: advertised,
        p2p_port,
        debug_allow_all,
        connected_peers,
        peer_book_counts,
        dial_failures,
    };

    Json(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_peer_state_conversion() {
        assert_eq!(
            ApiPeerState::from(PeerState::Connected),
            ApiPeerState::Connected
        );
        assert_eq!(
            ApiPeerState::from(PeerState::KnownOnly),
            ApiPeerState::KnownOnly
        );
    }

    #[test]
    fn test_api_peer_bucket_conversion() {
        assert_eq!(ApiPeerBucket::from(PeerBucket::Hot), ApiPeerBucket::Hot);
        assert_eq!(ApiPeerBucket::from(PeerBucket::Warm), ApiPeerBucket::Warm);
    }
}
