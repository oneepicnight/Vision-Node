//! Stub for airdrop module when staging is disabled

mod cash {
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CashAirdropRequest {
        pub recipients: Vec<String>,
        pub amount: u128,
        pub requested_by: String,
        pub confirm_phrase: Option<String>,
        pub reason: Option<String>,
    }
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CashAirdropResult {
        pub total_recipients: usize,
        pub total_cash: u128,
        pub failed: Vec<(String, String)>,
    }
    
    pub fn execute_cash_airdrop(
        _db: &sled::Db,
        _balances_tree: &str,
        request: &CashAirdropRequest,
        _limits: &super::CashAirdropLimits,
    ) -> Result<CashAirdropResult, String> {
        Ok(CashAirdropResult {
            total_recipients: request.recipients.len(),
            total_cash: 0,
            failed: Vec::new(),
        })
    }
    
    pub fn get_cash_total_supply(_db: &sled::Db) -> Result<u128, String> {
        Ok(0)
    }
    
    pub fn validate_airdrop_request(
        _request: &CashAirdropRequest,
        _limits: &super::CashAirdropLimits,
    ) -> Result<u128, String> {
        Ok(0)
    }
}

pub use cash::{
    execute_cash_airdrop, get_cash_total_supply, validate_airdrop_request, CashAirdropRequest,
    CashAirdropResult,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropLimits {
    pub max_per_address: u128,
    pub total_supply: u128,
    pub require_confirm_phrase: bool,
    pub confirm_threshold: u128,
}

impl Default for CashAirdropLimits {
    fn default() -> Self {
        Self {
            max_per_address: 0,
            total_supply: 0,
            require_confirm_phrase: false,
            confirm_threshold: 0,
        }
    }
}
