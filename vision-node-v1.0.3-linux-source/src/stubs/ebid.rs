//! Stub for ebid module when staging is disabled

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EternalBroadcastId {
    pub ebid: String,
    pub created_at: u64,
    pub node_tag: Option<String>,
}

impl EternalBroadcastId {
    pub fn generate() -> Self {
        Self {
            ebid: "stub-ebid-00000000".to_string(),
            created_at: 0,
            node_tag: None,
        }
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.node_tag = Some(tag);
        self
    }
}

pub struct EbidManager {
    ebid: EternalBroadcastId,
}

impl EbidManager {
    pub fn new(_db: &sled::Db) -> Result<Self, String> {
        Ok(Self {
            ebid: EternalBroadcastId::generate(),
        })
    }

    pub fn get_ebid(&self) -> &EternalBroadcastId {
        &self.ebid
    }
}
