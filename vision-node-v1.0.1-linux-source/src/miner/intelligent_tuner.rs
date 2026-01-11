//! Enhanced Auto-Tuning Engine with Full Intelligence Integration
//!
//! Integrates thermal monitoring, power detection, NUMA awareness, and telemetry
//! for intelligent mining optimization.

use crate::config::miner::MinerConfig;
use crate::miner::{
    auto_tuner::AutoTuneState,
    hint_manager::{HintManager, HintManagerConfig},
    numa::{NumaConfig, NumaCoordinator},
    perf_store::MinerPerfStore,
    power::{PowerConfig, PowerMonitor},
    telemetry::{normalize_cpu_model, TelemSnapshot, TelemetryClient},
    thermal::{ThermalConfig, ThermalMonitor},
};
use std::sync::{Arc, Mutex};
use tracing::info;

/// Enhanced auto-tuner with full intelligence
pub struct IntelligentTuner {
    perf_store: Arc<Mutex<MinerPerfStore>>,
    telemetry: Option<TelemetryClient>,
    thermal_monitor: Arc<Mutex<ThermalMonitor>>,
    power_monitor: Arc<Mutex<PowerMonitor>>,
    numa_coordinator: Arc<Mutex<NumaCoordinator>>,
    hint_manager: Arc<Mutex<HintManager>>,
    auto_tune_state: Arc<Mutex<AutoTuneState>>,
    pow_algo: String,
}

impl IntelligentTuner {
    pub fn new(
        perf_store: Arc<Mutex<MinerPerfStore>>,
        config: &MinerConfig,
        pow_algo: String,
    ) -> Self {
        // Initialize telemetry client
        let telemetry = if config.telemetry_enabled {
            Some(TelemetryClient::new(
                config.telemetry_endpoint.clone(),
                true,
            ))
        } else {
            None
        };

        // Initialize thermal monitor
        let thermal_config = ThermalConfig {
            enabled: config.thermal_protection_enabled,
            soft_limit_c: config.thermal_soft_limit_c.unwrap_or(80),
            hard_limit_c: config.thermal_hard_limit_c.unwrap_or(90),
            cooldown_secs: config.thermal_cooldown_secs,
        };
        let thermal_monitor = Arc::new(Mutex::new(ThermalMonitor::new(thermal_config)));

        // Initialize power monitor
        let power_config = PowerConfig {
            sensitivity_enabled: config.power_mode_sensitivity,
            battery_threads_cap: config.battery_threads_cap,
            battery_batch_cap: config.battery_batch_cap,
        };
        let power_monitor = Arc::new(Mutex::new(PowerMonitor::new(power_config)));

        // Initialize NUMA coordinator
        let numa_config = NumaConfig {
            enabled: config.numa_aware_enabled,
        };
        let numa_coordinator = Arc::new(Mutex::new(NumaCoordinator::new(numa_config)));

        // Initialize hint manager
        let hint_config = HintManagerConfig {
            enabled: config.p2p_hints_enabled,
            trial_threshold: config.hint_trial_threshold,
            max_pending: config.hint_max_pending,
            min_peer_reputation: config.hint_min_peer_reputation,
            evaluation_window_secs: 60,
            max_trials_per_hour: 10,
            broadcast_interval_mins: config.hint_broadcast_interval_mins,
        };
        let hint_manager = Arc::new(Mutex::new(HintManager::new(hint_config)));

        Self {
            perf_store,
            telemetry,
            thermal_monitor,
            power_monitor,
            numa_coordinator,
            hint_manager,
            auto_tune_state: Arc::new(Mutex::new(AutoTuneState::new())),
            pow_algo,
        }
    }

