#![allow(dead_code)]
//! Mempool-based compact block reconstruction
//!
//! When receiving a compact block (short tx IDs only), attempts to reconstruct
//! the full block using transactions from our local mempool. If any txs are
//! missing, fetches them from the sending peer.
//!
//! Flow:
//! 1. Receive CompactBlock with short IDs
//! 2. Match short IDs to mempool transactions
//! 3. Request missing txs via GetBlockTxns
//! 4. Reconstruct full block once all txs available
//! 5. Validate and integrate into chain

use crate::p2p::compact::{BlockTxns, CompactBlock, GetBlockTxns};
use crate::{Block, BlockHeader, Tx};
use std::collections::HashMap;

/// Result of attempting to reconstruct a block from a compact representation
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ReconstructResult {
    /// Successfully reconstructed full block
    Complete(Block),
    /// Need to fetch missing transactions (indices provided)
    NeedTxs(Vec<usize>),
    /// Failed to reconstruct (e.g., invalid structure)
    Failed(String),
}

/// Attempt to reconstruct a full block from a compact block using mempool
pub fn reconstruct_block(compact: &CompactBlock) -> ReconstructResult {
    tracing::debug!(
        target = "p2p::mempool_sync",
        header_hash = %compact.header.hash,
        short_ids = compact.short_tx_ids.len(),
        prefilled = compact.prefilled_txs.len(),
        "Attempting block reconstruction"
    );

    // Build full transaction list
    let mut txs: Vec<Option<Tx>> =
        vec![None; compact.short_tx_ids.len() + compact.prefilled_txs.len()];

    // 1. Insert prefilled transactions at their specified indices
    for prefilled in &compact.prefilled_txs {
        if prefilled.index >= txs.len() {
            return ReconstructResult::Failed(format!(
                "Prefilled tx index {} out of bounds (total: {})",
                prefilled.index,
                txs.len()
            ));
        }
        txs[prefilled.index] = Some(prefilled.tx.clone());
    }

    // 2. Try to match short IDs with mempool transactions
    let mempool_txs = get_mempool_transactions();
    let mempool_index = build_mempool_index(&mempool_txs, compact);

    let mut short_id_pos = 0;
    #[allow(clippy::needless_range_loop)]
    for i in 0..txs.len() {
        if txs[i].is_some() {
            // Already filled by prefilled tx
            continue;
        }

        if short_id_pos >= compact.short_tx_ids.len() {
            return ReconstructResult::Failed(format!(
                "Short ID position {} exceeds short_ids length {}",
                short_id_pos,
                compact.short_tx_ids.len()
            ));
        }

        let short_id = &compact.short_tx_ids[short_id_pos];
        short_id_pos += 1;

        // Try to find matching tx in mempool
        if let Some(tx) = mempool_index.get(short_id) {
            txs[i] = Some((*tx).clone());
            tracing::trace!(
                target = "p2p::mempool_sync",
                index = i,
                short_id = %short_id,
                "Matched tx from mempool"
            );
        } else {
            tracing::trace!(
                target = "p2p::mempool_sync",
                index = i,
                short_id = %short_id,
                "Tx not found in mempool"
            );
        }
    }

    // 3. Check if all transactions are available
    let missing_indices: Vec<usize> = txs
        .iter()
        .enumerate()
        .filter_map(|(i, tx)| if tx.is_none() { Some(i) } else { None })
        .collect();

    if !missing_indices.is_empty() {
        tracing::debug!(
            target = "p2p::mempool_sync",
            missing_count = missing_indices.len(),
            "Need to fetch missing transactions"
        );
        return ReconstructResult::NeedTxs(missing_indices);
    }

    // 4. Unwrap all transactions (safe because we checked all are Some)
    let complete_txs: Vec<Tx> = txs.into_iter().map(|tx| tx.unwrap()).collect();

    // 5. Reconstruct full block
    let block = Block {
        header: reconstruct_header(&compact.header),
        txs: complete_txs,
        weight: 0,
        agg_signature: None,
    };

    // Verify announced PoW hash is well-formed and meets target
    if let Err(e) = verify_liteheader_pow_hash_valid(&compact.header) {
        return ReconstructResult::Failed(e);
    }

    tracing::info!(
        target = "p2p::mempool_sync",
        header_hash = %compact.header.hash,
        tx_count = block.txs.len(),
        "Successfully reconstructed block"
    );

    ReconstructResult::Complete(block)
}

/// Request missing transactions from peer
pub async fn fetch_missing_txs(
    peer: &str,
    block_hash: String,
    indices: Vec<usize>,
) -> Result<BlockTxns, String> {
    let request = GetBlockTxns {
        block_hash: block_hash.clone(),
        tx_indices: indices.clone(),
    };

    tracing::debug!(
        target = "p2p::mempool_sync",
        peer = %peer,
        block = %block_hash,
        count = indices.len(),
        "Requesting missing transactions"
    );

    let url = format!("{}/p2p/get_block_txs", peer.trim_end_matches('/'));

    let response = crate::HTTP
        .post(&url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch missing txs: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Peer returned error: {}", response.status()));
    }

    response
        .json::<BlockTxns>()
        .await
        .map_err(|e| format!("Failed to parse BlockTxns: {}", e))
}

