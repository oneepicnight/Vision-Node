#![allow(dead_code)]
//! Vision Node Mining Pool Implementation
//!
//! Enables miners to host pools and workers to join pools for collaborative mining.
//! Pool hosts distribute work, track shares, and automatically split block rewards.

pub mod payouts;
pub mod performance;
pub mod protocol;
pub mod routes;
pub mod state;
pub mod worker;
pub mod worker_client;

pub use payouts::compute_pool_payouts;
pub use performance::{BAN_MANAGER, JOB_CACHE, POOL_METRICS, SHARE_RATE_LIMITER};
pub use state::{PoolConfig, PoolState};

/// Mining mode for the node
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum MiningMode {
    /// Solo mining (default)
    #[default]
    Solo,
    /// Hosting a mining pool
    HostPool,
    /// Joined as a worker to a pool
    JoinPool,
}

impl MiningMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MiningMode::Solo => "solo",
            MiningMode::HostPool => "host_pool",
            MiningMode::JoinPool => "join_pool",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "solo" => Some(MiningMode::Solo),
            "host" | "host_pool" => Some(MiningMode::HostPool),
            "join" | "join_pool" | "worker" => Some(MiningMode::JoinPool),
            _ => None,
        }
    }
}
