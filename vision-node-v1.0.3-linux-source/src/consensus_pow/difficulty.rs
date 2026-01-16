//! Smooth per-block difficulty adjustment using LWMA (Linearly Weighted Moving Average)
//!
//! Implements a responsive, oscillation-resistant difficulty adjustment algorithm
//! that maintains consistent 2-second block times even with hash rate fluctuations.
//!
//! Key features:
//! - Per-block adjustment (no waiting for intervals)
//! - LWMA gives more weight to recent blocks
//! - Clamped solve times prevent timestamp manipulation
//! - Clamped per-block changes prevent oscillation
//! - Smooth response to hash rate changes

use crate::pow::U256;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for LWMA difficulty adjustment
#[derive(Clone, Debug)]
pub struct DifficultyConfig {
    /// Target block time in seconds (e.g., 2)
    pub target_block_time: u64,

    /// LWMA window size in blocks (e.g., 120 for ~4 minutes of history at 2s blocks)
    pub adjustment_interval: u64, // Keep name for compatibility, but now it's window size

    /// Minimum solve time clamp (fraction of target, e.g., 4 means target/4 = 0.5s for 2s target)
    pub min_solve_divisor: u64,

    /// Maximum solve time multiplier (e.g., 10 means 10*target = 20s for 2s target)
    pub max_solve_multiplier: u64,

    /// Maximum per-block difficulty increase (e.g., 110 for +10%)
    pub max_change_up_percent: u64,

    /// Maximum per-block difficulty decrease (e.g., 90 for -10%)
    pub max_change_down_percent: u64,

    /// Maximum adjustment factor (kept for compatibility, deprecated in favor of per-block clamps)
    pub max_adjustment_factor: f64,

    /// Minimum difficulty (prevents going too low)
    pub min_difficulty: u64,
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        Self {
            target_block_time: 2,        // 2 seconds per block
            adjustment_interval: 120,    // 120 block window (~4 minutes)
            min_solve_divisor: 4,        // Min solve time = 0.5s
            max_solve_multiplier: 10,    // Max solve time = 20s
            max_change_up_percent: 110,  // Max +10% per block
            max_change_down_percent: 90, // Max -10% per block
            max_adjustment_factor: 4.0,  // Deprecated, kept for compatibility
            min_difficulty: 1000,        // Minimum difficulty floor
        }
    }
}

/// Multiply U256 by a ratio (num/den) using 64-bit arithmetic
/// Returns floor(target * num / den)
/// Note: u256_from_difficulty() stores targets in the upper 64 bits (bytes 0-8)
#[inline]
fn u256_mul_div(target: &U256, num: u128, den: u128) -> U256 {
    if num == den {
        return *target;
    }

    // Extract upper 64 bits where u256_from_difficulty stores the target
    let mut hi_bytes = [0u8; 8];
    hi_bytes.copy_from_slice(&target[0..8]);
    let hi = u64::from_be_bytes(hi_bytes);

    // Calculate: (hi * num) / den using u128 to prevent overflow
    let hi_u128 = hi as u128;
    let result = (hi_u128.saturating_mul(num) / den.max(1)) as u64;

    // Enforce minimum target to prevent zero (which would make mining impossible)
    // For difficulty 1_000_000: 0xFFFFFFFFFFFFFFFF / 1_000_000 ≈ 0x000010C6
    let min_target = 0x0000_1000u64; // Minimum ~4096 in upper 64 bits
    let result = result.max(min_target);

    // Convert back to U256 (big-endian, upper 64 bits only)
    let mut out = [0u8; 32];
    out[0..8].copy_from_slice(&result.to_be_bytes());
    out
}

/// Clamp a ratio to prevent extreme per-block changes
/// Returns (numerator, denominator) clamped to [down%, up%]
#[inline]
fn clamp_ratio(
    ratio_num: u128,
    ratio_den: u128,
    max_up_percent: u64,
    max_down_percent: u64,
) -> (u128, u128) {
    let ratio = (ratio_num as f64) / (ratio_den as f64);

    // Convert percentages to multipliers: 110% = 1.10, 90% = 0.90
    let max_up = (max_up_percent as f64) / 100.0;
    let max_down = (max_down_percent as f64) / 100.0;

    let clamped = ratio.clamp(max_down, max_up);

    // Return as scaled fraction with 1e9 granularity
    let scale = 1_000_000_000u128;
    let num = ((clamped * scale as f64).round() as u128).max(1);
    (num, scale)
}

