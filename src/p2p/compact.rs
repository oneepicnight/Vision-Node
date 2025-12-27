//! Compact block system for Vision (inspired by Bitcoin BIP-152).
//! Reduces bandwidth by sending only block headers + short tx IDs.
//!
//! Benefits:
//! - ~90% bandwidth reduction for block propagation
//! - Faster block relay (header + IDs arrive first)
//! - Mempool reconstruction (peers already have most txs)

use super::protocol::LiteHeader;
use serde::{Deserialize, Serialize};
use siphasher::sip::SipHasher24;

/// Represents a compact block (header + short transaction identifiers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactBlock {
    /// Block header (pow_hash, height, difficulty, etc.)
    pub header: LiteHeader,
    /// Short transaction IDs (6 bytes each, SipHash-2-4)
    pub short_tx_ids: Vec<u64>,
    /// Prefilled transactions (coinbase + any missing from peer mempool)
    pub prefilled_txs: Vec<PrefilledTx>,
    /// Nonce for short ID computation
    pub nonce: u64,
}

/// A prefilled transaction with its index in the block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefilledTx {
    /// Index in block's transaction list
    pub index: usize,
    /// Full transaction data
    pub tx: crate::Tx,
}

impl CompactBlock {
    /// Create a compact block from a full block with a random nonce
    pub fn from_block(block: &crate::Block, nonce: u64) -> Self {
        // Generate short IDs for all transactions
        let short_tx_ids: Vec<u64> = block.txs.iter().map(|tx| short_tx_id(tx, nonce)).collect();

        // Prefill coinbase (always include first tx)
        let mut prefilled_txs = Vec::new();
        if !block.txs.is_empty() {
            prefilled_txs.push(PrefilledTx {
                index: 0,
                tx: block.txs[0].clone(),
            });
        }

        Self {
            header: LiteHeader::from_block(block),
            short_tx_ids,
            prefilled_txs,
            nonce,
        }
    }

    /// Create a compact block with an auto-generated nonce
    pub fn from_block_auto(block: &crate::Block) -> Self {
        let nonce = generate_nonce();
        Self::from_block(block, nonce)
    }

    /// Estimate bandwidth savings vs full block
    pub fn estimated_savings(&self, full_block_size: usize) -> f64 {
        let compact_size = 80 + // Header (approximate)
            8 + // Nonce
            (self.short_tx_ids.len() * 6) + // 6 bytes per short ID
            (self.prefilled_txs.len() * 200); // Rough tx size estimate

        let savings = 1.0 - (compact_size as f64 / full_block_size as f64);
        savings.max(0.0)
    }

    /// Calculate actual compact block size in bytes
    pub fn size_bytes(&self) -> usize {
        // Approximate serialized size:
        // - LiteHeader: ~80 bytes
        // - Nonce: 8 bytes
        // - Short IDs: 6 bytes each (we store as u64 but only use 48 bits)
        // - Prefilled txs: variable, estimate ~200 bytes per tx
        80 + 8 + (self.short_tx_ids.len() * 6) + (self.prefilled_txs.len() * 200)
    }
}

/// Generate a short transaction ID using SipHash-2-4
///
/// Follows BIP-152 specification:
/// - Use SipHash-2-4 keyed with (block_hash || nonce)
/// - Hash the transaction witness ID (tx hash)
/// - Return first 48 bits (6 bytes) as u64
pub fn short_tx_id(tx: &crate::Tx, nonce: u64) -> u64 {
    use std::hash::Hasher;

    // Get transaction hash
    let tx_hash = crate::tx_hash(tx);

    // Create SipHash-2-4 with nonce as key
    // In BIP-152, key is derived from block_hash || nonce
    // For simplicity, we use nonce directly as the key material
    let k0 = nonce;
    let k1 = nonce.wrapping_mul(0x9e3779b97f4a7c15u64); // Golden ratio for key expansion

    let mut hasher = SipHasher24::new_with_keys(k0, k1);
    hasher.write(&tx_hash);
    let hash = hasher.finish();

    // Return only the lower 48 bits (6 bytes)
    // This gives us ~281 trillion unique IDs per block
    hash & 0x0000_FFFF_FFFF_FFFF
}

/// Generate a random nonce for compact block creation
/// Uses current timestamp + randomness to ensure uniqueness
fn generate_nonce() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    // Mix with a pseudo-random value based on timestamp
    timestamp.wrapping_mul(0x9e3779b97f4a7c15u64) ^ (timestamp >> 32)
}

/// Request missing transactions for compact block reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlockTxns {
    pub block_hash: String,
    /// Indices of missing transactions
    pub tx_indices: Vec<usize>,
}

