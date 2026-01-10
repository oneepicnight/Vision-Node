// Website API handlers integrated with actual node state
// Provides real-time data from blockchain, peers, and guardian systems

use axum::{extract::Query, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import node globals
use crate::{CHAIN, GUARDIAN_CONSCIOUSNESS, HEALTH_DB, PEERS};

// Import upstream module for Guardian HTTP follower mode

// ===== RESPONSE TYPES =====

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub live: bool,
    pub chain_height: u64,
    pub peer_count: usize,
    pub version: String,
    pub uptime_seconds: u64,
    // Mining lockout fields
    pub sync_status: String,
    pub can_mine: bool,
    pub sync_height: u64,
    pub network_estimated_height: u64,
    pub connected_peers: usize,
    // Slow peer tracking metrics
    pub behind_blocks: i64,
    pub slow_peers: usize,
    pub avg_peer_lag_blocks: f32,
    // Node identity
    pub node_id: String,
    pub node_pubkey: String,
    pub node_pubkey_fingerprint: String,
    pub wallet_address: Option<String>,
    // Node approval status
    pub approved: bool,
    pub approved_wallet: Option<String>,
    // Exchange readiness (anchor at tip with lag=0)
    pub exchange_ready: bool,
    // Auto-detected node role (Anchor or Edge)
    pub node_role: String,
    // HTTP backbone (7070) status
    pub http_backbone: HttpBackboneStatus,
    // Exchange endpoints availability
    pub exchange_ok: bool,
    // Legacy aliases for panel.html compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mempool: Option<usize>,
    // Mining eligibility (v3.0 approval-based gating)
    pub mining_eligible: bool,
    pub mining_blocked_reason: Option<String>,
    // P2P health and peer counts
    pub total_known_peers: usize,
    pub p2p_health: String,

    // Transport/reporting
    pub network_mode: String,
    pub transport: String,
    pub backbone_reachable: bool,

    // Warmup window (rewards disabled)
    pub warmup_active: bool,
    pub warmup_remaining_blocks: u64,

    // Chain identity (drop quarantine)
    pub chain_id: String,
    pub drop_prefix: String,
}

