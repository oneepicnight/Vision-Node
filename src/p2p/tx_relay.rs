#![allow(dead_code)]
//! Inventory-based relay for blocks & transactions (INV / GETDATA)
//!
//! Implements Bitcoin-style transaction and block propagation:
//! 1. Node announces inventory (INV) to peers
//! 2. Peer requests specific items (GETDATA)
//! 3. Node sends requested data
//!
//! Benefits:
//! - Bandwidth efficient (announce hash, not full object)
//! - Peer-driven (receiver controls what to fetch)
//! - Deduplication (don't send if peer already has it)

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Type of inventory item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvType {
    /// Block hash
    Block,
    /// Transaction hash
    Tx,
    /// Compact block
    CompactBlock,
}

/// A single inventory item (type + hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    #[serde(rename = "type")]
    pub inv_type: InvType,
    pub hash: String,
}

/// INV message: announces availability of objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inv {
    pub objects: Vec<InventoryItem>,
}

/// GETDATA message: requests specific objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetData {
    pub objects: Vec<InventoryItem>,
}

/// Tracks what we've announced to each peer (avoid redundant INVs)
static ANNOUNCED_TO_PEER: Lazy<Mutex<std::collections::HashMap<String, HashSet<String>>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

impl Inv {
    /// Create INV for a block
    pub fn block(hash: String) -> Self {
        Self {
            objects: vec![InventoryItem {
                inv_type: InvType::Block,
                hash,
            }],
        }
    }

    /// Create INV for a transaction
    pub fn tx(hash: String) -> Self {
        Self {
            objects: vec![InventoryItem {
                inv_type: InvType::Tx,
                hash,
            }],
        }
    }

    /// Create INV for multiple items
    pub fn batch(items: Vec<InventoryItem>) -> Self {
        Self { objects: items }
    }
}

impl GetData {
    /// Create GETDATA for specific hashes
    pub fn new(objects: Vec<InventoryItem>) -> Self {
        Self { objects }
    }
}

/// Handle an incoming INV message from a peer
pub async fn handle_inv(peer: String, inv: Inv) {
    tracing::debug!(
        target = "p2p::inv",
        peer = %peer,
        count = inv.objects.len(),
        "Received INV"
    );

    let mut requests = Vec::new();
    let num_objects = inv.objects.len();

    for item in inv.objects {
        match item.inv_type {
            InvType::Block => {
                // Enterprise: Check if we have this block in our chain
                let have_block = {
                    let g = crate::CHAIN.lock();
                    g.blocks.iter().any(|b| b.header.pow_hash == item.hash)
                };
                if !have_block {
                    let hash_clone = item.hash.clone();
                    requests.push(item);
                    tracing::debug!(
                        target = "p2p::inv",
                        hash = %hash_clone,
                        "Block announced, requesting"
                    );
                }
            }
            InvType::Tx => {
                // Enterprise: Check if we have this TX in mempool or chain
                let have_tx = {
                    let g = crate::CHAIN.lock();
                    g.seen_txs.contains_key(&item.hash)
                        || g.mempool_critical
                            .iter()
                            .any(|tx| hex::encode(crate::tx_hash(tx)) == item.hash)
                        || g.mempool_bulk
                            .iter()
                            .any(|tx| hex::encode(crate::tx_hash(tx)) == item.hash)
                };
                if !have_tx {
                    let hash_clone = item.hash.clone();
                    requests.push(item);
                    tracing::debug!(
                        target = "p2p::inv",
                        hash = %hash_clone,
                        "Tx announced, requesting"
                    );
                }
            }
            InvType::CompactBlock => {
                // Enterprise: Handle compact block announcement
                let hash_clone = item.hash.clone();
                // Request compact block if we don't have it
                let have_block = {
                    let g = crate::CHAIN.lock();
                    g.blocks.iter().any(|b| b.header.pow_hash == item.hash)
                };
                if !have_block {
                    requests.push(item);
                }
                tracing::debug!(
                    target = "p2p::inv",
                    hash = %hash_clone,
                    "Compact block announced"
                );
            }
        }
    }

    // Send GETDATA for items we need
    if !requests.is_empty() {
        let getdata = GetData::new(requests);
        send_getdata_to_peer(&peer, getdata).await;
    }

    // Update metrics
    crate::PROM_P2P_ANNOUNCES_RECEIVED.inc_by(num_objects as u64);
}

/// Handle a GETDATA request from a peer
pub async fn handle_getdata(peer: String, req: GetData) {
    tracing::debug!(
        target = "p2p::getdata",
        peer = %peer,
        count = req.objects.len(),
        "Received GETDATA"
    );

    use crate::PROM_TX_GETDATA_RECEIVED;

    // Track metrics
    if req
        .objects
        .iter()
        .any(|item| matches!(item.inv_type, InvType::Tx))
    {
        PROM_TX_GETDATA_RECEIVED.inc();
    }

    for item in req.objects {
        match item.inv_type {
            InvType::Block => {
                // Send full block if we have it
                if let Some(block) = get_block_by_hash(&item.hash) {
                    send_block_to_peer(&peer, block).await;
                } else {
                    tracing::warn!(
                        target = "p2p::getdata",
                        peer = %peer,
                        hash = %item.hash,
                        "Peer requested block we don't have"
                    );
                }
            }
            InvType::Tx => {
                // Send transaction if in mempool
                if let Some(tx) = get_tx_from_mempool_async(&item.hash).await {
                    tracing::debug!(
                        target = "p2p::getdata",
                        peer = %peer,
                        hash = %item.hash,
                        "Sending transaction to peer"
                    );
                    send_tx_to_peer(&peer, tx).await;
                } else {
                    tracing::warn!(
                        target = "p2p::getdata",
                        peer = %peer,
                        hash = %item.hash,
                        "Peer requested tx we don't have"
                    );
                }
            }
            InvType::CompactBlock => {
                // Send compact block
                tracing::debug!(
                    target = "p2p::getdata",
                    hash = %item.hash,
                    "Compact block requested"
                );
            }
        }
    }
}

