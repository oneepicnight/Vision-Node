#![allow(dead_code)]
//! Pool state management

use super::worker::PoolWorker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Pool configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Pool fee in basis points (e.g., 150 = 1.5%)
    pub pool_fee_bps: u16,

    /// Vision Foundation fee in basis points (e.g., 100 = 1%)
    pub foundation_fee_bps: u16,

    /// Foundation address for fee payments
    pub foundation_address: String,

    /// Pool host's wallet address (receives pool fee + mining share)
    pub host_address: String,

    /// Pool name (shown to world)
    pub pool_name: String,

    /// Pool server port (7072 or 8082 for pool operations)
    pub pool_port: u16,

    /// Worker timeout in seconds (inactive workers get pruned)
    pub worker_timeout_secs: u64,

    /// Share difficulty multiplier (relative to network difficulty)
    pub share_difficulty_divisor: u64,

    /// Mining mode (for persistence across restarts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mining_mode: Option<super::MiningMode>,

    /// Worker name (for joiners - displayed in host's worker list)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_name: Option<String>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            pool_fee_bps: 150,       // 1.5% pool fee
            foundation_fee_bps: 100, // 1% foundation fee
            foundation_address: crate::vision_constants::vault_address(),
            host_address: String::new(),
            pool_name: "Unnamed Pool".to_string(),
            pool_port: 7072,              // Default pool port (7072 or 8082)
            worker_timeout_secs: 300,     // 5 minutes
            share_difficulty_divisor: 10, // Shares are 10x easier than full blocks
            mining_mode: None,            // Will be set when mode is configured
            worker_name: None,            // Worker name for pool joiners
        }
    }
}

/// Pool state tracking all workers and shares
pub struct PoolState {
    /// Pool configuration
    pub config: PoolConfig,

    /// Current mining job ID
    pub active_job_id: Option<String>,

    /// Connected workers
    workers: Arc<Mutex<HashMap<String, PoolWorker>>>,

    /// Total shares across all workers for current job
    total_shares: Arc<Mutex<u64>>,

    /// Block statistics
    blocks_found: Arc<Mutex<u64>>,
    last_block_height: Arc<Mutex<Option<u64>>>,
}

impl PoolState {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            config,
            active_job_id: None,
            workers: Arc::new(Mutex::new(HashMap::new())),
            total_shares: Arc::new(Mutex::new(0)),
            blocks_found: Arc::new(Mutex::new(0)),
            last_block_height: Arc::new(Mutex::new(None)),
        }
    }

    /// Register a new worker
    pub fn register_worker(
        &self,
        worker_id: String,
        wallet_address: String,
        worker_name: Option<String>,
    ) -> Result<(), String> {
        let mut workers = self.workers.lock().unwrap();

        if workers.contains_key(&worker_id) {
            return Err(format!("Worker {} already registered", worker_id));
        }

        let worker = PoolWorker::new(worker_id.clone(), wallet_address, worker_name);
        workers.insert(worker_id, worker);

        Ok(())
    }

    /// Record a valid share from a worker
    pub fn record_share(&self, worker_id: &str, difficulty: u64) -> Result<(), String> {
        let mut workers = self.workers.lock().unwrap();

        let worker = workers
            .get_mut(worker_id)
            .ok_or_else(|| format!("Worker {} not registered", worker_id))?;

        worker.record_share(difficulty);

        let mut total = self.total_shares.lock().unwrap();
        *total += difficulty;

        Ok(())
    }

    /// Record an invalid share (for tracking bad workers)
    pub fn record_invalid_share(&self, worker_id: &str) -> Result<(), String> {
        let mut workers = self.workers.lock().unwrap();

        let worker = workers
            .get_mut(worker_id)
            .ok_or_else(|| format!("Worker {} not registered", worker_id))?;

        worker.record_invalid_share();

        Ok(())
    }

    /// Update worker's reported hashrate
    pub fn update_worker_hashrate(&self, worker_id: &str, hashrate: u64) -> Result<(), String> {
        let mut workers = self.workers.lock().unwrap();

        let worker = workers
            .get_mut(worker_id)
            .ok_or_else(|| format!("Worker {} not registered", worker_id))?;

        worker.update_hashrate(hashrate);

        Ok(())
    }

    /// Reset shares after a block is found (start fresh for next block)
    pub fn reset_shares_after_block(&self, block_height: u64) {
        let mut workers = self.workers.lock().unwrap();

        // Reset all worker shares
        for worker in workers.values_mut() {
            worker.total_shares = 0;
        }

        // Reset total
        *self.total_shares.lock().unwrap() = 0;

        // Update block stats
        *self.blocks_found.lock().unwrap() += 1;
        *self.last_block_height.lock().unwrap() = Some(block_height);
    }

    /// Get all workers (for display/monitoring)
    pub fn get_workers(&self) -> Vec<PoolWorker> {
        let workers = self.workers.lock().unwrap();
        workers.values().cloned().collect()
    }

    /// Get total shares
    pub fn get_total_shares(&self) -> u64 {
        *self.total_shares.lock().unwrap()
    }

    /// Prune stale workers (haven't submitted shares recently)
    pub fn prune_stale_workers(&self) -> usize {
        let mut workers = self.workers.lock().unwrap();
        let timeout = self.config.worker_timeout_secs;

        let before_count = workers.len();
        workers.retain(|_, worker| !worker.is_stale(timeout));
        let after_count = workers.len();

        before_count - after_count
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let workers = self.workers.lock().unwrap();
        let total_shares = *self.total_shares.lock().unwrap();
        let blocks_found = *self.blocks_found.lock().unwrap();
        let last_block_height = *self.last_block_height.lock().unwrap();

        let total_hashrate: u64 = workers.values().filter_map(|w| w.reported_hashrate).sum();

        PoolStats {
            worker_count: workers.len(),
            total_shares,
            total_hashrate,
            blocks_found,
            last_block_height,
        }
    }
}

/// Pool statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolStats {
    pub worker_count: usize,
    pub total_shares: u64,
    pub total_hashrate: u64,
    pub blocks_found: u64,
    pub last_block_height: Option<u64>,
}
