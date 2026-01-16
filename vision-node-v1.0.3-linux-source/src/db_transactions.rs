// Database transaction helpers for atomic state updates
//
// Ensures atomicity for multi-key operations like balance transfers,
// preventing partial state corruption on crashes.
#![allow(dead_code)]

use sled::Db;
use std::collections::BTreeMap;

/// Atomic balance transfer: debit source, credit destination
/// Returns Ok(()) if successful, Err if insufficient funds or DB error
pub fn atomic_transfer(db: &Db, from_key: &str, to_key: &str, amount: u128) -> Result<(), String> {
    let from_db_key = format!("bal:{}", from_key);
    let to_db_key = format!("bal:{}", to_key);

    // Use sled transaction for atomicity
    db.transaction(|tx_db| {
        // Read source balance
        let from_bal = tx_db
            .get(from_db_key.as_bytes())?
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        // Check sufficient funds
        if from_bal < amount {
            return sled::transaction::abort(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "insufficient funds",
            ));
        }

        // Read destination balance
        let to_bal = tx_db
            .get(to_db_key.as_bytes())?
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        // Perform transfer
        let new_from = from_bal - amount;
        let new_to = to_bal.saturating_add(amount);

        // Write both balances atomically
        tx_db.insert(from_db_key.as_bytes(), &new_from.to_be_bytes()[..])?;
        tx_db.insert(to_db_key.as_bytes(), &new_to.to_be_bytes()[..])?;

        Ok(())
    })
    .map_err(|e| format!("transaction failed: {}", e))?;

    Ok(())
}

/// Atomic multi-key update for balance changes from block execution
/// Applies all balance and nonce changes atomically
pub fn atomic_state_update(
    db: &Db,
    balance_updates: &BTreeMap<String, u128>,
    nonce_updates: &BTreeMap<String, u64>,
) -> Result<(), String> {
    tracing::debug!(
        balance_count = balance_updates.len(),
        nonce_count = nonce_updates.len(),
        "atomic_state_update starting"
    );

    db.transaction(|tx_db| {
        // Apply all balance updates
        for (key, balance) in balance_updates.iter() {
            let db_key = format!("bal:{}", key);
            tx_db.insert(db_key.as_bytes(), &balance.to_be_bytes()[..])?;
        }

        // Apply all nonce updates
        for (key, nonce) in nonce_updates.iter() {
            let db_key = format!("nonce:{}", key);
            tx_db.insert(db_key.as_bytes(), &nonce.to_be_bytes()[..])?;
        }

        Ok(())
    })
    .map_err(|e: sled::transaction::TransactionError| {
        format!("state update transaction failed: {}", e)
    })?;

    tracing::debug!("atomic_state_update completed successfully");
    Ok(())
}

/// Atomic mint operation: credit destination without debit
pub fn atomic_mint(db: &Db, to_key: &str, amount: u128) -> Result<(), String> {
    let db_key = format!("bal:{}", to_key);

    db.transaction(|tx_db| {
        let current = tx_db
            .get(db_key.as_bytes())?
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        let new_balance = current.saturating_add(amount);
        tx_db.insert(db_key.as_bytes(), &new_balance.to_be_bytes()[..])?;

        Ok(())
    })
    .map_err(|e: sled::transaction::TransactionError| format!("mint transaction failed: {}", e))?;

    Ok(())
}

