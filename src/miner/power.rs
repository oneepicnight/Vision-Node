//! Power Mode Detection and Battery Management
//!
//! Detects if system is on battery or AC power and adjusts mining accordingly.
//! Prevents laptops from draining battery too fast or overheating on portable power.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use sysinfo::System;

/// System power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    /// Running on AC power (unlimited)
    AC,
    /// Running on battery (limited)
    Battery,
    /// Unable to determine power state
    Unknown,
}

impl PowerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerState::AC => "AC",
            PowerState::Battery => "Battery",
            PowerState::Unknown => "Unknown",
        }
    }

    pub fn should_limit(&self) -> bool {
        matches!(self, PowerState::Battery)
    }
}

/// Power mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerConfig {
    /// Enable power-aware throttling
    pub sensitivity_enabled: bool,
    /// Maximum threads when on battery
    pub battery_threads_cap: Option<usize>,
    /// Maximum batch size when on battery
    pub battery_batch_cap: Option<u32>,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            sensitivity_enabled: true,
            battery_threads_cap: Some(2), // Minimal threads on battery
            battery_batch_cap: Some(4),   // Minimal batches on battery
        }
    }
}

/// Power mode detector
pub struct PowerMonitor {
    config: PowerConfig,
    system: sysinfo::System,
    last_state: Arc<Mutex<PowerState>>,
}

impl PowerMonitor {
    pub fn new(config: PowerConfig) -> Self {
        Self {
            config,
            system: System::new_all(),
            last_state: Arc::new(Mutex::new(PowerState::Unknown)),
        }
    }

    /// Detect current power state
    pub fn detect_power_state(&mut self) -> PowerState {
        if !self.config.sensitivity_enabled {
            return PowerState::AC; // Treat as AC if disabled
        }

        // Use battery crate for real detection
        let state = match battery::Manager::new() {
            Ok(manager) => {
                // Check if any battery is present and discharging
                let mut has_battery = false;
                let mut is_discharging = false;

                if let Ok(batteries) = manager.batteries() {
                    for battery in batteries.flatten() {
                        has_battery = true;
                        if battery.state() == battery::State::Discharging {
                            is_discharging = true;
                            break;
                        }
                    }
                }

                if is_discharging {
                    PowerState::Battery
                } else if has_battery {
                    PowerState::AC // Battery present but charging/full
                } else {
                    PowerState::AC // Desktop (no battery)
                }
            }
            Err(_) => PowerState::Unknown,
        };

        *self.last_state.lock().unwrap() = state;
        state
    }

    /// Get last detected power state without re-polling
    pub fn get_state(&self) -> PowerState {
        *self.last_state.lock().unwrap()
    }

    /// Apply battery caps to thread count
    pub fn apply_thread_cap(&self, desired_threads: usize) -> usize {
        let state = self.get_state();

        if !self.config.sensitivity_enabled || !state.should_limit() {
            return desired_threads;
        }

        if let Some(cap) = self.config.battery_threads_cap {
            desired_threads.min(cap)
        } else {
            desired_threads
        }
    }

    /// Apply battery caps to batch size
    pub fn apply_batch_cap(&self, desired_batch: u32) -> u32 {
        let state = self.get_state();

        if !self.config.sensitivity_enabled || !state.should_limit() {
            return desired_batch;
        }

        if let Some(cap) = self.config.battery_batch_cap {
            desired_batch.min(cap)
        } else {
            desired_batch
        }
    }

    /// Get battery level if available (0-100%)
    pub fn get_battery_level(&mut self) -> Option<f32> {
        match battery::Manager::new() {
            Ok(manager) => {
                if let Ok(mut batteries) = manager.batteries() {
                    if let Some(Ok(battery)) = batteries.next() {
                        // state_of_charge returns a Ratio (0.0-1.0)
                        let soc = battery.state_of_charge();
                        return Some(soc.get::<battery::units::ratio::percent>());
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Check if we should force laptop profile on battery
    pub fn should_force_laptop_profile(&self) -> bool {
        self.config.sensitivity_enabled && self.get_state() == PowerState::Battery
    }

    /// Get mutable access to state (for testing)
    #[cfg(test)]
    pub fn get_state_mut(&mut self) -> std::sync::MutexGuard<PowerState> {
        self.last_state.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_state_conversion() {
        assert_eq!(PowerState::AC.as_str(), "AC");
        assert_eq!(PowerState::Battery.as_str(), "Battery");
        assert!(!PowerState::AC.should_limit());
        assert!(PowerState::Battery.should_limit());
    }

    #[test]
    fn test_thread_capping() {
        let config = PowerConfig {
            sensitivity_enabled: true,
            battery_threads_cap: Some(4),
            battery_batch_cap: Some(8),
        };

        let monitor = PowerMonitor::new(config);

        // Simulate battery state
        *monitor.last_state.lock().unwrap() = PowerState::Battery;

        assert_eq!(monitor.apply_thread_cap(16), 4);
        assert_eq!(monitor.apply_batch_cap(32), 8);

        // Simulate AC state
        *monitor.last_state.lock().unwrap() = PowerState::AC;

        assert_eq!(monitor.apply_thread_cap(16), 16);
        assert_eq!(monitor.apply_batch_cap(32), 32);
    }

    #[test]
    fn test_disabled_sensitivity() {
        let config = PowerConfig {
            sensitivity_enabled: false,
            battery_threads_cap: Some(2),
            battery_batch_cap: Some(4),
        };

        let mut monitor = PowerMonitor::new(config);

        // Even if we detect battery, it should return AC
        let state = monitor.detect_power_state();
        assert_eq!(state, PowerState::AC);
    }
}
