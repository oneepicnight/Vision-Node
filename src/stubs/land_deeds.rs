//! Stub for land_deeds module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandDeed {
    pub deed_id: String,
    pub owner: String,
    pub parcel_id: u64,
    pub created_at: u64,
}

impl Default for LandDeed {
    fn default() -> Self {
        Self {
            deed_id: String::new(),
            owner: String::new(),
            parcel_id: 0,
            created_at: 0,
        }
    }
}

pub fn load_deed(_deed_id: &str) -> Result<Option<LandDeed>, String> {
    Ok(None)
}

pub fn save_deed(_deed: &LandDeed) -> Result<(), String> {
    Ok(())
}

pub fn wallet_has_deed(_db: &sled::Db, _address: &str) -> bool {
    false
}

pub fn all_deed_owners(_db: &sled::Db) -> Vec<String> {
    vec![]
}
