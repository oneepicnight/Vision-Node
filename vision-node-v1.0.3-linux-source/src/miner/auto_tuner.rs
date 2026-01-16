//! Auto-Tuning Engine for Mining Performance
//!
//! Intelligent decision-making for optimal mining configuration based on
//! historical performance data and exploration strategies.

use crate::config::miner::{AutoTuneMode, MinerConfig};
use crate::miner::perf_store::{MinerPerfStore, PerfKey};
use std::time::Instant;

/// State for auto-tuning process
#[derive(Debug)]
pub struct AutoTuneState {
    pub last_reeval: Instant,
    pub pending_change: Option<TuneDecision>,
}

impl AutoTuneState {
    pub fn new() -> Self {
        Self {
            last_reeval: Instant::now(),
            pending_change: None,
        }
    }

    pub fn should_reevaluate(&self, interval_secs: u64) -> bool {
        self.last_reeval.elapsed().as_secs() >= interval_secs
    }

    pub fn mark_reevaluated(&mut self) {
        self.last_reeval = Instant::now();
    }
}

impl Default for AutoTuneState {
    fn default() -> Self {
        Self::new()
    }
}

/// Decision for configuration change
#[derive(Debug, Clone)]
pub struct TuneDecision {
    pub new_threads: usize,
    pub new_batch: u32,
    pub reason: String,
}

/// Decide optimal mining configuration based on historical performance
pub fn decide_new_tuning(
    cpu_model: &str,
    profile: &str,
    current_threads: usize,
    current_batch: u32,
    config: &MinerConfig,
    perf_store: &MinerPerfStore,
) -> Option<TuneDecision> {
    // 1) Check if we have proven best configuration for this CPU+profile
    if let Some((best_key, best_stats)) = perf_store.best_for_cpu_and_profile(cpu_model, profile) {
        // If current settings are already optimal, no change needed
        if best_key.threads == current_threads && best_key.batch_size == current_batch {
            return None;
        }

        // Only adopt if we have enough samples and significant improvement
        if best_stats.sample_count >= 3 && best_stats.best_hashrate > 0.0 {
            // Check if we have current config stats to compare
            let current_key = PerfKey {
                cpu_model: cpu_model.to_string(),
                profile: profile.to_string(),
                pow_algo: "vision-pow-v1".to_string(), // Current PoW algorithm
                threads: current_threads,
                batch_size: current_batch,
            };

            let improvement_threshold = match config.auto_tune_mode {
                AutoTuneMode::Conservative => 0.05, // 5% improvement required
                AutoTuneMode::Normal => 0.03,       // 3% improvement required
                AutoTuneMode::Aggressive => 0.01,   // 1% improvement required
            };

            let should_switch = if let Some(current_stats) = perf_store.get(&current_key) {
                // Compare to current performance
                let improvement = (best_stats.best_hashrate - current_stats.avg_hashrate)
                    / current_stats.avg_hashrate;
                improvement > improvement_threshold
            } else {
                // No data on current config, trust the best known
                true
            };

            if should_switch {
                return Some(TuneDecision {
                    new_threads: best_key.threads,
                    new_batch: best_key.batch_size,
                    reason: format!(
                        "Using historical best for {}: {:.1} H/s with {} threads × batch {}",
                        cpu_model, best_stats.best_hashrate, best_key.threads, best_key.batch_size
                    ),
                });
            }
        }
    }

    // 2) Exploration: search for better configurations
    explore_new_configuration(
        cpu_model,
        profile,
        current_threads,
        current_batch,
        config,
        perf_store,
    )
}

