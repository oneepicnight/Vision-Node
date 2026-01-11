#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// High-level network mood we expose to UI / router / guardian.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMood {
    Calm,
    Warning,
    Storm,
    Celebration,
    Guardian,
    Wounded,
    Rage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainHealth {
    Bootstrapping,
    Healthy,
    Mature,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MempoolPressure {
    Low,
    Moderate,
    High,
    Maxed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkActivity {
    Isolated,
    Moderate,
    High,
    Surging,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodDetails {
    pub chain_health: ChainHealth,
    pub mempool_pressure: MempoolPressure,
    pub network_activity: NetworkActivity,
    pub guardian_active: bool,
    pub recent_trauma_count: u32,
    pub recent_anomaly_count: u32,
    pub testnet_phase: Option<String>, // "mining" | "hybrid" | "staking-only"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodSnapshot {
    pub mood: NetworkMood,
    pub score: f32, // 0.0 - 1.0, overall health
    pub reason: String,
    pub details: MoodDetails,
}

impl MoodSnapshot {
    pub fn as_public(&self) -> &Self {
        self
    }
}

/// Main entry point: compute current MoodSnapshot from chain + health DB.
pub fn compute_mood(
    chain_height: u64,
    peer_count: usize,
    mempool_size: usize,
    pending_anomalies: u32,
    recent_traumas: u32,
    guardian_active: bool,
    testnet_phase: Option<String>,
) -> MoodSnapshot {
    // ----- Chain health -----
    let chain_health = if chain_height < 10 {
        ChainHealth::Bootstrapping
    } else if pending_anomalies == 0 && recent_traumas == 0 {
        ChainHealth::Mature
    } else if pending_anomalies < 3 && recent_traumas < 2 {
        ChainHealth::Healthy
    } else if pending_anomalies < 5 {
        ChainHealth::Degraded
    } else {
        ChainHealth::Critical
    };

    // ----- Mempool pressure -----
    let mempool_pressure = if mempool_size == 0 {
        MempoolPressure::Low
    } else if mempool_size < 100 {
        MempoolPressure::Moderate
    } else if mempool_size < 1000 {
        MempoolPressure::High
    } else {
        MempoolPressure::Maxed
    };

    // ----- Network activity -----
    let network_activity = if peer_count == 0 {
        NetworkActivity::Isolated
    } else if peer_count < 5 {
        NetworkActivity::Moderate
    } else if peer_count < 20 {
        NetworkActivity::High
    } else {
        NetworkActivity::Surging
    };

    // ----- Base score -----
    let mut score: f32 = 1.0;

    // Anomalies & traumas reduce score
    score -= (pending_anomalies as f32) * 0.05;
    score -= (recent_traumas as f32) * 0.07;

    // Mempool pressure penalty
    match mempool_pressure {
        MempoolPressure::Low => {}
        MempoolPressure::Moderate => score -= 0.05,
        MempoolPressure::High => score -= 0.15,
        MempoolPressure::Maxed => score -= 0.25,
    }

    // Chain health penalties
    match chain_health {
        ChainHealth::Bootstrapping => score -= 0.05,
        ChainHealth::Healthy => {}
        ChainHealth::Mature => {}
        ChainHealth::Degraded => score -= 0.15,
        ChainHealth::Critical => score -= 0.35,
    }

    // Guardian gives a small safety bump if active
    if guardian_active {
        score += 0.05;
    }

    score = score.clamp(0.0, 1.0);

    // ----- Pick mood from score + context -----
    let mood = if recent_traumas >= 3 && score < 0.3 {
        NetworkMood::Rage
    } else if recent_traumas > 0 && score < 0.4 {
        NetworkMood::Wounded
    } else if guardian_active && pending_anomalies > 0 && score >= 0.3 {
        NetworkMood::Guardian
    } else if score >= 0.8 && recent_traumas == 0 && pending_anomalies == 0 {
        NetworkMood::Celebration
    } else if pending_anomalies >= 3 || score < 0.4 {
        NetworkMood::Storm
    } else if pending_anomalies > 0 || recent_traumas > 0 {
        NetworkMood::Warning
    } else {
        NetworkMood::Calm
    };

    // ----- Reason string -----
    let reason = match mood {
        NetworkMood::Calm => "Network stable. Constellation breathing in rhythm.".to_string(),
        NetworkMood::Warning => "Minor anomalies detected. Eyes open, Dreamer.".to_string(),
        NetworkMood::Storm => {
            "Elevated anomaly load. Storm clouds over the constellation.".to_string()
        }
        NetworkMood::Celebration => "Healed and humming. The chain sings tonight.".to_string(),
        NetworkMood::Guardian => "Guardian in motion. Self-healing routines engaged.".to_string(),
        NetworkMood::Wounded => {
            "Recent trauma recorded. Network is recovering its breath.".to_string()
        }
        NetworkMood::Rage => "Maximum chaos. The void howls. Every node matters now.".to_string(),
    };

    MoodSnapshot {
        mood,
        score,
        reason,
        details: MoodDetails {
            chain_health,
            mempool_pressure,
            network_activity,
            guardian_active,
            recent_trauma_count: recent_traumas,
            recent_anomaly_count: pending_anomalies,
            testnet_phase,
        },
    }
}

// ============================================================================
// PER-NODE MOOD COMPUTATION
// ============================================================================
// Track health metrics for individual peers in the Vision Peer Book.
// Scores nodes based on latency, sync status, reputation, and failure history.

use crate::p2p::peer_store::NodeMoodInfo;

/// Snapshot of a node's current health metrics
#[derive(Debug, Clone)]
pub struct NodeHealthSnapshot {
    pub latency_ms: Option<u64>,
    pub height_gap: i64,
    pub reputation: f32,
    pub recent_traumas: u32,
}

/// Compute per-node mood based on health metrics
/// Returns mood label (calm/warning/storm/wounded) with score 0.0-1.0
pub fn compute_node_mood(snapshot: &NodeHealthSnapshot) -> NodeMoodInfo {
    let mut score = 1.0_f32;
    let mut penalties = Vec::new();

    // Latency penalty
    if let Some(latency) = snapshot.latency_ms {
        if latency > 1000 {
            score -= 0.3;
            penalties.push(format!("High latency: {}ms", latency));
        } else if latency > 500 {
            score -= 0.2;
            penalties.push(format!("Elevated latency: {}ms", latency));
        }
    }

    // Height gap penalty (blockchain sync status)
    let height_gap = snapshot.height_gap.abs();
    if height_gap > 100 {
        score -= 0.4;
        penalties.push(format!(
            "Severely out of sync: {} blocks behind",
            height_gap
        ));
    } else if height_gap > 10 {
        score -= 0.3;
        penalties.push(format!("Out of sync: {} blocks behind", height_gap));
    }

    // Reputation penalty
    if snapshot.reputation < 0.7 {
        score -= 0.2;
        penalties.push(format!("Low reputation: {:.2}", snapshot.reputation));
    }

    // Trauma/failure penalty
    if snapshot.recent_traumas > 5 {
        score -= 0.4;
        penalties.push(format!(
            "High failure rate: {} recent traumas",
            snapshot.recent_traumas
        ));
    } else if snapshot.recent_traumas > 2 {
        score -= 0.2;
        penalties.push(format!(
            "Multiple failures: {} recent traumas",
            snapshot.recent_traumas
        ));
    }

    // Clamp score to valid range
    score = score.clamp(0.0, 1.0);

    // Determine mood label
    let label = if score >= 0.85 {
        "calm".to_string()
    } else if score >= 0.65 {
        "warning".to_string()
    } else if score >= 0.45 {
        "storm".to_string()
    } else {
        "wounded".to_string()
    };

    // Build reason string
    let reason = if penalties.is_empty() {
        "Node healthy and responsive".to_string()
    } else {
        penalties.join("; ")
    };

    NodeMoodInfo {
        label,
        score,
        reason,
        last_updated: chrono::Utc::now().timestamp(),
    }
}

// ============================================================================
// PEER MOOD QUERY FUNCTIONS (For API endpoints)
// ============================================================================

/// Get mood state label for a peer by node_id
pub fn peer_mood_for(node_id: &str) -> String {
    let chain = crate::CHAIN.lock();
    if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
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
    let chain = crate::CHAIN.lock();
    if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
        if let Some(peer) = peer_store.get(node_id) {
            if let Some(mood_info) = peer.mood {
                return mood_info.score;
            }
        }
    }
    0.0
}
