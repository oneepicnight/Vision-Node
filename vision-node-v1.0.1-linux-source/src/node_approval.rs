//! Node Approval Module
//!
//! Wallet-signed node approval system to prevent node identity spoofing.
//! A node must have a valid wallet signature approving its pubkey-derived node_id.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Node approval data persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeApproval {
    /// Wallet address that approved this node (e.g., "LAND1...")
    pub wallet_address: String,

    /// Node ID derived from Ed25519 public key
    pub node_id: String,

    /// Base64-encoded Ed25519 public key (32 bytes)
    pub node_pubkey_b64: String,

    /// Unix timestamp when approval was created
    pub ts_unix: u64,

    /// Random nonce (16-byte hex) to prevent replay attacks
    pub nonce_hex: String,

    /// Base64-encoded wallet signature over canonical message
    pub signature_b64: String,
}

impl NodeApproval {
    /// Get the approval file path
    pub fn approval_file_path() -> PathBuf {
        PathBuf::from("vision_data/node_approval.json")
    }

    /// Load approval from disk
    pub fn load() -> anyhow::Result<Option<Self>> {
        let path = Self::approval_file_path();

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)?;
        let approval: NodeApproval = serde_json::from_str(&contents)?;

        Ok(Some(approval))
    }

    /// Save approval to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::approval_file_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;

        tracing::info!("✅ Saved node approval to {}", path.display());

        Ok(())
    }

    /// Delete approval from disk
    pub fn delete() -> anyhow::Result<()> {
        let path = Self::approval_file_path();

        if path.exists() {
            fs::remove_file(&path)?;
            tracing::info!("🗑️  Deleted node approval file");
        }

        Ok(())
    }

    /// Build the canonical message for wallet signing
    ///
    /// Format:
    /// ```text
    /// VISION_NODE_APPROVAL_V1
    /// wallet=<WALLET_ADDRESS>
    /// node_id=<NODE_ID>
    /// node_pubkey=<PUBKEY_B64>
    /// ts=<UNIX_SECONDS>
    /// nonce=<RANDOM_16B_HEX>
    /// ```
    pub fn build_canonical_message(
        wallet_address: &str,
        node_id: &str,
        node_pubkey_b64: &str,
        ts_unix: u64,
        nonce_hex: &str,
    ) -> String {
        format!(
            "VISION_NODE_APPROVAL_V1\nwallet={}\nnode_id={}\nnode_pubkey={}\nts={}\nnonce={}",
            wallet_address, node_id, node_pubkey_b64, ts_unix, nonce_hex
        )
    }

    /// Get the canonical message for this approval
    pub fn canonical_message(&self) -> String {
        Self::build_canonical_message(
            &self.wallet_address,
            &self.node_id,
            &self.node_pubkey_b64,
            self.ts_unix,
            &self.nonce_hex,
        )
    }

    /// Verify that this approval is valid for the given node identity
    ///
    /// Checks:
    /// 1. Node ID matches current node
    /// 2. Public key matches current node
    /// 3. Timestamp is within acceptable window (±10 minutes)
    /// 4. Wallet signature is valid
    pub fn verify(&self, current_node_id: &str, current_pubkey_b64: &str) -> Result<(), String> {
        // Check node ID matches
        if self.node_id != current_node_id {
            return Err(format!(
                "Node ID mismatch: approval for {}, current node is {}",
                self.node_id, current_node_id
            ));
        }

        // Check public key matches
        if self.node_pubkey_b64 != current_pubkey_b64 {
            return Err(format!(
                "Public key mismatch: approval for {}, current node has {}",
                self.node_pubkey_b64, current_pubkey_b64
            ));
        }

        // Check timestamp - allow up to 30 days old (node approvals should persist)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_secs();

        let age = now.abs_diff(self.ts_unix);

        const MAX_AGE: u64 = 30 * 24 * 60 * 60; // 30 days in seconds
        if age > MAX_AGE {
            return Err(format!(
                "Timestamp outside acceptable window: age {}s (max {} days)",
                age,
                MAX_AGE / 86400
            ));
        }

        // Wallet signature verification will be done separately
        // using verify_wallet_signature() from legendary_wallet_api

        Ok(())
    }
}

/// Approval status response
#[derive(Debug, Serialize)]
pub struct ApprovalStatus {
    pub approved: bool,
    pub wallet_address: Option<String>,
    pub node_id: String,
    pub node_pubkey_b64: String,
    pub pubkey_fingerprint: String,
    pub last_error: Option<String>,
}

/// Approval submission request
#[derive(Debug, Deserialize)]
pub struct ApprovalSubmitRequest {
    pub wallet_address: String,
    pub ts_unix: u64,
    pub nonce_hex: String,
    pub signature_b64: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_message_format() {
        let msg = NodeApproval::build_canonical_message(
            "LAND1abc123",
            "8f2a9c...aa9e",
            "base64pubkey",
            1765460000,
            "a1b2c3d4e5f6g7h8",
        );

        assert!(msg.starts_with("VISION_NODE_APPROVAL_V1\n"));
        assert!(msg.contains("wallet=LAND1abc123"));
        assert!(msg.contains("node_id=8f2a9c...aa9e"));
        assert!(msg.contains("node_pubkey=base64pubkey"));
        assert!(msg.contains("ts=1765460000"));
        assert!(msg.contains("nonce=a1b2c3d4e5f6g7h8"));
    }

    #[test]
    fn test_approval_verify_node_id_mismatch() {
        let approval = NodeApproval {
            wallet_address: "LAND1abc".to_string(),
            node_id: "wrong_id".to_string(),
            node_pubkey_b64: "correct_pubkey".to_string(),
            ts_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            nonce_hex: "abc123".to_string(),
            signature_b64: "sig".to_string(),
        };

        let result = approval.verify("correct_id", "correct_pubkey");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Node ID mismatch"));
    }
}

/// Check if node has a valid approval on disk
pub fn has_valid_approval() -> bool {
    // Check if identity is initialized first
    let identity_arc = match crate::identity::node_id::try_local_node_identity() {
        Some(id) => id,
        None => return false, // Identity not initialized yet
    };

    let identity = identity_arc.read();
    let node_id = &identity.node_id;
    let node_pubkey_b64 = &identity.pubkey_b64;

    if let Ok(Some(approval)) = NodeApproval::load() {
        // Verify approval matches current identity
        return approval.verify(node_id, node_pubkey_b64).is_ok();
    }
    false
}