#[derive(Serialize, Deserialize)]
pub struct HttpBackboneStatus {
    pub connected: bool,
    pub anchor: Option<String>,
    pub latency_ms: Option<u64>,
    pub tip_height: Option<u64>,
    pub tip_hash: Option<String>,
    pub last_ok_unix: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
pub struct GuardianResponse {
    pub enabled: bool,
    pub active: bool,
    pub recent_actions: Vec<GuardianAction>,
}

#[derive(Serialize)]
pub struct GuardianAction {
    pub action_id: String,
    pub action_type: String,
    pub target: String,
    pub timestamp: u64,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChainStatusResponse {
    pub height: u64,
    pub latest_block_hash: String,
    pub difficulty: u64,
}

#[derive(Serialize, Deserialize)]
pub struct HealthPublicResponse {
    pub network_health: NetworkHealth,
}

#[derive(Serialize, Deserialize)]
pub struct NetworkHealth {
    pub mood: String,
    pub prediction: String,
    pub confidence: f64,
    pub incidents: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BeaconState {
    pub timestamp: u64,
    pub mood: String,
    pub score: f64,
    pub reason: String,
    pub chain_height: u64,
    pub peer_count: usize,
    pub mempool_size: usize,
    pub guardian_active: bool,
    // Wallet/command center expects this for globe visualization
    pub peers: Vec<String>,
}

#[derive(Serialize)]
pub struct DownloadsResponse {
    pub total_downloads: u64,
    pub unique_visitors: u64,
}

// ===== HANDLERS WITH REAL DATA =====

pub async fn get_status() -> Json<StatusResponse> {
    use crate::auto_sync::SyncHealthSnapshot;

    // Get guardian mode status
    let _guardian_mode = false; // All nodes are constellation nodes

    // FREEZE FIX: Use async version directly - no blocking, no timeout needed
    let snapshot = match tokio::time::timeout(
        std::time::Duration::from_millis(100),
        SyncHealthSnapshot::current_async(),
    )
    .await
    {
        Ok(snap) => snap,
        Err(_) => {
            // Timeout - return basic snapshot with local data only
            let local_height = {
                let chain = crate::CHAIN.lock();
                chain.blocks.len() as u64
            };
            SyncHealthSnapshot {
                sync_height: local_height,
                network_estimated_height: local_height,
                network_tip_hash: String::new(),
                connected_peers: 0,
                is_syncing: false,
                chain_id_matches: true,
                is_too_far_ahead: false,
                behind_blocks: 0,
                slow_peers: 0,
                avg_peer_lag_blocks: 0.0,
                public_reachable: None,
                anchor_sampled: 0,
            }
        }
    };

    // Peer-count truth source: validated connected peers
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await;

    // Transport selection for status UX
    let backbone_state = crate::control_plane::get_backbone_state();
    let peer_floor = crate::mining_readiness::MAINNET_MIN_PEERS_FOR_MINING as usize;
    let transport = if peer_count >= peer_floor {
        "p2p".to_string()
    } else if backbone_state.connected {
        "http_fallback".to_string()
    } else {
        "offline".to_string()
    };

    let network_mode = if crate::vision_constants::pure_swarm_mode() {
        "pure_swarm".to_string()
    } else {
        "normal".to_string()
    };

    // Local chain height - use canonical_head() for consistent tip across all subsystems
    let (chain_height, chain_tip_hash, chain_tip_work) = {
        let chain = CHAIN.lock();
        chain.canonical_head()
    };

    let warmup_active = chain_height < crate::vision_constants::WARMUP_BLOCKS;
    let warmup_remaining_blocks = if warmup_active {
        crate::vision_constants::WARMUP_BLOCKS.saturating_sub(chain_height)
    } else {
        0
    };

    // Determine sync status
    let sync_status = if snapshot.is_syncing {
        "syncing".to_string()
    } else if !snapshot.chain_id_matches {
        "incompatible".to_string()
    } else if snapshot.sync_height + 2 < snapshot.network_estimated_height {
        "behind".to_string()
    } else {
        "ready".to_string()
    };

    // Check if can mine (mining lockout logic)
    let can_mine = !snapshot.is_syncing
        && snapshot.connected_peers >= 2
        && snapshot.chain_id_matches
        && snapshot.sync_height + 1 >= snapshot.network_estimated_height
        && !snapshot.is_too_far_ahead;

    // Calculate uptime (static start time would be tracked elsewhere, using placeholder)
    let uptime_seconds = 3600; // TODO: Track actual start time

    // Fix D: Get node identity and wallet address
    let (node_id, node_pubkey, node_pubkey_fingerprint, wallet_address) = {
        let chain = CHAIN.lock();

        // Use NODE_IDENTITY directly (already initialized in main)
        let (node_id, node_pubkey, node_pubkey_fingerprint) =
            if let Some(identity_arc) = crate::identity::node_id::NODE_IDENTITY.get() {
                let guard = identity_arc.read();
                let node_id = guard.node_id.clone();
                let node_pubkey = guard.pubkey_b64.clone();
                let pubkey_fingerprint = guard.fingerprint();
                (node_id, node_pubkey, pubkey_fingerprint)
            } else {
                (
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                )
            };

        let wallet_address = chain
            .db
            .get(b"primary_wallet_address")
            .ok()
            .flatten()
            .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());
        (
            node_id,
            node_pubkey,
            node_pubkey_fingerprint,
            wallet_address,
        )
    };

    // Get node approval status
    let (approved, approved_wallet) = match crate::node_approval::NodeApproval::load() {
        Ok(Some(approval)) => {
            // Verify approval is valid for current node
            match approval.verify(&node_id, &node_pubkey) {
                Ok(_) => (true, Some(approval.wallet_address)),
                Err(_) => (false, None),
            }
        }
        _ => (false, None),
    };

    // Get auto-detected node role
    let role = crate::role::current_node_role();
    let node_role = role.as_str().to_string();

    // Calculate exchange readiness: anchor node at tip (lag=0)
    let _current_height = snapshot.sync_height;
    let exchange_ready =
        crate::role::is_anchor() && snapshot.height_lag() == 0 && snapshot.connected_peers >= 3;

    // Get HTTP backbone state (7070 control plane)
    let http_backbone = HttpBackboneStatus {
        connected: backbone_state.connected,
        anchor: backbone_state.best_anchor.clone(),
        latency_ms: backbone_state.latency_ms,
        tip_height: Some(backbone_state.observed_tip_height),
        tip_hash: backbone_state.observed_tip_hash.clone(),
        last_ok_unix: backbone_state
            .last_ok
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()),
        last_error: backbone_state.last_error.clone(),
    };

