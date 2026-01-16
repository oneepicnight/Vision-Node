//! Pool protocol messages and types

use serde::{Deserialize, Serialize};

/// Pool job sent to workers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolJob {
    /// Unique job identifier
    pub job_id: String,

    /// Block height being mined
    pub height: u64,

    /// Previous block hash
    pub prev_hash: String,

    /// Merkle root of transactions
    pub merkle_root: String,

    /// Target for full block
    pub target: String,

    /// Target for shares (easier than full block)
    pub share_target: String,

    /// Extra nonce range for this worker
    pub extra_nonce_start: u32,
    pub extra_nonce_end: u32,

    /// Difficulty
    pub difficulty: u64,
}

/// Share submission from worker
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareSubmission {
    /// Worker identifier
    pub worker_id: String,

    /// Worker's wallet address
    pub wallet_address: String,

    /// Job ID this share is for
    pub job_id: String,

    /// Nonce that produced this hash
    pub nonce: u64,

    /// Extra nonce used
    pub extra_nonce: u32,

    /// Resulting hash
    pub hash: String,

    /// Optional: worker's current hashrate
    pub hashrate: Option<u64>,
}

/// Worker registration request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationRequest {
    /// Worker identifier
    pub worker_id: String,

    /// Worker's wallet address for payouts
    pub wallet_address: String,

    /// Optional: worker name (for display)
    pub worker_name: Option<String>,

    /// Optional: worker software version
    pub version: Option<String>,
}

/// Registration response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub ok: bool,
    pub message: Option<String>,
    pub pool_fee_bps: u16,
    pub foundation_fee_bps: u16,
}

/// Share submission response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareResponse {
    pub ok: bool,
    pub message: Option<String>,
    pub is_block: bool,
    pub total_shares: u64,
    pub estimated_payout: Option<String>,
}

/// Pool stats response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolStatsResponse {
    pub worker_count: usize,
    pub total_shares: u64,
    pub total_hashrate: u64,
    pub blocks_found: u64,
    pub last_block_height: Option<u64>,
    pub workers: Vec<WorkerStats>,
    /// Pool name (visible to world)
    pub pool_name: String,
    /// Pool connection URL for joining miners
    pub pool_url: String,
    /// Pool port
    pub pool_port: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerStats {
    pub worker_id: String,
    pub worker_name: Option<String>,
    pub wallet_address: String,
    pub total_shares: u64,
    pub reported_hashrate: Option<u64>,
    pub estimated_payout: String,
}
