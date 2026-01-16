//! Trusted Peers + Mood Endpoints
//!
//! Provides HTTP API endpoints for querying Vision Peer Book with mood analysis:
//! - GET /api/peers/trusted - List of trusted peers with mood scores
//! - GET /api/peers/moods - Mood distribution and analysis for all peers

use crate::{p2p::peer_store::PeerStore, CHAIN};
use axum::Json;
use serde::{Deserialize, Serialize};

// ===== RESPONSE TYPES =====

#[derive(Serialize, Deserialize)]
pub struct TrustedPeerEntry {
    pub node_id: String,
    pub node_tag: String,
    pub vision_address: String,
    pub fingerprint: String,
    pub role: String,
    pub last_seen: u64,
    pub mood: String,
    pub mood_score: f32,
}

#[derive(Serialize, Deserialize)]
pub struct TrustedPeersResponse {
    pub peers: Vec<TrustedPeerEntry>,
    pub count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PeerMoodEntry {
    pub node_tag: String,
    pub vision_address: String,
    pub mood: String,
    pub mood_score: f32,
}

#[derive(Serialize, Deserialize)]
pub struct MoodDistribution {
    pub calm: usize,
    pub warning: usize,
    pub storm: usize,
    pub wounded: usize,
    pub celebration: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PeerMoodsResponse {
    pub total: usize,
    pub distribution: MoodDistribution,
    pub peers: Vec<PeerMoodEntry>,
}

// ===== MOOD ENGINE FUNCTIONS =====

/// Get mood state for a peer by node_id
pub fn peer_mood_for(node_id: &str) -> String {
    let chain = CHAIN.lock();
    if let Ok(peer_store) = PeerStore::new(&chain.db) {
        if let Some(peer) = peer_store.get(node_id) {
            if let Some(mood_info) = peer.mood {
                return mood_info.label;
            }
        }
    }
    "unknown".to_string()
}

/// Get mood score for a peer by node_id (0.0 - 1.0)
pub fn peer_mood_score(node_id: &str) -> f32 {
    let chain = CHAIN.lock();
    if let Ok(peer_store) = PeerStore::new(&chain.db) {
        if let Some(peer) = peer_store.get(node_id) {
            if let Some(mood_info) = peer.mood {
                return mood_info.score;
            }
        }
    }
    0.0
}

// ===== API ENDPOINTS =====

/// GET /api/peers/trusted
/// Returns list of trusted peers from the Vision Peer Book with mood analysis
pub async fn get_trusted_peers() -> Json<TrustedPeersResponse> {
    let chain = CHAIN.lock();

    let peers = match PeerStore::new(&chain.db) {
        Ok(peer_store) => peer_store
            .get_trusted()
            .into_iter()
            .map(|peer| {
                let mood = peer
                    .mood
                    .as_ref()
                    .map(|m| m.label.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let mood_score = peer.mood.as_ref().map(|m| m.score).unwrap_or(0.0);

                TrustedPeerEntry {
                    node_id: peer.node_id,
                    node_tag: peer.node_tag,
                    vision_address: peer.vision_address,
                    fingerprint: peer.admission_ticket_fingerprint,
                    role: peer.role,
                    last_seen: peer.last_seen as u64,
                    mood,
                    mood_score,
                }
            })
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    let count = peers.len();
    drop(chain);

    Json(TrustedPeersResponse { peers, count })
}

/// GET /api/peers/moods
/// Returns mood analysis and distribution for all peers in the Vision Peer Book
pub async fn get_peer_moods() -> Json<PeerMoodsResponse> {
    let chain = CHAIN.lock();

    let mut calm_count = 0;
    let mut warning_count = 0;
    let mut storm_count = 0;
    let mut wounded_count = 0;
    let mut celebration_count = 0;

    let peers = match PeerStore::new(&chain.db) {
        Ok(peer_store) => {
            peer_store
                .all()
                .into_iter()
                .filter_map(|peer| {
                    peer.mood.as_ref().map(|mood_info| {
                        // Count mood distribution
                        match mood_info.label.as_str() {
                            "calm" => calm_count += 1,
                            "warning" => warning_count += 1,
                            "storm" => storm_count += 1,
                            "wounded" => wounded_count += 1,
                            "celebration" => celebration_count += 1,
                            _ => {}
                        }

                        PeerMoodEntry {
                            node_tag: peer.node_tag,
                            vision_address: peer.vision_address,
                            mood: mood_info.label.clone(),
                            mood_score: mood_info.score,
                        }
                    })
                })
                .collect::<Vec<_>>()
        }
        Err(_) => vec![],
    };

    let total = peers.len();
    drop(chain);

    Json(PeerMoodsResponse {
        total,
        distribution: MoodDistribution {
            calm: calm_count,
            warning: warning_count,
            storm: storm_count,
            wounded: wounded_count,
            celebration: celebration_count,
        },
        peers,
    })
}
