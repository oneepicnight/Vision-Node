//! Dev Payout Configuration
//!
//! Manages the 15 dev-connected addresses:
//! - 6 board seats (elected via governance)
//! - 9 employee dev slots
//!
//! Payouts are distributed from the dev wallet according to basis points allocation.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::info;

const DEV_PAYOUT_CONFIG_TREE: &str = "dev_payout_config";

/// A single dev payout entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevPayoutEntry {
    /// Wallet address
    pub address: String,

    /// Payout allocation in basis points (out of 10000)
    /// e.g., 1000 = 10% of dev wallet
    pub payout_bps: u32,
}

/// Dev payout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevPayoutConfig {
    /// Total basis points allocated (should be <= 10000)
    pub total_bps: u32,

    /// Maximum board seats
    pub board_slots: usize,

    /// Maximum employee slots
    pub employee_slots: usize,

    /// Current board members
    pub board_members: Vec<DevPayoutEntry>,

    /// Current employees
    pub employees: Vec<DevPayoutEntry>,
}

impl Default for DevPayoutConfig {
    fn default() -> Self {
        Self {
            total_bps: 0,
            board_slots: 6,
            employee_slots: 9,
            board_members: Vec::new(),
            employees: Vec::new(),
        }
    }
}

impl DevPayoutConfig {
    /// Get total allocated basis points across all entries
    pub fn allocated_bps(&self) -> u32 {
        let board_bps: u32 = self.board_members.iter().map(|e| e.payout_bps).sum();
        let employee_bps: u32 = self.employees.iter().map(|e| e.payout_bps).sum();
        board_bps + employee_bps
    }

    /// Check if we can add a board member
    pub fn can_add_board_member(&self) -> bool {
        self.board_members.len() < self.board_slots
    }

    /// Check if we can add an employee
    pub fn can_add_employee(&self) -> bool {
        self.employees.len() < self.employee_slots
    }

    /// Add a board member
    pub fn add_board_member(&mut self, address: String, payout_bps: u32) -> Result<()> {
        if !self.can_add_board_member() {
            return Err(anyhow!(
                "Board seats are full ({}/{})",
                self.board_members.len(),
                self.board_slots
            ));
        }

        // Check if address already exists
        if self.find_entry(&address).is_some() {
            return Err(anyhow!("Address already exists in payout config"));
        }

        // Check if allocation would exceed 100%
        if self.allocated_bps() + payout_bps > 10_000 {
            return Err(anyhow!(
                "Total allocation would exceed 100% (current: {}%, adding: {}%)",
                self.allocated_bps() as f64 / 100.0,
                payout_bps as f64 / 100.0
            ));
        }

        self.board_members.push(DevPayoutEntry {
            address,
            payout_bps,
        });
        Ok(())
    }

    /// Remove a board member
    pub fn remove_board_member(&mut self, address: &str) -> Result<()> {
        let initial_len = self.board_members.len();
        self.board_members.retain(|e| e.address != address);

        if self.board_members.len() == initial_len {
            return Err(anyhow!("Board member not found: {}", address));
        }

        Ok(())
    }

    /// Update payout for an address
    pub fn update_payout(&mut self, address: &str, new_payout_bps: u32) -> Result<()> {
        // Find the entry and get old value
        let old_bps = self
            .find_entry(address)
            .ok_or_else(|| anyhow!("Address not found in payout config: {}", address))?
            .payout_bps;

        // Check if new allocation would exceed 100%
        let current_total = self.allocated_bps();
        let new_total = current_total - old_bps + new_payout_bps;

        if new_total > 10_000 {
            return Err(anyhow!(
                "New allocation would exceed 100% (current: {}%, new total: {}%)",
                current_total as f64 / 100.0,
                new_total as f64 / 100.0
            ));
        }

        // Now update the entry
        let entry = self.find_entry_mut(address).unwrap();
        entry.payout_bps = new_payout_bps;
        Ok(())
    }

    /// Find an entry by address (mutable)
    fn find_entry_mut(&mut self, address: &str) -> Option<&mut DevPayoutEntry> {
        self.board_members
            .iter_mut()
            .chain(self.employees.iter_mut())
            .find(|e| e.address == address)
    }

    /// Find an entry by address (immutable)
    fn find_entry(&self, address: &str) -> Option<&DevPayoutEntry> {
        self.board_members
            .iter()
            .chain(self.employees.iter())
            .find(|e| e.address == address)
    }

    /// Get all payout entries as a single list
    pub fn all_entries(&self) -> Vec<&DevPayoutEntry> {
        self.board_members
            .iter()
            .chain(self.employees.iter())
            .collect()
    }
}

/// Dev payout configuration manager
#[derive(Debug)]
pub struct DevPayoutManager {
    tree: sled::Tree,
    config: DevPayoutConfig,
}

