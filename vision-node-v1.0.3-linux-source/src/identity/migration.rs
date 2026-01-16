//! Identity Migration Module
//!
//! Handles migration from legacy node identity systems to Ed25519-based identity

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Identity migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityMigration {
    /// Legacy node ID (UUID or temp ID)
    pub legacy_node_id: String,

    /// New Ed25519-based node ID
    pub new_node_id: String,

    /// Base64-encoded Ed25519 public key
    pub new_pubkey_b64: String,

    /// Unix timestamp when migration occurred
    pub migrated_at_unix: u64,
}

impl IdentityMigration {
    /// Get the migration file path
    pub fn migration_file_path() -> PathBuf {
        PathBuf::from("vision_data/identity_migration.json")
    }

    /// Load migration record from disk
    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::migration_file_path();

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)?;
        let migration: IdentityMigration = serde_json::from_str(&contents)?;

        Ok(Some(migration))
    }

    /// Save migration record to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::migration_file_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;

        tracing::info!("✅ Saved identity migration record to {}", path.display());

        Ok(())
    }
}

/// Check for legacy node ID and perform migration if needed
///
/// Legacy sources:
/// - vision_data/node_id.txt
/// - Database key "node_id"
/// - UUID-based temporary IDs
pub fn check_and_migrate_legacy_identity(db: &sled::Db) -> anyhow::Result<Option<String>> {
    // Check if migration already occurred
    if let Some(migration) = IdentityMigration::load()? {
        tracing::info!(
            "🔄 Identity migration already completed: {} → {}",
            migration.legacy_node_id,
            migration.new_node_id
        );
        return Ok(Some(migration.legacy_node_id));
    }

    // Check for legacy node_id in database
    let legacy_id = if let Some(id_bytes) = db.get(b"node_id")? {
        let id_str = String::from_utf8(id_bytes.to_vec())?;

        // Check if it looks like a legacy ID (UUID format or temp-*)
        if id_str.starts_with("temp-")
            || id_str.starts_with("vnode-")
            || id_str.contains('-') && id_str.len() >= 32
        {
            Some(id_str)
        } else {
            None
        }
    } else {
        None
    };

    // Check vision_data/node_id.txt if it exists
    let legacy_id = legacy_id.or_else(|| {
        let path = PathBuf::from("vision_data/node_id.txt");
        if path.exists() {
            fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
        } else {
            None
        }
    });

    if let Some(legacy_id) = legacy_id {
        // Legacy identity found - migration needed
        tracing::warn!("⚠️  Legacy node identity detected: {}", legacy_id);
        tracing::info!("🔄 Migrating to Ed25519-based identity...");

        // Get new identity (should be already initialized by this point)
        let identity = crate::identity::local_node_identity();
        let guard = identity.read();

        // Create migration record
        let migration = IdentityMigration {
            legacy_node_id: legacy_id.clone(),
            new_node_id: guard.node_id.clone(),
            new_pubkey_b64: guard.pubkey_b64.clone(),
            migrated_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        migration.save()?;

        tracing::info!("✅ Identity migration complete:");
        tracing::info!("   Legacy ID: {}", legacy_id);
        tracing::info!("   New ID:    {}", guard.node_id);
        tracing::info!("   Public Key: {}", guard.pubkey_b64);

        Ok(Some(legacy_id))
    } else {
        // No legacy identity found - clean start
        Ok(None)
    }
}

/// Check if a peer is using legacy identity (no pubkey provided)
pub fn is_legacy_peer(pubkey_b64: Option<&String>) -> bool {
    pubkey_b64.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_serialization() {
        let migration = IdentityMigration {
            legacy_node_id: "temp-abc123".to_string(),
            new_node_id: "8f2a9c...aa9e".to_string(),
            new_pubkey_b64: "base64key".to_string(),
            migrated_at_unix: 1765460000,
        };

        let json = serde_json::to_string(&migration).unwrap();
        let deserialized: IdentityMigration = serde_json::from_str(&json).unwrap();

        assert_eq!(migration.legacy_node_id, deserialized.legacy_node_id);
        assert_eq!(migration.new_node_id, deserialized.new_node_id);
    }

    #[test]
    fn test_is_legacy_peer() {
        assert!(is_legacy_peer(None));
        assert!(!is_legacy_peer(Some(&"pubkey".to_string())));
    }
}
