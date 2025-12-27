//! Stub for land_stake module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandStake {
    pub stake_id: String,
    pub owner: String,
    pub amount: u128,
    pub created_at: u64,
}

impl Default for LandStake {
    fn default() -> Self {
        Self {
            stake_id: String::new(),
            owner: String::new(),
            amount: 0,
            created_at: 0,
        }
    }
}

pub fn load_stake(_stake_id: &str) -> Result<Option<LandStake>, String> {
    Ok(None)
}

pub fn save_stake(_stake: &LandStake) -> Result<(), String> {
    Ok(())
}

pub fn get_all_stakers(_db: &sled::Db) -> Vec<String> {
    vec![]
}

pub fn total_stake(_db: &sled::Db) -> u128 {
    0
}

pub fn get_stake(_db: &sled::Db, _address: &str) -> u128 {
    0
}

pub fn rebuild_owner_weights(_db: &sled::Db) -> Result<(), String> {
    Ok(())
}