/// Announce inventory using intelligent routing (Phase 3.5)
///
/// Selects peers based on latency, reliability, and geographic clustering
/// for optimal propagation speed and network coverage.
pub async fn announce_with_routing(
    peer_store: &crate::p2p::peer_store::PeerStore,
    local_region: Option<&str>,
    inv: Inv,
    max_peers: usize,
) -> Result<(), String> {
    // Use intelligent routing to select best peers
    let relay_targets =
        crate::p2p::routing::select_relay_targets(peer_store, local_region, max_peers);

    // Extract peer node IDs
    let peer_ids: Vec<String> = relay_targets.iter().map(|p| p.node_id.clone()).collect();

    tracing::debug!(
        target: "p2p::relay",
        peers = peer_ids.len(),
        inv_count = inv.objects.len(),
        "Announcing inventory using intelligent routing"
    );

    // Delegate to existing announce_to_peers function
    announce_to_peers(peer_ids, inv).await
}

/// Announce inventory to specific peers (legacy method)
pub async fn announce_to_peers(peers: Vec<String>, inv: Inv) -> Result<(), String> {
    for peer in peers {
        // Check what we've already announced to this peer and filter
        let new_items: Vec<InventoryItem> = {
            let mut announced = ANNOUNCED_TO_PEER.lock();
            let peer_set = announced.entry(peer.clone()).or_default();

            // Filter out items already announced
            let filtered: Vec<InventoryItem> = inv
                .objects
                .iter()
                .filter(|item| !peer_set.contains(&item.hash))
                .cloned()
                .collect();

            // Mark as announced
            for item in &filtered {
                peer_set.insert(item.hash.clone());
            }

            filtered
        }; // Mutex guard dropped here

        if new_items.is_empty() {
            continue;
        }

        // Send INV (after dropping the mutex)
        let peer_inv = Inv { objects: new_items };
        send_inv_to_peer(&peer, peer_inv).await?;
    }

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

fn check_have_block(_hash: &str) -> bool {
    // This is called from async context, so we do best-effort sync check
    // In production, this should be async or use try_lock
    false // Conservative: always request if unsure
}

async fn check_have_tx_async(hash: &str) -> bool {
    let g = crate::CHAIN.lock();
    g.seen_txs.contains_key(&hash.to_string())
}

fn check_have_tx(_hash: &str) -> bool {
    // Conservative: always request if unsure (proper async version above)
    false
}

fn get_block_by_hash(hash: &str) -> Option<crate::Block> {
    // Enterprise block lookup from chain storage
    let g = crate::CHAIN.lock();
    g.blocks.iter().find(|b| b.header.pow_hash == hash).cloned()
}

async fn get_tx_from_mempool_async(hash: &str) -> Option<crate::Tx> {
    let g = crate::CHAIN.lock();

    // Check critical mempool - iterate through VecDeque
    for tx in g.mempool_critical.iter() {
        let tx_hash = hex::encode(crate::tx_hash(tx));
        if tx_hash == hash {
            return Some(tx.clone());
        }
    }

    // Check bulk mempool
    for tx in g.mempool_bulk.iter() {
        let tx_hash = hex::encode(crate::tx_hash(tx));
        if tx_hash == hash {
            return Some(tx.clone());
        }
    }

    None
}

fn get_tx_from_mempool(_hash: &str) -> Option<crate::Tx> {
    // Sync wrapper - not ideal, but works for now
    // In production, make handle_getdata fully async
    None
}

async fn send_getdata_to_peer(peer: &str, getdata: GetData) {
    use crate::PROM_TX_GETDATA_SENT;

    let url = format!("{}/p2p/getdata", peer.trim_end_matches('/'));

    // Track metrics for tx GETDATA
    if getdata
        .objects
        .iter()
        .any(|item| matches!(item.inv_type, InvType::Tx))
    {
        PROM_TX_GETDATA_SENT.inc();
    }

    let _ = crate::HTTP
        .post(&url)
        .json(&getdata)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
}

async fn send_block_to_peer(peer: &str, block: crate::Block) {
    let url = format!("{}/p2p/block", peer.trim_end_matches('/'));
    let _ = crate::HTTP
        .post(&url)
        .json(&block)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
}

async fn send_tx_to_peer(peer: &str, tx: crate::Tx) {
    let url = format!("{}/p2p/tx", peer.trim_end_matches('/'));
    let _ = crate::HTTP
        .post(&url)
        .json(&tx)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
}

async fn send_inv_to_peer(peer: &str, inv: Inv) -> Result<(), String> {
    let url = format!("{}/p2p/inv", peer.trim_end_matches('/'));
    crate::HTTP
        .post(&url)
        .json(&inv)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|e| format!("Failed to send INV: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv_creation() {
        let inv = Inv::block("abc123".to_string());
        assert_eq!(inv.objects.len(), 1);
        assert_eq!(inv.objects[0].inv_type, InvType::Block);
    }
}