    /// Decide optimal configuration with full intelligence
    pub fn decide_optimal_config(
        &self,
        cpu_model: &str,
        profile: &str,
        logical_cores: u32,
        _physical_cores: u32,
        current_threads: usize,
        current_batch: u32,
        _config: &MinerConfig,
    ) -> OptimalConfig {
        let mut optimal = OptimalConfig {
            threads: current_threads,
            batch_size: current_batch,
            constraints_applied: Vec::new(),
            should_mine: true,
        };

        // 1) Check thermal constraints
        let thermal_factor = {
            let mut monitor = self.thermal_monitor.lock().unwrap();
            monitor.sample(); // Update temperature reading

            let factor = monitor.get_throttle_factor();
            if factor < 1.0 {
                let reduced_threads = ((current_threads as f64 * factor) as usize).max(1);
                optimal.threads = reduced_threads;
                optimal.constraints_applied.push(format!(
                    "Thermal throttling: {}% capacity",
                    (factor * 100.0) as u32
                ));

                // If critical, pause mining temporarily
                if factor < 0.5 && monitor.should_throttle() {
                    optimal.should_mine = false;
                    optimal
                        .constraints_applied
                        .push("Critical temperature: mining paused".to_string());
                }
            }
            factor
        };

        // 2) Check power constraints
        {
            let mut power_monitor = self.power_monitor.lock().unwrap();
            let power_state = power_monitor.detect_power_state();

            if power_state.should_limit() {
                let capped_threads = power_monitor.apply_thread_cap(optimal.threads);
                let capped_batch = power_monitor.apply_batch_cap(current_batch);

                if capped_threads < optimal.threads || capped_batch < current_batch {
                    optimal.threads = capped_threads;
                    optimal.batch_size = capped_batch;
                    optimal.constraints_applied.push(format!(
                        "Battery mode: threads={}, batch={}",
                        capped_threads, capped_batch
                    ));
                }
            }
        }

        // If mining is paused due to constraints, return early
        if !optimal.should_mine {
            return optimal;
        }

        // 3) Check for better configurations from local store or telemetry
        let perf_store = self.perf_store.lock().unwrap();

        // Try algorithm-specific lookup first
        let best_config = perf_store
            .best_for_cpu_profile_algo(cpu_model, profile, &self.pow_algo)
            .or_else(|| {
                // Fallback to generic lookup (backward compatibility)
                perf_store.best_for_cpu_and_profile(cpu_model, profile)
            });

        if let Some((best_key, best_stats)) = best_config {
            if best_stats.sample_count >= 3 {
                // Apply thermal/power constraints to historical best
                let constrained_threads = {
                    let power_monitor = self.power_monitor.lock().unwrap();
                    let threads = ((best_key.threads as f64 * thermal_factor) as usize).max(1);
                    power_monitor.apply_thread_cap(threads)
                };

                let constrained_batch = {
                    let power_monitor = self.power_monitor.lock().unwrap();
                    power_monitor.apply_batch_cap(best_key.batch_size)
                };

                optimal.threads = constrained_threads;
                optimal.batch_size = constrained_batch;
            }
        } else if let Some(telemetry) = &self.telemetry {
            // No local data, try telemetry suggestions
            drop(perf_store); // Release lock before network call

            let normalized_cpu = normalize_cpu_model(cpu_model);
            if let Ok(suggestions) =
                telemetry.fetch_suggestions(&normalized_cpu, logical_cores, &self.pow_algo, profile)
            {
                if let Some(suggestion) = suggestions.first() {
                    // Apply constraints to telemetry suggestion
                    let constrained_threads = {
                        let power_monitor = self.power_monitor.lock().unwrap();
                        let threads =
                            ((suggestion.threads as f64 * thermal_factor) as usize).max(1);
                        power_monitor.apply_thread_cap(threads)
                    };

                    let constrained_batch = {
                        let power_monitor = self.power_monitor.lock().unwrap();
                        power_monitor.apply_batch_cap(suggestion.batch_size)
                    };

                    optimal.threads = constrained_threads;
                    optimal.batch_size = constrained_batch;
                    optimal.constraints_applied.push(format!(
                        "Telemetry hint: {} H/s expected (confidence: {:.0}%)",
                        suggestion.expected_hashrate,
                        suggestion.confidence * 100.0
                    ));
                }
            }
        }

        // 4) Apply NUMA topology hints if enabled
        {
            let numa = self.numa_coordinator.lock().unwrap();
            if numa.should_use_numa() {
                let plan = numa.plan_thread_distribution(optimal.threads);
                let layout = numa.layout_string(&plan);
                optimal
                    .constraints_applied
                    .push(format!("NUMA layout: {}", layout));
            }
        }

        optimal
    }

    /// Report current performance to telemetry (if enabled)
    pub fn report_performance(
        &self,
        cpu_model: &str,
        profile: &str,
        logical_cores: u32,
        physical_cores: u32,
        threads: u32,
        batch_size: u32,
        avg_hashrate: f64,
        sample_count: u64,
    ) {
        if let Some(telemetry) = &self.telemetry {
            let snapshot = TelemSnapshot {
                cpu_model: normalize_cpu_model(cpu_model),
                logical_cores,
                physical_cores,
                pow_algo: self.pow_algo.clone(),
                profile: profile.to_string(),
                threads,
                batch_size,
                avg_hashrate_hs: avg_hashrate,
                sample_count,
                client_version: crate::vision_constants::VISION_VERSION.to_string(),
            };

            // Fire and forget (don't block mining on telemetry)
            let _ = telemetry.report_snapshot(&snapshot);
        }
    }

