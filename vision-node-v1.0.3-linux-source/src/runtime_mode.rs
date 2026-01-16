#![allow(dead_code)]
use serde::{Deserialize, Serialize};

/// Runtime configuration for different operational modes of the node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeModeConfig {
    /// Enable solo mining mode
    pub solo_enabled: bool,
    /// Enable pool hosting mode (accept external miners)
    pub pool_enabled: bool,
    /// Enable farm controller mode (manage LAN mining rigs)
    pub farm_enabled: bool,
}

impl Default for RuntimeModeConfig {
    fn default() -> Self {
        Self {
            solo_enabled: true,
            pool_enabled: false,
            farm_enabled: false,
        }
    }
}

impl RuntimeModeConfig {
    /// Load from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Validate configuration (ensure at least one mode is enabled)
    pub fn validate(&self) -> Result<(), String> {
        if !self.solo_enabled && !self.pool_enabled && !self.farm_enabled {
            return Err("At least one mode must be enabled".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeModeConfig::default();
        assert!(config.solo_enabled);
        assert!(!config.pool_enabled);
        assert!(!config.farm_enabled);
    }

    #[test]
    fn test_serialization() {
        let config = RuntimeModeConfig {
            solo_enabled: true,
            pool_enabled: true,
            farm_enabled: false,
        };

        let json = config.to_json().unwrap();
        let deserialized = RuntimeModeConfig::from_json(&json).unwrap();

        assert_eq!(config.solo_enabled, deserialized.solo_enabled);
        assert_eq!(config.pool_enabled, deserialized.pool_enabled);
        assert_eq!(config.farm_enabled, deserialized.farm_enabled);
    }

    #[test]
    fn test_validation() {
        let invalid = RuntimeModeConfig {
            solo_enabled: false,
            pool_enabled: false,
            farm_enabled: false,
        };
        assert!(invalid.validate().is_err());

        let valid = RuntimeModeConfig::default();
        assert!(valid.validate().is_ok());
    }
}
