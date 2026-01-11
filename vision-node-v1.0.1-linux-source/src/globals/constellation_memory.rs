//! Constellation Memory with safe pre-DB initialization
//! Supports in-memory operation until sled database is attached

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::Mutex;

/// Persistent peer memory that can boot without a database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationMemory {
    /// Optional database connection (None if in-memory)
    #[serde(skip)]
    db: Option<sled::Db>,

    /// In-memory peer cache (always available)
    peers: HashMap<String, PeerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerEntry {
    pub address: String,
    pub last_seen: u64,
    pub reputation: i32,
}

impl ConstellationMemory {
    /// Create an in-memory peer memory (no database yet)
    pub fn new_in_memory() -> Self {
        Self {
            db: None,
            peers: HashMap::new(),
        }
    }

    /// Attach a database connection (called at runtime once sled is ready)
    pub fn attach_db(&mut self, db: sled::Db) {
        self.db = Some(db);
        // Optionally: load persisted peers from DB here
    }

    /// Check if database is attached
    pub fn has_db(&self) -> bool {
        self.db.is_some()
    }

    /// Get a peer entry (in-memory lookup, no DB)
    pub fn get_peer(&self, address: &str) -> Option<PeerEntry> {
        self.peers.get(address).cloned()
    }

    /// Add or update a peer
    pub fn add_peer(&mut self, address: String, reputation: i32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = PeerEntry {
            address: address.clone(),
            last_seen: now,
            reputation,
        };

        self.peers.insert(address.clone(), entry.clone());

        // Persist to DB if attached
        if let Some(db) = &self.db {
            let tree = db
                .open_tree(b"peers")
                .expect("Failed to open peers tree");
            let _ = tree.insert(address, serde_json::to_vec(&entry).unwrap_or_default());
        }
    }

    /// Get all peers
    pub fn all_peers(&self) -> Vec<PeerEntry> {
        self.peers.values().cloned().collect()
    }

    /// Remove a peer from memory and DB
    pub fn remove_peer(&mut self, address: &str) {
        self.peers.remove(address);

        if let Some(db) = &self.db {
            let tree = db.open_tree(b"peers").expect("Failed to open peers tree");
            let _ = tree.remove(address);
        }
    }
}

impl Default for ConstellationMemory {
    fn default() -> Self {
        Self::new_in_memory()
    }
}
