//! consensus.rs — minimal PoW consensus helpers (demo-grade)
#![allow(dead_code)]
use crate::types::{blake3_hash, leading_zero_bits, work_from_hash};

/// Parameters for PoW target & timing.
#[derive(Clone, Debug)]
pub struct ConsensusParams {
    /// Required number of leading zero **bits** in block header hash.
    pub target_bits: u8,
    /// Retarget interval in blocks (not implemented in this demo, reserved).
    pub retarget_interval: u64,
    /// Target block time (seconds) for future retargeting (not used yet).
    pub target_block_time_secs: u64,
    /// Reject blocks whose timestamp is more than this far in the future.
    pub max_future_secs: u64,
    /// Require block.timestamp > median of last N (e.g., 11). Use 0 to disable.
    pub median_window: usize,
}

impl Default for ConsensusParams {
    fn default() -> Self {
        Self {
            target_bits: 16,            // ~= 2 zero bytes worth of difficulty
            retarget_interval: 100,     // reserved
            target_block_time_secs: 10, // reserved
            max_future_secs: 120,
            median_window: 11,
        }
    }
}

/// Return true if the hash meets target_bits.
pub fn meets_target(hash: &[u8; 32], target_bits: u8) -> bool {
    leading_zero_bits(hash) >= target_bits
}

/// Compute cumulative work for a chain: sum(2^leading_zero_bits(block_hash)).
pub fn accumulated_work(block_hashes: impl IntoIterator<Item = [u8; 32]>) -> u128 {
    block_hashes
        .into_iter()
        .map(|h| work_from_hash(&h))
        .fold(0u128, |a, w| a.saturating_add(w))
}

/// Calculate median of a slice of timestamps (seconds).
fn median_u64(ts: &[u64]) -> u64 {
    let mut v = ts.to_vec();
    v.sort_unstable();
    let n = v.len();
    if n == 0 {
        return 0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] / 2) + (v[n / 2] / 2)
    }
}

/// Validate header timestamp against future skew and median past time.
pub fn validate_time_rules(
    new_ts: u64,
    tip_ts: u64,
    recent_ts: &[u64],
    now_secs: u64,
    params: &ConsensusParams,
) -> Result<(), String> {
    if new_ts > now_secs.saturating_add(params.max_future_secs) {
        return Err("block timestamp too far in the future".into());
    }
    if params.median_window > 0 && !recent_ts.is_empty() {
        let med = median_u64(recent_ts);
        if new_ts <= med {
            return Err("block timestamp not greater than median of recent".into());
        }
    } else if new_ts <= tip_ts {
        return Err("block timestamp must be > tip timestamp".into());
    }
    Ok(())
}

/// Validate PoW: recompute header hash from a JSON-ish encoding function (provided by caller).
/// - `header_bytes`: serialization of the *header with pow_hash cleared* ("0..0").
/// - `claimed_pow_hex`: the pow_hash string claimed in the header.
pub fn validate_pow<F>(
    header_bytes: &[u8],
    claimed_pow_hex: &str,
    params: &ConsensusParams,
    meets: F,
) -> Result<[u8; 32], String>
where
    F: Fn(&[u8; 32], u8) -> bool,
{
    let h = blake3_hash(header_bytes);
    let claimed = hex::decode(claimed_pow_hex).map_err(|_| "bad pow_hash hex")?;
    if claimed.len() != 32 {
        return Err("pow_hash must be 32 bytes".into());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&claimed);
    // The simple demo: require our recomputed hash equals the claimed hash AND meets target bits.
    if h != arr {
        return Err("invalid PoW: header hash mismatch".into());
    }
    if !meets(&arr, params.target_bits) {
        return Err("PoW below target".into());
    }
    Ok(arr)
}