    // Exchange OK = full feature enabled
    let exchange_ok = cfg!(feature = "full");

    // Get mempool size for legacy field
    let chain = CHAIN.lock();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    drop(chain);

    // Calculate mining eligibility and blocked reason
    let mining_eligible = crate::mining_readiness::is_mining_eligible();
    let mining_blocked_reason = if !mining_eligible {
        Some(crate::mining_readiness::mining_status_message(&snapshot))
    } else {
        None
    };

    // Get P2P health status
    let p2p_health = if snapshot.connected_peers == 0 {
        "isolated".to_string()
    } else if snapshot.connected_peers == 1 {
        "weak".to_string()
    } else if snapshot.connected_peers >= 2 && snapshot.connected_peers < 8 {
        "ok".to_string()
    } else if snapshot.connected_peers >= 8 && snapshot.connected_peers < 32 {
        "stable".to_string()
    } else {
        "immortal".to_string()
    };

    // Total known peers (from P2P manager if available)
    let total_known_peers = snapshot.connected_peers; // Simplified for now

    Json(StatusResponse {
        live: true,
        chain_height,
        peer_count,
        version: crate::vision_constants::VISION_VERSION.to_string(),
        uptime_seconds,
        sync_status,
        can_mine,
        sync_height: snapshot.sync_height,
        network_estimated_height: snapshot.network_estimated_height,
        connected_peers: snapshot.connected_peers,
        behind_blocks: snapshot.behind_blocks,
        slow_peers: snapshot.slow_peers,
        avg_peer_lag_blocks: snapshot.avg_peer_lag_blocks,
        node_id,
        node_pubkey,
        node_pubkey_fingerprint,
        wallet_address,
        approved,
        approved_wallet,
        exchange_ready,
        node_role,
        http_backbone,
        exchange_ok,
        // Legacy aliases for panel.html compatibility
        height: Some(chain_height),
        peers: Some(vec![]), // Empty array, panel uses peer_count anyway
        mempool: Some(mempool_size),
        // Mining eligibility
        mining_eligible,
        mining_blocked_reason,
        total_known_peers,
        p2p_health,
        network_mode,
        transport,
        backbone_reachable: backbone_state.connected,
        warmup_active,
        warmup_remaining_blocks,
        chain_id: crate::vision_constants::expected_chain_id(),
        drop_prefix: crate::vision_constants::VISION_BOOTSTRAP_PREFIX.to_string(),
    })
}
#[cfg(feature = "staging")]
pub async fn get_guardian() -> impl axum::response::IntoResponse {
    let guardian_mode = false; // All nodes are constellation nodes

    axum::Json(GuardianResponse {
        enabled: guardian_mode,
        active: guardian_mode,
        recent_actions: vec![], // TODO: Integrate with guardian consciousness module
    })
}

#[cfg(not(feature = "staging"))]
pub async fn get_guardian() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

#[cfg(feature = "staging")]
pub async fn post_guardian(
    axum::Json(_payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<GuardianResponse>, axum::http::StatusCode> {
    let guardian_mode = false; // All nodes are constellation nodes

    Ok(axum::Json(GuardianResponse {
        enabled: guardian_mode,
        active: guardian_mode,
        recent_actions: vec![], // TODO: Integrate with guardian consciousness module
    }))
}

#[cfg(not(feature = "staging"))]
pub async fn post_guardian(
    _payload: axum::Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

pub async fn get_chain_status() -> Json<ChainStatusResponse> {
    // Guardian is an observer - always use local chain state (no HTTP upstream sync)
    // Website /api/upstream/* routes should rotate through constellation peer APIs

    // Use local chain data
    let chain = CHAIN.lock();
    let height = chain.blocks.len().saturating_sub(1) as u64;
    let difficulty = chain.difficulty;

    // Get latest block hash (pow_hash is already a String)
    let latest_block_hash = if let Some(last_block) = chain.blocks.last() {
        last_block.header.pow_hash.clone()
    } else {
        "0x0000000000000000".to_string()
    };

    drop(chain);

    Json(ChainStatusResponse {
        height,
        latest_block_hash,
        difficulty,
    })
}

pub async fn get_health_public() -> Json<HealthPublicResponse> {
    // Guardian is an observer - always use local chain state (no HTTP upstream sync)
    // Website /api/upstream/* routes should rotate through constellation peer APIs

    // Compute real mood from current state
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await;
    let (height, mempool_size) = {
        let chain = CHAIN.lock();
        (
            chain.blocks.len().saturating_sub(1) as u64,
            chain.mempool_critical.len() + chain.mempool_bulk.len(),
        )
    };

    let testnet_phase = detect_testnet_phase(&CHAIN.lock());
    let (anomalies, traumas) = HEALTH_DB.lock().get_summaries_for_mood();
    let guardian_active = std::env::var("VISION_GUARDIAN").ok().as_deref() == Some("1");

    use crate::mood;
    let mood_snapshot = mood::compute_mood(
        height,
        peer_count,
        mempool_size,
        anomalies,
        traumas,
        guardian_active,
        testnet_phase,
    );

    // Map mood to prediction
    let mood_str = format!("{:?}", mood_snapshot.mood);
    let prediction = match mood_str.as_str() {
        "Celebration" => "thriving",
        "Calm" => "stable",
        "Guardian" => "protected",
        "Warning" => "cautious",
        "Wounded" => "recovering",
        "Storm" => "volatile",
        "Rage" => "critical",
        _ => "unknown",
    };

    // Build incidents list
    let mut incidents = Vec::new();
    if anomalies > 0 {
        incidents.push(format!("{} pending anomalies", anomalies));
    }
    if traumas > 0 {
        incidents.push(format!("{} recent traumas", traumas));
    }

    Json(HealthPublicResponse {
        network_health: NetworkHealth {
            mood: mood_str,
            prediction: prediction.to_string(),
            confidence: mood_snapshot.score as f64,
            incidents,
        },
    })
}

pub async fn get_beacon_state() -> Json<BeaconState> {
    // Compute current mood state for beacon
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await;
    let (height, mempool_size) = {
        let chain = CHAIN.lock();
        (
            chain.blocks.len().saturating_sub(1) as u64,
            chain.mempool_critical.len() + chain.mempool_bulk.len(),
        )
    };

    let testnet_phase = detect_testnet_phase(&CHAIN.lock());
    let (anomalies, traumas) = HEALTH_DB.lock().get_summaries_for_mood();
    let guardian_active = std::env::var("VISION_GUARDIAN").ok().as_deref() == Some("1");

    use crate::mood;
    let mood_snapshot = mood::compute_mood(
        height,
        peer_count,
        mempool_size,
        anomalies,
        traumas,
        guardian_active,
        testnet_phase,
    );

    Json(BeaconState {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        mood: format!("{:?}", mood_snapshot.mood),
        score: mood_snapshot.score as f64,
        reason: mood_snapshot.reason,
        chain_height: height,
        peer_count,
        mempool_size,
        guardian_active,
        peers: vec![], // Empty for now, wallet uses peer_count
    })
}

pub async fn get_downloads_visitors() -> Json<DownloadsResponse> {
    Json(DownloadsResponse {
        total_downloads: 1234,
        unique_visitors: 567,
    })
}

pub async fn get_bootstrap() -> Json<serde_json::Value> {
    let chain = CHAIN.lock();
    let bootstrap_nodes: Vec<_> = chain.peers.iter().cloned().collect();
    drop(chain);

    // If no peers, return default sentinel
    let nodes = if bootstrap_nodes.is_empty() {
        vec!["sentinel.visionworld.tech:7070".to_string()]
    } else {
        bootstrap_nodes
    };

    Json(serde_json::json!({
        "bootstrap_nodes": nodes
    }))
}

pub async fn get_constellation() -> Json<serde_json::Value> {
    // Guardian is an observer - always use local chain state (no HTTP upstream sync)
    // Website /api/upstream/* routes should rotate through constellation peer APIs

    // Get peer data from Vision Peer Book with mood info
    let chain = CHAIN.lock();
    let peer_store = match crate::p2p::peer_store::PeerStore::new(&chain.db) {
        Ok(store) => store,
        Err(_) => {
            drop(chain);
            return Json(serde_json::json!({
                "overall_network_mood": "calm",
                "network_mood": "calm",
                "sentinel": null,
                "nodes": [],
                "total": 0
            }));
        }
    };

    let all_peers = peer_store.all();
    drop(chain);

    // Helper function to determine status tier based on mood score and trusted status
    fn get_status_tier(mood_score: f32, trusted: bool) -> String {
        if trusted && mood_score >= 0.90 {
            "legendary".to_string()
        } else if trusted && mood_score >= 0.80 {
            "elite".to_string()
        } else if mood_score >= 0.50 {
            "rare".to_string()
        } else if mood_score >= 0.30 {
            "uncommon".to_string()
        } else {
            "common".to_string()
        }
    }

    // Calculate network-wide mood from all peers
    let mut total_mood_score = 0.0;
    let mut mood_count = 0;
    let mut mood_distribution: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Find sentinel (guardian role) and regular nodes
    let mut sentinel = None;
    let mut nodes = Vec::new();

    for peer in all_peers {
        let mood = peer
            .mood
            .as_ref()
            .map(|m| m.label.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let mood_score = peer.mood.as_ref().map(|m| m.score).unwrap_or(0.0);
        let status_tier = get_status_tier(mood_score, peer.trusted);

        // Track mood distribution
        if mood != "unknown" {
            *mood_distribution.entry(mood.clone()).or_insert(0) += 1;
            total_mood_score += mood_score;
            mood_count += 1;
        }

        let node_info = serde_json::json!({
            "node_tag": peer.node_tag,
            "vision_address": peer.vision_address,
            "role": peer.role,
            "status_tier": status_tier,
            "mood": mood,
            "mood_score": mood_score,
            "last_seen": peer.last_seen
        });

        if peer.role == "guardian" {
            sentinel = Some(node_info);
        } else {
            nodes.push(node_info);
        }
    }

    // Calculate overall network mood based on average mood score and distribution
    let (overall_network_mood, network_mood) = if mood_count == 0 {
        ("calm".to_string(), "calm".to_string())
    } else {
        let avg_score = total_mood_score / mood_count as f32;

        // Determine dominant mood
        let dominant_mood = mood_distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(mood, _)| mood.clone())
            .unwrap_or_else(|| "calm".to_string());

        // Overall network mood based on average score
        let overall = if avg_score >= 0.85 {
            "celebration".to_string()
        } else if avg_score >= 0.70 {
            "calm".to_string()
        } else if avg_score >= 0.50 {
            "warning".to_string()
        } else if avg_score >= 0.30 {
            "storm".to_string()
        } else {
            "wounded".to_string()
        };

        (overall, dominant_mood)
    };

    // If no sentinel found, create a default one
    if sentinel.is_none() {
        sentinel = Some(serde_json::json!({
            "node_tag": "VNODE-GUARDIAN",
            "vision_address": "vision://guardian@local",
            "role": "guardian",
            "status_tier": "legendary",
            "mood": "calm",
            "mood_score": 0.98,
            "last_seen": chrono::Utc::now().timestamp()
        }));
    }

    let total = nodes.len();

    Json(serde_json::json!({
        "overall_network_mood": overall_network_mood,
        "network_mood": network_mood,
        "sentinel": sentinel,
        "nodes": nodes,
        "total": total
    }))
}

pub async fn get_constellation_history(
    Query(_params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "history": []
    }))
}

pub async fn get_new_stars(
    Query(_params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "new_stars": []
    }))
}

