//! Stub for runtime_mode module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeMode {
    Mining,
    Hybrid,
    StakingOnly,
    Maintenance,
}

impl Default for RuntimeMode {
    fn default() -> Self {
        Self::Maintenance
    }
}

impl RuntimeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeMode::Mining => "mining",
            RuntimeMode::Hybrid => "hybrid",
            RuntimeMode::StakingOnly => "staking_only",
            RuntimeMode::Maintenance => "maintenance",
        }
    }
}

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

pub fn current_mode() -> RuntimeMode {
    RuntimeMode::default()
}

pub fn set_mode(_mode: RuntimeMode) -> Result<(), String> {
    Ok(())
}
