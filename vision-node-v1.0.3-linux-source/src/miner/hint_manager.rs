#![allow(dead_code)]
//! # Hint Manager - Local Validation & Trial Scheduling
//!
//! This module manages received P2P tuning hints with strict local validation.
//! Philosophy: "No blind trust. Local validation required."
//!
//! ## Workflow
//! 1. Receive hint from P2P network
//! 2. Filter by CPU bucket, algo, reputation
//! 3. Schedule trial (test for 60s evaluation window)
//! 4. Compare results: adopt if ≥3% improvement, reject otherwise
//! 5. Track verified/rejected hints to build reputation signal

use crate::miner::tuning_hint::{CpuBucket, MinerTuningHint};
use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Status of a received hint's local validation trial.
#[derive(Debug, Clone, PartialEq)]
pub enum HintTrialStatus {
    /// Hint received, queued for trial
    Pending,

    /// Currently testing this hint in evaluation window
    Testing {
        started_at: u64,
        baseline_hashrate: f64,
    },

    /// Hint validated - improvement ≥ threshold, adopted permanently
    Verified { tested_at: u64, measured_gain: f64 },

    /// Hint rejected - improvement < threshold or failed trial
    Rejected {
        tested_at: u64,
        measured_gain: f64,
        reason: String,
    },
}

/// A received hint with tracking metadata.
#[derive(Debug, Clone)]
pub struct ReceivedHint {
    /// The hint data itself
    pub hint: MinerTuningHint,

    /// Reputation score of the peer who sent this hint (0.0-100.0)
    pub peer_reputation: f32,

    /// Unix timestamp when we received this hint
    pub received_at: u64,

    /// Current validation status
    pub status: HintTrialStatus,
}

impl ReceivedHint {
    /// Create a new received hint.
    pub fn new(hint: MinerTuningHint, peer_reputation: f32) -> Self {
        let received_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            hint,
            peer_reputation,
            received_at,
            status: HintTrialStatus::Pending,
        }
    }
}

/// Configuration for hint validation behavior.
#[derive(Debug, Clone)]
pub struct HintManagerConfig {
    /// Enable P2P hints system
    pub enabled: bool,

    /// Minimum gain ratio to adopt a hint (default: 0.03 = 3%)
    pub trial_threshold: f64,

    /// Maximum pending hints before dropping new ones
    pub max_pending: usize,

    /// Minimum peer reputation to accept hints from (0.0-100.0)
    pub min_peer_reputation: f32,

    /// Evaluation window duration in seconds for testing hints
    pub evaluation_window_secs: u64,

    /// Maximum hints to test per hour (rate limiting)
    pub max_trials_per_hour: usize,

    /// Broadcast interval in minutes for sharing elite configs
    pub broadcast_interval_mins: u64,
}

impl Default for HintManagerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trial_threshold: 0.03, // 3%
            max_pending: 50,
            min_peer_reputation: 30.0,
            evaluation_window_secs: 60,
            max_trials_per_hour: 10,
            broadcast_interval_mins: 30,
        }
    }
}

/// Manages received hints and coordinates local validation trials.
pub struct HintManager {
    /// Configuration
    config: HintManagerConfig,

    /// Our CPU bucket (for filtering relevant hints)
    our_cpu_bucket: CpuBucket,

    /// Currently pending hints (not yet tested)
    pending_hints: VecDeque<ReceivedHint>,

    /// Currently testing hint (if any)
    testing_hint: Option<ReceivedHint>,

    /// Verified hints (adopted) - keyed by config signature
    verified_hints: HashMap<String, ReceivedHint>,

    /// Rejected hints - keyed by config signature
    rejected_hints: HashMap<String, ReceivedHint>,

    /// Trial history for rate limiting (timestamps of trials)
    trial_timestamps: VecDeque<u64>,

    /// Last broadcast time (for elite config sharing)
    last_broadcast: u64,
}

