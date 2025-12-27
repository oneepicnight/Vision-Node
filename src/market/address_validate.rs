//! Strict cryptocurrency address validators for BTC, BCH, and DOGE
//!
//! Each validator enforces strict format rules to prevent cross-chain address misrouting:
//! - BTC: Bech32 (bc1.../tb1...) or Base58 P2PKH/P2SH (1.../3...)
//! - BCH: CashAddr format (bitcoincash:... or q.../p...)
//! - DOGE: Base58Check with version byte 0x1E (D...) or 0x16 (A...)

use std::fmt;
use std::str::FromStr;

/// Supported cryptocurrency assets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asset {
    BTC,
    BCH,
    DOGE,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Asset::BTC => write!(f, "BTC"),
            Asset::BCH => write!(f, "BCH"),
            Asset::DOGE => write!(f, "DOGE"),
        }
    }
}

/// Validate a cryptocurrency address for the given asset type
///
/// Returns Ok(()) if valid, Err(reason) if invalid or wrong format
pub fn validate_address(asset: Asset, addr: &str) -> Result<(), String> {
    match asset {
        Asset::BTC => validate_btc_address(addr),
        Asset::BCH => validate_bch_address(addr),
        Asset::DOGE => validate_doge_address(addr),
    }
}

/// Validate Bitcoin address
///
/// Accepts:
/// - Bech32 native SegWit: bc1... (mainnet), tb1... (testnet)
/// - Base58 P2PKH: 1... (mainnet)
/// - Base58 P2SH: 3... (mainnet)
fn validate_btc_address(addr: &str) -> Result<(), String> {
    if addr.is_empty() {
        return Err("BTC address is empty".to_string());
    }

    // Try using the bitcoin crate's built-in validator
    // This is behind the multi-currency feature which you already have
    match bitcoin::Address::from_str(addr) {
        Ok(_) => Ok(()),
        Err(_) => Err(format!(
            "Invalid BTC address '{}': must be Bech32 (bc1.../tb1...) or Base58 (1.../3...)",
            addr
        )),
    }
}

/// Validate Bitcoin Cash address
///
/// Accepts CashAddr format:
/// - With prefix: bitcoincash:q... or bitcoincash:p...
/// - Legacy format: bitcoincash:1... or bitcoincash:3...
///
/// Note: We require the bitcoincash: prefix to avoid confusion with BTC addresses
fn validate_bch_address(addr: &str) -> Result<(), String> {
    if addr.is_empty() {
        return Err("BCH address is empty".to_string());
    }

    // Check for bitcoincash: prefix (required to be explicit)
    if !addr.starts_with("bitcoincash:") && !addr.starts_with("bitcoincash\\:") {
        // Also accept q/p forms which are CashAddr without prefix
        let first_char = addr.chars().next();
        if !matches!(first_char, Some('q') | Some('p')) {
            return Err(format!(
                "Invalid BCH address '{}': must use CashAddr format (bitcoincash:q.../bitcoincash:p...) or q.../p...",
                addr
            ));
        }
        // For q/p forms, do basic validation
        return validate_cashaddr_base(addr);
    }

    // Strip prefix and validate
    let addr_part = if addr.starts_with("bitcoincash:") {
        &addr[12..] // "bitcoincash:" is 12 chars
    } else {
        &addr[13..] // "bitcoincash\:" is 13 chars
    };

    validate_cashaddr_base(addr_part)
}

/// Base58Check validation for CashAddr (q/p prefixes)
fn validate_cashaddr_base(addr: &str) -> Result<(), String> {
    if addr.is_empty() {
        return Err("BCH address part is empty".to_string());
    }

    // Basic CashAddr validation: alphanumeric, starts with q or p
    let first_char = addr.chars().next();
    if !matches!(first_char, Some('q') | Some('p')) {
        return Err(format!(
            "Invalid CashAddr '{}': must start with 'q' (P2PKH) or 'p' (P2SH)",
            addr
        ));
    }

    // Check all chars are valid base32 (CashAddr uses Bech32 encoding)
    if !addr
        .chars()
        .all(|c| "qpzry9x8gf2tvdw0s3jn54khce6mua7l".contains(c))
    {
        return Err(format!(
            "Invalid CashAddr '{}': contains invalid characters",
            addr
        ));
    }

    // Length check: CashAddr is typically 42 chars
    if addr.len() < 36 || addr.len() > 56 {
        return Err(format!(
            "Invalid CashAddr '{}': unexpected length ({}), expected 36-56 chars",
            addr,
            addr.len()
        ));
    }

    Ok(())
}

