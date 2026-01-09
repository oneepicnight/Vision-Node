//! Stub implementation of node_approval module when staging feature is disabled
//! Non-custodial: provides safe stubs, no signing/key operations

#[derive(Clone, Debug, Default)]
pub struct NodeApproval {
    pub wallet_address: String,
    pub node_id: String,
    pub node_pubkey_b64: String,
    pub ts_unix: u64,
    pub nonce_hex: String,
    pub signature_b64: String,
}

impl NodeApproval {
    /// Mirror real signature; staging-off returns None
    pub fn load() -> Result<Option<Self>, String> {
        Ok(None)
    }

    /// Mirror real verify; always deny in staging-off builds
    pub fn verify(&self, _node_id: &str, _node_pubkey: &str) -> Result<(), String> {
        Err("node approval is disabled in this build".to_string())
    }

    /// Build canonical message (stubbed)
    pub fn build_canonical_message(
        _wallet_address: &str,
        _node_id: &str,
        _node_pubkey_b64: &str,
        _ts_unix: u64,
        _nonce_hex: &str,
    ) -> String {
        "VISION_NODE_APPROVAL_V1\nstub".to_string()
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ApprovalSubmitRequest {
    pub wallet_address: String,
    pub ts_unix: u64,
    pub nonce_hex: String,
    pub signature_b64: String,
}