impl HintManager {
    /// Create a new hint manager.
    pub fn new(config: HintManagerConfig) -> Self {
        let our_cpu_bucket = CpuBucket::detect();

        Self {
            config,
            our_cpu_bucket,
            pending_hints: VecDeque::new(),
            testing_hint: None,
            verified_hints: HashMap::new(),
            rejected_hints: HashMap::new(),
            trial_timestamps: VecDeque::new(),
            last_broadcast: 0,
        }
    }

    /// Receive a hint from P2P network.
    ///
    /// Filters and queues hint for validation if it passes checks:
    /// 1. System enabled
    /// 2. Hint is sane (threads/batch bounds)
    /// 3. Hint is fresh (< 7 days old)
    /// 4. CPU bucket is similar to ours
    /// 5. Peer reputation meets minimum threshold
    /// 6. Not already verified/rejected
    /// 7. Pending queue not full
    pub fn receive_hint(&mut self, hint: MinerTuningHint, peer_reputation: f32) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Sanity checks
        if !hint.is_sane() {
            warn!(
                "Received insane hint: threads={}, batch={}",
                hint.threads, hint.batch_size
            );
            return false;
        }

        if !hint.is_fresh() {
            debug!("Received stale hint (age > 7 days)");
            return false;
        }

        // CPU bucket similarity check
        if !self.our_cpu_bucket.is_similar(&hint.cpu_bucket) {
            debug!(
                "Hint CPU bucket not similar to ours: {:?} vs {:?}",
                hint.cpu_bucket, self.our_cpu_bucket
            );
            return false;
        }

        // Reputation check
        if peer_reputation < self.config.min_peer_reputation {
            debug!(
                "Peer reputation {} below minimum {}",
                peer_reputation, self.config.min_peer_reputation
            );
            return false;
        }

        // Check if already verified/rejected
        let signature = self.hint_signature(&hint);
        if self.verified_hints.contains_key(&signature) {
            debug!("Hint already verified: {}", signature);
            return false;
        }
        if self.rejected_hints.contains_key(&signature) {
            debug!("Hint already rejected: {}", signature);
            return false;
        }

        // Check pending queue capacity
        if self.pending_hints.len() >= self.config.max_pending {
            warn!(
                "Pending hints queue full ({}), dropping hint",
                self.config.max_pending
            );
            return false;
        }

        // Queue for validation
        let received = ReceivedHint::new(hint.clone(), peer_reputation);
        self.pending_hints.push_back(received);

        info!("Queued hint for validation: algo={}, threads={}, batch={}, gain={:.1}%, confidence={:.2}, peer_rep={:.1}",
            hint.pow_algo, hint.threads, hint.batch_size, hint.gain_ratio * 100.0,
            hint.confidence, peer_reputation);

