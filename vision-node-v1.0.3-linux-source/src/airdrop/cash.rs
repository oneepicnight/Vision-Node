//! CASH Airdrop System
//!
//! Guardian/Admin-only feature to mint and distribute CASH to wallet addresses.
//! CASH is an elastic, internal-only currency separate from LAND and external coins.
//!
//! Rules:
//! - Only affects CASH balances
//! - Never touches LAND, BTC, BCH, or DOGE
//! - Updates global cash_total_supply counter
//! - Requires Guardian/Admin authentication
//! - Includes safety limits to prevent accidental hyperinflation
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sled::Db;
use std::fmt;
use tracing::{error, info};

/// Sled tree for storing cash supply metrics
pub const CASH_SUPPLY_TREE: &str = "cash_supply";
pub const CASH_SUPPLY_KEY: &[u8] = b"total_supply";

/// Errors that can occur during CASH airdrop
#[derive(Debug)]
pub enum CashAirdropError {
    InvalidAddress(String),
    EmptyRecipients,
    ExceedsLimit { requested: u128, max: u128 },
    DatabaseError(String),
    WalletNotFound(String),
    InvalidAmount,
}

impl fmt::Display for CashAirdropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CashAirdropError::InvalidAddress(addr) => write!(f, "Invalid address: {}", addr),
            CashAirdropError::EmptyRecipients => write!(f, "No recipients provided"),
            CashAirdropError::ExceedsLimit { requested, max } => {
                write!(f, "Requested {} CASH exceeds limit of {}", requested, max)
            }
            CashAirdropError::DatabaseError(e) => write!(f, "Database error: {}", e),
            CashAirdropError::WalletNotFound(addr) => write!(f, "Wallet not found: {}", addr),
            CashAirdropError::InvalidAmount => write!(f, "Invalid amount"),
        }
    }
}

impl std::error::Error for CashAirdropError {}

/// Single recipient for CASH airdrop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropRecipient {
    pub address: String,
    pub amount_cash: u128, // in smallest units (like LAND base units)
}

/// Request to execute a CASH airdrop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropRequest {
    pub recipients: Vec<CashAirdropRecipient>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub requested_by: String, // admin address or guardian ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_phrase: Option<String>, // for large airdrops
}

/// Result of executing a CASH airdrop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropResult {
    pub total_recipients: usize,
    pub total_cash: u128,
    pub successful: Vec<String>,       // addresses that succeeded
    pub failed: Vec<(String, String)>, // (address, error message)
}

/// Configuration limits for CASH airdrops
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAirdropLimits {
    /// Maximum total CASH per airdrop transaction
    pub max_cash_per_airdrop: u128,
    /// Whether to require confirmation phrase for large drops
    pub require_confirm_phrase: bool,
    /// Threshold for requiring confirmation (if above this, must confirm)
    pub confirm_threshold: u128,
}

impl Default for CashAirdropLimits {
    fn default() -> Self {
        Self {
            // Default: 1 billion CASH units (adjust based on your decimals)
            max_cash_per_airdrop: 1_000_000_000_000_000_000, // 1B with 9 decimals
            require_confirm_phrase: true,
            confirm_threshold: 100_000_000_000_000_000, // 100M with 9 decimals
        }
    }
}

/// Get current CASH total supply from database
pub fn get_cash_total_supply(db: &Db) -> Result<u128, CashAirdropError> {
    let tree = db
        .open_tree(CASH_SUPPLY_TREE)
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    match tree.get(CASH_SUPPLY_KEY) {
        Ok(Some(bytes)) => {
            if bytes.len() >= 16 {
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&bytes[..16]);
                Ok(u128::from_be_bytes(arr))
            } else {
                Ok(0)
            }
        }
        Ok(None) => Ok(0),
        Err(e) => Err(CashAirdropError::DatabaseError(e.to_string())),
    }
}

