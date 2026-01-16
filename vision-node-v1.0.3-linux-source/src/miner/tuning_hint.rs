#![allow(dead_code)]
//! # P2P Tuning Hints - Privacy-Safe Config Sharing
//!
//! This module enables miners to share proven configurations via P2P gossip without
//! revealing identity or full hardware specs.
//!
//! ## Key Principles
//! - **Privacy**: Only CPU bucket (normalized family/cores), never addresses
//! - **Survival of Fittest**: Good configs propagate, bad ones die locally
//! - **No Blind Trust**: All hints validated locally before adoption
//! - **Reputation-Aware**: Trust established peers more, but still validate

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Normalized CPU bucket for grouping similar hardware without revealing exact specs.
///
/// This provides enough information for relevant config matching while preserving privacy.
///
/// # Examples
/// ```
/// // Intel i9-9900K (8C/16T) -> Bucket { family: "Intel", cores: 8, threads: 16 }
/// // AMD Ryzen 9 5950X (16C/32T) -> Bucket { family: "AMD", cores: 16, threads: 32 }
/// // Generic 4C/4T -> Bucket { family: "Unknown", cores: 4, threads: 4 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CpuBucket {
    /// CPU family/vendor (Intel, AMD, ARM, Unknown)
    pub family: String,

    /// Physical core count
    pub cores: usize,

    /// Logical thread count (including SMT/HyperThreading)
    pub threads: usize,
}

impl CpuBucket {
    /// Detect current system's CPU bucket using sysinfo.
    pub fn detect() -> Self {
        use sysinfo::{CpuRefreshKind, RefreshKind, System};

        let mut sys =
            System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        sys.refresh_cpu();

        let cpus = sys.cpus();
        let threads = cpus.len();

        // Estimate physical cores (threads / 2 for SMT, or threads if no SMT)
        // This is a heuristic - real detection would need platform-specific code
        let cores = if threads >= 4 && threads.is_multiple_of(2) {
            threads / 2 // Assume SMT enabled
        } else {
            threads
        };

        // Detect vendor from CPU brand
        let family = if let Some(cpu) = cpus.first() {
            let brand = cpu.brand().to_lowercase();
            if brand.contains("intel") {
                "Intel".to_string()
            } else if brand.contains("amd") {
                "AMD".to_string()
            } else if brand.contains("arm") || brand.contains("apple") {
                "ARM".to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        Self {
            family,
            cores,
            threads,
        }
    }

    /// Check if another bucket is "close enough" to consider hints applicable.
    ///
    /// Hints are applicable if:
    /// - Same vendor family (Intel/AMD/ARM)
    /// - Core count within ±2
    /// - Thread count within ±4
    pub fn is_similar(&self, other: &CpuBucket) -> bool {
        if self.family != other.family {
            return false;
        }

        let core_diff = (self.cores as i32 - other.cores as i32).abs();
        let thread_diff = (self.threads as i32 - other.threads as i32).abs();

        core_diff <= 2 && thread_diff <= 4
    }
}

/// A privacy-safe mining configuration shared over P2P.
///
/// Contains enough information to be actionable without revealing miner identity.
/// Broadcasted when a miner discovers a high-performing config (hashrate improvement ≥5%).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerTuningHint {
    /// CPU bucket this hint applies to
    pub cpu_bucket: CpuBucket,

    /// PoW algorithm this config is for (vision-pow-v1, vision-pow-v2, etc.)
    pub pow_algo: String,

    /// Recommended thread count
    pub threads: usize,

    /// Recommended batch size
    pub batch_size: u32,

    /// Approximate hashrate improvement this config achieved (0.0 to 1.0)
    ///
    /// This is relative to the "default" config for this CPU bucket.
    /// Example: 0.15 means 15% improvement over baseline.
    pub gain_ratio: f64,

    /// How many samples this config was tested with
    ///
    /// Higher sample count = more confidence in gain_ratio.
    /// Minimum 5 samples required before broadcast.
    pub sample_count: usize,

    /// Confidence score (0.0 to 1.0) based on sample count and variance
    ///
    /// Formula: min(1.0, sample_count / 20.0)
    /// - 5 samples = 0.25 confidence
    /// - 10 samples = 0.50 confidence
    /// - 20+ samples = 1.0 confidence
    pub confidence: f64,

    /// Unix timestamp when this hint was created
    pub timestamp: u64,

    /// Optional: NUMA node this config was optimized for (if NUMA-aware)
    pub numa_node: Option<usize>,
}

impl MinerTuningHint {
    /// Create a new tuning hint from performance data.
    pub fn new(
        cpu_bucket: CpuBucket,
        pow_algo: String,
        threads: usize,
        batch_size: u32,
        gain_ratio: f64,
        sample_count: usize,
        numa_node: Option<usize>,
    ) -> Self {
        let confidence = Self::calculate_confidence(sample_count);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            cpu_bucket,
            pow_algo,
            threads,
            batch_size,
            gain_ratio,
            sample_count,
            confidence,
            timestamp,
            numa_node,
        }
    }

