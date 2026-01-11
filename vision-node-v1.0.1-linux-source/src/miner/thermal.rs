//! Thermal Monitoring and Protection
//!
//! Monitors CPU temperature and throttles mining when system gets too hot.
//! Prevents hardware damage and noisy fans on laptops/desktops.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::System;

/// CPU thermal state classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThermalState {
    /// CPU temperature is comfortable (< 70°C)
    Cool,
    /// CPU is warming up (70-80°C)
    Warm,
    /// CPU is hot, throttling recommended (80-90°C)
    Hot,
    /// CPU is critically hot, immediate action required (> 90°C)
    Critical,
}

impl ThermalState {
    pub fn from_celsius(temp_c: f32) -> Self {
        if temp_c >= 90.0 {
            ThermalState::Critical
        } else if temp_c >= 80.0 {
            ThermalState::Hot
        } else if temp_c >= 70.0 {
            ThermalState::Warm
        } else {
            ThermalState::Cool
        }
    }

    pub fn should_throttle(&self) -> bool {
        matches!(self, ThermalState::Hot | ThermalState::Critical)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ThermalState::Cool => "Cool",
            ThermalState::Warm => "Warm",
            ThermalState::Hot => "Hot",
            ThermalState::Critical => "Critical",
        }
    }
}

/// Thermal snapshot for a point in time
#[derive(Debug, Clone, Serialize)]
pub struct ThermalSnapshot {
    /// CPU temperature in Celsius
    pub temp_c: f32,
    /// When this reading was taken
    #[serde(skip)]
    pub timestamp: Instant,
    /// Thermal state classification
    pub state: ThermalState,
}

/// Thermal protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalConfig {
    /// Enable thermal protection
    pub enabled: bool,
    /// Soft limit: start reducing load (°C)
    pub soft_limit_c: u32,
    /// Hard limit: aggressive throttling (°C)
    pub hard_limit_c: u32,
    /// Cooldown period before ramping back up (seconds)
    pub cooldown_secs: u64,
}

impl Default for ThermalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            soft_limit_c: 80,
            hard_limit_c: 90,
            cooldown_secs: 120,
        }
    }
}

/// Thermal monitor with history and throttling logic
pub struct ThermalMonitor {
    config: ThermalConfig,
    history: Arc<Mutex<Vec<ThermalSnapshot>>>,
    system: sysinfo::System,
    last_throttle_time: Arc<Mutex<Option<Instant>>>,
}

