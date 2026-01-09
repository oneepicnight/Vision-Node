//! Stub for tip module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipConfig {
    pub enabled: bool,
    pub min_amount: u128,
    pub max_amount: u128,
    pub tip_usd_amount: f64,
    pub tip_address: String,
    pub has_tipped: bool,
    pub tip_allowed_coins: Vec<String>,
    pub coin: Option<String>,
    pub amount: Option<u128>,
    pub last_tip_at: Option<u64>,
}
impl TipConfig {
    pub fn tipped(&mut self, _coin: &str, _amount: u128, _timestamp: u64) {
        // Stub - no-op
    }
}


impl Default for TipConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_amount: 0,
            max_amount: 0,
                    tip_usd_amount: 0.0,
                    tip_address: String::new(),
            has_tipped: false,
            tip_allowed_coins: Vec::new(),
                    coin: None,
                    amount: None,
                    last_tip_at: None,
        }
    }
}
pub fn load_tip_state(_db: &sled::Db, _wallet_address: &str) -> Result<TipConfig, String> {
    Ok(TipConfig::default())
}

pub fn save_tip_state(_db: &sled::Db, _config: &TipConfig) -> Result<(), String> {
    Ok(())
}

pub fn usd_to_coin_amount(_coin: &str, _usd: f64) -> Result<u128, String> {
    Ok(0)
}
