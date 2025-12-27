//! Pending Rewards System
//!
//! Banks rewards for nodes that don't have a payout address set.
//! Rewards are minted to VAULT_ADDRESS and tracked in pending_rewards ledger.
//! When user sets payout address, a transaction is auto-created to pay them out.
#![allow(dead_code)]

use crate::vision_constants::PENDING_REWARDS_TREE;
use tracing::{info, warn};

/// Get pending reward balance for a node_id
pub fn pending_get(db: &sled::Db, node_id: &str) -> u64 {
    let tree = match db.open_tree(PENDING_REWARDS_TREE) {
        Ok(t) => t,
        Err(e) => {
            warn!("[PENDING_REWARDS] Failed to open tree: {}", e);
            return 0;
        }
    };

    match tree.get(node_id.as_bytes()) {
        Ok(Some(bytes)) => {
            if bytes.len() == 8 {
                u64::from_le_bytes(bytes.as_ref().try_into().unwrap_or([0u8; 8]))
            } else {
                warn!("[PENDING_REWARDS] Invalid bytes length for {}", node_id);
                0
            }
        }
        Ok(None) => 0,
        Err(e) => {
            warn!("[PENDING_REWARDS] Error reading {}: {}", node_id, e);
            0
        }
    }
}

/// Add amount to pending rewards for a node_id
pub fn pending_add(db: &sled::Db, node_id: &str, amount: u64) {
    let tree = match db.open_tree(PENDING_REWARDS_TREE) {
        Ok(t) => t,
        Err(e) => {
            warn!("[PENDING_REWARDS] Failed to open tree for add: {}", e);
            return;
        }
    };

    let current = pending_get(db, node_id);
    let new_total = current.saturating_add(amount);

    if let Err(e) = tree.insert(node_id.as_bytes(), &new_total.to_le_bytes()) {
        warn!(
            "[PENDING_REWARDS] Failed to save pending for {}: {}",
            node_id, e
        );
        return;
    }

    if let Err(e) = tree.flush() {
        warn!("[PENDING_REWARDS] Failed to flush pending tree: {}", e);
    }

    info!(
        "[PENDING_REWARDS] Banked {} (total: {}) for node_id={}",
        amount, new_total, node_id
    );
}

/// Clear pending rewards for a node_id (after successful payout)
pub fn pending_clear(db: &sled::Db, node_id: &str) {
    let tree = match db.open_tree(PENDING_REWARDS_TREE) {
        Ok(t) => t,
        Err(e) => {
            warn!("[PENDING_REWARDS] Failed to open tree for clear: {}", e);
            return;
        }
    };

    if let Err(e) = tree.remove(node_id.as_bytes()) {
        warn!(
            "[PENDING_REWARDS] Failed to clear pending for {}: {}",
            node_id, e
        );
        return;
    }

    if let Err(e) = tree.flush() {
        warn!("[PENDING_REWARDS] Failed to flush after clear: {}", e);
    }

    info!(
        "[PENDING_REWARDS] Cleared pending rewards for node_id={}",
        node_id
    );
}
/// Attempt to pay out pending rewards to user address
/// Returns Ok(amount_paid) if successful, Err(reason) if failed
pub fn try_payout_pending(
    chain: &mut crate::Chain,
    to_address: &str,
    node_id: &str,
) -> Result<u64, String> {
    let pending = pending_get(&chain.db, node_id);

    if pending == 0 {
        return Ok(0);
    }

    info!(
        "[PENDING_REWARDS] 💰 Attempting payout of {} to {} for node_id={}",
        pending, to_address, node_id
    );

    // Directly transfer from Vault to user (consensus-safe, deterministic)
    // This happens when user sets their payout address
    let vault_key = crate::acct_key(crate::vision_constants::VAULT_ADDRESS);
    let user_key = crate::acct_key(to_address);

    // Check Vault has sufficient balance
    let vault_balance = chain.balances.get(&vault_key).copied().unwrap_or(0);
    if vault_balance < pending as u128 {
        return Err(format!(
            "Insufficient Vault balance: {} < {}",
            vault_balance, pending
        ));
    }

    // Transfer from Vault to user
    *chain.balances.entry(vault_key).or_insert(0) -= pending as u128;
    *chain.balances.entry(user_key).or_insert(0) += pending as u128;

    // Clear pending rewards
    pending_clear(&chain.db, node_id);

    info!(
        "[PENDING_REWARDS] ✅ Successfully paid out {} to {} for node_id={}",
        pending, to_address, node_id
    );

    Ok(pending)
}

/// Get all pending rewards (for debugging/status)
pub fn pending_all(db: &sled::Db) -> Vec<(String, u64)> {
    let tree = match db.open_tree(PENDING_REWARDS_TREE) {
        Ok(t) => t,
        Err(e) => {
            warn!("[PENDING_REWARDS] Failed to open tree for all: {}", e);
            return Vec::new();
        }
    };

    let mut results = Vec::new();

    for item in tree.iter() {
        match item {
            Ok((key, value)) => {
                if let Ok(node_id) = String::from_utf8(key.to_vec()) {
                    if value.len() == 8 {
                        let amount =
                            u64::from_le_bytes(value.as_ref().try_into().unwrap_or([0u8; 8]));
                        results.push((node_id, amount));
                    }
                }
            }
            Err(e) => {
                warn!("[PENDING_REWARDS] Error iterating: {}", e);
                break;
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_operations() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        let node_id = "test-node-123";

        // Initially zero
        assert_eq!(pending_get(&db, node_id), 0);

        // Add some rewards
        pending_add(&db, node_id, 1000);
        assert_eq!(pending_get(&db, node_id), 1000);

        // Add more
        pending_add(&db, node_id, 500);
        assert_eq!(pending_get(&db, node_id), 1500);

        // Clear
        pending_clear(&db, node_id);
        assert_eq!(pending_get(&db, node_id), 0);
    }

    #[test]
    fn test_pending_all() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        pending_add(&db, "node1", 100);
        pending_add(&db, "node2", 200);
        pending_add(&db, "node3", 300);

        let all = pending_all(&db);
        assert_eq!(all.len(), 3);

        let total: u64 = all.iter().map(|(_, amt)| amt).sum();
        assert_eq!(total, 600);
    }
}
