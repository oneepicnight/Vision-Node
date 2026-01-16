#![allow(dead_code)]

use crate::auto_sync::SyncHealthSnapshot;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MinerCfg {
    pub threads: usize,
    pub enabled: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MinerSpeed {
    pub current_hashrate: f64, // H/s
    pub average_hashrate: f64, // H/s over window
    pub history: Vec<f64>,     // Last 120 seconds
    pub threads: usize,
}

#[derive(Clone)]
pub struct MinerManager {
    inner: Arc<Mutex<MinerManagerInner>>,
}

struct MinerManagerInner {
    threads: usize,
    enabled: bool,
    hash_samples: VecDeque<(Instant, u64)>, // (timestamp, hash_count)
    window_duration: Duration,
}

impl MinerManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MinerManagerInner {
                threads: num_cpus::get().max(1),
                enabled: true,
                hash_samples: VecDeque::with_capacity(120),
                window_duration: Duration::from_secs(120),
            })),
        }
    }

    /// Set the number of active CPU worker threads
    pub fn set_threads(&self, threads: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.threads = threads.max(1).min(num_cpus::get() * 2);
    }

    /// Get the current number of threads
    pub fn get_threads(&self) -> usize {
        self.inner.lock().unwrap().threads
    }

    /// Set mining enabled/disabled
    pub fn set_enabled(&self, enabled: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.enabled = enabled;
    }

    /// Get mining enabled status
    pub fn is_enabled(&self) -> bool {
        self.inner.lock().unwrap().enabled
    }

    /// Get current configuration
    pub fn get_config(&self) -> MinerCfg {
        let inner = self.inner.lock().unwrap();
        MinerCfg {
            threads: inner.threads,
            enabled: inner.enabled,
        }
    }

    /// Record hashes computed (call this periodically from mining loop)
    pub fn record_hashes(&self, count: u64) {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();

        // Add new sample
        inner.hash_samples.push_back((now, count));

        // Remove samples older than window
        let cutoff = now - inner.window_duration;
        while let Some((ts, _)) = inner.hash_samples.front() {
            if *ts < cutoff {
                inner.hash_samples.pop_front();
            } else {
                break;
            }
        }
    }

    /// Check if node can start mining based on sync health
    pub fn can_start_mining(&self, snapshot: &SyncHealthSnapshot) -> bool {
        // Must not be actively syncing
        if snapshot.is_syncing {
            return false;
        }

        // Require at least 2 peers
        if snapshot.connected_peers < 2 {
            return false;
        }

        // Require chain ID match
        if !snapshot.chain_id_matches {
            return false;
        }

        // Require to be at or very near the tip
        if snapshot.sync_height + 1 < snapshot.network_estimated_height {
            return false;
        }

        true
    }

    /// Calculate statistics over the rolling window
    pub fn stats(&self) -> MinerSpeed {
        let inner = self.inner.lock().unwrap();
        let now = Instant::now();

        // Calculate total hashes in window
        let total_hashes: u64 = inner.hash_samples.iter().map(|(_, count)| count).sum();

        // Calculate time span
        let oldest = inner.hash_samples.front().map(|(ts, _)| *ts);
        let newest = inner.hash_samples.back().map(|(ts, _)| *ts);

        let (current_hashrate, average_hashrate) =
            if let (Some(oldest), Some(newest)) = (oldest, newest) {
                let duration_secs = newest.duration_since(oldest).as_secs_f64().max(1.0);
                let avg = total_hashes as f64 / duration_secs;

                // Current hashrate (last 5 seconds)
                let recent_cutoff = now - Duration::from_secs(5);
                let recent_hashes: u64 = inner
                    .hash_samples
                    .iter()
                    .filter(|(ts, _)| *ts >= recent_cutoff)
                    .map(|(_, count)| count)
                    .sum();
                let current = recent_hashes as f64 / 5.0;

                (current, avg)
            } else {
                (0.0, 0.0)
            };

        // Build history (last 120 seconds, 1 sample per second)
        let mut history = Vec::with_capacity(120);
        for i in (0..120).rev() {
            let bucket_start = now - Duration::from_secs(i + 1);
            let bucket_end = now - Duration::from_secs(i);

            let bucket_hashes: u64 = inner
                .hash_samples
                .iter()
                .filter(|(ts, _)| *ts >= bucket_start && *ts < bucket_end)
                .map(|(_, count)| count)
                .sum();

            history.push(bucket_hashes as f64);
        }

        MinerSpeed {
            current_hashrate,
            average_hashrate,
            history,
            threads: inner.threads,
        }
    }
}

impl Default for MinerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_miner_manager_threads() {
        let manager = MinerManager::new();

        // Test setting threads
        manager.set_threads(4);
        assert_eq!(manager.get_threads(), 4);

        // Test bounds
        manager.set_threads(0);
        assert_eq!(manager.get_threads(), 1); // Min 1
    }

    #[test]
    fn test_miner_manager_stats() {
        let manager = MinerManager::new();

        // Record some hashes
        manager.record_hashes(1000);
        thread::sleep(Duration::from_millis(100));
        manager.record_hashes(1000);

        let stats = manager.stats();
        assert!(stats.average_hashrate > 0.0);
        assert_eq!(stats.threads, manager.get_threads());
    }

    #[test]
    fn test_miner_manager_enable_disable() {
        let manager = MinerManager::new();

        assert!(manager.is_enabled());
        manager.set_enabled(false);
        assert!(!manager.is_enabled());
        manager.set_enabled(true);
        assert!(manager.is_enabled());
    }
}