impl DevPayoutManager {
    /// Create a new dev payout manager
    pub fn new(db: &Db) -> Result<Self> {
        let tree = db.open_tree(DEV_PAYOUT_CONFIG_TREE)?;

        // Load or initialize config
        let config = if let Some(bytes) = tree.get(b"config")? {
            serde_json::from_slice(&bytes)?
        } else {
            let default_config = DevPayoutConfig::default();
            let bytes = serde_json::to_vec(&default_config)?;
            tree.insert(b"config", bytes)?;
            tree.flush()?;
            default_config
        };

        info!("[DEV_PAYOUT] Manager initialized");
        info!(
            "[DEV_PAYOUT] Board slots: {}/{}",
            config.board_members.len(),
            config.board_slots
        );
        info!(
            "[DEV_PAYOUT] Employee slots: {}/{}",
            config.employees.len(),
            config.employee_slots
        );
        info!(
            "[DEV_PAYOUT] Total allocated: {}%",
            config.allocated_bps() as f64 / 100.0
        );

        Ok(Self { tree, config })
    }

    /// Get current config
    pub fn config(&self) -> &DevPayoutConfig {
        &self.config
    }

    /// Save config to database
    pub fn save(&mut self) -> Result<()> {
        let bytes = serde_json::to_vec(&self.config)?;
        self.tree.insert(b"config", bytes)?;
        self.tree.flush()?;
        Ok(())
    }

    /// Apply a governance action to this config
    pub fn apply_governance_action(
        &mut self,
        action: &crate::governance::GovernanceAction,
    ) -> Result<()> {
        use crate::governance::GovernanceAction;

        match action {
            GovernanceAction::TextOnly => {
                // No-op
                Ok(())
            }

            GovernanceAction::AddBoardMember {
                address,
                payout_bps,
            } => {
                info!(
                    "[DEV_PAYOUT] Adding board member: {} ({}%)",
                    address,
                    *payout_bps as f64 / 100.0
                );
                self.config.add_board_member(address.clone(), *payout_bps)?;
                self.save()?;
                Ok(())
            }

            GovernanceAction::RemoveBoardMember { address } => {
                info!("[DEV_PAYOUT] Removing board member: {}", address);
                self.config.remove_board_member(address)?;
                self.save()?;
                Ok(())
            }

            GovernanceAction::UpdateDevPayout {
                address,
                payout_bps,
            } => {
                info!(
                    "[DEV_PAYOUT] Updating payout for {}: {}%",
                    address,
                    *payout_bps as f64 / 100.0
                );
                self.config.update_payout(address, *payout_bps)?;
                self.save()?;
                Ok(())
            }
        }
    }

    /// Get payout distribution for a given dev wallet balance
    /// Returns map of address -> amount
    pub fn calculate_payouts(&self, dev_wallet_balance: u128) -> Vec<(String, u128)> {
        let mut payouts = Vec::new();

        for entry in self.config.all_entries() {
            let amount = (dev_wallet_balance * entry.payout_bps as u128) / 10_000;
            if amount > 0 {
                payouts.push((entry.address.clone(), amount));
            }
        }

        payouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_payout_config() {
        let mut config = DevPayoutConfig::default();

        // Add board member
        config.add_board_member("board1".to_string(), 1000).unwrap(); // 10%
        assert_eq!(config.allocated_bps(), 1000);

        // Add another
        config.add_board_member("board2".to_string(), 500).unwrap(); // 5%
        assert_eq!(config.allocated_bps(), 1500);

        // Update payout
        config.update_payout("board1", 1500).unwrap();
        assert_eq!(config.allocated_bps(), 2000);

        // Remove
        config.remove_board_member("board2").unwrap();
        assert_eq!(config.allocated_bps(), 1500);
    }

    #[test]
    fn test_cannot_exceed_100_percent() {
        let mut config = DevPayoutConfig::default();

        // Try to add 101%
        assert!(config
            .add_board_member("board1".to_string(), 10_100)
            .is_err());

        // Add 50%
        config.add_board_member("board1".to_string(), 5000).unwrap();

        // Try to add another 51%
        assert!(config.add_board_member("board2".to_string(), 5100).is_err());
    }

    #[test]
    fn test_calculate_payouts() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut manager = DevPayoutManager::new(&db).unwrap();

        manager
            .config
            .add_board_member("addr1".to_string(), 3000)
            .unwrap(); // 30%
        manager
            .config
            .add_board_member("addr2".to_string(), 2000)
            .unwrap(); // 20%

        let payouts = manager.calculate_payouts(1000_000_000_000); // 1000 LAND

        assert_eq!(payouts.len(), 2);
        assert_eq!(payouts[0].1, 300_000_000_000); // 30% = 300 LAND
        assert_eq!(payouts[1].1, 200_000_000_000); // 20% = 200 LAND
    }
}