/// Response with missing transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTxns {
    pub block_hash: String,
    pub txs: Vec<crate::Tx>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_id_generation() {
        // Create a dummy transaction
        let tx = crate::Tx {
            nonce: 0,
            sender_pubkey: "alice".to_string(),
            access_list: vec![],
            module: "transfer".to_string(),
            method: "send".to_string(),
            args: vec![],
            tip: 1,
            fee_limit: 100,
            sig: "".to_string(),
            max_priority_fee_per_gas: 0,
            max_fee_per_gas: 0,
        };

        let nonce1 = 12345u64;
        let nonce2 = 67890u64;

        // Same tx + nonce should produce same short ID
        let id1a = short_tx_id(&tx, nonce1);
        let id1b = short_tx_id(&tx, nonce1);
        assert_eq!(
            id1a, id1b,
            "Same tx and nonce should produce identical short IDs"
        );

        // Different nonce should produce different short ID
        let id2 = short_tx_id(&tx, nonce2);
        assert_ne!(
            id1a, id2,
            "Different nonce should produce different short IDs"
        );

        // Verify short IDs are actually 48-bit (top 16 bits are zero)
        assert_eq!(id1a >> 48, 0, "Short ID should only use lower 48 bits");
        assert_eq!(id2 >> 48, 0, "Short ID should only use lower 48 bits");
    }

    #[test]
    fn test_compact_block_creation() {
        // Create a test block with transactions
        let block = crate::Block {
            header: crate::BlockHeader {
                parent_hash: "0000".to_string(),
                number: 1,
                timestamp: 1234567890,
                difficulty: 1000,
                nonce: 42,
                pow_hash: "abcd1234".to_string(),
                state_root: "".to_string(),
                tx_root: "".to_string(),
                receipts_root: "".to_string(),
                da_commitment: None,
                base_fee_per_gas: 0,
            },
            txs: vec![
                crate::Tx {
                    nonce: 0,
                    sender_pubkey: "miner".to_string(),
                    access_list: vec![],
                    module: "coinbase".to_string(),
                    method: "reward".to_string(),
                    args: vec![],
                    tip: 0,
                    fee_limit: 0,
                    sig: "".to_string(),
                    max_priority_fee_per_gas: 0,
                    max_fee_per_gas: 0,
                },
                crate::Tx {
                    nonce: 0,
                    sender_pubkey: "alice".to_string(),
                    access_list: vec![],
                    module: "transfer".to_string(),
                    method: "send".to_string(),
                    args: vec![],
                    tip: 1,
                    fee_limit: 100,
                    sig: "".to_string(),
                    max_priority_fee_per_gas: 0,
                    max_fee_per_gas: 0,
                },
                crate::Tx {
                    nonce: 0,
                    sender_pubkey: "bob".to_string(),
                    access_list: vec![],
                    module: "transfer".to_string(),
                    method: "send".to_string(),
                    args: vec![],
                    tip: 1,
                    fee_limit: 50,
                    sig: "".to_string(),
                    max_priority_fee_per_gas: 0,
                    max_fee_per_gas: 0,
                },
            ],
            weight: 0,
            agg_signature: None,
        };

        let compact = CompactBlock::from_block_auto(&block);

        // Should have short IDs for all txs
        assert_eq!(compact.short_tx_ids.len(), block.txs.len());

        // Should prefill coinbase (first tx)
        assert_eq!(compact.prefilled_txs.len(), 1);
        assert_eq!(compact.prefilled_txs[0].index, 0);

        // All short IDs should be 48-bit
        for short_id in &compact.short_tx_ids {
            assert_eq!(short_id >> 48, 0, "All short IDs should be 48-bit");
        }
    }

    #[test]
    fn test_bandwidth_savings() {
        // Create a block with several transactions
        let block = crate::Block {
            header: crate::BlockHeader {
                parent_hash: "0000".to_string(),
                number: 1,
                timestamp: 1234567890,
                difficulty: 1000,
                nonce: 42,
                pow_hash: "abcd1234".to_string(),
                state_root: "".to_string(),
                tx_root: "".to_string(),
                receipts_root: "".to_string(),
                da_commitment: None,
                base_fee_per_gas: 0,
            },
            txs: vec![
                crate::Tx {
                    nonce: 0,
                    sender_pubkey: "alice".to_string(),
                    access_list: vec![],
                    module: "transfer".to_string(),
                    method: "send".to_string(),
                    args: vec![],
                    tip: 1,
                    fee_limit: 100,
                    sig: "".to_string(),
                    max_priority_fee_per_gas: 0,
                    max_fee_per_gas: 0,
                };
                10
            ], // 10 identical transactions
            weight: 0,
            agg_signature: None,
        };

        let compact = CompactBlock::from_block_auto(&block);

        // Rough full block size estimate: header + txs
        let full_size = 200 + (block.txs.len() * 150); // ~1700 bytes
        let compact_size = compact.size_bytes();
        let savings = compact.estimated_savings(full_size);

        // Should save significant bandwidth (>50%)
        assert!(
            savings > 0.5,
            "Should save >50% bandwidth, got {:.2}%",
            savings * 100.0
        );
        assert!(
            compact_size < full_size,
            "Compact should be smaller than full block"
        );

        println!("Full block: ~{} bytes", full_size);
        println!("Compact block: ~{} bytes", compact_size);
        println!("Bandwidth savings: {:.2}%", savings * 100.0);
    }

    #[test]
    fn test_nonce_uniqueness() {
        // Generate multiple nonces and ensure they're different
        let nonce1 = generate_nonce();
        std::thread::sleep(std::time::Duration::from_nanos(100));
        let nonce2 = generate_nonce();

        assert_ne!(nonce1, nonce2, "Sequential nonces should be unique");
    }

    #[test]
    fn test_collision_resistance() {
        // Create multiple different transactions
        let txs: Vec<crate::Tx> = (0..100)
            .map(|i| crate::Tx {
                nonce: i,
                sender_pubkey: format!("user{}", i),
                access_list: vec![],
                module: "transfer".to_string(),
                method: "send".to_string(),
                args: vec![i as u8],
                tip: i as u64,
                fee_limit: 100,
                sig: "".to_string(),
                max_priority_fee_per_gas: 0,
                max_fee_per_gas: 0,
            })
            .collect();

        let nonce = 12345u64;
        let mut short_ids = std::collections::HashSet::new();

        // Generate short IDs for all txs
        for tx in &txs {
            let id = short_tx_id(tx, nonce);
            short_ids.insert(id);
        }

        // Should have no collisions (or very few with 48-bit space)
        let collision_rate = 1.0 - (short_ids.len() as f64 / txs.len() as f64);
        assert!(
            collision_rate < 0.01,
            "Collision rate should be <1%, got {:.2}%",
            collision_rate * 100.0
        );
    }
}