/// Calculate next difficulty using LWMA over recent block timestamps
///
/// This is the core algorithm that runs PER BLOCK to smoothly adjust difficulty.
///
/// # Arguments
/// * `timestamps` - Slice of block timestamps (oldest to newest), ideally config.adjustment_interval in length
/// * `prev_target` - Current difficulty target (big-endian U256, lower = harder)
/// * `config` - Difficulty adjustment configuration
///
/// # Returns
/// New target for the next block (lower = harder difficulty)
pub fn next_target_lwma(timestamps: &[u64], prev_target: &U256, config: &DifficultyConfig) -> U256 {
    let n = timestamps.len();
    if n < 2 {
        return *prev_target;
    }

    let target_secs = config.target_block_time as i64;
    let min_dt = (config.target_block_time / config.min_solve_divisor).max(1) as i64;
    let max_dt = (config.target_block_time * config.max_solve_multiplier) as i64;

    // Compute LWMA of solve times with clamps
    // Weights: 1, 2, 3, ..., N (linearly increasing for recency emphasis)
    let mut sum_weights: i64 = 0;
    let mut weighted_sum: i128 = 0;

    for k in 1..n {
        let weight = k as i64; // More weight to newer blocks
        sum_weights += weight;

        // Calculate solve time for this block
        let raw_dt = (timestamps[k] as i64) - (timestamps[k - 1] as i64);

        // Clamp solve time to prevent timestamp manipulation
        let clamped_dt = raw_dt.clamp(min_dt, max_dt);

        weighted_sum += (clamped_dt as i128) * (weight as i128);
    }

    if sum_weights == 0 {
        return *prev_target;
    }

    // Calculate weighted average solve time
    let lwma_dt = (weighted_sum / (sum_weights as i128)).max(1) as i64;

    // Calculate ratio: actual_time / target_time
    // If blocks are faster than target, ratio < 1, so target decreases (harder)
    // If blocks are slower than target, ratio > 1, so target increases (easier)
    let ratio_num = lwma_dt.max(1) as u128;
    let ratio_den = target_secs.max(1) as u128;

    // Clamp single-step change to prevent oscillation
    let (num_clamped, den_clamped) = clamp_ratio(
        ratio_num,
        ratio_den,
        config.max_change_up_percent,
        config.max_change_down_percent,
    );

    // Apply ratio: next_target = prev_target * (actual / target)

    u256_mul_div(prev_target, num_clamped, den_clamped)
}

/// Legacy function kept for compatibility
/// Now uses LWMA internally
pub fn calculate_next_difficulty(
    config: &DifficultyConfig,
    current_difficulty: u64,
    block_times: &[u64], // timestamps of recent blocks
) -> u64 {
    let current_target = difficulty_to_target(current_difficulty);
    let next_target = next_target_lwma(block_times, &current_target, config);
    target_to_difficulty(&next_target).max(config.min_difficulty)
}

/// Convert difficulty to U256 target
/// Higher difficulty = lower target (harder to mine)
/// Convert difficulty to U256 target
/// Higher difficulty = lower target (harder to mine)
pub fn difficulty_to_target(difficulty: u64) -> U256 {
    crate::pow::u256_from_difficulty(difficulty)
}

/// Convert U256 target to difficulty scalar
/// Lower target = higher difficulty
/// Note: u256_from_difficulty() stores targets in the upper 64 bits (bytes 0-8)
pub fn target_to_difficulty(target: &U256) -> u64 {
    // Extract upper 64 bits for calculation
    let mut hi_bytes = [0u8; 8];
    hi_bytes.copy_from_slice(&target[0..8]);
    let hi = u64::from_be_bytes(hi_bytes);

    if hi == 0 {
        return u64::MAX;
    }

    // Invert: difficulty ≈ max_target / target
    let max_target = 0xFFFFFFFFFFFFFFFFu64;

    (max_target / hi.max(1)).max(1000)
}