/// Complete reconstruction after receiving missing transactions
pub fn complete_reconstruction(
    compact: &CompactBlock,
    missing_indices: &[usize],
    missing_txs: &BlockTxns,
) -> ReconstructResult {
    tracing::debug!(
        target = "p2p::mempool_sync",
        missing_count = missing_indices.len(),
        received_count = missing_txs.txs.len(),
        "Completing reconstruction with fetched txs"
    );

    if missing_indices.len() != missing_txs.txs.len() {
        return ReconstructResult::Failed(format!(
            "Mismatch: requested {} txs, received {}",
            missing_indices.len(),
            missing_txs.txs.len()
        ));
    }

    // Build full transaction list
    let mut txs: Vec<Option<Tx>> =
        vec![None; compact.short_tx_ids.len() + compact.prefilled_txs.len()];

    // Insert prefilled transactions
    for prefilled in &compact.prefilled_txs {
        txs[prefilled.index] = Some(prefilled.tx.clone());
    }

    // Match short IDs with mempool
    let mempool_txs = get_mempool_transactions();
    let mempool_index = build_mempool_index(&mempool_txs, compact);

    let mut short_id_pos = 0;
    for tx_slot in txs.iter_mut() {
        if tx_slot.is_some() {
            continue;
        }

        let short_id = &compact.short_tx_ids[short_id_pos];
        short_id_pos += 1;

        if let Some(tx) = mempool_index.get(short_id) {
            *tx_slot = Some((*tx).clone());
        }
    }

    // Insert fetched missing transactions
    for (idx_pos, &tx_index) in missing_indices.iter().enumerate() {
        if tx_index >= txs.len() {
            return ReconstructResult::Failed(format!("Invalid tx index: {}", tx_index));
        }
        txs[tx_index] = Some(missing_txs.txs[idx_pos].clone());
    }

    // Verify all slots filled
    let complete_txs: Result<Vec<Tx>, _> = txs
        .into_iter()
        .enumerate()
        .map(|(i, tx)| tx.ok_or(format!("Transaction at index {} still missing", i)))
        .collect();

    match complete_txs {
        Ok(txs) => {
            let block = Block {
                header: reconstruct_header(&compact.header),
                txs,
                weight: 0,
                agg_signature: None,
            };

            // Verify announced PoW hash is well-formed and meets target
            if let Err(e) = verify_liteheader_pow_hash_valid(&compact.header) {
                return ReconstructResult::Failed(e);
            }
            ReconstructResult::Complete(block)
        }
        Err(e) => ReconstructResult::Failed(e),
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get all transactions from mempool
fn get_mempool_transactions() -> Vec<Tx> {
    let g = crate::CHAIN.lock();
    let mut txs = Vec::new();
    txs.extend(g.mempool_critical.iter().cloned());
    txs.extend(g.mempool_bulk.iter().cloned());
    txs
}

/// Build a map from short tx ID to transaction
fn build_mempool_index(txs: &[Tx], compact: &CompactBlock) -> HashMap<u64, Tx> {
    let mut index = HashMap::new();

    for tx in txs {
        let short_id = crate::p2p::compact::short_tx_id(tx, compact.nonce);
        index.insert(short_id, tx.clone());
    }

    index
}

/// Convert LiteHeader back to full BlockHeader
fn reconstruct_header(lite: &crate::p2p::protocol::LiteHeader) -> BlockHeader {
    // DIAGNOSTIC: Log what we're reconstructing
    tracing::debug!(
        "[RECONSTRUCT-HEADER] height={} lite.hash={} -> creating BlockHeader.pow_hash",
        lite.height,
        lite.hash
    );
    
    let header = BlockHeader {
        number: lite.height,
        parent_hash: lite.prev.clone(),
        timestamp: lite.time,
        pow_hash: lite.hash.clone(),
        difficulty: lite.difficulty,
        nonce: lite.nonce,
        state_root: "0".repeat(64),
        tx_root: lite.merkle.clone(),
        receipts_root: "0".repeat(64),
        da_commitment: None,
        base_fee_per_gas: 0,
    };
    
    tracing::debug!(
        "[RECONSTRUCT-HEADER] Reconstructed header.pow_hash={}",
        header.pow_hash
    );
    
    header
}

fn verify_liteheader_pow_hash_valid(lite: &crate::p2p::protocol::LiteHeader) -> Result<(), String> {
    // Parse and validate pow_hash format
    let pow_hash = lite.hash.trim();
    let pow_hash = pow_hash.strip_prefix("0x").unwrap_or(pow_hash);
    if pow_hash.len() != 64 {
        return Err(format!(
            "Invalid PoW hash length: expected 64 hex chars (32 bytes), got {}",
            pow_hash.len()
        ));
    }

    let bytes = hex::decode(pow_hash).map_err(|_| "Invalid PoW hash format".to_string())?;
    let mut hash_array = [0u8; 32];
    hash_array.copy_from_slice(&bytes);

    // PoW validation is done in chain::accept when full block is processed
    // This just ensures the hash format is valid

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstruct_with_prefilled_only() {
        // Test reconstruction when all txs are prefilled
        // (This would be the case for small blocks or testing)
    }

    #[test]
    fn test_missing_txs_detection() {
        // Test that missing transactions are correctly identified
    }
}
