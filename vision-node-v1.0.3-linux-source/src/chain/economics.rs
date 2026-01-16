//! Economics Fingerprint - Consensus Lock for Vault Addresses
//! 
//! This module provides a cryptographic fingerprint of the chain's economic parameters
//! (vault addresses and split percentages) that is locked into genesis and enforced
//! during P2P handshake and block validation.

use serde::{Deserialize, Serialize};
use blake3::Hasher;

/// Economics configuration fingerprint - must match across all nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Economics {
    /// Staking vault address (receives 50% of emissions)
    pub staking_vault: String,
    /// Ecosystem/operations fund address (receives 30% of emissions)
    pub ecosystem_fund: String,
    /// Founder 1 address (receives 10% of emissions)
    pub founder1: String,
    /// Founder 2 address (receives 10% of emissions)
    pub founder2: String,
    /// Staking vault split in basis points (5000 = 50%)
    pub split_staking_bps: u32,
    /// Fund split in basis points (3000 = 30%)
    pub split_fund_bps: u32,
    /// Founder 1 split in basis points (1000 = 10%)
    pub split_f1_bps: u32,
    /// Founder 2 split in basis points (1000 = 10%)
    pub split_f2_bps: u32,
}

impl Economics {
    /// Create Economics from TokenAccountsCfg
    pub fn from_config(cfg: &crate::accounts::TokenAccountsCfg) -> Self {
        // Convert percentages to basis points
        let treasury_bps = cfg.treasury_pct * 100;
        let f1_bps = (treasury_bps * cfg.founder1_pct) / 100;
        let f2_bps = (treasury_bps * cfg.founder2_pct) / 100;
        
        Self {
            staking_vault: cfg.vault_address.clone(),
            ecosystem_fund: cfg.fund_address.clone(),
            founder1: cfg.founder1_address.clone(),
            founder2: cfg.founder2_address.clone(),
            split_staking_bps: cfg.vault_pct * 100,
            split_fund_bps: cfg.fund_pct * 100,
            split_f1_bps: f1_bps,
            split_f2_bps: f2_bps,
        }
    }
    
    /// Validate that splits sum to 10000 basis points (100%)
    pub fn validate(&self) -> Result<(), String> {
        let total = self.split_staking_bps + self.split_fund_bps + self.split_f1_bps + self.split_f2_bps;
        if total != 10000 {
            return Err(format!(
                "Economics splits must sum to 10000 bps (100%), got {}. Staking: {}, Fund: {}, F1: {}, F2: {}",
                total, self.split_staking_bps, self.split_fund_bps, self.split_f1_bps, self.split_f2_bps
            ));
        }
        
        // Validate addresses are not empty
        if self.staking_vault.is_empty() {
            return Err("Staking vault address cannot be empty".to_string());
        }
        if self.ecosystem_fund.is_empty() {
            return Err("Ecosystem fund address cannot be empty".to_string());
        }
        if self.founder1.is_empty() {
            return Err("Founder 1 address cannot be empty".to_string());
        }
        if self.founder2.is_empty() {
            return Err("Founder 2 address cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    /// Compute stable cryptographic hash of economics parameters
    /// This hash is used for consensus locking and P2P handshake verification
    pub fn hash(&self) -> [u8; 32] {
        econ_hash(self)
    }
    
    /// Get hash as hex string
    pub fn hash_hex(&self) -> String {
        hex::encode(self.hash())
    }
}

/// Compute stable cryptographic hash of economics parameters
/// Order is fixed for deterministic hashing across all nodes
pub fn econ_hash(e: &Economics) -> [u8; 32] {
    let mut hasher = Hasher::new();
    
    // Hash in fixed order (addresses first, then splits)
    // DO NOT change this order - it would break consensus
    hasher.update(e.staking_vault.as_bytes());
    hasher.update(e.ecosystem_fund.as_bytes());
    hasher.update(e.founder1.as_bytes());
    hasher.update(e.founder2.as_bytes());
    hasher.update(&e.split_staking_bps.to_le_bytes());
    hasher.update(&e.split_fund_bps.to_le_bytes());
    hasher.update(&e.split_f1_bps.to_le_bytes());
    hasher.update(&e.split_f2_bps.to_le_bytes());
    
    let hash = hasher.finalize();
    *hash.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_econ_hash_deterministic() {
        let econ = Economics {
            staking_vault: "0xb977c16e539670ddfecc0ac902fcb916ec4b944e".to_string(),
            ecosystem_fund: "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd".to_string(),
            founder1: "0xdf7a79291bb96e9dd1c77da089933767999eabf0".to_string(),
            founder2: "0x083f95edd48e3e9da396891b704994b86e7790e7".to_string(),
            split_staking_bps: 5000,
            split_fund_bps: 3000,
            split_f1_bps: 1000,
            split_f2_bps: 1000,
        };
        
        let hash1 = econ.hash();
        let hash2 = econ.hash();
        assert_eq!(hash1, hash2, "Hash must be deterministic");
    }
    
    #[test]
    fn test_validate_splits() {
        let mut econ = Economics {
            staking_vault: "0xtest1".to_string(),
            ecosystem_fund: "0xtest2".to_string(),
            founder1: "0xtest3".to_string(),
            founder2: "0xtest4".to_string(),
            split_staking_bps: 5000,
            split_fund_bps: 3000,
            split_f1_bps: 1000,
            split_f2_bps: 1000,
        };
        
        assert!(econ.validate().is_ok());
        
        // Break the splits
        econ.split_staking_bps = 6000;
        assert!(econ.validate().is_err());
    }

    #[test]
    fn test_mainnet_econ_hash() {
        // CRITICAL: This test computes the canonical ECON_HASH for mainnet
        // Run with: cargo test --bin vision-node test_mainnet_econ_hash -- --nocapture
        // Copy the output to genesis.rs ECON_HASH constant
        let econ = Economics {
            staking_vault: "0xb977c16e539670ddfecc0ac902fcb916ec4b944e".to_string(),
            ecosystem_fund: "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd".to_string(),
            founder1: "0xdf7a79291bb96e9dd1c77da089933767999eabf0".to_string(),
            founder2: "0x083f95edd48e3e9da396891b704994b86e7790e7".to_string(),
            split_staking_bps: 5000,
            split_fund_bps: 3000,
            split_f1_bps: 1000,
            split_f2_bps: 1000,
        };

        // Validate splits
        econ.validate().expect("Mainnet splits must be valid");

        let hash_hex = econ.hash_hex();
        
        println!("\n=== CANONICAL MAINNET ECON_HASH ===");
        println!("Copy this value to src/genesis.rs ECON_HASH:");
        println!("{}", hash_hex);
        println!("\nInputs:");
        println!("  Staking vault (50%): {}", econ.staking_vault);
        println!("  Ecosystem fund (30%): {}", econ.ecosystem_fund);
        println!("  Founder1 (10%): {}", econ.founder1);
        println!("  Founder2 (10%): {}", econ.founder2);
        println!("===================================\n");
        
        // Ensure it's 64-char hex
        assert_eq!(hash_hex.len(), 64);
    }
}