/// Difficulty metrics for monitoring and UI display
#[derive(Debug, Clone)]
pub struct DifficultyMetrics {
    /// Current difficulty scalar (higher = harder)
    pub difficulty: u64,
    /// Current target (lower = harder)
    pub target: U256,
    /// Last block time in seconds
    pub last_block_time: f64,
    /// Average block time over window in seconds
    pub avg_block_time: f64,
    /// Adjustment ratio applied (1.0 = no change, >1.0 = easier, <1.0 = harder)
    pub adjustment_ratio: f64,
    /// Number of blocks in calculation window
    pub window_size: usize,
}

/// Calculate difficulty metrics for the current chain state
pub fn calculate_metrics(
    timestamps: &[u64],
    current_target: &U256,
    next_target: &U256,
) -> DifficultyMetrics {
    let n = timestamps.len();

    // Calculate average block time
    let avg_block_time = if n > 1 {
        let total_time = timestamps[n - 1] - timestamps[0];
        (total_time as f64) / ((n - 1) as f64)
    } else {
        2.0
    };

    // Last block time
    let last_block_time = if n >= 2 {
        (timestamps[n - 1] - timestamps[n - 2]) as f64
    } else {
        2.0
    };

    // Scalar difficulty (inverse of target, normalized)
    let difficulty = target_to_difficulty(current_target);

    // Adjustment ratio (how much target changed)
    let adjustment_ratio = calculate_target_ratio(current_target, next_target);

    DifficultyMetrics {
        difficulty,
        target: *current_target,
        last_block_time,
        avg_block_time,
        adjustment_ratio,
        window_size: n,
    }
}

/// Calculate the ratio between two targets (next / current)
fn calculate_target_ratio(current: &U256, next: &U256) -> f64 {
    // Targets are stored in the upper 64 bits (bytes 0..8) by u256_from_difficulty
    let mut curr_bytes = [0u8; 8];
    curr_bytes.copy_from_slice(&current[0..8]);
    let curr_hi = u64::from_be_bytes(curr_bytes) as f64;

    let mut next_bytes = [0u8; 8];
    next_bytes.copy_from_slice(&next[0..8]);
    let next_hi = u64::from_be_bytes(next_bytes) as f64;

    if curr_hi == 0.0 {
        return 1.0;
    }

    next_hi / curr_hi
}

/// Get current unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Difficulty tracker for managing per-block adjustments
pub struct DifficultyTracker {
    config: DifficultyConfig,
    current_difficulty: u64,
    current_target: U256,
    block_timestamps: Vec<u64>,
    last_metrics: Option<DifficultyMetrics>,
}

impl DifficultyTracker {
    pub fn new(config: DifficultyConfig, initial_difficulty: u64) -> Self {
        let current_target = difficulty_to_target(initial_difficulty);
        Self {
            config,
            current_difficulty: initial_difficulty,
            current_target,
            block_timestamps: Vec::new(),
            last_metrics: None,
        }
    }

    /// Record a new block timestamp and adjust difficulty
    /// This now runs PER BLOCK (smooth adjustment)
    pub fn record_block(&mut self, timestamp: u64) {
        self.block_timestamps.push(timestamp);

        // Keep window + some extra for metrics
        let keep = (self.config.adjustment_interval * 2) as usize;
        if self.block_timestamps.len() > keep {
            self.block_timestamps
                .drain(0..self.block_timestamps.len() - keep);
        }

        // Adjust difficulty EVERY block (not just at intervals)
        if self.block_timestamps.len() >= 2 {
            self.adjust_difficulty();
        }
    }

    /// Perform per-block difficulty adjustment using LWMA
    fn adjust_difficulty(&mut self) {
        // Use most recent N blocks for LWMA window
        let window_size = self.config.adjustment_interval as usize;
        let start = self.block_timestamps.len().saturating_sub(window_size);
        let timestamps = &self.block_timestamps[start..];

        if timestamps.len() < 2 {
            return;
        }

        // Calculate next target using LWMA
        let next_target = next_target_lwma(timestamps, &self.current_target, &self.config);

        // Calculate metrics before updating
        let metrics = calculate_metrics(timestamps, &self.current_target, &next_target);

        // Update target (difficulty is just for display)
        let old_target = self.current_target;
        self.current_target = next_target;

        // Keep difficulty synchronized (for display purposes)
        // Don't recalculate from target as that can cause drift
        let target_ratio = calculate_target_ratio(&old_target, &next_target);
        self.current_difficulty = ((self.current_difficulty as f64) / target_ratio) as u64;
        self.current_difficulty = self.current_difficulty.max(self.config.min_difficulty);

        // Log significant changes (more than 2%)
        let change_pct = (target_ratio - 1.0) * 100.0;
        if change_pct.abs() > 2.0 {
            eprintln!(
                "⚡ Difficulty adjusted (target ratio: {:+.1}%), avg block time: {:.2}s",
                change_pct, metrics.avg_block_time
            );
        }

        self.last_metrics = Some(metrics);
    }

