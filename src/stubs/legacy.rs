//! Stub implementation of legacy module when staging feature is disabled
//! Non-custodial: provides safe stubs with default values

use serde::{Deserialize, Serialize};

/// Status of a legacy route stub
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyStatus {
    Draft,
    Armed,
    Executed,
    Cancelled,
}

impl LegacyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LegacyStatus::Draft => "draft",
            LegacyStatus::Armed => "armed",
            LegacyStatus::Executed => "executed",
            LegacyStatus::Cancelled => "cancelled",
        }
    }
}

/// Stub legacy route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyRoute {
    pub owner_address: String,
    pub legacy_address: String,
    pub star_id: String,
    pub created_at: u64,
    pub armed_at: Option<u64>,
    pub executed_at: Option<u64>,
    pub status: LegacyStatus,
    pub epitaph: Option<String>,
    pub transferred_amount: Option<u128>,
}

impl LegacyRoute {
    pub fn new(owner_address: String, legacy_address: String, star_id: String) -> Self {
        Self {
            owner_address,
            legacy_address,
            star_id,
            created_at: 0,
            armed_at: None,
            executed_at: None,
            status: LegacyStatus::Draft,
            epitaph: None,
            transferred_amount: None,
        }
    }
}

/// Stub LegacyManager
pub struct LegacyManager {
    // Stub - no database
}

impl LegacyManager {
    pub fn new(_db: &sled::Db) -> Result<Self, String> {
        Ok(Self {})
    }

    pub fn save_route(&self, _route: &LegacyRoute) -> Result<(), String> {
        Ok(())
    }

    pub fn load_route(&self, _owner: &str, _star_id: &str) -> Result<Option<LegacyRoute>, String> {
        Ok(None)
    }
}

/// Stub legacy_message
pub fn legacy_message(owner: &str, legacy: &str) -> String {
    format!("Legacy message from {} to {} (stub)", owner, legacy)
}
