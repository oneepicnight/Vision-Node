//! Stub for oracle module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OraclePrice {
    pub symbol: String,
    pub price: f64,
    pub timestamp: u64,
}

impl Default for OraclePrice {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            price: 0.0,
            timestamp: 0,
        }
    }
}

pub fn get_price(_symbol: &str) -> Result<OraclePrice, String> {
    Ok(OraclePrice::default())
}

pub fn get_latest_price(_symbol: &str) -> Option<f64> {
    None
}

pub fn update_price(_symbol: &str, _price: f64) -> Result<(), String> {
    Ok(())
}
