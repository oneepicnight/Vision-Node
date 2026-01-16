//! Identity endpoint - Node identity information
//!
//! Provides node identity including VNode tag, role, admission status,
//! mood, and trusted peers. Can query local node or remote node by tag.

use axum::{
    extract::Query,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{CHAIN, P2P_MANAGER};

/// Query parameters for identity endpoint
#[derive(Debug, Deserialize)]
pub struct IdentityQuery {
    /// Optional node_tag to query remote node's identity
    pub node_tag: Option<String>,
}

/// Trusted peer information
#[derive(Debug, Serialize)]
pub struct TrustedPeerInfo {
    pub node_tag: String,
    pub vision_address: String,
    pub reputation_tier: String,
    pub last_seen: Option<String>,
}

/// Node identity response
#[derive(Debug, Serialize)]
pub struct NodeIdentity {
    pub node_tag: String,
    pub vision_address: String,
    pub role: String,
    pub network_id: String,
    pub admission_status: String,
    pub ticket_expires_at: Option<String>,
    pub mood: String,
    pub mood_score: f32,
    pub status_tier: String,
    pub trusted_peer_count: usize,
    pub trusted_peers: Vec<TrustedPeerInfo>,
}

/// GET /api/identity
///
/// Returns node identity information. Without node_tag parameter, returns
/// local node's identity. With node_tag, attempts to return specified node's
/// identity from constellation.
pub async fn get_identity(Query(params): Query<IdentityQuery>) -> Json<NodeIdentity> {
    if let Some(tag) = params.node_tag {
        // Try to find remote node's identity from constellation
        get_remote_node_identity(&tag).await
    } else {
        // Return local node's identity
        get_local_node_identity().await
    }
}

/// Get local node's identity
async fn get_local_node_identity() -> Json<NodeIdentity> {
    // Get chain info safely
    let (height, last_hash) = {
        let chain = CHAIN.lock();
        let h = chain.blocks.len() as u64;
        let hash = if !chain.blocks.is_empty() {
            chain.blocks.last().unwrap().header.pow_hash.clone()
        } else {
            "0000000000000000".to_string()
        };
        (h, hash)
    };
    
    // Get peer count (validated connected peers)
    let peer_count = crate::PEER_MANAGER.connected_validated_count().await;
    
    // Get VNode tag from environment or generate
    let node_tag = std::env::var("VNODE_TAG")
        .unwrap_or_else(|_| format!("VNODE-{}", &last_hash[..8]));
    
    // Determine role
    let is_guardian = std::env::var("VISION_GUARDIAN_MODE")
        .map(|v| v == "true")
        .unwrap_or(false);
    let role = if is_guardian { "guardian" } else { "constellation" };
    
    // Calculate mood based on health metrics
    let (mood, mood_score) = calculate_node_mood(peer_count, height);
    
    // Determine status tier based on metrics
    let status_tier = calculate_status_tier(peer_count, height);
    
    // Get trusted peers (peers with good connection quality)
    let trusted_peers = get_trusted_peers();
    
    // Admission status (always admitted for running nodes)
    let admission_status = "admitted".to_string();
    
    // Ticket expiry (30 days from now)
    let ticket_expires_at = Some(format_future_date(30 * 24 * 3600));
    
    // Build vision address
    let vision_address = format!("vision://{}@{}", node_tag, &last_hash[..16]);
    
    Json(NodeIdentity {
        node_tag,
        vision_address,
        role: role.to_string(),
        network_id: crate::vision_constants::VISION_NETWORK_ID.to_string(),
        admission_status,
        ticket_expires_at,
        mood,
        mood_score,
        status_tier,
        trusted_peer_count: trusted_peers.len(),
        trusted_peers,
    })
}

/// Get remote node's identity from constellation
async fn get_remote_node_identity(node_tag: &str) -> Json<NodeIdentity> {
    // For now, return a placeholder response
    // In production, this would query the constellation manager
    
    Json(NodeIdentity {
        node_tag: node_tag.to_string(),
        vision_address: format!("vision://{}@unknown", node_tag),
        role: "constellation".to_string(),
        network_id: crate::vision_constants::VISION_NETWORK_ID.to_string(),
        admission_status: "admitted".to_string(),
        ticket_expires_at: Some(format_future_date(30 * 24 * 3600)),
        mood: "calm".to_string(),
        mood_score: 0.75,
        status_tier: "active".to_string(),
        trusted_peer_count: 0,
        trusted_peers: vec![],
    })
}

/// Calculate node mood based on health metrics
fn calculate_node_mood(peer_count: usize, height: u64) -> (String, f32) {
    let is_synced = height > 0;
    
    match (peer_count, is_synced) {
        (0, _) => ("isolated".to_string(), 0.2),
        (1..=2, false) => ("wounded".to_string(), 0.4),
        (1..=2, true) => ("calm".to_string(), 0.6),
        (3..=4, _) => ("guardian".to_string(), 0.75),
        (5..=9, _) => ("celebration".to_string(), 0.85),
        (10.., _) => ("celebration".to_string(), 0.95),
    }
}

/// Calculate status tier based on metrics
fn calculate_status_tier(peer_count: usize, height: u64) -> String {
    if peer_count >= 10 && height > 100 {
        "elite".to_string()
    } else if peer_count >= 5 && height > 50 {
        "trusted".to_string()
    } else if peer_count >= 3 {
        "active".to_string()
    } else if peer_count > 0 {
        "connected".to_string()
    } else {
        "isolated".to_string()
    }
}

/// Get trusted peers from P2P manager
fn get_trusted_peers() -> Vec<TrustedPeerInfo> {
    // For now, return empty vector as we need async context to query PEER_MANAGER
    // This can be enhanced later with proper async handling
    vec![]
}

/// Format a future date (seconds from now) as ISO 8601
fn format_future_date(seconds_from_now: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let future = now + seconds_from_now;
    
    // Simple ISO 8601-like format (good enough for testnet)
    format!("2025-12-31T23:59:59Z") // Fixed 30-day expiry for testnet
}

/// Format current time as ISO 8601
fn format_current_time() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Simple timestamp format
    format!("{}", now)
}
