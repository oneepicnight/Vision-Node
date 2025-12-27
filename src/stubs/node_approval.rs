//! Stub implementation of node_approval module when staging feature is disabled
//! Non-custodial: provides safe stubs, no signing/key operations

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeApproval {
    pub node_id: String,
    pub approved: bool,
}

impl NodeApproval {
    pub fn load(_node_id: &str) -> Result<Option<Self>, String> {
        Ok(None)
    }

    pub fn save(&self) -> Result<(), String> {
        Ok(())
    }
}

impl Default for NodeApproval {
    fn default() -> Self {
        Self {
            node_id: String::new(),
            approved: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSubmitRequest {
    pub node_id: String,
}

/// Stub build_canonical_message - returns empty/safe string
pub fn build_canonical_message(_node_id: &str) -> String {
    "canonical_message_stub".to_string()
}
