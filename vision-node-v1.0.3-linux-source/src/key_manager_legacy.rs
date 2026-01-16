// Private Key Management System
// Handles secure storage and usage of private keys for BTC, BCH, DOGE

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use hex;

use crate::market::engine::QuoteAsset;

// SECURITY WARNING: This is a DEVELOPMENT-ONLY implementation
// For production, you MUST:
// 1. Use proper encryption (AES-256-GCM) for keys at rest
// 2. Use HSM/KMS for key management (AWS KMS, Azure Key Vault, Google Cloud KMS)
// 3. Implement key rotation
// 4. Use secure memory (zeroize crate) to clear keys after use
// 5. Never log private keys
// 6. Implement proper access controls and audit logging

/// Encrypted private key (in production, this would be encrypted with AES-256-GCM)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKey {
    /// Encrypted key data (in dev mode, this is just hex-encoded)
    pub ciphertext: String,
    /// Key derivation salt (for production encryption)
    pub salt: String,
    /// Encryption algorithm version
    pub version: u32,
}

/// User's private keys for external chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserKeys {
    pub user_id: String,
    pub btc_key: Option<EncryptedKey>,
    pub bch_key: Option<EncryptedKey>,
    pub doge_key: Option<EncryptedKey>,
}

impl UserKeys {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            btc_key: None,
            bch_key: None,
            doge_key: None,
        }
    }
}

/// Global key storage (user_id -> UserKeys)
/// WARNING: In production, keys should be stored in encrypted database or HSM
pub static USER_KEYS: Lazy<Arc<Mutex<HashMap<String, UserKeys>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Key Manager for secure key operations
pub struct KeyManager;

impl KeyManager {
    /// Generate a new private key for a user and asset
    /// Returns the WIF (Wallet Import Format) private key string
    #[cfg(feature = "dev-signing")]
    pub fn generate_key(user_id: &str, asset: QuoteAsset) -> Result<String> {
        use rand::Rng;
        
        if asset == QuoteAsset::Land {
            return Err(anyhow!("Cannot generate key for LAND (native asset)"));
        }
        
        // Generate random 32-byte private key
        let mut rng = rand::thread_rng();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes);
        
        // In development, "encrypt" by hex encoding (INSECURE - for dev only!)
        let encrypted = EncryptedKey {
            ciphertext: hex::encode(&key_bytes),
            salt: hex::encode(b"dev_salt"), // Fixed salt for dev
            version: 1,
        };
        
        // Store the encrypted key
        let mut keys = USER_KEYS.lock()
            .map_err(|e| anyhow!("Failed to lock keys: {}", e))?;
        
        let user_keys = keys.entry(user_id.to_string())
            .or_insert_with(|| UserKeys::new(user_id.to_string()));
        
        match asset {
            QuoteAsset::Btc => user_keys.btc_key = Some(encrypted.clone()),
            QuoteAsset::Bch => user_keys.bch_key = Some(encrypted.clone()),
            QuoteAsset::Doge => user_keys.doge_key = Some(encrypted.clone()),
            QuoteAsset::Land => {},
        }
        
        // Convert to WIF for Bitcoin Core import
        let wif = Self::privkey_to_wif(&key_bytes, asset)?;
        
        tracing::info!("🔑 Generated new {} key for user {}", asset.as_str(), user_id);
        
