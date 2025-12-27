//! Stub for airdrop module when staging is disabled

mod cash {
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CashAirdropRequest {
        pub address: String,
        pub amount: u128,
    }
    
    pub fn execute_cash_airdrop(_request: &CashAirdropRequest) -> Result<String, String> {
        Err("Airdrop disabled in stub mode".to_string())
    }
    
    pub fn get_cash_total_supply() -> u128 {
        0
    }
    
    pub fn validate_airdrop_request(_request: &CashAirdropRequest) -> Result<(), String> {
        Ok(())
    }
}

pub use cash::{
    execute_cash_airdrop, get_cash_total_supply, validate_airdrop_request, CashAirdropRequest,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropLimits {
    pub max_per_address: u128,
    pub total_supply: u128,
}

impl Default for CashAirdropLimits {
    fn default() -> Self {
        Self {
            max_per_address: 0,
            total_supply: 0,
        }
    }
}
