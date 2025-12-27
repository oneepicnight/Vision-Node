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

pub use block_builder::BlockBuilder;
pub use difficulty::{DifficultyConfig, DifficultyTracker};
pub use submit::{BlockSubmitter, MiningStats, SubmitResult};

/// Block found by local miner with full header and PoW digest
/// The header contains ALL fields needed for validation (state_root, tx_root, etc.)
#[derive(Clone, Debug)]
pub struct FoundPowBlock {
    pub header: crate::BlockHeader, // Full BlockHeader from main chain
    pub digest: crate::pow::U256,
    pub nonce: u64,
}
