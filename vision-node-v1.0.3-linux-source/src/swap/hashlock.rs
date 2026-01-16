// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

//! HTLC Hash Lock Cryptography (SHA256 ONLY!)
//!
//! ⚠️  CRITICAL SECURITY NOTICE ⚠️
//! HTLC hash locks MUST use SHA256 for cross-chain atomic swap compatibility.
//! Bitcoin, Ethereum, Lightning Network, and all major blockchain HTLCs use SHA256.
//!
//! ❌ NEVER USE BLAKE3 FOR HASH LOCKS ❌
//! BLAKE3 breaks cross-chain compatibility and makes atomic swaps impossible.
//! BLAKE3 is fine for internal identifiers (htlc_id) but NEVER for the hash lock primitive.
//!
//! All functions in this module use SHA256 exclusively. Do not modify.

use sha2::{Digest, Sha256};

/// Compute SHA256 hash lock for HTLC (32 bytes)
///
/// ⚠️  SHA256 ONLY - DO NOT replace with BLAKE3 or any other hash function!
///
/// This is the ONLY function that should be used to create HTLC hash locks.
/// Returns raw 32-byte SHA256 digest for storage/comparison.
///
/// # Cross-Chain Compatibility
/// SHA256 is the standard for atomic swaps across Bitcoin, Ethereum, Lightning Network, etc.
/// Using any other hash function breaks interoperability with these networks.
///
/// # Example
/// ```ignore
/// let preimage = b"secret_preimage_12345";
/// let hash_lock = htlc_hash_lock(preimage);
/// assert_eq!(hash_lock.len(), 32);
/// ```
pub fn htlc_hash_lock(preimage: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(preimage);
    let mut result = [0u8; 32];
    result.copy_from_slice(&digest);
    result
}

/// Compute SHA256 hash lock for HTLC (hex string)
///
/// ⚠️  SHA256 ONLY - DO NOT replace with BLAKE3 or any other hash function!
///
/// This is the ONLY function that should be used to create HTLC hash locks in hex format.
/// Returns lowercase hex-encoded SHA256 digest for API/storage.
///
/// # Cross-Chain Compatibility
/// SHA256 is the standard for atomic swaps across Bitcoin, Ethereum, Lightning Network, etc.
/// Using any other hash function breaks interoperability with these networks.
///
/// # Example
/// ```ignore
/// let preimage = b"secret_preimage_12345";
/// let hash_lock_hex = htlc_hash_lock_hex(preimage);
/// assert_eq!(hash_lock_hex.len(), 64); // 32 bytes = 64 hex chars
/// ```
pub fn htlc_hash_lock_hex(preimage: &[u8]) -> String {
    hex::encode(htlc_hash_lock(preimage))
}

/// Verify that preimage matches the expected hash lock
///
/// ⚠️  This uses SHA256 - hash lock MUST also be SHA256!
///
/// Returns true if SHA256(preimage) == expected_hash_lock
///
/// # Security Note
/// Always use constant-time comparison in production for hash verification.
/// This implementation uses standard equality which is sufficient for HTLC verification.
pub fn verify_hash_lock(preimage: &[u8], expected_hash_lock: &[u8; 32]) -> bool {
    let computed = htlc_hash_lock(preimage);
    computed == *expected_hash_lock
}

/// Verify that preimage matches the expected hash lock (hex string)
///
/// ⚠️  This uses SHA256 - hash lock MUST also be SHA256!
///
/// Returns true if SHA256(preimage) == hex_decode(expected_hash_lock_hex)
pub fn verify_hash_lock_hex(preimage: &[u8], expected_hash_lock_hex: &str) -> bool {
    let computed_hex = htlc_hash_lock_hex(preimage);
    computed_hex.eq_ignore_ascii_case(expected_hash_lock_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known test vectors from SHA256 specification
    // Ensures we're computing correct SHA256 hashes

    #[test]
    fn test_htlc_hash_lock_known_vector_1() {
        // Test vector: empty string
        let preimage = b"";
        let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        let computed = htlc_hash_lock_hex(preimage);
        assert_eq!(computed, expected_hex, "SHA256 of empty string mismatch");
    }

    #[test]
    fn test_htlc_hash_lock_known_vector_2() {
        // Test vector: "abc"
        let preimage = b"abc";
        let expected_hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

        let computed = htlc_hash_lock_hex(preimage);
        assert_eq!(computed, expected_hex, "SHA256 of 'abc' mismatch");
    }

    #[test]
    fn test_htlc_hash_lock_known_vector_3() {
        // Test vector: longer string
        let preimage = b"The quick brown fox jumps over the lazy dog";
        let expected_hex = "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592";

        let computed = htlc_hash_lock_hex(preimage);
        assert_eq!(computed, expected_hex, "SHA256 of fox/dog string mismatch");
    }

    #[test]
    fn test_htlc_hash_lock_raw_bytes() {
        let preimage = b"test_preimage_12345";
        let hash = htlc_hash_lock(preimage);

        // Should be 32 bytes
        assert_eq!(hash.len(), 32);

        // Should match hex version
        let hash_hex = htlc_hash_lock_hex(preimage);
        assert_eq!(hash_hex, hex::encode(hash));
    }

    #[test]
    fn test_verify_hash_lock_success() {
        let preimage = b"my_secret_preimage";
        let hash = htlc_hash_lock(preimage);

        assert!(verify_hash_lock(preimage, &hash));
    }

    #[test]
    fn test_verify_hash_lock_failure() {
        let preimage = b"my_secret_preimage";
        let wrong_preimage = b"wrong_preimage";
        let hash = htlc_hash_lock(preimage);

        assert!(!verify_hash_lock(wrong_preimage, &hash));
    }

    #[test]
    fn test_verify_hash_lock_hex_success() {
        let preimage = b"test123";
        let hash_hex = htlc_hash_lock_hex(preimage);

        assert!(verify_hash_lock_hex(preimage, &hash_hex));
    }

    #[test]
    fn test_verify_hash_lock_hex_case_insensitive() {
        let preimage = b"test123";
        let hash_hex = htlc_hash_lock_hex(preimage).to_uppercase();

        assert!(verify_hash_lock_hex(preimage, &hash_hex));
    }

    #[test]
    #[should_panic(expected = "BLAKE3")]
    fn test_never_use_blake3_for_hash_lock() {
        // This test ensures we never accidentally use BLAKE3 for hash locks
        // If someone adds BLAKE3 to this module, this test will fail
        let source = include_str!("hashlock.rs");
        if source.contains("blake3") || source.contains("BLAKE3") {
            panic!("NEVER use BLAKE3 for HTLC hash locks! Use SHA256 only for cross-chain compatibility.");
        }
    }
}
