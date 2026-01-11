#![allow(dead_code)]
//! Swarm Intelligence - Adaptive peer scoring and reputation system
//!
//! This module implements intelligent peer reputation management that prioritizes
//! reliable, fast, and well-synced peers while deprioritizing slow or unreliable nodes.
//!
//! Scoring Factors:
//! - Successful handshakes: +10 points
//! - Fast response (<100ms): +5 points
//! - Synced blockchain height: +5 points
//! - Valid messages: +2 points per message
//! - Timeouts: -15 points
//! - Invalid messages: -10 points
//! - Stale height (>10 blocks behind): -8 points
//! - Disconnects: -5 points
//!
//! Score Range: -1000 to +1000
//! Priority Tiers:
//! - Elite (800+): Anchor candidates, always preferred
//! - Excellent (500-799): High priority
//! - Good (200-499): Normal priority
//! - Fair (0-199): Low priority
//! - Poor (<0): Only used after exhausting all others

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Reputation score range
pub const MIN_REPUTATION_SCORE: i32 = -1000;
pub const MAX_REPUTATION_SCORE: i32 = 1000;

/// Score thresholds for priority tiers
pub const ELITE_THRESHOLD: i32 = 800;
pub const EXCELLENT_THRESHOLD: i32 = 500;
pub const GOOD_THRESHOLD: i32 = 200;
pub const FAIR_THRESHOLD: i32 = 0;

/// Anchor node eligibility threshold
pub const ANCHOR_MIN_SCORE: i32 = 750;

/// Score adjustments
pub const SCORE_HANDSHAKE_SUCCESS: i32 = 10;
pub const SCORE_FAST_RESPONSE: i32 = 5; // <100ms latency
pub const SCORE_SYNCED_HEIGHT: i32 = 5; // Within 2 blocks
pub const SCORE_VALID_MESSAGE: i32 = 2;

pub const SCORE_TIMEOUT: i32 = -15;
pub const SCORE_INVALID_MESSAGE: i32 = -10;
pub const SCORE_STALE_HEIGHT: i32 = -8; // >10 blocks behind
pub const SCORE_DISCONNECT: i32 = -5;

/// Reputation tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationTier {
    Elite,     // 800+
    Excellent, // 500-799
    Good,      // 200-499
    Fair,      // 0-199
    Poor,      // <0
}

impl ReputationTier {
    pub fn from_score(score: i32) -> Self {
        if score >= ELITE_THRESHOLD {
            ReputationTier::Elite
        } else if score >= EXCELLENT_THRESHOLD {
            ReputationTier::Excellent
        } else if score >= GOOD_THRESHOLD {
            ReputationTier::Good
        } else if score >= FAIR_THRESHOLD {
            ReputationTier::Fair
        } else {
            ReputationTier::Poor
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ReputationTier::Elite => "elite",
            ReputationTier::Excellent => "excellent",
            ReputationTier::Good => "good",
            ReputationTier::Fair => "fair",
            ReputationTier::Poor => "poor",
        }
    }
}

/// Peer reputation scoring metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReputation {
    /// Overall reputation score (-1000 to +1000)
    pub score: i32,

    /// Reputation tier
    pub tier: ReputationTier,

    /// Average response latency (milliseconds)
    pub avg_latency_ms: u32,

    /// Last known blockchain height
    pub last_height: u64,

    /// Number of valid messages received
    pub valid_messages: u32,

    /// Number of invalid messages received
    pub invalid_messages: u32,

    /// Number of successful handshakes
    pub successful_handshakes: u32,

    /// Number of timeouts
    pub timeout_count: u32,

    /// Last score update timestamp
    pub last_updated: u64,

    /// Anchor candidate flag
    pub is_anchor_candidate: bool,
}

impl Default for PeerReputation {
    fn default() -> Self {
        Self {
            score: 100, // New peers start slightly positive
            tier: ReputationTier::Fair,
            avg_latency_ms: 150,
            last_height: 0,
            valid_messages: 0,
            invalid_messages: 0,
            successful_handshakes: 0,
            timeout_count: 0,
            last_updated: current_time(),
            is_anchor_candidate: false,
        }
    }
}

impl PeerReputation {
    /// Update score after successful handshake
    pub fn record_handshake_success(&mut self, latency_ms: u32) {
        self.score = (self.score + SCORE_HANDSHAKE_SUCCESS).min(MAX_REPUTATION_SCORE);
        self.successful_handshakes += 1;

        // Fast response bonus
        if latency_ms < 100 {
            self.score = (self.score + SCORE_FAST_RESPONSE).min(MAX_REPUTATION_SCORE);
        }

        // Update average latency
        self.update_latency(latency_ms);

        self.update_tier();
        self.last_updated = current_time();

        info!(
            "[SWARM] Peer reputation increased: score={}, tier={}, latency={}ms",
            self.score,
            self.tier.label(),
            latency_ms
        );
    }

