#![allow(dead_code)]
use blake3;
use serde::{Deserialize, Serialize};

/// Canonical node id type - deterministic and wallet-bound
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Deterministically derive a node id from a wallet address.
/// This ensures: same wallet address = same node id.
///
/// # Arguments
/// * `wallet_address` - The primary LAND wallet address
///
/// # Returns
/// A deterministic NodeId based on the wallet address
pub fn derive_node_id_from_address(wallet_address: &str) -> NodeId {
    // Add a domain separator so this hash is only used for node ids
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"vision-node-id-v1");
    hasher.update(wallet_address.as_bytes());

    let hash = hasher.finalize();
    // Take first 16 hex chars to keep it short & readable
    let short = &hash.to_hex()[..16];
    NodeId(format!("vnode-{}", short))
}

/// Load existing node id for this wallet, or create + persist one.
/// This makes node id both wallet-bound and restart-stable.
///
/// # Arguments
/// * `db` - The sled database
/// * `wallet_address` - The primary LAND wallet address
///
/// # Returns
/// The persistent NodeId for this wallet
pub fn load_or_create_node_id(db: &sled::Db, wallet_address: &str) -> anyhow::Result<NodeId> {
    let key = format!("node_id:{}", wallet_address);

    if let Some(existing) = db.get(key.as_bytes())? {
        let id_str = String::from_utf8(existing.to_vec())?;
        tracing::info!("✅ Loaded existing node ID: {}", id_str);
        return Ok(NodeId(id_str));
    }

    // Derive new id from wallet address
    let node_id = derive_node_id_from_address(wallet_address);

    // Persist it
    db.insert(key.as_bytes(), node_id.0.as_bytes())?;
    db.flush()?;

    tracing::info!(
        "🆔 Created new node ID: {} (derived from wallet: {})",
        node_id.0,
        wallet_address
    );

    Ok(node_id)
}

/// Ensure node wallet consistency - detect if wallet has changed.
/// This prevents accidentally reusing the same node DB with a different wallet.
///
/// # Arguments
/// * `db` - The sled database
/// * `wallet_address` - The current wallet address
///
/// # Returns
/// Ok if consistent, Err if wallet mismatch detected
pub fn ensure_node_wallet_consistency(db: &sled::Db, wallet_address: &str) -> anyhow::Result<()> {
    const KEY: &[u8] = b"node_wallet_address";

    if let Some(existing) = db.get(KEY)? {
        let existing_addr = String::from_utf8(existing.to_vec())?;
        if existing_addr != wallet_address {
            anyhow::bail!(
                "❌ CRITICAL: Wallet mismatch detected!\n\
                 Node was previously initialized with wallet: {}\n\
                 Current wallet: {}\n\
                 \n\
                 This node's identity is bound to the original wallet.\n\
                 \n\
                 RESOLUTION OPTIONS:\n\
                 1. Use the original wallet ({})\n\
                 2. Use a fresh data directory for the new wallet\n\
                 3. Delete the node data directory to reset identity\n\
                 \n\
                 DO NOT proceed - this would cause P2P identity conflicts.",
                existing_addr,
                wallet_address,
                existing_addr
            );
        }
    } else {
        db.insert(KEY, wallet_address.as_bytes())?;
        db.flush()?;
        tracing::info!(
            "🔒 Wallet address locked to node identity: {}",
            wallet_address
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_node_id_deterministic() {
        let addr = "land_test_address_123";
        let id1 = derive_node_id_from_address(addr);
        let id2 = derive_node_id_from_address(addr);

        assert_eq!(id1, id2, "Same address should produce same node ID");
        assert!(
            id1.0.starts_with("vnode-"),
            "Node ID should have vnode- prefix"
        );
        assert_eq!(
            id1.0.len(),
            22,
            "Node ID should be vnode- (6) + 16 hex chars"
        );
    }

    #[test]
    fn test_different_addresses_different_ids() {
        let addr1 = "land_address_1";
        let addr2 = "land_address_2";

        let id1 = derive_node_id_from_address(addr1);
        let id2 = derive_node_id_from_address(addr2);

        assert_ne!(
            id1, id2,
            "Different addresses should produce different node IDs"
        );
    }

    #[test]
    fn test_load_or_create_persistence() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        let wallet_addr = "test_wallet_123";

        // First call creates and persists
        let id1 = load_or_create_node_id(&db, wallet_addr).unwrap();

        // Second call loads existing
        let id2 = load_or_create_node_id(&db, wallet_addr).unwrap();

        assert_eq!(id1, id2, "Should load same ID from persistence");
    }

    #[test]
    fn test_wallet_consistency_check() {
        let config = sled::Config::new().temporary(true);
        let db = config.open().unwrap();

        let wallet1 = "wallet_1";
        let wallet2 = "wallet_2";

        // First wallet should succeed
        ensure_node_wallet_consistency(&db, wallet1).unwrap();

        // Same wallet again should succeed
        ensure_node_wallet_consistency(&db, wallet1).unwrap();

        // Different wallet should fail
        let result = ensure_node_wallet_consistency(&db, wallet2);
        assert!(result.is_err(), "Should detect wallet mismatch");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Wallet mismatch"),
            "Error should mention wallet mismatch"
        );
    }
}
