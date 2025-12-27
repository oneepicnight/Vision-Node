//! Block builder for VisionX PoW mining
//!
//! Creates PowJob from pre-computed message bytes (from pow_message_bytes).
//! The miner no longer constructs headers - it receives frozen message bytes
//! that include ALL header fields (parent_hash, state_root, tx_root, etc.)

use crate::pow::{visionx::PowJob, U256};
use serde::{Deserialize, Serialize};

/// Simplified transaction for block building
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub nonce: u64,
    pub signature: Vec<u8>,
}

impl Transaction {
    /// Calculate transaction hash
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(self.from.as_bytes());
        data.extend_from_slice(self.to.as_bytes());
        data.extend_from_slice(&self.amount.to_be_bytes());
        data.extend_from_slice(&self.nonce.to_be_bytes());
        data.extend_from_slice(&self.signature);
        blake3::hash(&data).into()
    }
}

/// Calculate Merkle root of transactions
pub fn calculate_merkle_root(transactions: &[Transaction]) -> [u8; 32] {
    if transactions.is_empty() {
        return [0u8; 32];
    }

    let mut hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.hash()).collect();

    while hashes.len() > 1 {
        let mut next_level = Vec::new();

        for chunk in hashes.chunks(2) {
            let mut data = Vec::new();
            data.extend_from_slice(&chunk[0]);
            if chunk.len() > 1 {
                data.extend_from_slice(&chunk[1]);
            } else {
                data.extend_from_slice(&chunk[0]); // Duplicate last hash if odd
            }
            next_level.push(blake3::hash(&data).into());
        }

        hashes = next_level;
    }

    hashes[0]
}

/// Block builder for creating PowJobs from pre-computed message bytes
pub struct BlockBuilder;

impl BlockBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Create PowJob from pre-computed message bytes
    /// The message bytes come from pow_message_bytes() and include all header fields
    /// except nonce (which is what we're searching for)
    pub fn create_pow_job(
        &self,
        message_bytes: Vec<u8>,
        height: u64,
        prev_hash: [u8; 32],
        target: U256,
    ) -> PowJob {
        PowJob {
            header: message_bytes,
            nonce_offset: 0, // Not used - nonce passed separately to visionx_hash
            target,
            prev_hash32: prev_hash,
            height,
        }
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_root_single_tx() {
        let tx = Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            nonce: 1,
            signature: vec![1, 2, 3],
        };

        let root = calculate_merkle_root(&[tx.clone()]);
        assert_eq!(root, tx.hash());
    }

    #[test]
    fn test_merkle_root_multiple_tx() {
        let txs = vec![
            Transaction {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 100,
                nonce: 1,
                signature: vec![1, 2, 3],
            },
            Transaction {
                from: "bob".to_string(),
                to: "charlie".to_string(),
                amount: 50,
                nonce: 2,
                signature: vec![4, 5, 6],
            },
        ];

        let root = calculate_merkle_root(&txs);
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_block_builder() {
        let builder = BlockBuilder::new();
        let prev_hash = [1u8; 32];
        let txs = vec![Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            nonce: 1,
            signature: vec![1, 2, 3],
        }];

        let block = builder.build_block(1, prev_hash, 10000, txs.clone());

        assert_eq!(block.header.height, 1);
        assert_eq!(block.header.prev_hash, prev_hash);
        assert_eq!(block.header.difficulty, 10000);
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn test_pow_job_creation() {
        let builder = BlockBuilder::new();
        let prev_hash = [2u8; 32];
        let block = builder.build_block(2, prev_hash, 5000, vec![]);

        let target = crate::pow::u256_from_difficulty(5000);
        let job = builder.create_pow_job(&block, target);

        assert_eq!(job.height, 2);
        assert_eq!(job.prev_hash32, prev_hash);
        assert_eq!(job.target, target);
        assert_eq!(job.nonce_offset, 60);
    }

    #[test]
    fn test_block_finalization() {
        let builder = BlockBuilder::new();
        let block = builder.build_block(3, [3u8; 32], 1000, vec![]);

        let finalized = builder.finalize_block(block.clone(), 123456);

        assert_eq!(finalized.header.nonce, 123456);
        assert_eq!(finalized.header.height, block.header.height);
    }

    #[test]
    fn test_header_hash() {
        let header = BlockHeader {
            version: 1,
            height: 1,
            prev_hash: [0u8; 32],
            timestamp: 1000,
            difficulty: 5000,
            nonce: 12345,
            transactions_root: [1u8; 32],
        };

        let hash1 = header.hash();
        let hash2 = header.hash();

        // Same header should produce same hash
        assert_eq!(hash1, hash2);

        // Hash should not be zero
        assert_ne!(hash1, [0u8; 32]);
    }
}