        true
    }

    /// Get the next hint to test, if rate limits allow.
    ///
    /// Returns highest priority hint from pending queue, prioritized by:
    /// 1. priority_score() (gain * confidence * freshness)
    /// 2. peer_reputation (tie-breaker)
    pub fn get_next_trial(&mut self) -> Option<ReceivedHint> {
        if !self.config.enabled || self.testing_hint.is_some() {
            return None;
        }

        // Check rate limits
        if !self.can_start_trial() {
            debug!(
                "Rate limit: {} trials in last hour, max {}",
                self.recent_trial_count(),
                self.config.max_trials_per_hour
            );
            return None;
        }

        if self.pending_hints.is_empty() {
            return None;
        }

        // Find highest priority hint
        let mut best_idx = 0;
        let mut best_score = 0.0;

        for (i, received) in self.pending_hints.iter().enumerate() {
            let hint_score = received.hint.priority_score();
            let reputation_bonus = (received.peer_reputation / 100.0) as f64;
            let total_score = hint_score + reputation_bonus * 0.1; // 10% reputation weight

            if total_score > best_score {
                best_score = total_score;
                best_idx = i;
            }
        }

        // Remove from pending queue
        let mut hint = self.pending_hints.remove(best_idx)?;

        // Mark as testing
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        hint.status = HintTrialStatus::Testing {
            started_at: now,
            baseline_hashrate: 0.0, // Will be set by caller
        };

        self.testing_hint = Some(hint.clone());
        self.trial_timestamps.push_back(now);

        info!(
            "Starting trial for hint: algo={}, threads={}, batch={}, expected_gain={:.1}%",
            hint.hint.pow_algo,
            hint.hint.threads,
            hint.hint.batch_size,
            hint.hint.gain_ratio * 100.0
        );

        Some(hint)
    }

    /// Complete current trial with results.
    ///
    /// Compares measured hashrate against baseline:
    /// - If gain ≥ threshold: mark Verified, move to verified_hints
    /// - If gain < threshold: mark Rejected, move to rejected_hints
    pub fn complete_trial(&mut self, measured_hashrate: f64, baseline_hashrate: f64) {
        let Some(mut hint) = self.testing_hint.take() else {
            warn!("complete_trial called but no trial in progress");
            return;
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let measured_gain = (measured_hashrate - baseline_hashrate) / baseline_hashrate;
        let signature = self.hint_signature(&hint.hint);

        if measured_gain >= self.config.trial_threshold {
            // Verified - adopt this hint
            hint.status = HintTrialStatus::Verified {
                tested_at: now,
                measured_gain,
            };

            info!("✓ Hint VERIFIED: algo={}, threads={}, batch={}, measured_gain={:.1}%, expected={:.1}%",
                hint.hint.pow_algo, hint.hint.threads, hint.hint.batch_size,
                measured_gain * 100.0, hint.hint.gain_ratio * 100.0);

            self.verified_hints.insert(signature, hint);
        } else {
            // Rejected - not worth adopting
            let reason = format!(
                "Measured gain {:.1}% below threshold {:.1}%",
                measured_gain * 100.0,
                self.config.trial_threshold * 100.0
            );

            hint.status = HintTrialStatus::Rejected {
                tested_at: now,
                measured_gain,
                reason: reason.clone(),
            };

            info!("✗ Hint REJECTED: algo={}, threads={}, batch={}, measured_gain={:.1}%, expected={:.1}% - {}",
                hint.hint.pow_algo, hint.hint.threads, hint.hint.batch_size,
                measured_gain * 100.0, hint.hint.gain_ratio * 100.0, reason);

            self.rejected_hints.insert(signature, hint);
        }
    }

    /// Cancel current trial (e.g., due to error or timeout).
    pub fn cancel_trial(&mut self, reason: &str) {
        if let Some(mut hint) = self.testing_hint.take() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            hint.status = HintTrialStatus::Rejected {
                tested_at: now,
                measured_gain: 0.0,
                reason: reason.to_string(),
            };

            let signature = self.hint_signature(&hint.hint);
            self.rejected_hints.insert(signature, hint);

            warn!("Trial cancelled: {}", reason);
        }
    }

    /// Check if we can start a new trial (rate limiting).
    fn can_start_trial(&mut self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Remove trials older than 1 hour
        let one_hour_ago = now.saturating_sub(3600);
        while let Some(&ts) = self.trial_timestamps.front() {
            if ts < one_hour_ago {
                self.trial_timestamps.pop_front();
            } else {
                break;
            }
        }

        self.trial_timestamps.len() < self.config.max_trials_per_hour
    }

    /// Get count of trials in last hour.
    fn recent_trial_count(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let one_hour_ago = now.saturating_sub(3600);

        self.trial_timestamps
            .iter()
            .filter(|&&ts| ts >= one_hour_ago)
            .count()
    }

    /// Check if it's time to broadcast our elite configs.
    pub fn should_broadcast(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let elapsed = now.saturating_sub(self.last_broadcast);
        elapsed >= self.config.broadcast_interval_mins * 60
    }

    /// Mark that we just broadcasted.
    pub fn mark_broadcasted(&mut self) {
        self.last_broadcast = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Generate a unique signature for a hint (for deduplication).
    fn hint_signature(&self, hint: &MinerTuningHint) -> String {
        format!(
            "{}-{}-{}-{}",
            hint.pow_algo,
            hint.threads,
            hint.batch_size,
            hint.numa_node.unwrap_or(0)
        )
    }

    /// Get statistics for monitoring.
    pub fn stats(&self) -> HintStats {
        HintStats {
            pending_count: self.pending_hints.len(),
            testing: self.testing_hint.is_some(),
            verified_count: self.verified_hints.len(),
            rejected_count: self.rejected_hints.len(),
            recent_trials: self.recent_trial_count(),
        }
    }
}

