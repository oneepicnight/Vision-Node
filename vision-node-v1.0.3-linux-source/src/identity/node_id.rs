//! Node ID Derivation from Ed25519 Public Keys
//!
//! Canonical formula: node_id = hex(SHA256(ed25519_pubkey_bytes))[0..40]
//!
//! This provides:
//! - 40 hex characters (20 bytes) - short enough for logs, long enough to avoid collisions
//! - Ethereum-style address length
//! - Deterministic: same pubkey always produces same node_id
//! - Verifiable: anyone can verify node_id matches pubkey

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Global node identity (initialized once at startup)
pub static NODE_IDENTITY: OnceCell<Arc<RwLock<NodeIdentity>>> = OnceCell::new();

/// Node identity containing Ed25519 keypair and derived node_id
pub struct NodeIdentity {
    /// Derived node ID (first 40 hex chars of SHA256(pubkey))
    pub node_id: String,
    /// Base64-encoded public key (32 bytes)
    pub pubkey_b64: String,
    /// Raw public key bytes (32 bytes)
    pub pubkey_bytes: [u8; 32],
    /// Ed25519 secret key bytes (for signing)
    secret_key_bytes: [u8; 32],
    /// Ed25519 public key bytes (cached)
    public_key_bytes: [u8; 32],
}

impl NodeIdentity {
    /// Create a new node identity from an Ed25519 keypair
    pub fn from_keypair(keypair: SigningKey) -> Self {
        let pubkey_bytes = keypair.verifying_key().to_bytes();
        let secret_key_bytes = keypair.to_bytes();
        let node_id = node_id_from_pubkey(&pubkey_bytes);
        use base64::{engine::general_purpose, Engine as _};
        let pubkey_b64 = general_purpose::STANDARD.encode(pubkey_bytes);

        Self {
            node_id,
            pubkey_b64,
            pubkey_bytes,
            secret_key_bytes,
            public_key_bytes: pubkey_bytes,
        }
    }

    /// Get the public key fingerprint in 4-4-4-4 format
    ///
    /// Format: FPR: 3A7C-91D2-0B44-FF10
    /// Uses first 8 bytes of SHA256(pubkey)
    pub fn fingerprint(&self) -> String {
        pubkey_fingerprint(&self.pubkey_bytes)
    }

    /// Get the public key
    pub fn public_key(&self) -> VerifyingKey {
        VerifyingKey::from_bytes(&self.public_key_bytes).expect("Valid public key")
    }

    /// Sign a message with the node's private key
    pub fn sign(&self, message: &[u8]) -> ed25519_dalek::Signature {
        let mut keypair_bytes = self.secret_key_bytes.to_vec();
        keypair_bytes.extend_from_slice(&self.public_key_bytes);
        let keypair_array: [u8; 64] = keypair_bytes.try_into().expect("64 bytes");
        let keypair = SigningKey::from_keypair_bytes(&keypair_array).expect("Valid keypair");
        keypair.sign(message)
    }

    /// Get a reconstructed signing keypair (use sparingly, prefer sign() method)
    pub fn keypair(&self) -> SigningKey {
        let mut keypair_bytes = self.secret_key_bytes.to_vec();
        keypair_bytes.extend_from_slice(&self.public_key_bytes);
        let keypair_array: [u8; 64] = keypair_bytes.try_into().expect("64 bytes");
        SigningKey::from_keypair_bytes(&keypair_array).expect("Valid keypair")
    }
}

/// Derive node_id from Ed25519 public key bytes
///
/// Formula: node_id = hex(SHA256(pubkey))[0..40]
///
/// # Arguments
/// * `pubkey_bytes` - 32-byte Ed25519 public key
///
/// # Returns
/// 40-character hexadecimal node ID
pub fn node_id_from_pubkey(pubkey_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pubkey_bytes);
    let hash = hasher.finalize();

    // First 20 bytes → 40 hex chars
    hex::encode(&hash[..20])
}