/// Explore new configurations based on current settings and mode
fn explore_new_configuration(
    cpu_model: &str,
    profile: &str,
    current_threads: usize,
    current_batch: u32,
    config: &MinerConfig,
    perf_store: &MinerPerfStore,
) -> Option<TuneDecision> {
    // Determine exploration parameters based on mode
    let (thread_step, batch_multiplier) = match config.auto_tune_mode {
        AutoTuneMode::Conservative => (0.10, 1.5), // 10% thread change, 1.5x batch
        AutoTuneMode::Normal => (0.25, 2.0),       // 25% thread change, 2x batch
        AutoTuneMode::Aggressive => (0.50, 2.0),   // 50% thread change, 2x batch
    };

    // Get min/max bounds
    let min_threads = config
        .min_threads
        .unwrap_or((current_threads as f64 * 0.5) as usize)
        .max(1);
    let max_threads = config
        .max_threads
        .unwrap_or((current_threads as f64 * 2.0) as usize);
    let min_batch = config.min_batch_size.unwrap_or(1);
    let max_batch = config.max_batch_size.unwrap_or(32);

    // Get all tried configurations for this CPU+profile
    let _tried_configs = perf_store.all_for_cpu_and_profile(cpu_model, profile);

    // Generate candidate configurations
    let mut candidates = vec![
        // Try more threads
        (
            ((current_threads as f64 * (1.0 + thread_step)) as usize)
                .clamp(min_threads, max_threads),
            current_batch,
            "more threads",
        ),
        // Try fewer threads
        (
            ((current_threads as f64 * (1.0 - thread_step)) as usize)
                .clamp(min_threads, max_threads),
            current_batch,
            "fewer threads",
        ),
        // Try larger batch
        (
            current_threads,
            ((current_batch as f64 * batch_multiplier) as u32).clamp(min_batch, max_batch),
            "larger batch",
        ),
        // Try smaller batch (if not already at minimum)
        (
            current_threads,
            (current_batch / 2).clamp(min_batch, max_batch),
            "smaller batch",
        ),
        // Try both more threads and larger batch
        (
            ((current_threads as f64 * (1.0 + thread_step)) as usize)
                .clamp(min_threads, max_threads),
            ((current_batch as f64 * batch_multiplier) as u32).clamp(min_batch, max_batch),
            "more threads + larger batch",
        ),
    ];

    // Filter out candidates we've already tried extensively
    candidates.retain(|(threads, batch, _)| {
        let key = PerfKey {
            cpu_model: cpu_model.to_string(),
            profile: profile.to_string(),
            pow_algo: "vision-pow-v1".to_string(), // Current PoW algorithm
            threads: *threads,
            batch_size: *batch,
        };

        // Keep if we haven't tried it, or tried it but not enough samples
        match perf_store.get(&key) {
            None => true,
            Some(stats) => stats.sample_count < 5, // Allow retry if < 5 samples
        }
    });

    // Filter out candidates that are same as current
    candidates.retain(|(threads, batch, _)| *threads != current_threads || *batch != current_batch);

    // Pick the first untried or under-sampled candidate
    if let Some((new_threads, new_batch, reason)) = candidates.first() {
        return Some(TuneDecision {
            new_threads: *new_threads,
            new_batch: *new_batch,
            reason: format!(
                "Exploring {}: {} threads × batch {}",
                reason, new_threads, new_batch
            ),
        });
    }

    None
}

/// Calculate safe min/max bounds based on CPU and profile
pub fn calculate_safe_bounds(
    physical_cores: usize,
    logical_cores: usize,
    profile: &str,
    config: &MinerConfig,
) -> (usize, usize, u32, u32) {
    let (default_min_threads, default_max_threads) = match profile {
        "laptop" => (1, 4),
        "balanced" => (physical_cores / 2, logical_cores),
        "beast" => (logical_cores / 2, logical_cores * 2),
        _ => (physical_cores / 2, logical_cores),
    };

    let min_threads = config.min_threads.unwrap_or(default_min_threads).max(1);
    let max_threads = config.max_threads.unwrap_or(default_max_threads);

    let min_batch = config.min_batch_size.unwrap_or(1);
    let max_batch = config.max_batch_size.unwrap_or(match profile {
        "laptop" => 8,
        "balanced" => 16,
        "beast" => 32,
        _ => 16,
    });

    (min_threads, max_threads, min_batch, max_batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_auto_tune_state() {
        let mut state = AutoTuneState::new();
        assert!(!state.should_reevaluate(10));

        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!state.should_reevaluate(1));

        state.mark_reevaluated();
        assert!(!state.should_reevaluate(10));
    }

    #[test]
    fn test_calculate_safe_bounds() {
        let config = MinerConfig::default();

        let (min_t, max_t, min_b, max_b) = calculate_safe_bounds(8, 16, "balanced", &config);
        assert_eq!(min_t, 4);
        assert_eq!(max_t, 16);
        assert_eq!(min_b, 1);
        assert_eq!(max_b, 16);

        let (min_t, max_t, min_b, max_b) = calculate_safe_bounds(8, 16, "laptop", &config);
        assert_eq!(min_t, 1);
        assert_eq!(max_t, 4);
        assert_eq!(min_b, 1);
        assert_eq!(max_b, 8);
    }
}
