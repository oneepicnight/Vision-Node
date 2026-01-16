//! Stub for mood_router module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropagationStrategy {
    Broadcast,
    Targeted,
    Silent,
}

impl Default for PropagationStrategy {
    fn default() -> Self {
        Self::Broadcast
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxRelayPriority {
    Low,
    Normal,
    High,
}

impl Default for TxRelayPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStrategy {
    Full,
    Partial,
    None,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        Self::None
    }
}

pub fn route_by_mood(_mood: &str) -> PropagationStrategy {
    PropagationStrategy::default()
}

pub fn select_relay_priority(_mood: &str) -> TxRelayPriority {
    TxRelayPriority::default()
}

pub fn select_sync_strategy(_mood: &str) -> SyncStrategy {
    SyncStrategy::default()
}