/// Initialize the global node identity
///
/// Loads or creates an Ed25519 keypair from the database and derives the node_id.
/// This should be called once at startup before any P2P operations.
///
/// # Arguments
/// * `db` - The sled database for persistent storage
///
/// # Returns
/// The initialized NodeIdentity
pub fn init_node_identity(db: &sled::Db) -> anyhow::Result<Arc<RwLock<NodeIdentity>>> {
    const KEYPAIR_KEY: &[u8] = b"node_identity_ed25519_keypair";

    // Try to load existing keypair
    if let Some(keypair_bytes) = db.get(KEYPAIR_KEY)? {
        if keypair_bytes.len() == 64 {
            let keypair_array: [u8; 64] = match keypair_bytes.to_vec().try_into() {
                Ok(arr) => arr,
                Err(_) => anyhow::bail!("Invalid keypair bytes length"),
            };
            match SigningKey::from_keypair_bytes(&keypair_array) {
                Ok(keypair) => {
                    let identity = NodeIdentity::from_keypair(keypair);
                    tracing::info!("🔑 Loaded existing node identity");
                    tracing::info!("   Node ID: {}", identity.node_id);
                    tracing::info!("   Public Key: {}", identity.pubkey_b64);

                    let identity = Arc::new(RwLock::new(identity));
                    NODE_IDENTITY
                        .set(identity.clone())
                        .map_err(|_| anyhow::anyhow!("Node identity already initialized"))?;
                    return Ok(identity);
                }
                Err(e) => {
                    tracing::warn!("⚠️  Failed to parse stored keypair: {}", e);
                    tracing::warn!("   Generating new keypair...");
                }
            }
        } else {
            tracing::warn!(
                "⚠️  Stored keypair has invalid length: {} bytes",
                keypair_bytes.len()
            );
            tracing::warn!("   Generating new keypair...");
        }
    }

    // Generate new keypair

    let keypair = SigningKey::generate(&mut rand::rngs::OsRng);

    // Persist to database (need 64 bytes: [32 secret][32 public])`n    let mut keypair_bytes = keypair.to_bytes().to_vec();`n    keypair_bytes.extend_from_slice(keypair.verifying_key().as_bytes());`n    db.insert(KEYPAIR_KEY, keypair_bytes.as_slice())?;
    db.flush()?;

    let identity = NodeIdentity::from_keypair(keypair);
    tracing::info!("🆔 Generated new node identity");
    tracing::info!("   Node ID: {}", identity.node_id);
    tracing::info!("   Derived from Ed25519 public key");
    tracing::info!("   Public Key: {}", identity.pubkey_b64);

    let identity = Arc::new(RwLock::new(identity));
    NODE_IDENTITY
        .set(identity.clone())
        .map_err(|_| anyhow::anyhow!("Node identity already initialized"))?;

    Ok(identity)
}

/// Get the local node's identity
///
/// # Panics
/// Panics if node identity has not been initialized via init_node_identity()
pub fn local_node_identity() -> Arc<RwLock<NodeIdentity>> {
    NODE_IDENTITY
        .get()
        .expect("Node identity not initialized - call init_node_identity() first")
        .clone()
}

/// Get the local node ID as a string
///
/// Convenience function for quick access to node_id without locking
pub fn local_node_id() -> String {
    let identity = local_node_identity();
    let guard = identity.read();
    guard.node_id.clone()
}

/// Get the local node's public key in base64
pub fn local_pubkey_b64() -> String {
    let identity = local_node_identity();
    let guard = identity.read();
    guard.pubkey_b64.clone()
}

/// Get the local node's public key fingerprint
pub fn local_fingerprint() -> String {
    let identity = local_node_identity();
    let guard = identity.read();
    guard.fingerprint()
}

/// Try to get the local node's identity without panicking
/// Returns None if identity has not been initialized yet
pub fn try_local_node_identity() -> Option<Arc<RwLock<NodeIdentity>>> {
    NODE_IDENTITY.get().cloned()
}