#[cfg(feature = "staging")]
pub async fn get_guardian_feed() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "messages": []
    }))
}

#[cfg(not(feature = "staging"))]
pub async fn get_guardian_feed() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

pub async fn get_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "predictions": [],
        "anomalies": []
    }))
}

#[cfg(feature = "staging")]
pub async fn get_mood() -> impl axum::response::IntoResponse {
    use crate::mood;

    // Guardian is an observer - always use local chain state (no HTTP upstream sync)
    // Website /api/upstream/* routes should rotate through constellation peer APIs

    // Get guardian mode status
    let guardian_mode = false; // All nodes are constellation nodes

    // Gather real metrics from chain
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await;

    let chain = CHAIN.lock();
    let height = chain.blocks.len().saturating_sub(1);
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();

    // Detect testnet phase from environment or chain metadata
    let testnet_phase = detect_testnet_phase(&chain);

    drop(chain);

    // Get health summaries from the health DB
    let (pending_anomalies, recent_traumas) = get_health_summaries();

    // Compute mood using the new mood engine
    let snapshot = mood::compute_mood(
        height as u64,
        peer_count,
        mempool_size,
        pending_anomalies,
        recent_traumas,
        guardian_mode,
        testnet_phase,
    );

    // Return the full snapshot as JSON
    axum::Json(serde_json::to_value(&snapshot).unwrap_or_else(|_| serde_json::json!({})))
}

