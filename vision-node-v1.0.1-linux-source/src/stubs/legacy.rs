//! Stub implementation of legacy module when staging feature is disabled
//! Non-custodial: provides safe stubs with default values

use serde::{Deserialize, Serialize};
use anyhow::Result;

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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            owner_address,
            legacy_address,
            star_id,
            created_at: now,
            armed_at: None,
            executed_at: None,
            status: LegacyStatus::Draft,
            epitaph: None,
            transferred_amount: None,
        }
    }

    /// Arm the legacy route (make it active)
    pub fn arm(&mut self) -> Result<()> {
        match self.status {
            LegacyStatus::Draft | LegacyStatus::Cancelled => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                self.status = LegacyStatus::Armed;
                self.armed_at = Some(now);
                Ok(())
            }
            LegacyStatus::Armed => anyhow::bail!("Legacy route is already armed"),
            LegacyStatus::Executed => anyhow::bail!("Cannot arm an executed legacy route"),
        }
    }

    /// Cancel the legacy route
    pub fn cancel(&mut self) -> Result<()> {
        match self.status {
            LegacyStatus::Draft | LegacyStatus::Armed => {
                self.status = LegacyStatus::Cancelled;
                Ok(())
            }
            LegacyStatus::Executed => anyhow::bail!("Cannot cancel an executed legacy route"),
            LegacyStatus::Cancelled => anyhow::bail!("Legacy route is already cancelled"),
        }
    }

    /// Execute the legacy route (pass the torch)
    pub fn execute(&mut self, amount: u128) -> Result<()> {
        if self.status != LegacyStatus::Armed {
            anyhow::bail!("Legacy route must be armed to execute");
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.status = LegacyStatus::Executed;
        self.executed_at = Some(now);
        self.transferred_amount = Some(amount);
        Ok(())
    }

    /// Set the epitaph message
    pub fn set_epitaph(&mut self, epitaph: String) {
        self.epitaph = Some(epitaph);
    }
}

/// Stub LegacyManager
pub struct LegacyManager {
    // Stub - no database
}

impl LegacyManager {
    pub fn new(_db: &sled::Db) -> Result<Self> {
        Ok(Self {})
    }

    /// Validate a legacy route before saving (stubbed checks)
    pub fn validate(&self, route: &LegacyRoute) -> Result<()> {
        if route.owner_address == route.legacy_address {
            anyhow::bail!("Owner and legacy addresses cannot be the same");
        }
        if route.star_id.is_empty() {
            anyhow::bail!("Star ID is required");
        }
        if route.legacy_address.is_empty() {
            anyhow::bail!("Legacy address cannot be empty");
        }
        Ok(())
    }

    /// Get a legacy route by owner address (stub returns None)
    pub fn get(&self, _owner_address: &str) -> Result<Option<LegacyRoute>> {
        Ok(None)
    }

    /// Save or update a legacy route (no-op stub)
    pub fn save(&self, _route: &LegacyRoute) -> Result<()> {
        Ok(())
    }
}

/// Helper to format a legacy message for the user (stub)
pub fn legacy_message(status: LegacyStatus) -> &'static str {
    match status {
        LegacyStatus::Draft => "Your legacy route is drafted. Arm it when you're ready.",
        LegacyStatus::Armed => "Your legacy is armed. When your time comes, Vision will carry your torch forward.",
        LegacyStatus::Executed => "This star has passed its torch. You are looking at a legacy record.",
        LegacyStatus::Cancelled => "This legacy route was cancelled.",
    }
}
