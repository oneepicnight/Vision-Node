#![allow(dead_code)]

use crate::mood::{MoodSnapshot, NetworkMood};
use tracing::{info, warn};

/// Mood-influenced routing decisions for the P2P network
pub struct MoodRouter {
    current_mood: Option<MoodSnapshot>,
}

impl MoodRouter {
    pub fn new() -> Self {
        Self { current_mood: None }
    }

    pub fn update_mood(&mut self, mood: MoodSnapshot) {
        self.current_mood = Some(mood);
    }

    /// Get recommended connection count based on mood
    pub fn recommended_peer_count(&self) -> usize {
        match self.current_mood.as_ref().map(|m| m.mood) {
            Some(NetworkMood::Calm) => 8,
            Some(NetworkMood::Warning) => 12,
            Some(NetworkMood::Storm) => 16,
            Some(NetworkMood::Celebration) => 6,
            Some(NetworkMood::Guardian) => 20, // Guardian needs more peers to monitor
            Some(NetworkMood::Wounded) => 10,
            Some(NetworkMood::Rage) => 24, // Maximum connectivity during chaos
            None => 8,                     // Default
        }
    }

    /// Get block propagation strategy based on mood
    pub fn block_propagation_strategy(&self) -> PropagationStrategy {
        match self.current_mood.as_ref().map(|m| m.mood) {
            Some(NetworkMood::Calm) | Some(NetworkMood::Celebration) => {
                PropagationStrategy::Standard
            }
            Some(NetworkMood::Warning) | Some(NetworkMood::Wounded) => {
                PropagationStrategy::Cautious
            }
            Some(NetworkMood::Storm) | Some(NetworkMood::Rage) => PropagationStrategy::Aggressive,
            Some(NetworkMood::Guardian) => PropagationStrategy::Guardian,
            None => PropagationStrategy::Standard,
        }
    }

    /// Get transaction relay priority based on mood
    pub fn tx_relay_priority(&self) -> TxRelayPriority {
        match self.current_mood.as_ref().map(|m| m.mood) {
            Some(NetworkMood::Calm) | Some(NetworkMood::Celebration) => TxRelayPriority::Normal,
            Some(NetworkMood::Warning) => TxRelayPriority::CriticalOnly,
            Some(NetworkMood::Storm) | Some(NetworkMood::Rage) => TxRelayPriority::MinimalCritical,
            Some(NetworkMood::Guardian) => TxRelayPriority::GuardianControlled,
            Some(NetworkMood::Wounded) => TxRelayPriority::Throttled,
            None => TxRelayPriority::Normal,
        }
    }

    /// Should this node participate in consensus?
    pub fn should_participate_in_consensus(&self) -> bool {
        match self.current_mood.as_ref().map(|m| m.mood) {
            Some(NetworkMood::Rage) => {
                warn!("[MOOD ROUTER] Network in RAGE - limiting consensus participation");
                false
            }
            Some(NetworkMood::Storm) => {
                // Participate but with caution
                true
            }
            _ => true,
        }
    }

    /// Get sync strategy based on mood
    pub fn sync_strategy(&self) -> SyncStrategy {
        match self.current_mood.as_ref().map(|m| m.mood) {
            Some(NetworkMood::Calm) | Some(NetworkMood::Celebration) => SyncStrategy::Fast,
            Some(NetworkMood::Warning) | Some(NetworkMood::Wounded) => SyncStrategy::Validated,
            Some(NetworkMood::Storm) | Some(NetworkMood::Rage) => SyncStrategy::Conservative,
            Some(NetworkMood::Guardian) => SyncStrategy::GuardianAssisted,
            None => SyncStrategy::Fast,
        }
    }

    /// Log mood-based decision
    pub fn log_decision(&self, decision: &str) {
        if let Some(mood_data) = &self.current_mood {
            info!(
                "[MOOD ROUTER] Decision: {} | Mood: {:?} | Score: {:.2}",
                decision, mood_data.mood, mood_data.score
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PropagationStrategy {
    Standard,   // Normal gossip
    Cautious,   // Verify before relay
    Aggressive, // Broadcast to all peers immediately
    Guardian,   // Guardian-coordinated propagation
}

#[derive(Debug, Clone, Copy)]
pub enum TxRelayPriority {
    Normal,             // Relay all valid transactions
    CriticalOnly,       // Only relay high-priority/critical txs
    MinimalCritical,    // Minimal relay, critical only
    GuardianControlled, // Guardian decides what to relay
    Throttled,          // Rate-limited relay
}

#[derive(Debug, Clone, Copy)]
pub enum SyncStrategy {
    Fast,             // Trust and sync quickly
    Validated,        // Verify each block
    Conservative,     // Deep validation, slow but safe
    GuardianAssisted, // Use Guardian as trusted source
}

impl Default for MoodRouter {
    fn default() -> Self {
        Self::new()
    }
}
