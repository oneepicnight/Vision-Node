//! Legacy Routing Protocol - Torch-Passing System
//!
//! "His star will be in the sky forever."
//!
//! This module implements the Legacy Routing Protocol, allowing users to designate
//! a legacy wallet that receives their funds when they pass. Their star remains
//! immortal in the constellation, forever marking their contribution to Vision.
#![allow(dead_code)]

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::info;

const LEGACY_TREE_NAME: &str = "legacy_routes";

/// Status of a legacy route
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyStatus {
    /// Draft - not yet activated
    Draft,
    /// Armed - ready to execute when time comes
    Armed,
    /// Executed - torch has been passed
    Executed,
    /// Cancelled - user changed their mind
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

/// A legacy route defines where a user's funds go after they pass
/// Their star remains forever in the constellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyRoute {
    /// Main wallet address (the star owner)
    pub owner_address: String,

    /// Legacy wallet (receives funds after passing)
    pub legacy_address: String,

    /// Star ID this legacy route is for
    pub star_id: String,

    /// When the legacy route was created
    pub created_at: u64,

    /// When the user armed it (made it active)
    pub armed_at: Option<u64>,

    /// When the torch was passed (funds transferred)
    pub executed_at: Option<u64>,

    /// Current status
    pub status: LegacyStatus,

    /// Optional message/epitaph
    pub epitaph: Option<String>,

    /// Amount transferred on execution (for historical record)
    pub transferred_amount: Option<u128>,
}

impl LegacyRoute {
    /// Create a new draft legacy route
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

                info!(
                    "Legacy armed for {} → {} (star: {})",
                    self.owner_address, self.legacy_address, self.star_id
                );

                Ok(())
            }
            LegacyStatus::Armed => Err(anyhow!("Legacy route is already armed")),
            LegacyStatus::Executed => Err(anyhow!("Cannot arm an executed legacy route")),
        }
    }

    /// Cancel the legacy route
    pub fn cancel(&mut self) -> Result<()> {
        match self.status {
            LegacyStatus::Draft | LegacyStatus::Armed => {
                self.status = LegacyStatus::Cancelled;

                info!(
                    "Legacy cancelled for {} (star: {})",
                    self.owner_address, self.star_id
                );

                Ok(())
            }
            LegacyStatus::Executed => Err(anyhow!("Cannot cancel an executed legacy route")),
            LegacyStatus::Cancelled => Err(anyhow!("Legacy route is already cancelled")),
        }
    }

    /// Execute the legacy route (pass the torch)
    pub fn execute(&mut self, amount: u128) -> Result<()> {
        if self.status != LegacyStatus::Armed {
            return Err(anyhow!("Legacy route must be armed to execute"));
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.status = LegacyStatus::Executed;
        self.executed_at = Some(now);
        self.transferred_amount = Some(amount);

        info!(
            "🕊️  Torch passed: {} → {} ({} LAND, star: {})",
            self.owner_address,
            self.legacy_address,
            amount as f64 / 1_000_000_000.0,
            self.star_id
        );

        Ok(())
    }

    /// Set an epitaph message
    pub fn set_epitaph(&mut self, epitaph: String) {
        self.epitaph = Some(epitaph);
    }
}

/// Legacy route manager with sled persistence
#[derive(Debug)]
pub struct LegacyManager {
    tree: sled::Tree,
}

impl LegacyManager {
    /// Create a new legacy manager
    pub fn new(db: &Db) -> Result<Self> {
        let tree = db.open_tree(LEGACY_TREE_NAME)?;

        info!("[LEGACY] Legacy routing system initialized");

        Ok(Self { tree })
    }

    /// Get a legacy route by owner address
    pub fn get(&self, owner_address: &str) -> Result<Option<LegacyRoute>> {
        let key = owner_address.as_bytes();

        if let Some(bytes) = self.tree.get(key)? {
            let route: LegacyRoute = serde_json::from_slice(&bytes)?;
            Ok(Some(route))
        } else {
            Ok(None)
        }
    }

