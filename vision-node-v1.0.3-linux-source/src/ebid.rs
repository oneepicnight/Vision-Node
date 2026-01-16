//! Eternal Broadcast ID (EBID) Management
//!
//! Provides a stable, persistent node identity that survives restarts,
//! IP changes, and hardware migrations.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

const EBID_KEY: &[u8] = b"node_ebid";

/// Node's eternal identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalBroadcastId {
    /// The EBID value (UUID v4)
    pub ebid: String,

    /// Unix timestamp of EBID creation
    pub created_at: u64,

    /// Node tag for human readability (if configured)
    pub node_tag: Option<String>,
}

impl EternalBroadcastId {
    /// Generate a new EBID
    pub fn generate() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            ebid: Uuid::new_v4().to_string(),
            created_at: now,
            node_tag: None,
        }
    }

    /// Set the human-readable node tag
    pub fn with_tag(mut self, tag: String) -> Self {
        self.node_tag = Some(tag);
        self
    }
}

/// EBID manager - ensures stable node identity
pub struct EbidManager {
    /// Persistent storage
    db: Arc<Db>,

    /// Cached EBID
    ebid: EternalBroadcastId,
}

impl EbidManager {
    /// Create or load EBID manager
    pub fn new(db: &Db) -> Result<Self, String> {
        let ebid = Self::load_or_generate(db)?;

        info!(
            target: "vision_node::ebid",
            "[EBID] Node eternal ID: {} (created: {})",
            ebid.ebid,
            ebid.created_at
        );

        if let Some(ref tag) = ebid.node_tag {
            info!(
                target: "vision_node::ebid",
                "[EBID] Node tag: {}",
                tag
            );
        }

        Ok(Self {
            db: Arc::new(db.clone()),
            ebid,
        })
    }

    /// Load existing EBID or generate new one
    fn load_or_generate(db: &Db) -> Result<EternalBroadcastId, String> {
        if let Some(data) = db
            .get(EBID_KEY)
            .map_err(|e| format!("Failed to read EBID: {}", e))?
        {
            // Load existing EBID
            let ebid = bincode::deserialize::<EternalBroadcastId>(&data)
                .map_err(|e| format!("Failed to deserialize EBID: {}", e))?;

            info!(
                target: "vision_node::ebid",
                "[EBID] Loaded existing eternal ID from storage"
            );

            Ok(ebid)
        } else {
            // Generate new EBID
            let ebid = EternalBroadcastId::generate();

            info!(
                target: "vision_node::ebid",
                "[EBID] ✨ Generated new eternal broadcast ID"
            );

            // Persist immediately
            Self::save_to_db(db, &ebid)?;

            Ok(ebid)
        }
    }

    /// Save EBID to persistent storage
    fn save_to_db(db: &Db, ebid: &EternalBroadcastId) -> Result<(), String> {
        let serialized =
            bincode::serialize(ebid).map_err(|e| format!("Failed to serialize EBID: {}", e))?;

        db.insert(EBID_KEY, serialized)
            .map_err(|e| format!("Failed to save EBID: {}", e))?;

        db.flush()
            .map_err(|e| format!("Failed to flush EBID: {}", e))?;

        Ok(())
    }

    /// Get the EBID value
    pub fn get_ebid(&self) -> &str {
        &self.ebid.ebid
    }

    /// Get the full EBID structure
    pub fn get_full(&self) -> &EternalBroadcastId {
        &self.ebid
    }

    /// Set node tag (human-readable name)
    pub fn set_tag(&mut self, tag: String) -> Result<(), String> {
        self.ebid.node_tag = Some(tag.clone());
        Self::save_to_db(&self.db, &self.ebid)?;

        info!(
            target: "vision_node::ebid",
            "[EBID] Updated node tag: {}",
            tag
        );

        Ok(())
    }

    /// Get node tag if set
    pub fn get_tag(&self) -> Option<&str> {
        self.ebid.node_tag.as_deref()
    }

    /// Get age of this EBID in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now.saturating_sub(self.ebid.created_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebid_generation() {
        let ebid = EternalBroadcastId::generate();
        assert!(!ebid.ebid.is_empty());
        assert!(ebid.created_at > 0);
    }

    #[test]
    fn test_ebid_with_tag() {
        let ebid = EternalBroadcastId::generate().with_tag("test-node".to_string());
        assert_eq!(ebid.node_tag, Some("test-node".to_string()));
    }
}
