// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

// Real Address Derivation - Non-Custodial Architecture
// Generates chain-valid addresses using proper encoding per asset

use anyhow::{anyhow, Result};
use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use bitcoin::util::address::Payload;
use bitcoin::util::key::PublicKey as BitcoinPublicKey;
use bitcoin::{Address, Network};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

type HmacSha256 = Hmac<Sha256>;

/// External master seed location (per-node secret)
/// CRITICAL: This file must be backed up by users or funds are lost on reinstall
///
/// Windows: %APPDATA%\Vision\external_master_seed.bin
/// Unix: ./data/external_master_seed.bin
fn seed_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            Path::new(&appdata)
                .join("Vision")
                .join("external_master_seed.bin")
        } else {
            Path::new("data").join("external_master_seed.bin")
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Path::new("data").join("external_master_seed.bin")
    }
}

/// Public accessor for seed file path (used by deposits.rs for import)
pub fn seed_file_path_public() -> PathBuf {
    seed_file_path()
}

/// Load or generate external master seed (32 bytes)
/// On first run: generates cryptographically secure random seed
/// On subsequent runs: loads existing seed from disk
pub fn get_or_create_master_seed() -> Result<[u8; 32]> {
    let seed_path = seed_file_path();

    // Create data directory if it doesn't exist
    if let Some(parent) = seed_path.parent() {
        fs::create_dir_all(parent)?;
        tracing::info!("📁 Created seed directory: {}", parent.display());
    }

    // Try to load existing seed
    if seed_path.exists() {
        let mut file = fs::File::open(&seed_path)?;
        let mut seed = [0u8; 32];
        file.read_exact(&mut seed)?;

        tracing::info!(
            "🔑 Loaded external master seed from {}",
            seed_path.display()
        );
        return Ok(seed);
    }

    // Generate new seed
    use rand::RngCore;
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);

    // Save seed to file with restricted permissions
    let mut file = fs::File::create(&seed_path)?;
    file.write_all(&seed)?;
    file.sync_all()?;

    // Attempt to set restrictive permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600); // Read/write for owner only
        fs::set_permissions(&seed_path, perms)?;
        tracing::info!("🔒 Unix permissions set to 0600 (owner-only)");
    }

    // Windows: Attempt to remove inheritance and grant only to current user (best-effort)
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("icacls")
            .arg(&seed_path)
            .arg("/inheritance:r")
            .arg("/grant:r")
            .arg(format!(
                "{}:F",
                std::env::var("USERNAME").unwrap_or_else(|_| "*S-1-5-32-544".to_string())
            ))
            .output()
        {
            if output.status.success() {
                tracing::info!("🔒 Windows ACL tightened (owner-only access)");
            } else {
                tracing::warn!("⚠️  Windows ACL tightening failed (non-critical)");
            }
        }
    }

    tracing::warn!(
        "🔐 Generated NEW external master seed: {}",
        seed_path.display()
    );
    tracing::warn!("⚠️  BACKUP THIS FILE or funds will be LOST on reinstall!");
    tracing::warn!("    Export via: GET /api/wallet/external/export");

    Ok(seed)
}

/// Derive deterministic child key material using HMAC-SHA256
/// Formula: child_key_material = HMAC-SHA256(master_seed, "VISION::<COIN>::<INDEX>")
fn derive_child_key_material(master_seed: &[u8; 32], coin: &str, index: u32) -> Result<[u8; 32]> {
    let message = format!("VISION::{}::{}", coin, index);

    let mut mac =
        HmacSha256::new_from_slice(master_seed).map_err(|e| anyhow!("HMAC init failed: {}", e))?;
    mac.update(message.as_bytes());

    let result = mac.finalize();
    let bytes = result.into_bytes();

    let mut key_material = [0u8; 32];
    key_material.copy_from_slice(&bytes);

    Ok(key_material)
}

/// Convert key material to secp256k1 secret key (with curve order check)
fn key_material_to_secret_key(key_material: &[u8; 32]) -> Result<SecretKey> {
    SecretKey::from_slice(key_material)
        .map_err(|e| anyhow!("Invalid secret key (out of curve order or zero): {}", e))
}

/// Derive compressed public key from secret key
fn derive_public_key(secret_key: &SecretKey) -> Result<PublicKey> {
    let secp = Secp256k1::new();
    Ok(PublicKey::from_secret_key(&secp, secret_key))
}

/// Generate real BTC address (bech32 P2WPKH: bc1...)
/// Uses mainnet by default; testnet would use Network::Testnet for tb1...
pub fn derive_btc_address(master_seed: &[u8; 32], index: u32) -> Result<String> {
    let key_material = derive_child_key_material(master_seed, "BTC", index)?;
    let secret_key = key_material_to_secret_key(&key_material)?;
    let public_key = derive_public_key(&secret_key)?;

    // Convert to bitcoin crate public key
    let btc_pubkey = BitcoinPublicKey {
        compressed: true,
        inner: public_key,
    };

    // Create P2WPKH (bech32) address
    let address = Address::p2wpkh(&btc_pubkey, Network::Bitcoin)
        .map_err(|e| anyhow!("Failed to create P2WPKH address: {}", e))?;

    Ok(address.to_string())
}