/// Update CASH total supply in database
pub fn update_cash_total_supply(db: &Db, new_supply: u128) -> Result<(), CashAirdropError> {
    let tree = db
        .open_tree(CASH_SUPPLY_TREE)
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    tree.insert(CASH_SUPPLY_KEY, &new_supply.to_be_bytes())
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    tree.flush()
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Validate airdrop request against limits
pub fn validate_airdrop_request(
    request: &CashAirdropRequest,
    limits: &CashAirdropLimits,
) -> Result<u128, CashAirdropError> {
    // Check not empty
    if request.recipients.is_empty() {
        return Err(CashAirdropError::EmptyRecipients);
    }

    // Calculate total
    let mut total_cash: u128 = 0;
    for recipient in &request.recipients {
        if recipient.amount_cash == 0 {
            return Err(CashAirdropError::InvalidAmount);
        }
        total_cash = total_cash
            .checked_add(recipient.amount_cash)
            .ok_or(CashAirdropError::InvalidAmount)?;
    }

    // Check against limit
    if total_cash > limits.max_cash_per_airdrop {
        return Err(CashAirdropError::ExceedsLimit {
            requested: total_cash,
            max: limits.max_cash_per_airdrop,
        });
    }

    // Check confirmation phrase if required
    if limits.require_confirm_phrase && total_cash >= limits.confirm_threshold {
        match &request.confirm_phrase {
            Some(phrase) if phrase == "VISION AIRDROP" => {
                // OK
            }
            _ => {
                return Err(CashAirdropError::DatabaseError(
                    "Large airdrop requires confirmation phrase: 'VISION AIRDROP'".to_string(),
                ));
            }
        }
    }

    Ok(total_cash)
}

/// Execute CASH airdrop
///
/// This function:
/// 1. Validates all recipients
/// 2. Credits CASH to each wallet's available balance
/// 3. Updates global cash_total_supply
/// 4. Returns detailed results (successes and failures)
///
/// IMPORTANT: This ONLY affects CASH balances. Never touches LAND or external coins.
pub fn execute_cash_airdrop(
    db: &Db,
    balances_tree_name: &str,
    request: &CashAirdropRequest,
    limits: &CashAirdropLimits,
) -> Result<CashAirdropResult, CashAirdropError> {
    // Validate request
    let total_cash = validate_airdrop_request(request, limits)?;

    info!(
        "[CASH AIRDROP] Starting airdrop: {} recipients, {} total CASH, reason: {:?}, by: {}",
        request.recipients.len(),
        total_cash,
        request.reason,
        request.requested_by
    );

    let mut successful = Vec::new();
    let mut failed = Vec::new();

    // Open balances tree
    let balances_tree = db
        .open_tree(balances_tree_name)
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    // Process each recipient
    for recipient in &request.recipients {
        match credit_cash_to_wallet(&balances_tree, &recipient.address, recipient.amount_cash) {
            Ok(()) => {
                successful.push(recipient.address.clone());
                info!(
                    "[CASH AIRDROP] ✓ Credited {} CASH to {}",
                    recipient.amount_cash, recipient.address
                );
            }
            Err(e) => {
                failed.push((recipient.address.clone(), e.to_string()));
                error!(
                    "[CASH AIRDROP] ✗ Failed to credit {} to {}: {}",
                    recipient.amount_cash, recipient.address, e
                );
            }
        }
    }

    // Update global CASH supply
    let current_supply = get_cash_total_supply(db)?;
    let new_supply = current_supply + total_cash;
    update_cash_total_supply(db, new_supply)?;

    info!(
        "[CASH AIRDROP] Updated total supply: {} → {} (+{})",
        current_supply, new_supply, total_cash
    );

    let result = CashAirdropResult {
        total_recipients: successful.len(),
        total_cash,
        successful,
        failed,
    };

    info!(
        "[CASH AIRDROP] Complete: {} successful, {} failed",
        result.total_recipients,
        result.failed.len()
    );

    Ok(result)
}

/// Credit CASH to a wallet's available balance
///
/// This is a simplified version that assumes wallet data is stored in the balances tree.
/// In production, integrate with your actual wallet storage system.
fn credit_cash_to_wallet(
    balances_tree: &sled::Tree,
    address: &str,
    amount: u128,
) -> Result<(), CashAirdropError> {
    // Key for CASH balance: "cash:address"
    let key = format!("cash:{}", address);

    // Get current CASH balance (default to 0 if not exists)
    let current_balance = match balances_tree.get(key.as_bytes()) {
        Ok(Some(bytes)) => {
            if bytes.len() >= 16 {
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&bytes[..16]);
                u128::from_be_bytes(arr)
            } else {
                0
            }
        }
        Ok(None) => 0,
        Err(e) => {
            return Err(CashAirdropError::DatabaseError(e.to_string()));
        }
    };

    // Add amount
    let new_balance = current_balance
        .checked_add(amount)
        .ok_or(CashAirdropError::InvalidAmount)?;

    // Store new balance
    balances_tree
        .insert(key.as_bytes(), &new_balance.to_be_bytes())
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    balances_tree
        .flush()
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Get CASH balance for a wallet
pub fn get_cash_balance(
    db: &Db,
    balances_tree_name: &str,
    address: &str,
) -> Result<u128, CashAirdropError> {
    let balances_tree = db
        .open_tree(balances_tree_name)
        .map_err(|e| CashAirdropError::DatabaseError(e.to_string()))?;

    let key = format!("cash:{}", address);

    match balances_tree.get(key.as_bytes()) {
        Ok(Some(bytes)) => {
            if bytes.len() >= 16 {
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&bytes[..16]);
                Ok(u128::from_be_bytes(arr))
            } else {
                Ok(0)
            }
        }
        Ok(None) => Ok(0),
        Err(e) => Err(CashAirdropError::DatabaseError(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_airdrop_limits() {
        let limits = CashAirdropLimits::default();
        let mut request = CashAirdropRequest {
            recipients: vec![CashAirdropRecipient {
                address: "0x123".to_string(),
                amount_cash: 1000,
            }],
            reason: Some("Test".to_string()),
            requested_by: "admin".to_string(),
            confirm_phrase: None,
        };

        // Should pass for small amount
        assert!(validate_airdrop_request(&request, &limits).is_ok());

        // Should fail for too large
        request.recipients[0].amount_cash = limits.max_cash_per_airdrop + 1;
        assert!(validate_airdrop_request(&request, &limits).is_err());
    }

    #[test]
    fn test_empty_recipients() {
        let limits = CashAirdropLimits::default();
        let request = CashAirdropRequest {
            recipients: vec![],
            reason: None,
            requested_by: "admin".to_string(),
            confirm_phrase: None,
        };

        assert!(matches!(
            validate_airdrop_request(&request, &limits),
            Err(CashAirdropError::EmptyRecipients)
        ));
    }
}
