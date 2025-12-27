//! Stub implementation of mood module when staging feature is disabled
//! Non-custodial: provides safe default mood values, no signing/keys

use serde::{Deserialize, Serialize};

/// High-level network mood stub
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
    pub testnet_phase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodSnapshot {
    pub mood: NetworkMood,
    pub score: f32,
    pub reason: String,
    pub details: MoodDetails,
}

impl MoodSnapshot {
    pub fn as_public(&self) -> &Self {
        self
    }
}

impl Default for MoodSnapshot {
    fn default() -> Self {
        Self {
            mood: NetworkMood::Calm,
            score: 0.5,
            reason: "Default stub mood (staging disabled)".to_string(),
            details: MoodDetails {
                chain_health: ChainHealth::Bootstrapping,
                mempool_pressure: MempoolPressure::Low,
                network_activity: NetworkActivity::Isolated,
                guardian_active: false,
                recent_trauma_count: 0,
                recent_anomaly_count: 0,
                testnet_phase: None,
            },
        }
    }
}

/// Stub compute_mood: always returns default calm mood
pub fn compute_mood(
    _chain_height: u64,
    _peer_count: usize,
    _mempool_size: usize,
    _pending_anomalies: u32,
    _recent_traumas: u32,
    _guardian_active: bool,
    _testnet_phase: Option<String>,
) -> MoodSnapshot {
    MoodSnapshot::default()
}
