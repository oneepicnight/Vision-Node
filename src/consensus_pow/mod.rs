//! Consensus PoW module for VisionX blockchain
//!
//! Provides block building, difficulty adjustment, and submission handling
//! for the VisionX proof-of-work consensus mechanism.
//!
//! Note: Renamed to consensus_pow to avoid conflict with existing consensus.rs
#![allow(dead_code)]

pub mod block_builder;
pub mod difficulty;
pub mod submit;
pub mod encoding;
pub mod params;

pub use block_builder::BlockBuilder;
pub use difficulty::{DifficultyConfig, DifficultyTracker};
pub use submit::{BlockSubmitter, MiningStats, SubmitResult};
pub use encoding::pow_message_bytes;
pub use params::{VISIONX_CONSENSUS_PARAMS, consensus_params_to_visionx};

/// Deterministic, fork-independent epoch seed derivation
/// 
/// This function ensures all nodes use the SAME dataset for a given epoch,
/// regardless of forks or which blocks they've seen. The seed is derived from:
/// - chain_id: Network identifier (prevents cross-chain replay)
/// - genesis_pow_hash: Genesis block's pow_hash (unique per chain)
/// - epoch: Epoch number (0, 1, 2, ...)
///
/// This prevents the catastrophic failure mode where nodes fork and then
/// can't validate each other's blocks because they're using different datasets.
pub fn visionx_epoch_seed(chain_id: &str, genesis_pow_hash: [u8; 32], epoch: u64) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut h = Sha256::new();
    h.update(b"VISIONX_EPOCH_SEED_V1");
    h.update(chain_id.as_bytes());
    h.update(genesis_pow_hash);
    h.update(epoch.to_le_bytes());
    let out = h.finalize();

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&out[..32]);
    seed
}

/// Block found by local miner with full header and PoW digest
/// The header contains ALL fields needed for validation (state_root, tx_root, etc.)
#[derive(Clone, Debug)]
pub struct FoundPowBlock {
    pub header: crate::BlockHeader, // Full BlockHeader from main chain
    pub digest: crate::pow::U256,
    pub nonce: u64,
}