/// Generate real BCH address (CashAddr: bitcoincash:q... or bitcoincash:p...)
/// BCH uses CashAddr format to distinguish from BTC addresses
pub fn derive_bch_address(master_seed: &[u8; 32], index: u32) -> Result<String> {
    let key_material = derive_child_key_material(master_seed, "BCH", index)?;
    let secret_key = key_material_to_secret_key(&key_material)?;
    let public_key = derive_public_key(&secret_key)?;

    // Convert to bitcoin crate public key
    let btc_pubkey = BitcoinPublicKey {
        compressed: true,
        inner: public_key,
    };

    // Create P2PKH payload using bitcoin crate, then encode as CashAddr (pure Rust)
    let address = Address::p2pkh(&btc_pubkey, Network::Bitcoin);
    let pubkey_hash = match address.payload {
        Payload::PubkeyHash(hash) => hash,
        _ => return Err(anyhow!("Expected P2PKH payload")),
    };
    let encoded =
        crate::market::cashaddr::cashaddr_encode("bitcoincash", 0x00, pubkey_hash.as_ref())?;
    Ok(encoded)
}

// Removed simplified CashAddr encoder; using shared pure Rust encoder in market::cashaddr

/// Generate real DOGE address (base58check P2PKH: D...)
/// Uses Dogecoin version byte 0x1E for mainnet P2PKH
pub fn derive_doge_address(master_seed: &[u8; 32], index: u32) -> Result<String> {
    let key_material = derive_child_key_material(master_seed, "DOGE", index)?;
    let secret_key = key_material_to_secret_key(&key_material)?;
    let public_key = derive_public_key(&secret_key)?;

    // Convert to bitcoin crate public key (same curve as BTC)
    let btc_pubkey = BitcoinPublicKey {
        compressed: true,
        inner: public_key,
    };

    // Get pubkey hash (HASH160 = RIPEMD160(SHA256(pubkey)))
    // Compute manually since WPubkeyHash might not be available
    let pubkey_bytes = btc_pubkey.to_bytes();
    let sha_hash = Sha256::digest(&pubkey_bytes);
    let pubkey_hash = ripemd::Ripemd160::digest(&sha_hash);

    // Encode as base58check with Dogecoin version byte
    let doge_address = encode_dogecoin_address(&pubkey_hash)?;

    Ok(doge_address)
}

/// Encode Dogecoin P2PKH address using base58check
/// Version byte: 0x1E (30 decimal) for mainnet, produces addresses starting with 'D'
fn encode_dogecoin_address(pubkey_hash: &[u8]) -> Result<String> {
    const DOGE_P2PKH_VERSION: u8 = 0x1E; // Mainnet P2PKH version

    // Prepend version byte
    let mut payload = vec![DOGE_P2PKH_VERSION];
    payload.extend_from_slice(pubkey_hash);

    // Calculate checksum (first 4 bytes of double SHA256)
    let hash1 = Sha256::digest(&payload);
    let hash2 = Sha256::digest(&hash1);
    let checksum = &hash2[..4];

    // Append checksum
    payload.extend_from_slice(checksum);

    // Encode as base58
    Ok(bs58::encode(&payload).into_string())
}

/// Master function: derive address for any supported asset
pub fn derive_address(coin: &str, index: u32) -> Result<String> {
    let master_seed = get_or_create_master_seed()?;

    match coin.to_uppercase().as_str() {
        "BTC" => derive_btc_address(&master_seed, index),
        "BCH" => derive_bch_address(&master_seed, index),
        "DOGE" => derive_doge_address(&master_seed, index),
        _ => Err(anyhow!("Unsupported coin: {}", coin)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_derivation_is_deterministic() {
        let seed = [42u8; 32];

        let key1 = derive_child_key_material(&seed, "BTC", 0).unwrap();
        let key2 = derive_child_key_material(&seed, "BTC", 0).unwrap();

        assert_eq!(key1, key2, "HMAC derivation must be deterministic");
    }

    #[test]
    fn test_different_indices_produce_different_keys() {
        let seed = [42u8; 32];

        let key0 = derive_child_key_material(&seed, "BTC", 0).unwrap();
        let key1 = derive_child_key_material(&seed, "BTC", 1).unwrap();

        assert_ne!(key0, key1, "Different indices must produce different keys");
    }

    #[test]
    fn test_different_coins_produce_different_keys() {
        let seed = [42u8; 32];

        let btc_key = derive_child_key_material(&seed, "BTC", 0).unwrap();
        let bch_key = derive_child_key_material(&seed, "BCH", 0).unwrap();

        assert_ne!(
            btc_key, bch_key,
            "Different coins must produce different keys"
        );
    }

    #[test]
    fn test_btc_address_format() {
        let seed = [42u8; 32];
        let addr = derive_btc_address(&seed, 0).unwrap();

        assert!(
            addr.starts_with("bc1"),
            "BTC address must start with bc1 (bech32)"
        );
        assert!(
            addr.len() >= 42 && addr.len() <= 62,
            "BTC bech32 address length"
        );
    }

    #[test]
    fn test_bch_address_format() {
        let seed = [42u8; 32];
        let addr = derive_bch_address(&seed, 0).unwrap();

        assert!(
            addr.starts_with("bitcoincash:"),
            "BCH address must have bitcoincash: prefix"
        );
    }

    #[test]
    fn test_doge_address_format() {
        let seed = [42u8; 32];
        let addr = derive_doge_address(&seed, 0).unwrap();

        assert!(
            addr.starts_with("D") || addr.starts_with("9"),
            "DOGE address must start with D or 9 (version 0x1E)"
        );
        assert!(addr.len() >= 34, "DOGE address minimum length");
    }

    #[test]
    fn test_master_function() {
        // Test that master derive_address function routes correctly
        let btc = derive_address("BTC", 0);
        let bch = derive_address("BCH", 0);
        let doge = derive_address("DOGE", 0);

        assert!(btc.is_ok(), "BTC derivation should succeed");
        assert!(bch.is_ok(), "BCH derivation should succeed");
        assert!(doge.is_ok(), "DOGE derivation should succeed");

        let invalid = derive_address("INVALID", 0);
        assert!(invalid.is_err(), "Invalid coin should fail");
    }
}