#[cfg(not(feature = "staging"))]
pub async fn get_mood() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

/// Helper function to get health summaries for mood calculation
/// Returns (pending_anomalies, recent_traumas)
fn get_health_summaries() -> (u32, u32) {
    // Query the health DB for real trauma/anomaly counts
    let health_db = HEALTH_DB.lock();
    health_db.get_summaries_for_mood()
}

/// Detect testnet phase from environment variables or chain metadata
pub fn detect_testnet_phase(chain: &crate::Chain) -> Option<String> {
    // Check environment variable first
    if let Ok(phase) = std::env::var("VISION_TESTNET_PHASE") {
        return Some(phase.to_lowercase());
    }

    // Try to detect from chain metadata
    if let Ok(Some(phase_data)) = chain.db.get(b"testnet_phase") {
        if let Ok(phase_str) = String::from_utf8(phase_data.to_vec()) {
            return Some(phase_str);
        }
    }

    // Check if we're in a testnet based on port or other indicators
    let port: u16 = std::env::var("VISION_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7070);

    // Testnet typically runs on different ports
    if port != 7070 && (8000..9000).contains(&port) {
        // Default to mining phase for testnets
        return Some("mining".to_string());
    }

    None
}

pub async fn get_trauma(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "events": []
    }))
}

pub async fn get_patterns(
    Query(_params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "patterns": []
    }))
}