    /// Get thermal monitor for external access
    pub fn thermal_monitor(&self) -> Arc<Mutex<ThermalMonitor>> {
        Arc::clone(&self.thermal_monitor)
    }

    /// Get power monitor for external access
    pub fn power_monitor(&self) -> Arc<Mutex<PowerMonitor>> {
        Arc::clone(&self.power_monitor)
    }

    /// Get NUMA coordinator for external access
    pub fn numa_coordinator(&self) -> Arc<Mutex<NumaCoordinator>> {
        Arc::clone(&self.numa_coordinator)
    }

    /// Get hint manager for external access
    pub fn hint_manager(&self) -> Arc<Mutex<HintManager>> {
        Arc::clone(&self.hint_manager)
    }

    /// Consider peer hints when deciding configuration.
    ///
    /// This is called periodically to check if any peer hints are worth testing.
    /// If a trial is available, returns the suggested config from the hint.
    pub fn consider_peer_hints(&self) -> Option<(usize, u32, String)> {
        let mut manager = self.hint_manager.lock().unwrap();

        if let Some(trial) = manager.get_next_trial() {
            info!(
                "Starting P2P hint trial: threads={}, batch={}, expected_gain={:.1}%",
                trial.hint.threads,
                trial.hint.batch_size,
                trial.hint.gain_ratio * 100.0
            );

            return Some((
                trial.hint.threads,
                trial.hint.batch_size,
                format!(
                    "P2P hint from peer (reputation: {:.1})",
                    trial.peer_reputation
                ),
            ));
        }

        None
    }

    /// Check if it's time to broadcast our elite configs to the network.
    pub fn should_broadcast_hints(&self) -> bool {
        let manager = self.hint_manager.lock().unwrap();
        manager.should_broadcast()
    }

    /// Get broadcast-worthy hints to share with the network.
    ///
    /// Returns elite configs that have been proven locally and meet broadcast criteria:
    /// - Sample count ≥ 5
    /// - Gain ratio ≥ 5%
    /// - Confidence ≥ 0.25
    pub fn get_broadcast_hints(&self) -> Vec<crate::miner::tuning_hint::MinerTuningHint> {
        use crate::miner::tuning_hint::CpuBucket;

        let _perf_store = self.perf_store.lock().unwrap();
        let _cpu_bucket = CpuBucket::detect();
        let hints = Vec::new();

        // Query performance store for high-performing configs
        // This is a simplified version - real implementation would scan all stored configs
        // and identify those with significant gains over baseline

        // For now, return empty - full implementation would:
        // 1. Iterate through all PerfKeys in perf_store
        // 2. Compare each config's hashrate to baseline for that CPU profile
        // 3. Create MinerTuningHint for configs with ≥5% gain
        // 4. Filter by is_broadcast_worthy()

        let mut manager = self.hint_manager.lock().unwrap();
        manager.mark_broadcasted();

        hints
    }
}

/// Optimal configuration decision with constraints
#[derive(Debug, Clone)]
pub struct OptimalConfig {
    pub threads: usize,
    pub batch_size: u32,
    pub constraints_applied: Vec<String>,
    pub should_mine: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_intelligent_tuner_creation() {
        let perf_store = Arc::new(Mutex::new(
            MinerPerfStore::load(PathBuf::from("test_perf.json")).unwrap(),
        ));

        let config = MinerConfig::default();
        let tuner = IntelligentTuner::new(perf_store, &config, "vision-pow-v1".to_string());

        // Should be initialized successfully
        assert!(tuner.telemetry.is_none()); // Default is disabled
    }

    #[test]
    fn test_optimal_config_with_constraints() {
        let perf_store = Arc::new(Mutex::new(
            MinerPerfStore::load(PathBuf::from("test_perf.json")).unwrap(),
        ));

        let mut config = MinerConfig::default();
        config.power_mode_sensitivity = true;
        config.battery_threads_cap = Some(2);

        let tuner = IntelligentTuner::new(perf_store, &config, "vision-pow-v1".to_string());

        // Note: In real usage, power state is detected automatically
        // For this test, we just verify the tuner initializes correctly

        let optimal = tuner.decide_optimal_config("Test CPU", "balanced", 8, 4, 8, 16, &config);

        // Verify optimal config is returned
        assert!(optimal.threads > 0);
        assert!(optimal.batch_size > 0);
    }
}