/// Atomic multi-mint: credit multiple destinations
pub fn atomic_multi_mint(db: &Db, recipients: &[(String, u128)]) -> Result<(), String> {
    tracing::debug!(
        recipient_count = recipients.len(),
        "atomic_multi_mint starting"
    );

    db.transaction(|tx_db| {
        for (key, amount) in recipients.iter() {
            let db_key = format!("bal:{}", key);

            let current = tx_db
                .get(db_key.as_bytes())?
                .map(|v| {
                    let mut bytes = [0u8; 16];
                    bytes.copy_from_slice(&v);
                    u128::from_be_bytes(bytes)
                })
                .unwrap_or(0);

            let new_balance = current.saturating_add(*amount);
            tx_db.insert(db_key.as_bytes(), &new_balance.to_be_bytes()[..])?;
        }

        Ok(())
    })
    .map_err(|e: sled::transaction::TransactionError| {
        format!("multi-mint transaction failed: {}", e)
    })?;

    tracing::debug!("atomic_multi_mint completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_transfer_success() {
        let tmp = TempDir::new().expect("tmpdir");
        let db = sled::open(tmp.path()).expect("db");

        // Setup: alice has 100
        let _ = atomic_mint(&db, "alice", 100);

        // Transfer 30 from alice to bob
        let result = atomic_transfer(&db, "alice", "bob", 30);
        assert!(result.is_ok());

        // Verify balances
        let alice_bal = db
            .get(b"bal:alice")
            .unwrap()
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        let bob_bal = db
            .get(b"bal:bob")
            .unwrap()
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        assert_eq!(alice_bal, 70);
        assert_eq!(bob_bal, 30);
    }

    #[test]
    fn test_atomic_transfer_insufficient_funds() {
        let tmp = TempDir::new().expect("tmpdir");
        let db = sled::open(tmp.path()).expect("db");

        // Setup: alice has 50
        let _ = atomic_mint(&db, "alice", 50);

        // Try to transfer 100 (should fail)
        let result = atomic_transfer(&db, "alice", "bob", 100);
        assert!(result.is_err());

        // Verify alice still has 50, bob has 0
        let alice_bal = db
            .get(b"bal:alice")
            .unwrap()
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        let bob_bal = db
            .get(b"bal:bob")
            .unwrap()
            .map(|v| {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&v);
                u128::from_be_bytes(bytes)
            })
            .unwrap_or(0);

        assert_eq!(alice_bal, 50);
        assert_eq!(bob_bal, 0);
    }

    #[test]
    fn test_atomic_multi_mint() {
        let tmp = TempDir::new().expect("tmpdir");
        let db = sled::open(tmp.path()).expect("db");

        let recipients = vec![
            ("alice".to_string(), 100),
            ("bob".to_string(), 200),
            ("carol".to_string(), 50),
        ];

        let result = atomic_multi_mint(&db, &recipients);
        assert!(result.is_ok());

        // Verify all balances
        let alice_bal = db.get(b"bal:alice").unwrap().unwrap();
        let bob_bal = db.get(b"bal:bob").unwrap().unwrap();
        let carol_bal = db.get(b"bal:carol").unwrap().unwrap();

        assert_eq!(u128::from_be_bytes(alice_bal[..].try_into().unwrap()), 100);
        assert_eq!(u128::from_be_bytes(bob_bal[..].try_into().unwrap()), 200);
        assert_eq!(u128::from_be_bytes(carol_bal[..].try_into().unwrap()), 50);
    }

    #[test]
    fn test_atomic_state_update() {
        let tmp = TempDir::new().expect("tmpdir");
        let db = sled::open(tmp.path()).expect("db");

        let mut balances = BTreeMap::new();
        balances.insert("acct:alice".to_string(), 100u128);
        balances.insert("acct:bob".to_string(), 200u128);

        let mut nonces = BTreeMap::new();
        nonces.insert("acct:alice".to_string(), 5u64);
        nonces.insert("acct:bob".to_string(), 10u64);

        let result = atomic_state_update(&db, &balances, &nonces);
        assert!(result.is_ok());

        // Verify data persisted
        let alice_bal = db.get(b"bal:acct:alice").unwrap().unwrap();
        let alice_nonce = db.get(b"nonce:acct:alice").unwrap().unwrap();

        assert_eq!(u128::from_be_bytes(alice_bal[..].try_into().unwrap()), 100);
        assert_eq!(u64::from_be_bytes(alice_nonce[..].try_into().unwrap()), 5);
    }
}