pub async fn get_nodes() -> Json<serde_json::Value> {
    let peers_map = PEERS.lock();
    let node_list: Vec<_> = peers_map.keys().cloned().collect();
    let total = node_list.len();
    drop(peers_map);

    Json(serde_json::json!({
        "nodes": node_list,
        "total": total
    }))
}

pub async fn get_nodes_with_identity() -> Json<serde_json::Value> {
    let peers_map = PEERS.lock();

    // Build detailed peer info
    let nodes: Vec<_> = peers_map
        .iter()
        .map(|(addr, meta)| {
            serde_json::json!({
                "address": addr,
                "reputation": meta.reputation_score,
                "last_active": meta.last_active,
                "response_time_ms": meta.avg_response_time_ms,
                "blocks_contributed": meta.blocks_contributed,
            })
        })
        .collect();

    drop(peers_map);

    Json(serde_json::json!({
        "nodes": nodes
    }))
}

pub async fn get_reputation() -> Json<serde_json::Value> {
    let peers_map = PEERS.lock();

    // Build reputation map
    let mut reputations = serde_json::Map::new();
    for (addr, meta) in peers_map.iter() {
        reputations.insert(
            addr.clone(),
            serde_json::json!({
                "score": meta.reputation_score,
                "blocks_contributed": meta.blocks_contributed,
                "success_rate": if meta.total_requests > 0 {
                    (meta.successful_requests as f64 / meta.total_requests as f64) * 100.0
                } else {
                    0.0
                }
            }),
        );
    }

    drop(peers_map);

    Json(serde_json::json!({
        "reputations": reputations
    }))
}

pub async fn get_snapshots_recent(
    Query(_params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "snapshots": []
    }))
}

// ===== HEALTH DB ENDPOINTS =====

pub async fn get_health_anomalies() -> Json<serde_json::Value> {
    let health = HEALTH_DB.lock();
    let anomalies = health.get_pending_anomalies().unwrap_or_default();
    let count = anomalies.len();
    drop(health);

    Json(serde_json::json!({
        "anomalies": anomalies,
        "count": count
    }))
}

pub async fn get_health_traumas() -> Json<serde_json::Value> {
    let health = HEALTH_DB.lock();
    let traumas = health.get_recent_traumas().unwrap_or_default();
    let count = traumas.len();
    drop(health);

    Json(serde_json::json!({
        "traumas": traumas,
        "count": count
    }))
}

// ===== GUARDIAN CONSCIOUSNESS ENDPOINTS =====