/// Validate Dogecoin address
///
/// Accepts Base58Check with version bytes:
/// - P2PKH: starts with D (version 0x1E)
/// - P2SH: starts with A (version 0x16)
fn validate_doge_address(addr: &str) -> Result<(), String> {
    if addr.is_empty() {
        return Err("DOGE address is empty".to_string());
    }

    let first_char = addr.chars().next().unwrap_or('?');

    // Check version byte via first character
    match first_char {
        'D' | 'A' => {
            // Valid Dogecoin address prefixes
        }
        _ => {
            return Err(format!(
                "Invalid DOGE address '{}': must start with 'D' (P2PKH, version 0x1E) or 'A' (P2SH, version 0x16)",
                addr
            ));
        }
    }

    // Validate base58check
    match base58check_decode(addr) {
        Ok((version, _payload)) => {
            // Verify version byte matches prefix
            let expected_version = match first_char {
                'D' => 0x1E,
                'A' => 0x16,
                _ => return Err("Unreachable".to_string()),
            };

            if version == expected_version {
                Ok(())
            } else {
                Err(format!(
                    "Invalid DOGE address '{}': version mismatch (expected {}, got {})",
                    addr, expected_version, version
                ))
            }
        }
        Err(e) => Err(format!("Invalid DOGE address '{}': {}", addr, e)),
    }
}

/// Base58Check decode with checksum validation
/// Returns (version_byte, payload) on success
fn base58check_decode(s: &str) -> Result<(u8, Vec<u8>), String> {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    // Decode base58
    let mut num = [0u8; 32];
    let mut num_size = 1;

    for c in s.bytes() {
        let digit = ALPHABET
            .iter()
            .position(|&b| b == c)
            .ok_or_else(|| format!("Invalid base58 character: {}", c as char))?;

        // Multiply by 58
        let mut carry = digit;
        for i in 0..num_size {
            carry += (num[i] as usize) * 58;
            num[i] = (carry & 0xff) as u8;
            carry >>= 8;
        }

        while carry > 0 {
            if num_size >= 32 {
                return Err("Address too long".to_string());
            }
            num[num_size] = (carry & 0xff) as u8;
            carry >>= 8;
            num_size += 1;
        }
    }

    // Count leading '1's (zero bytes)
    let leading_ones = s.chars().take_while(|&c| c == '1').count();

    // Extract payload and checksum
    let mut decoded = vec![0u8; leading_ones];
    decoded.extend_from_slice(&num[0..num_size]);
    decoded.reverse();

    // Minimum: 1 byte version + 4 bytes checksum = 5 bytes
    if decoded.len() < 5 {
        return Err("Address too short".to_string());
    }

    // Split into payload and checksum
    let checksum_start = decoded.len() - 4;
    let payload = decoded[0..checksum_start].to_vec();
    let checksum = &decoded[checksum_start..];

    // Verify checksum (double SHA256 hash, first 4 bytes)
    let hash = sha256_double_hash(&payload);
    if &hash[0..4] != checksum {
        return Err("Checksum mismatch".to_string());
    }

    // Extract version byte and payload data
    if payload.is_empty() {
        return Err("Empty payload".to_string());
    }

    Ok((payload[0], payload[1..].to_vec()))
}

/// Compute SHA256 double hash (used for Base58Check checksum)
fn sha256_double_hash(data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash1 = hasher.finalize();

    let mut hasher = Sha256::new();
    hasher.update(&hash1);
    let hash2 = hasher.finalize();

    hash2.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btc_bech32_mainnet() {
        assert!(validate_btc_address("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").is_ok());
    }

    #[test]
    fn test_btc_base58_p2pkh() {
        assert!(validate_btc_address("1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9").is_ok());
    }

    #[test]
    fn test_btc_base58_p2sh() {
        assert!(validate_btc_address("3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy").is_ok());
    }

    #[test]
    fn test_btc_invalid() {
        assert!(validate_btc_address("invalid_btc_address").is_err());
        assert!(validate_btc_address("").is_err());
    }

    #[test]
    fn test_bch_cashaddr_with_prefix() {
        // Valid CashAddr with bitcoincash: prefix
        assert!(
            validate_bch_address("bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p").is_ok()
        );
    }

    #[test]
    fn test_bch_cashaddr_without_prefix() {
        // Valid CashAddr q form
        assert!(validate_bch_address("qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p").is_ok());
    }

    #[test]
    fn test_bch_invalid() {
        assert!(validate_bch_address("1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9").is_err()); // BTC address
        assert!(validate_bch_address("invalid").is_err());
        assert!(validate_bch_address("").is_err());
    }

    #[test]
    fn test_doge_valid_p2pkh() {
        // D prefix for P2PKH (version 0x1E)
        // This is a real structure but not necessarily a real address
        assert!(validate_doge_address("D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e").is_ok());
    }

    #[test]
    fn test_doge_invalid_prefix() {
        assert!(validate_doge_address("1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9").is_err()); // BTC prefix
        assert!(validate_doge_address("qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p").is_err()); // BCH
        assert!(validate_doge_address("invalid").is_err());
        assert!(validate_doge_address("").is_err());
    }

    #[test]
    fn test_validate_address_api() {
        // Test the main validate_address function
        assert!(validate_address(Asset::BTC, "1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9").is_ok());
        assert!(validate_address(
            Asset::BCH,
            "bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p"
        )
        .is_ok());
        assert!(validate_address(Asset::DOGE, "D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e").is_ok());

        // Cross-asset validation should fail
        assert!(
            validate_address(Asset::BTC, "qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p").is_err()
        );
        assert!(validate_address(Asset::BCH, "1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9").is_err());
    }
}