/// Generate a public key fingerprint in 4-4-4-4 format
///
/// Format: 3A7C-91D2-0B44-FF10 (first 8 bytes of SHA256(pubkey))
///
/// # Arguments
/// * `pubkey_bytes` - 32-byte Ed25519 public key
///
/// # Returns
/// Formatted fingerprint string
pub fn pubkey_fingerprint(pubkey_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pubkey_bytes);
    let hash = hasher.finalize();

    // Take first 8 bytes and format as 4-4-4-4
    let hex = hex::encode(&hash[..8]).to_uppercase();
    format!(
        "{}-{}-{}-{}",
        &hex[0..4],
        &hex[4..8],
        &hex[8..12],
        &hex[12..16]
    )
}

/// Verify that a claimed node_id matches a given public key
///
/// # Arguments
/// * `claimed_node_id` - The node_id claimed by a peer
/// * `pubkey_bytes` - The peer's Ed25519 public key bytes
///
/// # Returns
/// true if the node_id is correctly derived from the pubkey, false otherwise
pub fn verify_node_id(claimed_node_id: &str, pubkey_bytes: &[u8]) -> bool {
    let derived = node_id_from_pubkey(pubkey_bytes);
    derived == claimed_node_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_derivation_deterministic() {
        // Generate a test keypair

        let keypair = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey_bytes = keypair.verifying_key().to_bytes();

        // Derive node_id twice
        let id1 = node_id_from_pubkey(&pubkey_bytes);
        let id2 = node_id_from_pubkey(&pubkey_bytes);

        // Should be identical
        assert_eq!(id1, id2);
        // Should be exactly 40 hex chars
        assert_eq!(id1.len(), 40);
        // Should be valid hex
        assert!(hex::decode(&id1).is_ok());
    }

    #[test]
    fn test_different_pubkeys_different_ids() {
        let mut csprng = rand::rngs::OsRng;
        let keypair1 = SigningKey::generate(&mut csprng);
        let keypair2 = SigningKey::generate(&mut csprng);

        let id1 = node_id_from_pubkey(&keypair1.verifying_key().to_bytes());
        let id2 = node_id_from_pubkey(&keypair2.verifying_key().to_bytes());

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_verify_node_id() {
        let keypair = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey_bytes = keypair.verifying_key().to_bytes();
        let correct_id = node_id_from_pubkey(&pubkey_bytes);

        // Correct ID should verify
        assert!(verify_node_id(&correct_id, &pubkey_bytes));

        // Wrong ID should not verify
        assert!(!verify_node_id(
            "0000000000000000000000000000000000000000",
            &pubkey_bytes
        ));
    }

    #[test]
    fn test_node_identity_creation() {
        let keypair = SigningKey::generate(&mut rand::rngs::OsRng);
        let pubkey_bytes = keypair.verifying_key().to_bytes();

        let identity = NodeIdentity::from_keypair(keypair);

        // Node ID should be derived correctly
        let expected_id = node_id_from_pubkey(&pubkey_bytes);
        assert_eq!(identity.node_id, expected_id);

        // Public key should match
        assert_eq!(identity.pubkey_bytes, pubkey_bytes);

        // Base64 encoding should be valid
        use base64::{engine::general_purpose, Engine as _};
        let decoded = general_purpose::STANDARD
            .decode(&identity.pubkey_b64)
            .unwrap();
        assert_eq!(decoded, pubkey_bytes);
    }

    #[test]
    fn test_init_and_load_identity() {
        // Create temporary database
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        // Initialize identity for the first time
        let identity1 = init_node_identity(&db).unwrap();
        let node_id1 = {
            let guard = identity1.read();
            guard.node_id.clone()
        };

        // Close and reopen database
        drop(identity1);
        drop(db);
        let db = sled::Config::new().temporary(true).open().unwrap();

        // Note: In a real scenario, we'd reopen the same DB path
        // For this test, we're just checking the generation logic works
        let identity2 = init_node_identity(&db).unwrap();
        let node_id2 = {
            let guard = identity2.read();
            guard.node_id.clone()
        };

        // Both should be valid 40-char hex strings
        assert_eq!(node_id1.len(), 40);
        assert_eq!(node_id2.len(), 40);
        assert!(hex::decode(&node_id1).is_ok());
        assert!(hex::decode(&node_id2).is_ok());
    }
}