#[cfg(feature = "staging")]
pub async fn get_guardian_consciousness() -> impl axum::response::IntoResponse {
    let consciousness = GUARDIAN_CONSCIOUSNESS.lock();
    let report = consciousness.generate_report();
    drop(consciousness);

    axum::Json(serde_json::json!({
        "timestamp": report.timestamp,
        "current_mood": report.current_mood,
        "mood_trend": format!("{:?}", report.mood_trend),
        "intervention_count": report.intervention_count,
        "last_intervention": report.last_intervention,
        "consciousness_level": report.consciousness_level
    }))
}

#[cfg(not(feature = "staging"))]
pub async fn get_guardian_consciousness() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

#[cfg(feature = "staging")]
pub async fn post_guardian_intervene() -> impl axum::response::IntoResponse {
    // Compute current mood first
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await as u32;
    let (height, mempool_size) = {
        let chain = CHAIN.lock();
        (
            chain.blocks.len().saturating_sub(1) as u64,
            (chain.mempool_critical.len() + chain.mempool_bulk.len()) as u32,
        )
    };

    let testnet_phase = detect_testnet_phase(&CHAIN.lock());
    let (anomalies, traumas) = HEALTH_DB.lock().get_summaries_for_mood();
    let guardian_active = std::env::var("VISION_GUARDIAN").ok().as_deref() == Some("1");

    use crate::mood;
    let mood_snapshot = mood::compute_mood(
        height,
        peer_count as usize,
        mempool_size as usize,
        anomalies,
        traumas,
        guardian_active,
        testnet_phase,
    );

    let mut consciousness = GUARDIAN_CONSCIOUSNESS.lock();

    match consciousness.should_intervene(&mood_snapshot) {
        Some(intervention_type) => {
            let result = consciousness.execute_intervention(intervention_type);
            drop(consciousness);

            axum::Json(serde_json::json!({
                "success": true,
                "intervention": format!("{:?}", intervention_type),
                "actions": result.actions_taken,
                "expected_recovery_time": result.expected_recovery_time
            }))
        }
        None => {
            drop(consciousness);
            axum::Json(serde_json::json!({
                "success": false,
                "message": "No intervention needed or cooldown active"
            }))
        }
    }
}

#[cfg(not(feature = "staging"))]
pub async fn post_guardian_intervene() -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        "staged endpoint disabled in v1.0",
    )
}

// ===== ROUTER FUNCTION =====

use crate::api::peers_api;
use crate::api::routing_api;
use axum::{
    routing::{get, post},
    Router,
};

pub fn website_api_router() -> Router {
    Router::new()
        // Core Status Endpoints
        .route("/api/status", get(get_status))
        .route("/api/bootstrap", get(get_bootstrap))
        .route("/api/chain/status", get(get_chain_status))
        // Network Data Endpoints
        .route("/api/constellation", get(get_constellation))
        .route("/api/constellation/history", get(get_constellation_history))
        .route("/api/constellation/new-stars", get(get_new_stars))
        // Guardian Endpoints
        .route("/api/guardian", get(get_guardian).post(post_guardian))
        .route("/api/guardian/feed", get(get_guardian_feed))
        .route(
            "/api/guardian/consciousness",
            get(get_guardian_consciousness),
        )
        .route("/api/guardian/intervene", post(post_guardian_intervene))
        // Health & Monitoring Endpoints
        .route("/api/health", get(get_health))
        .route("/api/health/public", get(get_health_public))
        .route("/api/health/anomalies", get(get_health_anomalies))
        .route("/api/health/traumas", get(get_health_traumas))
        .route("/api/beacon/state", get(get_beacon_state))
        // Network Mood Endpoint
        .route("/api/mood", get(get_mood))
        // Trauma & Patterns Endpoints
        .route("/api/trauma", get(get_trauma))
        .route("/api/patterns", get(get_patterns))
        // Node Identity Endpoints
        .route("/api/nodes", get(get_nodes))
        .route("/api/nodes/with-identity", get(get_nodes_with_identity))
        .route("/api/reputation", get(get_reputation))
        // Vision Peer Book Endpoints (Phase 8: Trusted Peers & Mood Tracking)
        .route("/api/peers/trusted", get(peers_api::get_trusted_peers))
        .route("/api/peers/moods", get(peers_api::get_peer_moods))
        // Routing Intelligence Dashboard Endpoints (Phase 4: Routing Intelligence)
        .route(
            "/api/p2p/routing/cluster_stats",
            get(routing_api::get_cluster_stats_handler),
        )
        .route(
            "/api/p2p/routing/top_peers",
            get(routing_api::get_top_peers_handler),
        )
        .route(
            "/api/p2p/routing/events",
            get(routing_api::get_routing_events_handler),
        )
        .route(
            "/api/p2p/routing/events_stream",
            get(routing_api::routing_events_ws_handler),
        )
        // Prometheus Metrics for Grafana
        .route("/metrics", get(routing_api::get_metrics_handler))
        // Analytics Endpoints
        .route("/api/downloads/visitors", get(get_downloads_visitors))
        .route("/api/snapshots/recent", get(get_snapshots_recent))
        // Debug: Pre-DB → DB migration report
        .route(
            "/api/p2p/debug/predb_migration",
            axum::routing::get(get_predb_migration_report),
        )
        // Debug: Peer store stats (counts only)
        .route(
            "/api/p2p/debug/peer_store_stats",
            axum::routing::get(get_peer_store_stats),
        )
}