/// Statistics about hint validation.
#[derive(Debug, Clone)]
pub struct HintStats {
    pub pending_count: usize,
    pub testing: bool,
    pub verified_count: usize,
    pub rejected_count: usize,
    pub recent_trials: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_hint(threads: usize, batch: u32, gain: f64) -> MinerTuningHint {
        MinerTuningHint::new(
            CpuBucket {
                family: "Intel".to_string(),
                cores: 8,
                threads: 16,
            },
            "vision-pow-v1".to_string(),
            threads,
            batch,
            gain,
            10,
            None,
        )
    }

    #[test]
    fn test_receive_hint_basic() {
        let mut manager = HintManager::new(HintManagerConfig::default());
        let hint = make_test_hint(12, 256, 0.10);

        assert!(manager.receive_hint(hint, 50.0));
        assert_eq!(manager.pending_hints.len(), 1);
    }

    #[test]
    fn test_receive_hint_low_reputation() {
        let mut manager = HintManager::new(HintManagerConfig::default());
        let hint = make_test_hint(12, 256, 0.10);

        // Reputation below minimum (30.0)
        assert!(!manager.receive_hint(hint, 20.0));
        assert_eq!(manager.pending_hints.len(), 0);
    }

    #[test]
    fn test_trial_priority() {
        let config = HintManagerConfig {
            max_trials_per_hour: 100, // No rate limit for test
            ..Default::default()
        };
        let mut manager = HintManager::new(config);

        // Add hints with different gains
        let weak_hint = make_test_hint(8, 128, 0.05); // 5% gain
        let strong_hint = make_test_hint(12, 256, 0.15); // 15% gain

        manager.receive_hint(weak_hint, 50.0);
        manager.receive_hint(strong_hint, 50.0);

        // Should select strong hint first
        let trial = manager.get_next_trial().unwrap();
        assert_eq!(trial.hint.threads, 12);
        assert_eq!(trial.hint.gain_ratio, 0.15);
    }

    #[test]
    fn test_complete_trial_verified() {
        let config = HintManagerConfig {
            trial_threshold: 0.03,
            max_trials_per_hour: 100,
            ..Default::default()
        };
        let mut manager = HintManager::new(config);

        let hint = make_test_hint(12, 256, 0.10);
        manager.receive_hint(hint, 50.0);

        let _trial = manager.get_next_trial().unwrap();

        // Measured 5% gain (above 3% threshold)
        manager.complete_trial(105.0, 100.0);

        assert!(manager.testing_hint.is_none());
        assert_eq!(manager.verified_hints.len(), 1);
        assert_eq!(manager.rejected_hints.len(), 0);
    }

    #[test]
    fn test_complete_trial_rejected() {
        let config = HintManagerConfig {
            trial_threshold: 0.03,
            max_trials_per_hour: 100,
            ..Default::default()
        };
        let mut manager = HintManager::new(config);

        let hint = make_test_hint(12, 256, 0.10);
        manager.receive_hint(hint, 50.0);

        let _trial = manager.get_next_trial().unwrap();

        // Measured 1% gain (below 3% threshold)
        manager.complete_trial(101.0, 100.0);

        assert!(manager.testing_hint.is_none());
        assert_eq!(manager.verified_hints.len(), 0);
        assert_eq!(manager.rejected_hints.len(), 1);
    }
}
