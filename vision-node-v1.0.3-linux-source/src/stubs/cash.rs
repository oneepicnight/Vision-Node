//! Stub for airdrop cash submodule

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropRequest {
    pub address: String,
    pub amount: u128,
}

pub fn execute_cash_airdrop(_request: &CashAirdropRequest) -> Result<String, String> {
    Err("Airdrop disabled".to_string())
}
