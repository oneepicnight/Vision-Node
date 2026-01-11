//! Canonical consensus parameters for VisionX PoW
//!
//! This is the single source of truth for consensus parameters.
//! All miners and validators MUST use these exact params or the chain will fork.

use crate::consensus::ConsensusParams;
use crate::pow::visionx::VisionXParams;

/// Canonical VisionX consensus parameters
/// Used by all PoW validation paths (accept.rs, miner, pool)
pub static VISIONX_CONSENSUS_PARAMS: ConsensusParams = ConsensusParams {
    target_bits: 16,            // ~= 2 zero bytes worth of difficulty
    retarget_interval: 100,     // reserved for future retargeting
    target_block_time_secs: 10, // reserved for future retargeting
    max_future_secs: 120,       // reject blocks > 120 secs in the future
    median_window: 11,          // use median of last 11 timestamps
};

/// Convert ConsensusParams to VisionX hashing params used by validators/miners
/// This is the CANONICAL source of VisionX parameters.
/// Both miners and validators MUST use this function to avoid parameter drift.
pub fn consensus_params_to_visionx(_params: &ConsensusParams) -> VisionXParams {
    // CRITICAL: These params affect the PoW digest.
    // If miner and validator differ, blocks will be rejected (pow_hash mismatch).
    // Using VisionXParams::default() which is the mainnet security target.
    VisionXParams::default()
}