    /// Calculate confidence score based on sample count.
    ///
    /// More samples = higher confidence in the gain_ratio measurement.
    fn calculate_confidence(sample_count: usize) -> f64 {
        (sample_count as f64 / 20.0).min(1.0)
    }

    /// Check if this hint is worth broadcasting to the network.
    ///
    /// Broadcast criteria:
    /// - Sample count ≥ 5 (minimum statistical significance)
    /// - Gain ratio ≥ 5% (meaningful improvement)
    /// - Confidence ≥ 0.25 (not just random noise)
    pub fn is_broadcast_worthy(&self) -> bool {
        self.sample_count >= 5 && self.gain_ratio >= 0.05 && self.confidence >= 0.25
    }

    /// Check if this hint's configuration is sane (within reasonable bounds).
    ///
    /// Sanity checks:
    /// - Threads: 1-128 (beyond 128 is suspicious)
    /// - Batch size: 1-10000 (beyond 10k is likely spam)
    /// - Gain ratio: 0.0-10.0 (beyond 10x is unrealistic)
    pub fn is_sane(&self) -> bool {
        self.threads >= 1
            && self.threads <= 128
            && self.batch_size >= 1
            && self.batch_size <= 10000
            && self.gain_ratio >= 0.0
            && self.gain_ratio <= 10.0
    }

    /// Check if this hint is fresh enough to consider.
    ///
    /// Hints older than 7 days are discarded (network dynamics change).
    pub fn is_fresh(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let age_secs = now.saturating_sub(self.timestamp);
        const MAX_AGE_SECS: u64 = 7 * 24 * 60 * 60; // 7 days

        age_secs < MAX_AGE_SECS
    }

    /// Get a priority score for this hint (used for trial scheduling).
    ///
    /// Higher score = higher priority for local validation.
    /// Score formula: gain_ratio * confidence * freshness_factor
    ///
    /// Freshness factor decays linearly:
    /// - 0-1 day: 1.0x
    /// - 1-3 days: 0.8x
    /// - 3-7 days: 0.5x
    pub fn priority_score(&self) -> f64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let age_secs = now.saturating_sub(self.timestamp);
        let age_days = age_secs as f64 / 86400.0;

        let freshness = if age_days < 1.0 {
            1.0
        } else if age_days < 3.0 {
            0.8
        } else if age_days < 7.0 {
            0.5
        } else {
            0.1 // Very stale
        };

        self.gain_ratio * self.confidence * freshness
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_bucket_detection() {
        let bucket = CpuBucket::detect();
        assert!(bucket.threads > 0);
        assert!(bucket.cores > 0);
        assert!(bucket.cores <= bucket.threads);
    }

    #[test]
    fn test_cpu_bucket_similarity() {
        let intel_8c = CpuBucket {
            family: "Intel".to_string(),
            cores: 8,
            threads: 16,
        };

        let intel_6c = CpuBucket {
            family: "Intel".to_string(),
            cores: 6,
            threads: 12,
        };

        let amd_8c = CpuBucket {
            family: "AMD".to_string(),
            cores: 8,
            threads: 16,
        };

        // Same vendor, close core count
        assert!(intel_8c.is_similar(&intel_6c));

        // Different vendor
        assert!(!intel_8c.is_similar(&amd_8c));
    }

    #[test]
    fn test_hint_broadcast_worthy() {
        let hint = MinerTuningHint::new(
            CpuBucket {
                family: "Intel".to_string(),
                cores: 8,
                threads: 16,
            },
            "vision-pow-v1".to_string(),
            12,
            256,
            0.08, // 8% gain
            6,    // 6 samples
            None,
        );

        assert!(hint.is_broadcast_worthy());

        // Too few samples
        let weak_hint = MinerTuningHint::new(
            CpuBucket {
                family: "Intel".to_string(),
                cores: 8,
                threads: 16,
            },
            "vision-pow-v1".to_string(),
            12,
            256,
            0.08,
            3, // Only 3 samples
            None,
        );

        assert!(!weak_hint.is_broadcast_worthy());
    }

    #[test]
    fn test_hint_sanity_checks() {
        let sane = MinerTuningHint::new(
            CpuBucket {
                family: "AMD".to_string(),
                cores: 16,
                threads: 32,
            },
            "vision-pow-v1".to_string(),
            24,
            512,
            0.12,
            10,
            None,
        );

        assert!(sane.is_sane());

        // Insane thread count
        let mut insane = sane.clone();
        insane.threads = 500;
        assert!(!insane.is_sane());

        // Insane batch size
        let mut insane = sane.clone();
        insane.batch_size = 50000;
        assert!(!insane.is_sane());
    }

    #[test]
    fn test_hint_priority_score() {
        let hint = MinerTuningHint::new(
            CpuBucket {
                family: "Intel".to_string(),
                cores: 8,
                threads: 16,
            },
            "vision-pow-v1".to_string(),
            12,
            256,
            0.15, // 15% gain
            20,   // Max confidence
            None,
        );

        let score = hint.priority_score();

        // Fresh hint with 15% gain and 1.0 confidence
        // Score should be close to 0.15 * 1.0 * 1.0 = 0.15
        assert!(score >= 0.14 && score <= 0.16);
    }
}