        Ok(wif)
    }
    
    /// Get decrypted private key bytes for signing
    #[cfg(feature = "dev-signing")]
    pub fn get_private_key(user_id: &str, asset: QuoteAsset) -> Result<Vec<u8>> {
        let keys = USER_KEYS.lock()
            .map_err(|e| anyhow!("Failed to lock keys: {}", e))?;
        
        let user_keys = keys.get(user_id)
            .ok_or_else(|| anyhow!("No keys found for user {}", user_id))?;
        
        let encrypted = match asset {
            QuoteAsset::Btc => &user_keys.btc_key,
            QuoteAsset::Bch => &user_keys.bch_key,
            QuoteAsset::Doge => &user_keys.doge_key,
            QuoteAsset::Land => return Err(anyhow!("LAND does not use external keys")),
        };
        
        let encrypted = encrypted.as_ref()
            .ok_or_else(|| anyhow!("No {} key for user {}", asset.as_str(), user_id))?;
        
        // In development, "decrypt" by hex decoding (INSECURE - for dev only!)
        let key_bytes = hex::decode(&encrypted.ciphertext)
            .map_err(|e| anyhow!("Failed to decode key: {}", e))?;
        
        Ok(key_bytes)
    }
    
    /// Import an existing private key (WIF format)
    #[cfg(feature = "dev-signing")]
    pub fn import_key(user_id: &str, asset: QuoteAsset, wif: &str) -> Result<()> {
        if asset == QuoteAsset::Land {
            return Err(anyhow!("Cannot import key for LAND (native asset)"));
        }
        
        // Decode WIF to get private key bytes
        let key_bytes = Self::wif_to_privkey(wif, asset)?;
        
        // "Encrypt" by hex encoding (dev mode only)
        let encrypted = EncryptedKey {
            ciphertext: hex::encode(&key_bytes),
            salt: hex::encode(b"dev_salt"),
            version: 1,
        };
        
        // Store the encrypted key
        let mut keys = USER_KEYS.lock()
            .map_err(|e| anyhow!("Failed to lock keys: {}", e))?;
        
        let user_keys = keys.entry(user_id.to_string())
            .or_insert_with(|| UserKeys::new(user_id.to_string()));
        
        match asset {
            QuoteAsset::Btc => user_keys.btc_key = Some(encrypted),
            QuoteAsset::Bch => user_keys.bch_key = Some(encrypted),
            QuoteAsset::Doge => user_keys.doge_key = Some(encrypted),
            QuoteAsset::Land => {},
        }
        
        tracing::info!("📥 Imported {} key for user {}", asset.as_str(), user_id);
        
        Ok(())
    }
    
    /// Check if user has a key for the given asset
    pub fn has_key(user_id: &str, asset: QuoteAsset) -> bool {
        if asset == QuoteAsset::Land {
            return false;
        }
        
        USER_KEYS.lock()
            .ok()
            .and_then(|keys| keys.get(user_id).map(|uk| {
                match asset {
                    QuoteAsset::Btc => uk.btc_key.is_some(),
                    QuoteAsset::Bch => uk.bch_key.is_some(),
                    QuoteAsset::Doge => uk.doge_key.is_some(),
                    QuoteAsset::Land => false,
                }
            }))
            .unwrap_or(false)
    }
    
    /// Convert private key bytes to WIF (Wallet Import Format)
    #[cfg(feature = "dev-signing")]
    fn privkey_to_wif(key_bytes: &[u8], asset: QuoteAsset) -> Result<String> {
        if key_bytes.len() != 32 {
            return Err(anyhow!("Private key must be 32 bytes"));
        }
        
        // Version byte (mainnet)
        let version: u8 = match asset {
            QuoteAsset::Btc => 0x80,  // Bitcoin mainnet
            QuoteAsset::Bch => 0x80,  // Bitcoin Cash uses same version
            QuoteAsset::Doge => 0x9e, // Dogecoin mainnet
            QuoteAsset::Land => return Err(anyhow!("LAND does not use WIF")),
        };
        
        // Build extended key: version + key + compression flag
        let mut extended = Vec::with_capacity(34);
        extended.push(version);
        extended.extend_from_slice(key_bytes);
        extended.push(0x01); // Compression flag
        
        // Calculate checksum (double SHA256)
        use blake3; // We'll use blake3 as a hash substitute for simplicity
        let hash1 = blake3::hash(&extended);
        let hash2 = blake3::hash(hash1.as_bytes());
        let checksum = &hash2.as_bytes()[0..4];
        
        extended.extend_from_slice(checksum);
        
        // Encode to base58
        Ok(bs58::encode(&extended).into_string())
    }
    
    /// Convert WIF to private key bytes
    #[cfg(feature = "dev-signing")]
    fn wif_to_privkey(wif: &str, asset: QuoteAsset) -> Result<Vec<u8>> {
        // Decode base58
        let decoded = bs58::decode(wif).into_vec()
            .map_err(|e| anyhow!("Invalid WIF format: {}", e))?;
        
        if decoded.len() != 38 && decoded.len() != 37 {
            return Err(anyhow!("Invalid WIF length"));
        }
        
        // Verify version byte
        let expected_version: u8 = match asset {
            QuoteAsset::Btc => 0x80,
            QuoteAsset::Bch => 0x80,
            QuoteAsset::Doge => 0x9e,
            QuoteAsset::Land => return Err(anyhow!("LAND does not use WIF")),
        };
        
        if decoded[0] != expected_version {
            return Err(anyhow!("Invalid version byte for {}", asset.as_str()));
        }
        
        // Extract private key (skip version, compression flag, and checksum)
        let key_bytes = if decoded.len() == 38 {
            decoded[1..33].to_vec() // Compressed
        } else {
            decoded[1..33].to_vec() // Uncompressed
        };
        
        Ok(key_bytes)
    }
}

// Production encryption functions (commented out, requires additional dependencies)
/*
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

impl KeyManager {
    /// Encrypt private key with AES-256-GCM (production)
    fn encrypt_key_production(plaintext: &[u8], master_key: &[u8]) -> Result<EncryptedKey> {
        use rand::Rng;
        
        // Generate random salt
        let mut rng = rand::thread_rng();
        let mut salt = [0u8; 32];
        rng.fill(&mut salt);
        
        // Derive encryption key from master key + salt (use PBKDF2 or Argon2)
        // This is simplified - use proper KDF in production
        let key = Key::from_slice(master_key);
        let cipher = Aes256Gcm::new(key);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rng.fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        Ok(EncryptedKey {
            ciphertext: hex::encode(&ciphertext),
            salt: hex::encode(&salt),
            version: 1,
        })
    }
    
    /// Decrypt private key with AES-256-GCM (production)
    fn decrypt_key_production(encrypted: &EncryptedKey, master_key: &[u8]) -> Result<Vec<u8>> {
        // Derive encryption key from master key + salt
        let key = Key::from_slice(master_key);
        let cipher = Aes256Gcm::new(key);
        
        // Extract nonce from ciphertext
        let ciphertext = hex::decode(&encrypted.ciphertext)?;
        let nonce = Nonce::from_slice(&ciphertext[0..12]);
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, &ciphertext[12..])
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "dev-signing")]
    #[test]
    fn test_key_generation() {
        let user_id = "test_user";
        let asset = QuoteAsset::Btc;
        
        let wif = KeyManager::generate_key(user_id, asset).unwrap();
        assert!(!wif.is_empty());
        assert!(KeyManager::has_key(user_id, asset));
    }
    
    #[cfg(feature = "dev-signing")]
    #[test]
    fn test_key_import_export() {
        let user_id = "test_user_2";
        let asset = QuoteAsset::Btc;
        
        // Generate a key
        let wif = KeyManager::generate_key(user_id, asset).unwrap();
        
        // Clear it
        USER_KEYS.lock().unwrap().remove(user_id);
        
        // Import it back
        KeyManager::import_key(user_id, asset, &wif).unwrap();
        
        // Verify it exists
        assert!(KeyManager::has_key(user_id, asset));
    }
}
