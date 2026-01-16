//! Stub for pending_rewards module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReward {
    pub recipient: String,
    pub amount: u128,
    pub pending_since: u64,
}

impl Default for PendingReward {
    fn default() -> Self {
        Self {
            recipient: String::new(),
            amount: 0,
            pending_since: 0,
        }
    }
}

pub fn load_pending_reward(_recipient: &str) -> Result<Option<PendingReward>, String> {
    Ok(None)
}

pub fn save_pending_reward(_reward: &PendingReward) -> Result<(), String> {
    Ok(())
}

pub fn claim_reward(_recipient: &str) -> Result<u128, String> {
    Ok(0)
}