    /// Get current difficulty
    pub fn current_difficulty(&self) -> u64 {
        self.current_difficulty
    }

    /// Get current target
    pub fn current_target(&self) -> U256 {
        self.current_target
    }

    /// Get last calculated metrics
    pub fn metrics(&self) -> Option<&DifficultyMetrics> {
        self.last_metrics.as_ref()
    }

    /// Get average block time over recent blocks
    pub fn average_block_time(&self) -> Option<f64> {
        if self.block_timestamps.len() < 2 {
            return None;
        }

        let window_size = self
            .config
            .adjustment_interval
            .min(self.block_timestamps.len() as u64) as usize;
        let start = self.block_timestamps.len().saturating_sub(window_size);
        let timestamps = &self.block_timestamps[start..];

        if timestamps.len() < 2 {
            return None;
        }

        let total_time = timestamps.last().unwrap() - timestamps.first().unwrap();
        let num_intervals = (timestamps.len() - 1) as f64;

        Some(total_time as f64 / num_intervals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lwma_steady_state() {
        let config = DifficultyConfig::default();

        // Simulate 120 blocks at exactly 2s intervals (steady state)
        let mut timestamps = Vec::new();
        for i in 0..120 {
            timestamps.push(i * 2);
        }

        // Start with some arbitrary target
        let mut target_bytes = [0u8; 32];
        target_bytes[0..8].copy_from_slice(&100_000_000_000u64.to_be_bytes());
        let initial_target = target_bytes;

        let next = next_target_lwma(&timestamps, &initial_target, &config);

        // Should be very close to initial target (steady state)
        let ratio = calculate_target_ratio(&initial_target, &next);
        assert!(
            (ratio - 1.0).abs() < 0.15,
            "Ratio {} should be near 1.0",
            ratio
        );
    }

    #[test]
    fn test_lwma_faster_blocks() {
        let config = DifficultyConfig::default();

        // Simulate 120 blocks at 1s intervals (too fast - need harder difficulty)
        let mut timestamps = Vec::new();
        for i in 0..120 {
            timestamps.push(i * 1);
        }

        let mut target_bytes = [0u8; 32];
        target_bytes[0..8].copy_from_slice(&100_000_000_000u64.to_be_bytes());
        let initial_target = target_bytes;

        let next = next_target_lwma(&timestamps, &initial_target, &config);

        // Target should decrease (harder) because blocks are too fast
        let ratio = calculate_target_ratio(&initial_target, &next);
        assert!(
            ratio < 1.0,
            "Next target should be lower (harder) but ratio is {}",
            ratio
        );
    }

    #[test]
    fn test_lwma_slower_blocks() {
        let config = DifficultyConfig::default();

        // Simulate 120 blocks at 4s intervals (too slow - need easier difficulty)
        let mut timestamps = Vec::new();
        for i in 0..120 {
            timestamps.push(i * 4);
        }

        let mut target_bytes = [0u8; 32];
        target_bytes[0..8].copy_from_slice(&100_000_000_000u64.to_be_bytes());
        let initial_target = target_bytes;

        let next = next_target_lwma(&timestamps, &initial_target, &config);

        // Target should increase (easier) because blocks are too slow
        let ratio = calculate_target_ratio(&initial_target, &next);
        assert!(
            ratio > 1.0,
            "Next target should be higher (easier) but ratio is {}",
            ratio
        );
    }

    #[test]
    fn test_per_block_change_clamped() {
        let config = DifficultyConfig::default();

        // Simulate extremely fast blocks (1 per 0.1s)
        let mut timestamps = Vec::new();
        for i in 0..120 {
            timestamps.push(i / 10);
        }

        let mut target_bytes = [0u8; 32];
        target_bytes[0..8].copy_from_slice(&100_000_000_000u64.to_be_bytes());
        let initial_target = target_bytes;

        let next = next_target_lwma(&timestamps, &initial_target, &config);

        // Even with extremely fast blocks, change should be clamped to ~10% per block
        let ratio = calculate_target_ratio(&initial_target, &next);
        assert!(
            ratio >= 0.85,
            "Change should be clamped to ~-10%, got ratio {}",
            ratio
        );
        assert!(
            ratio <= 1.15,
            "Change should be clamped to ~+10%, got ratio {}",
            ratio
        );
    }

    #[test]
    fn test_difficulty_tracker_per_block() {
        let config = DifficultyConfig {
            adjustment_interval: 10, // Small window for test
            target_block_time: 2,
            ..Default::default()
        };
        let mut tracker = DifficultyTracker::new(config, 10000);

        // Record 20 blocks at target time (2s)
        for i in 0..20 {
            tracker.record_block(i * 2);
        }

        // Difficulty should remain relatively stable
        let final_diff = tracker.current_difficulty();
        let change_pct = ((final_diff as f64 / 10000.0) - 1.0).abs() * 100.0;
        assert!(
            change_pct < 20.0,
            "Difficulty changed by {:.1}%, expected <20%",
            change_pct
        );
    }

    #[test]
    fn test_difficulty_tracker_responds_to_hash_change() {
        let config = DifficultyConfig {
            adjustment_interval: 20,
            target_block_time: 2,
            ..Default::default()
        };
        let mut tracker = DifficultyTracker::new(config, 10000);

        // Start with normal 2s blocks
        for i in 0..20 {
            tracker.record_block(i * 2);
        }
        let initial_diff = tracker.current_difficulty();

        // Simulate hash rate spike (1s blocks)
        for i in 20..40 {
            let base = 20 * 2;
            tracker.record_block(base + (i - 20) * 1);
        }

        let final_diff = tracker.current_difficulty();

        // Difficulty should have increased due to faster blocks
        assert!(
            final_diff > initial_diff,
            "Difficulty should increase with faster blocks: {} -> {}",
            initial_diff,
            final_diff
        );
    }

    #[test]
    fn test_average_block_time() {
        let config = DifficultyConfig::default();
        let mut tracker = DifficultyTracker::new(config, 10000);

        // Record blocks at 2.5s intervals
        for i in 0..60 {
            tracker.record_block((i * 25) / 10); // 2.5s per block
        }

        let avg = tracker.average_block_time().unwrap();
        assert!(
            (avg - 2.5).abs() < 0.1,
            "Average should be ~2.5s, got {:.2}s",
            avg
        );
    }

    #[test]
    fn test_metrics_calculation() {
        let config = DifficultyConfig::default();
        let mut tracker = DifficultyTracker::new(config, 10000);

        // Record some blocks
        for i in 0..30 {
            tracker.record_block(i * 2);
        }

        let metrics = tracker.metrics();
        assert!(metrics.is_some(), "Metrics should be available");

        let m = metrics.unwrap();
        assert!(
            m.avg_block_time > 0.0,
            "Average block time should be positive"
        );
        assert!(m.window_size > 0, "Window size should be positive");
    }

    // Legacy compatibility tests
    #[test]
    fn test_difficulty_increases_when_blocks_fast() {
        let config = DifficultyConfig::default();
        let current = 10000;

        // Simulate blocks coming faster than target (1s instead of 2s)
        let timestamps: Vec<u64> = (0..120).map(|i| i * 1).collect();

        let new_difficulty = calculate_next_difficulty(&config, current, &timestamps);

        // Difficulty should increase (blocks too fast)
        assert!(new_difficulty > current, "Expected difficulty to increase");
    }

    #[test]
    fn test_difficulty_decreases_when_blocks_slow() {
        let config = DifficultyConfig::default();
        let current = 10000;

        // Simulate blocks coming slower than target (4s instead of 2s)
        let timestamps: Vec<u64> = (0..120).map(|i| i * 4).collect();

        let new_difficulty = calculate_next_difficulty(&config, current, &timestamps);

        // Difficulty should decrease (blocks too slow)
        assert!(new_difficulty < current, "Expected difficulty to decrease");
    }
}