/// GET /api/p2p/debug/predb_migration
/// Returns the last migration report if available; otherwise 404.
pub async fn get_predb_migration_report(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let token = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok());
    if let Err((_code, _msg)) = crate::api::security::verify_p2p_debug_access(&addr, token) {
        return (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": "not found" })),
        );
    }

    match crate::p2p::predb_migration_report::get_report() {
        Some(report) => {
            let val = serde_json::to_value(report).unwrap_or(serde_json::json!({
                "error": "serialization_failed"
            }));
            (axum::http::StatusCode::OK, axum::Json(val))
        }
        None => (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "no migration report yet"
            })),
        ),
    }
}

#[derive(serde::Serialize)]
pub struct PeerStoreStatsResponse {
    pub db_total: u64,
    pub mem_total: u64,
    pub connected_peers: u64,
    pub validated_peers: u64,
    pub anchors: u64,
    pub banned: u64,
    pub hot: u64,
    pub warm: u64,
    pub cold: u64,
    pub dial_failures: u64,
}

/// GET /api/p2p/debug/peer_store_stats
/// Returns counts only; no peer identities leaked.
pub async fn get_peer_store_stats(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let token = headers
        .get("x-admin-token")
        .and_then(|v| v.to_str().ok());
    if let Err((_code, _msg)) = crate::api::security::verify_p2p_debug_access(&addr, token) {
        return (
            axum::http::StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": "not found" })),
        );
    }

    // db_total via sled tree length
    let db_total = match crate::CHAIN.lock().db.open_tree("constellation_memory") {
        Ok(tree) => tree.len() as u64,
        Err(_) => 0,
    };

    // mem_total via global CONSTELLATION_MEMORY
    let mem_total = crate::CONSTELLATION_MEMORY.lock().peer_count() as u64;

    // PeerManager counts
    let peers = crate::PEER_MANAGER.get_all_peers().await;
    let connected_peers = peers
        .iter()
        .filter(|p| p.state == crate::p2p::peer_manager::PeerState::Connected)
        .count() as u64;
    let hot = peers
        .iter()
        .filter(|p| p.bucket == crate::p2p::peer_manager::PeerBucket::Hot)
        .count() as u64;
    let warm = peers
        .iter()
        .filter(|p| p.bucket == crate::p2p::peer_manager::PeerBucket::Warm)
        .count() as u64;
    let cold = peers
        .iter()
        .filter(|p| p.bucket == crate::p2p::peer_manager::PeerBucket::Cold)
        .count() as u64;

    // anchors via ConstellationMemory
    let anchors = crate::CONSTELLATION_MEMORY.lock().get_anchor_peers().len() as u64;

    // banned via sled key prefix scan
    let banned = crate::CHAIN
        .lock()
        .db
        .scan_prefix("banned_")
        .count() as u64;

    // dial failures count
    let dial_failures = crate::p2p::dial_tracker::get_dial_failures().len() as u64;

    let resp = PeerStoreStatsResponse {
        db_total,
        mem_total,
        connected_peers,
        validated_peers: mem_total,
        anchors,
        banned,
        hot,
        warm,
        cold,
        dial_failures,
    };

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::to_value(resp).unwrap_or(serde_json::json!({
            "error": "serialization_failed"
        }))),
    )
}
