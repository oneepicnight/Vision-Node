// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

use anyhow::{anyhow, Result};

/// **CRITICAL: HARDCODED GENESIS HASH**
///
/// This is the canonical, deterministic hash of the Vision Node genesis block.
/// This hash MUST match the computed hash of the genesis block at startup.
///
/// DO NOT change this value unless performing a hard fork or network reset.
/// DO NOT fetch genesis information from remote nodes - this is the source of truth.
///
/// Genesis block parameters:
/// - version: 1
/// - height: 0
/// - prev_hash: [0; 32]
/// - timestamp: 0
/// - difficulty: 1
/// - nonce: 0
/// - transactions_root: [0; 32]
pub const GENESIS_HASH: &str = "d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13";

/// Compute a consensus-style genesis pow_hash using the same layout used by
/// the PoW block header hashing (version, height, prev_hash, timestamp,
/// difficulty, nonce, transactions_root). Returns a hex string (without 0x).
pub fn compute_genesis_pow_hash() -> String {
    let mut bytes = Vec::with_capacity(4 + 8 + 32 + 8 + 8 + 8 + 32);
    bytes.extend_from_slice(&1u32.to_be_bytes()); // version
    bytes.extend_from_slice(&0u64.to_be_bytes()); // height
    bytes.extend_from_slice(&[0u8; 32]); // prev_hash
    bytes.extend_from_slice(&0u64.to_be_bytes()); // timestamp
    bytes.extend_from_slice(&1u64.to_be_bytes()); // difficulty
    bytes.extend_from_slice(&0u64.to_be_bytes()); // nonce
    bytes.extend_from_slice(&[0u8; 32]); // transactions_root
    let hash = blake3::hash(&bytes);
    let result = hex::encode(hash.as_bytes());
    eprintln!("🔍 compute_genesis_pow_hash() computed: {}", result);
    eprintln!("🔍 GENESIS_HASH constant is: {}", GENESIS_HASH);
    result
}

/// **CRITICAL SECURITY FUNCTION**
///
/// Validates that the computed genesis hash matches the hardcoded canonical hash.
/// This prevents chain substitution attacks and ensures all nodes start with
/// the same genesis block.
///
/// # Errors
///
/// Returns an error if the computed hash does not match GENESIS_HASH.
/// The node MUST NOT continue startup if this validation fails.
///
/// # Security
///
/// This validation:
/// - MUST be called during chain initialization
/// - MUST NOT be skipped or bypassed
/// - MUST abort startup on failure
/// - MUST NOT rely on any remote data
pub fn validate_genesis_hash() -> Result<()> {
    let computed_hash = compute_genesis_pow_hash();

    if computed_hash != GENESIS_HASH {
        return Err(anyhow!(
            "CRITICAL: Genesis hash mismatch!\n\
             Expected (canonical): {}\n\
             Computed: {}\n\
             \n\
             This indicates:\n\
             1. Chain database corruption\n\
             2. Incorrect genesis block parameters\n\
             3. Hard fork or network split\n\
             \n\
             ACTION REQUIRED:\n\
             - DO NOT proceed with startup\n\
             - Verify genesis block configuration\n\
             - Check for database corruption\n\
             - Contact network administrators if on official network\n\
             - Reset chain data if on test network",
            GENESIS_HASH,
            computed_hash
        ));
    }

    tracing::info!("✅ Genesis hash validation PASSED: {}", GENESIS_HASH);

    Ok(())
}

/// Verify genesis block integrity during chain initialization.
///
/// This function should be called BEFORE the chain database is used
/// for any operations. It ensures the genesis block in the database
/// matches the hardcoded canonical genesis block.
///
/// # Arguments
///
/// * `stored_genesis_hash` - The hash of the genesis block currently stored in the database
///
/// # Returns
///
/// Returns Ok(()) if validation passes, otherwise returns an error that MUST
/// cause the node to abort startup.
pub fn verify_stored_genesis(stored_genesis_hash: &str) -> Result<()> {
    if stored_genesis_hash != GENESIS_HASH {
        return Err(anyhow!(
            "CRITICAL: Stored genesis block does not match canonical genesis!\n\
             Canonical hash: {}\n\
             Stored hash: {}\n\
             \n\
             The chain database contains an incorrect genesis block.\n\
             This could indicate:\n\
             1. Database corruption\n\
             2. Incorrect network/chain data\n\
             3. Tampering attempt\n\
             \n\
             ACTION REQUIRED:\n\
             - DELETE the chain database\n\
             - Restart the node to regenerate correct genesis\n\
             - DO NOT attempt to sync from this state",
            GENESIS_HASH,
            stored_genesis_hash
        ));
    }

    tracing::info!(
        "✅ Stored genesis block validation PASSED: {}",
        GENESIS_HASH
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_hash_computation() {
        let hash = compute_genesis_pow_hash();
        assert_eq!(hash, GENESIS_HASH, "Genesis hash computation changed!");
    }

    #[test]
    fn test_genesis_validation_success() {
        // Should pass with correct hash
        let result = validate_genesis_hash();
        assert!(result.is_ok(), "Genesis validation should pass");
    }

    #[test]
    fn test_stored_genesis_validation_success() {
        // Should pass with correct hash
        let result = verify_stored_genesis(GENESIS_HASH);
        assert!(result.is_ok(), "Stored genesis validation should pass");
    }

    #[test]
    fn test_stored_genesis_validation_failure() {
        // Should fail with incorrect hash
        let result = verify_stored_genesis(
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert!(
            result.is_err(),
            "Stored genesis validation should fail with wrong hash"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("CRITICAL"),
            "Error should be marked as critical"
        );
        assert!(
            err_msg.contains(GENESIS_HASH),
            "Error should show canonical hash"
        );
    }
}
