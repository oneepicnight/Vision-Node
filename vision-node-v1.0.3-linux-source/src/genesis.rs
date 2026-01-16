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
/// Genesis hash (computed from deterministic genesis block parameters)
pub const GENESIS_HASH: &str = "d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13";

/// **CRITICAL: ECONOMICS FINGERPRINT**
///
/// This is the canonical cryptographic hash of the chain's economic parameters
/// (vault addresses and split percentages). This hash is consensus-locked and
/// MUST match across all nodes on the network.
///
/// The economics fingerprint ensures:
/// - All nodes use the same vault addresses for reward distribution
/// - Reward split percentages are identical across the network
/// - No node can modify vault addresses without being detected and rejected
///
/// This value is computed from:
/// - Staking vault address (50%): 0xb977c16e539670ddfecc0ac902fcb916ec4b944e
/// - Ecosystem fund address (30%): 0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd
/// - Founder1 address (10%): 0xdf7a79291bb96e9dd1c77da089933767999eabf0
/// - Founder2 address (10%): 0x083f95edd48e3e9da396891b704994b86e7790e7
/// - Split basis points: 5000/3000/1000/1000
///
/// DO NOT change this value - it is part of network consensus.
/// To compute: See chain::economics::econ_hash()
pub const ECON_HASH: &str = "a18f9f82aeb6276b5cfb353e351cd0cf9b34aad962e29f4ac6268f0659c55f95";

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

/// **CRITICAL ECONOMICS VALIDATION FUNCTION**
///
/// Validates that the computed economics fingerprint matches the hardcoded
/// canonical ECON_HASH. This ensures all nodes use the same vault addresses
/// and reward distribution percentages.
///
/// # Errors
///
/// Returns an error if the computed econ hash does not match ECON_HASH.
/// The node MUST NOT continue startup if this validation fails.
///
/// # Security
///
/// This validation:
/// - MUST be called during chain initialization
/// - MUST NOT be skipped or bypassed  
/// - MUST abort startup on failure
/// - Prevents vault address tampering
/// - Ensures network-wide consensus on reward distribution
pub fn validate_econ_hash() -> Result<()> {
    // Load token accounts config
    let config_path = std::path::Path::new("config/token_accounts.toml");
    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| anyhow!("Failed to read token_accounts.toml: {}", e))?;
    
    let config: crate::accounts::TokenAccountsCfg = toml::from_str(&config_str)
        .map_err(|e| anyhow!("Failed to parse token_accounts.toml: {}", e))?;

    // Build Economics struct
    let economics = crate::chain::economics::Economics::from_config(&config);
    
    // Validate splits sum to 100%
    economics.validate()
        .map_err(|e| anyhow!("Economics validation failed: {}", e))?;

    // Compute hash
    let computed_hash = economics.hash_hex();

    if computed_hash != ECON_HASH {
        return Err(anyhow!(
            "CRITICAL: Economics fingerprint mismatch!\n\
             Expected (canonical): {}\n\
             Computed: {}\n\
             \n\
             Vault addresses or splits have been tampered with!\n\
             \n\
             Expected configuration:\n\
             - Staking vault (50%): 0xb977c16e539670ddfecc0ac902fcb916ec4b944e\n\
             - Ecosystem fund (30%): 0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd\n\
             - Founder1 (10%): 0xdf7a79291bb96e9dd1c77da089933767999eabf0\n\
             - Founder2 (10%): 0x083f95edd48e3e9da396891b704994b86e7790e7\n\
             \n\
             Current configuration:\n\
             - Staking vault: {}\n\
             - Ecosystem fund: {}\n\
             - Founder1: {}\n\
             - Founder2: {}\n\
             - Splits (BPS): {}/{}/{}/{}\n\
             \n\
             ACTION REQUIRED:\n\
             - DO NOT proceed with startup\n\
             - Restore correct token_accounts.toml from official source\n\
             - This node CANNOT participate in consensus with wrong vault addresses\n\
             - Contact network administrators if uncertain",
            ECON_HASH,
            computed_hash,
            economics.staking_vault,
            economics.ecosystem_fund,
            economics.founder1,
            economics.founder2,
            economics.split_staking_bps,
            economics.split_fund_bps,
            economics.split_f1_bps,
            economics.split_f2_bps,
        ));
    }

    tracing::info!("✅ Economics fingerprint validation PASSED: {}", ECON_HASH);
    tracing::info!("   Staking vault: {}", economics.staking_vault);
    tracing::info!("   Ecosystem fund: {}", economics.ecosystem_fund);
    tracing::info!("   Founder1: {}", economics.founder1);
    tracing::info!("   Founder2: {}", economics.founder2);

    Ok(())
}

/// Verify economics fingerprint during P2P handshake.
///
/// This function should be called when establishing P2P connections to ensure
/// the remote peer is using the same vault addresses and reward distribution.
///
/// # Arguments
///
/// * `peer_econ_hash` - The economics hash reported by the remote peer
///
/// # Returns
///
/// Returns Ok(()) if the peer's econ hash matches our canonical hash,
/// otherwise returns an error and the connection MUST be rejected.
pub fn verify_peer_econ_hash(peer_econ_hash: &str) -> Result<()> {
    if peer_econ_hash != ECON_HASH {
        return Err(anyhow!(
            "CRITICAL: Peer economics fingerprint mismatch!\n\
             Our canonical econ hash: {}\n\
             Peer's econ hash: {}\n\
             \n\
             This peer is using different vault addresses or reward splits.\n\
             It CANNOT participate in consensus with this network.\n\
             \n\
             ACTION: Connection will be REJECTED.",
            ECON_HASH,
            peer_econ_hash
        ));
    }

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