impl ThermalMonitor {
    pub fn new(config: ThermalConfig) -> Self {
        let system = System::new_all();

        Self {
            config,
            history: Arc::new(Mutex::new(Vec::with_capacity(60))),
            system,
            last_throttle_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Sample current CPU temperature
    pub fn sample(&mut self) -> Option<ThermalSnapshot> {
        if !self.config.enabled {
            return None;
        }

        // Try to get real temperature from platform-specific APIs
        let temp_c = self.read_cpu_temperature().or_else(|| {
            // Fallback: estimate based on CPU usage if sensors unavailable
            self.system.refresh_cpu();
            let global_cpu = self.system.global_cpu_info();
            let cpu_usage = global_cpu.cpu_usage();
            let estimated_temp = 40.0 + (cpu_usage * 0.4);
            Some(estimated_temp.min(95.0).max(30.0))
        });

        if let Some(temp_c) = temp_c {
            let snapshot = ThermalSnapshot {
                temp_c,
                timestamp: Instant::now(),
                state: ThermalState::from_celsius(temp_c),
            };

            // Store in history
            let mut history = self.history.lock().unwrap();
            history.push(snapshot.clone());

            // Keep last 60 samples (5 minutes at 5-second intervals)
            if history.len() > 60 {
                history.remove(0);
            }

            Some(snapshot)
        } else {
            None
        }
    }

    /// Get current thermal state
    pub fn current_state(&self) -> Option<ThermalState> {
        self.history.lock().unwrap().last().map(|s| s.state)
    }

    /// Calculate average temperature over last N seconds
    pub fn average_temp(&self, last_secs: u64) -> Option<f32> {
        let history = self.history.lock().unwrap();
        let cutoff = Instant::now() - Duration::from_secs(last_secs);

        let recent: Vec<f32> = history
            .iter()
            .filter(|s| s.timestamp >= cutoff)
            .map(|s| s.temp_c)
            .collect();

        if recent.is_empty() {
            None
        } else {
            Some(recent.iter().sum::<f32>() / recent.len() as f32)
        }
    }

    /// Check if we should throttle mining based on sustained heat
    pub fn should_throttle(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Check average temp over last 30 seconds
        if let Some(avg_temp) = self.average_temp(30) {
            let state = ThermalState::from_celsius(avg_temp);
            state.should_throttle()
        } else {
            false
        }
    }

    /// Get recommended thread reduction percentage (0.0-1.0)
    pub fn get_throttle_factor(&self) -> f64 {
        if !self.config.enabled {
            return 1.0;
        }

        if let Some(avg_temp) = self.average_temp(30) {
            if avg_temp >= self.config.hard_limit_c as f32 {
                // Critical: reduce to 25% capacity
                return 0.25;
            } else if avg_temp >= self.config.soft_limit_c as f32 {
                // Hot: linear scale from 100% at soft_limit down to 50% at hard_limit
                let range = (self.config.hard_limit_c - self.config.soft_limit_c) as f32;
                let over_soft = avg_temp - self.config.soft_limit_c as f32;
                let reduction = (over_soft / range) * 0.5; // Up to 50% reduction
                return (1.0 - reduction as f64).max(0.5);
            }
        }

        1.0 // No throttling needed
    }

    /// Record that we throttled, start cooldown timer
    pub fn mark_throttled(&self) {
        *self.last_throttle_time.lock().unwrap() = Some(Instant::now());
    }

    /// Check if we're still in cooldown period
    pub fn in_cooldown(&self) -> bool {
        if let Some(throttle_time) = *self.last_throttle_time.lock().unwrap() {
            let elapsed = Instant::now().duration_since(throttle_time);
            elapsed < Duration::from_secs(self.config.cooldown_secs)
        } else {
            false
        }
    }

    /// Get recent thermal history for visualization
    pub fn get_history(&self) -> Vec<ThermalSnapshot> {
        self.history.lock().unwrap().clone()
    }

    /// Read CPU temperature from platform-specific sources
    #[cfg(target_os = "linux")]
    fn read_cpu_temperature(&self) -> Option<f32> {
        use std::fs;

        // Try thermal zones in order of preference
        let thermal_zones = [
            "/sys/class/thermal/thermal_zone0/temp",
            "/sys/class/thermal/thermal_zone1/temp",
            "/sys/class/thermal/thermal_zone2/temp",
        ];

        for zone_path in &thermal_zones {
            if let Ok(content) = fs::read_to_string(zone_path) {
                if let Ok(temp_millicelsius) = content.trim().parse::<f32>() {
                    let temp_celsius = temp_millicelsius / 1000.0;
                    if temp_celsius > 0.0 && temp_celsius < 150.0 {
                        return Some(temp_celsius);
                    }
                }
            }
        }

        // Try hwmon (hardware monitoring) sensors
        let hwmon_paths = [
            "/sys/class/hwmon/hwmon0/temp1_input",
            "/sys/class/hwmon/hwmon1/temp1_input",
            "/sys/class/hwmon/hwmon2/temp1_input",
        ];

        for hwmon_path in &hwmon_paths {
            if let Ok(content) = fs::read_to_string(hwmon_path) {
                if let Ok(temp_millicelsius) = content.trim().parse::<f32>() {
                    let temp_celsius = temp_millicelsius / 1000.0;
                    if temp_celsius > 0.0 && temp_celsius < 150.0 {
                        return Some(temp_celsius);
                    }
                }
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn read_cpu_temperature(&self) -> Option<f32> {
        // Windows temperature reading requires WMI queries
        // This is a placeholder - full implementation would use wmi crate
        // or PowerShell: Get-WmiObject MSAcpi_ThermalZoneTemperature

        // For now, return None to fallback to CPU usage estimation
        None
    }

    #[cfg(target_os = "macos")]
    fn read_cpu_temperature(&self) -> Option<f32> {
        // macOS temperature reading requires IOKit SMC sensors
        // This is a placeholder - full implementation would use IOKit bindings
        // or use 'osx-cpu-temp' command-line tool

        // For now, return None to fallback to CPU usage estimation
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    fn read_cpu_temperature(&self) -> Option<f32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_state_classification() {
        assert_eq!(ThermalState::from_celsius(60.0), ThermalState::Cool);
        assert_eq!(ThermalState::from_celsius(75.0), ThermalState::Warm);
        assert_eq!(ThermalState::from_celsius(85.0), ThermalState::Hot);
        assert_eq!(ThermalState::from_celsius(95.0), ThermalState::Critical);
    }

    #[test]
    fn test_throttle_factor() {
        let config = ThermalConfig {
            enabled: true,
            soft_limit_c: 80,
            hard_limit_c: 90,
            cooldown_secs: 120,
        };

        let mut monitor = ThermalMonitor::new(config);

        // Simulate hot temperature
        {
            let mut history = monitor.history.lock().unwrap();
            for _ in 0..10 {
                history.push(ThermalSnapshot {
                    temp_c: 85.0, // Midway between soft and hard
                    timestamp: Instant::now(),
                    state: ThermalState::Hot,
                });
            }
        }

        let factor = monitor.get_throttle_factor();
        assert!(factor < 1.0 && factor >= 0.5);
    }
}
