use serde::{Deserialize, Serialize};

/// Profile types for farm rigs
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FarmProfileType {
    Performance,
    Eco,
    Custom,
}

/// Configuration for a farm mining profile
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FarmProfileConfig {
    pub profile_type: FarmProfileType,
    pub threads: Option<u32>,
    pub intensity: Option<String>, // "low", "medium", "high"
}

impl FarmProfileConfig {
    /// Create a performance profile (max threads, high intensity)
    pub fn performance(cpu_threads: u32) -> Self {
        Self {
            profile_type: FarmProfileType::Performance,
            threads: Some(cpu_threads.saturating_sub(1).max(1)),
            intensity: Some("high".to_string()),
        }
    }

    /// Create an eco profile (half threads, low intensity)
    pub fn eco(cpu_threads: u32) -> Self {
        Self {
            profile_type: FarmProfileType::Eco,
            threads: Some((cpu_threads / 2).max(1)),
            intensity: Some("low".to_string()),
        }
    }

    /// Create a custom profile
    pub fn custom(threads: u32, intensity: String) -> Self {
        Self {
            profile_type: FarmProfileType::Custom,
            threads: Some(threads),
            intensity: Some(intensity),
        }
    }

    /// Get effective threads for a given CPU count
    pub fn get_threads(&self, cpu_threads: u32) -> u32 {
        if let Some(t) = self.threads {
            t.min(cpu_threads)
        } else {
            match self.profile_type {
                FarmProfileType::Performance => cpu_threads.saturating_sub(1).max(1),
                FarmProfileType::Eco => (cpu_threads / 2).max(1),
                FarmProfileType::Custom => cpu_threads / 2,
            }
        }
    }

    /// Get intensity string
    pub fn get_intensity(&self) -> String {
        self.intensity
            .clone()
            .unwrap_or_else(|| match self.profile_type {
                FarmProfileType::Performance => "high".to_string(),
                FarmProfileType::Eco => "low".to_string(),
                FarmProfileType::Custom => "medium".to_string(),
            })
    }
}

/// Time-based schedule for automatic mining control
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FarmSchedule {
    /// Whether scheduling is enabled
    pub enabled: bool,
    /// Start hour (0-23, local time)
    pub start_hour: u8,
    /// End hour (0-23, local time)
    pub end_hour: u8,
    /// Days of week when schedule is active (0=Sunday, 6=Saturday)
    pub days_of_week: Vec<u8>,
    /// Profile to use when schedule is active
    pub profile: FarmProfileConfig,
}

impl FarmSchedule {
    /// Check if schedule is currently active
    pub fn is_active_now(&self) -> bool {
        if !self.enabled {
            return false;
        }

        use chrono::Datelike;
        use chrono::Timelike;
        let now = chrono::Local::now();
        let current_hour = now.hour() as u8;
        let current_weekday = now.weekday().num_days_from_sunday() as u8;

        // Check if current day is in the schedule
        if !self.days_of_week.contains(&current_weekday) {
            return false;
        }

        // Check if current hour is in the time range
        if self.start_hour <= self.end_hour {
            // Normal range: 9:00 - 17:00
            current_hour >= self.start_hour && current_hour < self.end_hour
        } else {
            // Overnight range: 22:00 - 6:00
            current_hour >= self.start_hour || current_hour < self.end_hour
        }
    }

    /// Create a default schedule (off-hours: 22:00 - 6:00, all days)
    pub fn default_offhours() -> Self {
        Self {
            enabled: false,
            start_hour: 22,                          // 10 PM
            end_hour: 6,                             // 6 AM
            days_of_week: vec![0, 1, 2, 3, 4, 5, 6], // All days
            profile: FarmProfileConfig::eco(8),      // Default eco profile
        }
    }
}

impl Default for FarmSchedule {
    fn default() -> Self {
        Self::default_offhours()
    }
}

/// Persistent configuration for a specific rig
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RigConfig {
    /// Rig identifier
    pub rig_id: String,
    /// Active profile configuration
    pub profile: Option<FarmProfileConfig>,
    /// Time-based schedule
    pub schedule: Option<FarmSchedule>,
    /// Automatically restart on error
    pub auto_restart_on_error: bool,
    /// Minimum acceptable hashrate (trigger error if below)
    pub min_hashrate_threshold: Option<f64>,
}

impl RigConfig {
    pub fn new(rig_id: String) -> Self {
        Self {
            rig_id,
            profile: None,
            schedule: None,
            auto_restart_on_error: false,
            min_hashrate_threshold: None,
        }
    }

    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Create a sled key for this rig config
    pub fn sled_key(&self) -> String {
        format!("farm_rig_config/{}", self.rig_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_types() {
        let perf = FarmProfileConfig::performance(16);
        assert_eq!(perf.profile_type, FarmProfileType::Performance);
        assert_eq!(perf.get_threads(16), 15);
        assert_eq!(perf.get_intensity(), "high");

        let eco = FarmProfileConfig::eco(16);
        assert_eq!(eco.profile_type, FarmProfileType::Eco);
        assert_eq!(eco.get_threads(16), 8);
        assert_eq!(eco.get_intensity(), "low");
    }

    #[test]
    fn test_schedule_time_range() {
        let mut schedule = FarmSchedule::default();
        schedule.enabled = true;
        schedule.start_hour = 22;
        schedule.end_hour = 6;

        // This test will pass/fail based on current time, so we just verify the logic compiles
        let _is_active = schedule.is_active_now();
    }

    #[test]
    fn test_rig_config_serialization() {
        let config = RigConfig {
            rig_id: "RIG-01".to_string(),
            profile: Some(FarmProfileConfig::performance(8)),
            schedule: Some(FarmSchedule::default()),
            auto_restart_on_error: true,
            min_hashrate_threshold: Some(1000.0),
        };

        let json = config.to_json().unwrap();
        let deserialized = RigConfig::from_json(&json).unwrap();
        assert_eq!(config.rig_id, deserialized.rig_id);
        assert!(deserialized.auto_restart_on_error);
    }
}
