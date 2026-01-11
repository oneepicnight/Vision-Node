#![allow(dead_code)]
//! Pool worker tracking

use serde::Serialize;
use std::time::Instant;

/// Represents a connected pool worker
#[derive(Clone, Debug, Serialize)]
pub struct PoolWorker {
    /// Unique worker identifier
    pub id: String,

    /// Worker's wallet address for payouts
    pub wallet_address: String,

    /// Optional worker name for display
    pub worker_name: Option<String>,

    /// Total valid shares submitted
    pub total_shares: u64,

    /// Last time worker submitted a share
    #[serde(skip)]
    pub last_seen: Instant,

    /// Reported hashrate (self-reported by worker)
    pub reported_hashrate: Option<u64>,

    /// Worker connected timestamp
    #[serde(skip)]
    pub connected_at: Instant,

    /// Number of invalid shares (for tracking/banning)
    pub invalid_shares: u64,
}

impl PoolWorker {
    pub fn new(id: String, wallet_address: String, worker_name: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            id,
            wallet_address,
            worker_name,
            total_shares: 0,
            last_seen: now,
            reported_hashrate: None,
            connected_at: now,
            invalid_shares: 0,
        }
    }

    /// Record a valid share
    pub fn record_share(&mut self, difficulty: u64) {
        self.total_shares += difficulty;
        self.last_seen = Instant::now();
    }

    /// Record an invalid share
    pub fn record_invalid_share(&mut self) {
        self.invalid_shares += 1;
        self.last_seen = Instant::now();
    }

    /// Update reported hashrate
    pub fn update_hashrate(&mut self, hashrate: u64) {
        self.reported_hashrate = Some(hashrate);
        self.last_seen = Instant::now();
    }

    /// Check if worker has been idle for too long
    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        self.last_seen.elapsed().as_secs() > timeout_secs
    }

    /// Get estimated payout for this worker based on shares
    pub fn estimated_payout(&self, total_shares: u64, total_reward: u128) -> u128 {
        if total_shares == 0 {
            return 0;
        }

        // Calculate proportional share

        (self.total_shares as u128 * total_reward) / total_shares as u128
    }
}