    /// Update score for synced blockchain height
    pub fn record_synced_height(&mut self, height: u64, our_height: u64) {
        self.last_height = height;

        let height_diff = our_height.abs_diff(height);

        if height_diff <= 2 {
            // Synced within 2 blocks
            self.score = (self.score + SCORE_SYNCED_HEIGHT).min(MAX_REPUTATION_SCORE);
            debug!(
                "[SWARM] Peer synced: height={}, diff={}",
                height, height_diff
            );
        } else if height_diff > 10 {
            // More than 10 blocks behind
            self.score = (self.score + SCORE_STALE_HEIGHT).max(MIN_REPUTATION_SCORE);
            warn!(
                "[SWARM] Peer has stale height: height={}, diff={}",
                height, height_diff
            );
        }

        self.update_tier();
        self.last_updated = current_time();
    }

    /// Update score for valid message
    pub fn record_valid_message(&mut self) {
        self.score = (self.score + SCORE_VALID_MESSAGE).min(MAX_REPUTATION_SCORE);
        self.valid_messages += 1;
        self.update_tier();
        self.last_updated = current_time();
    }

    /// Update score for invalid message
    pub fn record_invalid_message(&mut self) {
        self.score = (self.score + SCORE_INVALID_MESSAGE).max(MIN_REPUTATION_SCORE);
        self.invalid_messages += 1;
        self.update_tier();
        self.last_updated = current_time();

        warn!(
            "[SWARM] Peer sent invalid message: score={}, tier={}",
            self.score,
            self.tier.label()
        );
    }

    /// Update score for timeout
    pub fn record_timeout(&mut self) {
        self.score = (self.score + SCORE_TIMEOUT).max(MIN_REPUTATION_SCORE);
        self.timeout_count += 1;
        self.update_tier();
        self.last_updated = current_time();

        warn!(
            "[SWARM] Peer timeout: score={}, tier={}, total_timeouts={}",
            self.score,
            self.tier.label(),
            self.timeout_count
        );
    }

    /// Update score for disconnect
    pub fn record_disconnect(&mut self) {
        self.score = (self.score + SCORE_DISCONNECT).max(MIN_REPUTATION_SCORE);
        self.update_tier();
        self.last_updated = current_time();
    }

    /// Update average latency with exponential moving average
    fn update_latency(&mut self, new_latency_ms: u32) {
        // EMA with alpha = 0.3
        self.avg_latency_ms =
            ((self.avg_latency_ms as f32 * 0.7) + (new_latency_ms as f32 * 0.3)) as u32;
    }

    /// Recalculate reputation tier based on current score
    fn update_tier(&mut self) {
        let new_tier = ReputationTier::from_score(self.score);

        if new_tier != self.tier {
            info!(
                "[SWARM] Peer tier changed: {} → {}",
                self.tier.label(),
                new_tier.label()
            );
            self.tier = new_tier;
        }

        // Update anchor candidate status
        self.is_anchor_candidate = self.score >= ANCHOR_MIN_SCORE && self.avg_latency_ms < 150;
    }

    /// Check if peer is anchor candidate
    pub fn is_anchor_eligible(&self) -> bool {
        self.is_anchor_candidate
    }

    /// Get priority weight for connection attempts (higher = more priority)
    pub fn priority_weight(&self) -> i32 {
        match self.tier {
            ReputationTier::Elite => 1000,
            ReputationTier::Excellent => 500,
            ReputationTier::Good => 200,
            ReputationTier::Fair => 50,
            ReputationTier::Poor => 1,
        }
    }
}

/// Get current Unix timestamp
fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_tiers() {
        assert_eq!(ReputationTier::from_score(900), ReputationTier::Elite);
        assert_eq!(ReputationTier::from_score(600), ReputationTier::Excellent);
        assert_eq!(ReputationTier::from_score(300), ReputationTier::Good);
        assert_eq!(ReputationTier::from_score(50), ReputationTier::Fair);
        assert_eq!(ReputationTier::from_score(-100), ReputationTier::Poor);
    }

    #[test]
    fn test_score_updates() {
        let mut rep = PeerReputation::default();

        // Record successful handshake
        rep.record_handshake_success(50);
        assert!(rep.score > 100);
        assert_eq!(rep.successful_handshakes, 1);

        // Record timeout
        let score_before = rep.score;
        rep.record_timeout();
        assert!(rep.score < score_before);
        assert_eq!(rep.timeout_count, 1);
    }

    #[test]
    fn test_anchor_eligibility() {
        let mut rep = PeerReputation::default();
        rep.score = 800;
        rep.avg_latency_ms = 100;
        rep.update_tier();

        assert!(rep.is_anchor_eligible());
        assert_eq!(rep.tier, ReputationTier::Elite);
    }
}
