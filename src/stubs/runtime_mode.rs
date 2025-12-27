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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeModeConfig {
    pub mode: RuntimeMode,
}

impl Default for RuntimeModeConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::default(),
        }
    }
}

pub fn current_mode() -> RuntimeMode {
    RuntimeMode::default()
}

pub fn set_mode(_mode: RuntimeMode) -> Result<(), String> {
    Ok(())
}
