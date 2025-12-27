//! Stub for tip module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipConfig {
    pub enabled: bool,
    pub min_amount: u128,
    pub max_amount: u128,
}

impl Default for TipConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_amount: 0,
            max_amount: 0,
        }
    }
}

pub fn load_tip_state() -> Result<TipConfig, String> {
    Ok(TipConfig::default())
}

pub fn save_tip_state(_config: &TipConfig) -> Result<(), String> {
    Ok(())
}

pub fn usd_to_coin_amount(_usd: f64) -> u128 {
    0
}
