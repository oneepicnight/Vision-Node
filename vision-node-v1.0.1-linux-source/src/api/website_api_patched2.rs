// Website API handlers integrated with actual node state
// Provides real-time data from blockchain, peers, and guardian systems

use axum::{http::StatusCode, Json, extract::Query};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import node globals
use crate::{CHAIN, PEERS, GUARDIAN_MODE, PROM_P2P_PEERS};

// Import upstream module for Guardian HTTP follower mode
use super::upstream;

// ===== RESPONSE TYPES =====

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub live: bool,
    pub chain_height: u64,
    pub peer_count: usize,
    pub version: String,
    pub uptime_seconds: u64,
    pub guardian_mode: bool,
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
    pub network_mood: String,
    pub mood_score: f32,
    pub chaos_intensity: f32,
    pub peer_count: usize,
    pub anomalies_last_24h: u32,
    pub traumas_last_24h: u32,
    pub details: crate::mood::MoodDetails,
}

#[derive(Serialize)]
pub struct BeaconStateResponse {
    pub mood: String,
    pub chaos_intensity: f32,
    pub active_traumas: u32,
    pub last_trauma_at: Option<String>,
}

#[derive(Serialize)]
pub struct DownloadsResponse {
    pub total_downloads: u64,
    pub unique_visitors: u64,
}

// ===== HANDLERS WITH REAL DATA =====

pub async fn get_status() -> Json<StatusResponse> {
    // Get guardian mode status
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    
    // If in Guardian mode, try to fetch from upstream first
    if guardian_mode {
        if let Some(upstream_status) = upstream::upstream_status().await {
            return Json(upstream_status);
        }
    }
    
    // Fall back to local chain data
    let chain = CHAIN.lock();
    let chain_height = chain.blocks.len().saturating_sub(1) as u64;
    let peer_count = chain.peers.len();
    drop(chain);
    
    // Calculate uptime (static start time would be tracked elsewhere, using placeholder)
    let uptime_seconds = 3600; // TODO: Track actual start time
    
    Json(StatusResponse {
        live: true,
        chain_height,
        peer_count,
        version: "0.8.0".to_string(),
        uptime_seconds,
        guardian_mode,
    })
}

pub async fn get_guardian() -> Json<GuardianResponse> {
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    
    Json(GuardianResponse {
        enabled: guardian_mode,
        active: guardian_mode,
        recent_actions: vec![], // TODO: Integrate with guardian consciousness module
    })
}

pub async fn post_guardian(Json(_payload): Json<serde_json::Value>) -> Result<Json<GuardianResponse>, StatusCode> {
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    
    Ok(Json(GuardianResponse {
        enabled: guardian_mode,
        active: guardian_mode,
        recent_actions: vec![], // TODO: Integrate with guardian consciousness module
    }))
}

pub async fn get_chain_status() -> Json<ChainStatusResponse> {
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_chain) = upstream::upstream_chain_status().await {
            return Json(upstream_chain);
        }
    }
    
    // Fall back to local chain data
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
    use crate::mood;
    
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_health) = upstream::upstream_health_public().await {
            return Json(upstream_health);
        }
    }

    // Gather real metrics from chain
    let chain = CHAIN.lock();
    let height = chain.best_height();
    let peer_count = chain.peers.len();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    
    // Get testnet phase if applicable
    let testnet_phase = if chain.testnet_48h_checkpoint.is_some() {
        let checkpoint_time = chain.testnet_48h_checkpoint.unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_hours = (now.saturating_sub(checkpoint_time)) / 3600;
        
        if elapsed_hours < 24 {
            Some("mining".to_string())
        } else if elapsed_hours < 48 {
            Some("hybrid".to_string())
        } else {
            Some("staking-only".to_string())
        }
    } else {
        None
    };
    
    drop(chain);
    
    // Get health summaries
    let (pending_anomalies, recent_traumas) = get_health_summaries();
    
    // Compute mood using the mood engine
    let snapshot = mood::compute_mood(
        height as u64,
        peer_count,
        mempool_size,
        pending_anomalies,
        recent_traumas,
        guardian_mode,
        testnet_phase,
    );
    
    let chaos_intensity = 1.0 - snapshot.score;
    
    Json(HealthPublicResponse {
        network_mood: format!("{:?}", snapshot.mood).to_lowercase(),
        mood_score: snapshot.score,
        chaos_intensity,
        peer_count,
        anomalies_last_24h: pending_anomalies,
        traumas_last_24h: recent_traumas,
        details: snapshot.details,
    })
}


