//! Mining performance tracking and statistics
//!
//! Tracks hashrate, per-miner stats, and provides leaderboard functionality.

use std::collections::{HashMap, VecDeque};
use std::time::{Instant, Duration, SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use parking_lot::RwLock;

/// Rolling window for hashrate calculation
pub struct HashrateTracker {
    samples: VecDeque<(Instant, u64)>,
    window: Duration,
    total_hashes: u64,
}

impl HashrateTracker {
    pub fn new(window_secs: u64) -> Self {
        Self {
            samples: VecDeque::with_capacity(120),
            window: Duration::from_secs(window_secs),
            total_hashes: 0,
        }
    }
    
    /// Record hashes computed
    pub fn record(&mut self, hashes: u64) {
        let now = Instant::now();
        self.total_hashes += hashes;
        self.samples.push_back((now, hashes));
        
        // Clean old samples outside window
        let cutoff = now - self.window;
        while let Some(&(ts, _)) = self.samples.front() {
            if ts < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }
    
    /// Get current hashrate (hashes per second)
    pub fn hashrate(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        
        let first = self.samples.front().unwrap();
        let last = self.samples.back().unwrap();
        let elapsed = last.0.duration_since(first.0).as_secs_f64();
        
        if elapsed < 0.001 {
            return 0.0;
        }
        
        let total: u64 = self.samples.iter().map(|(_, h)| h).sum();
        total as f64 / elapsed
    }
    
    /// Get total hashes computed lifetime
    pub fn total_hashes(&self) -> u64 {
        self.total_hashes
    }
}

/// Per-miner statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MinerStats {
    /// Miner address (coinbase/reward address)
    pub address: String,
    
    /// Total blocks found by this miner
    pub blocks_found: u64,
    
    /// Height of last block found
    pub last_block_height: u64,
    
    /// Last seen timestamp (unix seconds)
    pub last_seen_ts: u64,
    
    /// Share of last 100 blocks
    pub share_last_100: f64,
    
    /// Pool suspicion flag (>50% of last 100 blocks)
    pub suspected_pool: bool,
}

/// Mining leaderboard and pool detection
pub struct MiningLeaderboard {
    miners: RwLock<HashMap<String, MinerStats>>,
    recent_blocks: RwLock<VecDeque<(String, u64)>>, // (miner_address, height)
    recent_window: usize, // How many recent blocks to track
}

impl MiningLeaderboard {
    pub fn new(recent_window: usize) -> Self {
        Self {
            miners: RwLock::new(HashMap::new()),
            recent_blocks: RwLock::new(VecDeque::with_capacity(recent_window)),
            recent_window,
        }
    }
    
    /// Record a block found by a miner
    pub fn record_block(&self, miner_address: String, height: u64) {
        let now_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Update miner stats
        {
            let mut miners = self.miners.write();
            let stats = miners.entry(miner_address.clone()).or_insert(MinerStats {
                address: miner_address.clone(),
                blocks_found: 0,
                last_block_height: 0,
                last_seen_ts: 0,
                share_last_100: 0.0,
                suspected_pool: false,
            });
            
            stats.blocks_found += 1;
            stats.last_block_height = height;
            stats.last_seen_ts = now_ts;
        }
        
        // Update recent blocks
        {
            let mut recent = self.recent_blocks.write();
            recent.push_back((miner_address.clone(), height));
            
            // Keep only recent window
            while recent.len() > self.recent_window {
                recent.pop_front();
            }
        }
        
        // Recalculate shares and pool suspicion
        self.update_shares();
    }
    
    /// Update share percentages and pool detection
    fn update_shares(&self) {
        let recent = self.recent_blocks.read();
        if recent.is_empty() {
            return;
        }
        
        // Count blocks per miner in recent window
        let mut counts: HashMap<String, usize> = HashMap::new();
        for (addr, _) in recent.iter() {
            *counts.entry(addr.clone()).or_insert(0) += 1;
        }
        
        let total = recent.len() as f64;
        
        // Update shares
        let mut miners = self.miners.write();
        for (addr, count) in counts.iter() {
            if let Some(stats) = miners.get_mut(addr) {
                stats.share_last_100 = (*count as f64) / total;
                stats.suspected_pool = stats.share_last_100 > 0.5;
            }
        }
    }
    
    /// Get top N miners by blocks found
    pub fn get_leaderboard(&self, limit: usize) -> Vec<MinerStats> {
        let miners = self.miners.read();
        let mut entries: Vec<MinerStats> = miners.values().cloned().collect();
        
        // Sort by blocks_found descending
        entries.sort_by(|a, b| b.blocks_found.cmp(&a.blocks_found));
        
        entries.into_iter().take(limit).collect()
    }
    
    /// Get stats for a specific miner
    pub fn get_miner_stats(&self, address: &str) -> Option<MinerStats> {
        self.miners.read().get(address).cloned()
    }
    
    /// Get all miners
    pub fn get_all_miners(&self) -> Vec<MinerStats> {
        self.miners.read().values().cloned().collect()
    }
}

/// Mining status for API endpoints
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningStatus {
    /// Number of threads
    pub threads: usize,
    
    /// Current hashrate (H/s)
    pub hashrate_hps: f64,
    
    /// Current epoch
    pub epoch: u64,
    
    /// Current difficulty
    pub difficulty: u64,
    
    /// High-core profile enabled
    pub high_core_profile: bool,
    
    /// Total hashes computed
    pub total_hashes: u64,
    
    /// Mining enabled
    pub enabled: bool,
    
    /// Current block height being mined
    pub current_height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hashrate_tracker() {
        let mut tracker = HashrateTracker::new(30);
        
        tracker.record(1000);
        std::thread::sleep(Duration::from_millis(100));
        tracker.record(1000);
        
        let rate = tracker.hashrate();
        assert!(rate > 0.0);
        assert_eq!(tracker.total_hashes(), 2000);
    }
    
    #[test]
    fn test_mining_leaderboard() {
        let lb = MiningLeaderboard::new(100);
        
        lb.record_block("miner1".to_string(), 1);
        lb.record_block("miner1".to_string(), 2);
        lb.record_block("miner2".to_string(), 3);
        
        let top = lb.get_leaderboard(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].address, "miner1");
        assert_eq!(top[0].blocks_found, 2);
    }
    
    #[test]
    fn test_pool_detection() {
        let lb = MiningLeaderboard::new(10);
        
        // Single miner finds 6 out of 10 blocks
        for i in 0..6 {
            lb.record_block("pool".to_string(), i);
        }
        for i in 6..10 {
            lb.record_block("solo".to_string(), i);
        }
        
        let stats = lb.get_miner_stats("pool").unwrap();
        assert!(stats.suspected_pool);
        assert!(stats.share_last_100 > 0.5);
    }
}