    /// Save or update a legacy route
    pub fn save(&self, route: &LegacyRoute) -> Result<()> {
        let key = route.owner_address.as_bytes();
        let value = serde_json::to_vec(route)?;

        self.tree.insert(key, value)?;
        self.tree.flush()?;

        Ok(())
    }

    /// Get all armed legacy routes (for monitoring)
    pub fn get_armed_routes(&self) -> Result<Vec<LegacyRoute>> {
        let mut routes = Vec::new();

        for item in self.tree.iter() {
            let (_, value) = item?;
            let route: LegacyRoute = serde_json::from_slice(&value)?;

            if route.status == LegacyStatus::Armed {
                routes.push(route);
            }
        }

        Ok(routes)
    }

    /// Get all executed legacy routes (for history)
    pub fn get_executed_routes(&self) -> Result<Vec<LegacyRoute>> {
        let mut routes = Vec::new();

        for item in self.tree.iter() {
            let (_, value) = item?;
            let route: LegacyRoute = serde_json::from_slice(&value)?;

            if route.status == LegacyStatus::Executed {
                routes.push(route);
            }
        }

        Ok(routes)
    }

    /// Check if an address has an armed legacy route
    pub fn has_armed_route(&self, owner_address: &str) -> Result<bool> {
        if let Some(route) = self.get(owner_address)? {
            Ok(route.status == LegacyStatus::Armed)
        } else {
            Ok(false)
        }
    }

    /// Validate a legacy route before saving
    pub fn validate(&self, route: &LegacyRoute) -> Result<()> {
        // Cannot be same address
        if route.owner_address == route.legacy_address {
            return Err(anyhow!("Owner and legacy addresses cannot be the same"));
        }

        // Star ID required
        if route.star_id.is_empty() {
            return Err(anyhow!("Star ID is required"));
        }

        // Legacy address must be valid (basic check)
        if route.legacy_address.is_empty() {
            return Err(anyhow!("Legacy address cannot be empty"));
        }

        Ok(())
    }
}

/// Helper to format a legacy message for the user
pub fn legacy_message(status: LegacyStatus) -> &'static str {
    match status {
        LegacyStatus::Draft => "Your legacy route is drafted. Arm it when you're ready.",
        LegacyStatus::Armed => {
            "Your legacy is armed. When your time comes, Vision will carry your torch forward."
        }
        LegacyStatus::Executed => {
            "This star has passed its torch. You are looking at a legacy record."
        }
        LegacyStatus::Cancelled => "This legacy route was cancelled.",
    }
}

/// Message for legacy recipient
pub fn legacy_received_message(star_id: &str, amount: u128) -> String {
    format!(
        "You have received a Legacy Transfer from star {}. {} LAND has been passed to you. Vision salutes the dreamer.",
        star_id,
        amount as f64 / 1_000_000_000.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_route_lifecycle() {
        let mut route = LegacyRoute::new(
            "owner123".to_string(),
            "legacy456".to_string(),
            "star789".to_string(),
        );

        // Initial state
        assert_eq!(route.status, LegacyStatus::Draft);
        assert!(route.armed_at.is_none());
        assert!(route.executed_at.is_none());

        // Arm it
        route.arm().unwrap();
        assert_eq!(route.status, LegacyStatus::Armed);
        assert!(route.armed_at.is_some());

        // Execute it
        route.execute(1000_000_000_000).unwrap();
        assert_eq!(route.status, LegacyStatus::Executed);
        assert!(route.executed_at.is_some());
        assert_eq!(route.transferred_amount, Some(1000_000_000_000));
    }

    #[test]
    fn test_cannot_execute_unarmed() {
        let mut route = LegacyRoute::new(
            "owner123".to_string(),
            "legacy456".to_string(),
            "star789".to_string(),
        );

        // Try to execute without arming
        assert!(route.execute(1000).is_err());
    }

    #[test]
    fn test_legacy_manager() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let manager = LegacyManager::new(&db).unwrap();

        let route = LegacyRoute::new(
            "owner123".to_string(),
            "legacy456".to_string(),
            "star789".to_string(),
        );

        // Save
        manager.save(&route).unwrap();

        // Retrieve
        let loaded = manager.get("owner123").unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().star_id, "star789");
    }
}