pub async fn get_beacon_state() -> Json<BeaconStateResponse> {
    use crate::mood;
    
    // Gather metrics from chain
    let chain = CHAIN.lock();
    let height = chain.best_height();
    let peer_count = chain.peers.len();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    
    // Get testnet phase
    let testnet_phase = if chain.testnet_48h_checkpoint.is_some() {
        let checkpoint_time = chain.testnet_48h_checkpoint.unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_hours = (now.saturating_sub(checkpoint_time)) / 3600;
        
        if elapsed_hours < 24 {
            Some("mining".to_string())
        } else if elapsed_hours < 48 {
            Some("hybrid".to_string())
        } else {
            Some("staking-only".to_string())
        }
    } else {
        None
    };
    
    drop(chain);
    
    let guardian_active = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    let (pending_anomalies, recent_traumas) = get_health_summaries();
    
    // Compute mood snapshot
    let snapshot = mood::compute_mood(
        height as u64,
        peer_count,
        mempool_size,
        pending_anomalies,
        recent_traumas,
        guardian_active,
        testnet_phase,
    );
    
    let chaos_intensity = 1.0 - snapshot.score;
    
    // TODO: Implement last_trauma_timestamp() when trauma DB is ready
    let last_trauma_at = None;
    
    Json(BeaconStateResponse {
        mood: format!("{:?}", snapshot.mood).to_lowercase(),
        chaos_intensity,
        active_traumas: snapshot.details.recent_trauma_count,
        last_trauma_at,
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
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_constellation) = upstream::upstream_constellation().await {
            return Json(upstream_constellation);
        }
    }
    
    // Fall back to local peers data
    let peers_map = PEERS.lock();
    let node_list: Vec<_> = peers_map.keys().cloned().collect();
    let total = node_list.len();
    drop(peers_map);
    
    Json(serde_json::json!({
        "nodes": node_list,
        "total": total
    }))
}

pub async fn get_constellation_history(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "history": []
    }))
}

pub async fn get_new_stars(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "new_stars": []
    }))
}

pub async fn get_guardian_feed() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "messages": []
    }))
}

pub async fn get_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "predictions": [],
        "anomalies": []
    }))
}

pub async fn get_mood() -> Json<serde_json::Value> {
    use crate::mood;
    
    // If in Guardian mode, try to fetch from upstream first
    let guardian_mode = GUARDIAN_MODE.load(std::sync::atomic::Ordering::Relaxed);
    if guardian_mode {
        if let Some(upstream_mood) = upstream::upstream_mood().await {
            return Json(upstream_mood);
        }
    }
    
    // Gather real metrics from chain
    let chain = CHAIN.lock();
    let height = chain.best_height();
    let peer_count = chain.peers.len();
    let mempool_size = chain.mempool_critical.len() + chain.mempool_bulk.len();
    
    // Get testnet phase if applicable
    let testnet_phase = if chain.testnet_48h_checkpoint.is_some() {
        // Determine current phase based on time since checkpoint
        let checkpoint_time = chain.testnet_48h_checkpoint.unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed_hours = (now.saturating_sub(checkpoint_time)) / 3600;
        
        if elapsed_hours < 24 {
            Some("mining".to_string())
        } else if elapsed_hours < 48 {
            Some("hybrid".to_string())
        } else {
            Some("staking-only".to_string())
        }
    } else {
        None
    };
    
    drop(chain);
    
    // Get health summaries (currently placeholder, can be enhanced with real trauma/anomaly tracking)
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
    Json(serde_json::to_value(&snapshot).unwrap_or_else(|_| serde_json::json!({})))
}

/// Helper function to get health summaries for mood calculation
/// Returns (pending_anomalies, recent_traumas)
/// Currently returns placeholder values - can be enhanced with real health DB tracking
fn get_health_summaries() -> (u32, u32) {
    // TODO: When health/trauma tracking is implemented, query the health DB here
    // For now, return zeros to indicate healthy state
    (0, 0)
}

pub async fn get_trauma(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "events": []
    }))
}

pub async fn get_patterns(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
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
    let nodes: Vec<_> = peers_map.iter().map(|(addr, meta)| {
        serde_json::json!({
            "address": addr,
            "reputation": meta.reputation_score,
            "last_active": meta.last_active,
            "response_time_ms": meta.avg_response_time_ms,
            "blocks_contributed": meta.blocks_contributed,
        })
    }).collect();
    
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
            })
        );
    }
    
    drop(peers_map);
    
    Json(serde_json::json!({
        "reputations": reputations
    }))
}

pub async fn get_snapshots_recent(Query(_params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "snapshots": []
    }))
}

// ===== ROUTER FUNCTION =====

use axum::{routing::{get, post}, Router};

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
        // Health & Monitoring Endpoints
        .route("/api/health", get(get_health))
        .route("/api/health/public", get(get_health_public))
        // Network Mood Endpoint
        .route("/api/mood", get(get_mood))
        // Trauma & Patterns Endpoints
        .route("/api/trauma", get(get_trauma))
        .route("/api/patterns", get(get_patterns))
        // Node Identity Endpoints
        .route("/api/nodes", get(get_nodes))
        .route("/api/nodes/with-identity", get(get_nodes_with_identity))
        .route("/api/reputation", get(get_reputation))
        // Analytics Endpoints
        .route("/api/downloads/visitors", get(get_downloads_visitors))
        .route("/api/snapshots/recent", get(get_snapshots_recent))
}
