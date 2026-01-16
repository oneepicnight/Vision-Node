//! Miner Performance Store
//!
//! Persistent storage for mining performance data per CPU model, profile, and configuration.
//! Tracks hashrate samples over time to enable intelligent auto-tuning.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Performance key identifying a specific mining configuration
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct PerfKey {
    pub cpu_model: String,
    pub profile: String,
    pub pow_algo: String, // Algorithm-specific learning (e.g., "vision-pow-v1")
    pub threads: usize,
    pub batch_size: u32,
}

/// Performance statistics for a specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfStats {
    pub sample_count: u64,
    pub avg_hashrate: f64,  // H/s (exponential moving average)
    pub best_hashrate: f64, // Best observed H/s
    pub last_update: i64,   // Unix timestamp
}

/// Persistent store for miner performance data
#[derive(Debug)]
pub struct MinerPerfStore {
    path: PathBuf,
    entries: HashMap<PerfKey, PerfStats>,
}

impl MinerPerfStore {
    /// Load performance store from disk or create new
    pub fn load(path: PathBuf) -> Result<Self> {
        let entries = if path.exists() {
            let content =
                fs::read_to_string(&path).context("Failed to read miner performance store")?;
            serde_json::from_str(&content).context("Failed to parse miner performance store")?
        } else {
            HashMap::new()
        };

        Ok(Self { path, entries })
    }

    /// Record a new hashrate sample for a configuration
    pub fn record_sample(&mut self, key: &PerfKey, hashrate_hs: f64, now_ts: i64) {
        let entry = self.entries.entry(key.clone()).or_insert(PerfStats {
            sample_count: 0,
            avg_hashrate: 0.0,
            best_hashrate: 0.0,
            last_update: now_ts,
        });

        entry.sample_count += 1;

        // Exponential moving average for stability
        let alpha = 0.2_f64;
        if entry.sample_count == 1 {
            entry.avg_hashrate = hashrate_hs;
        } else {
            entry.avg_hashrate = alpha * hashrate_hs + (1.0 - alpha) * entry.avg_hashrate;
        }

        // Track best observed hashrate
        if hashrate_hs > entry.best_hashrate {
            entry.best_hashrate = hashrate_hs;
        }

        entry.last_update = now_ts;
    }

    /// Find the best known configuration for a CPU model and profile
    pub fn best_for_cpu_and_profile(
        &self,
        cpu_model: &str,
        profile: &str,
    ) -> Option<(PerfKey, PerfStats)> {
        self.entries
            .iter()
            .filter(|(k, _)| k.cpu_model == cpu_model && k.profile == profile)
            .max_by(|(_, a), (_, b)| {
                a.best_hashrate
                    .partial_cmp(&b.best_hashrate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
    }

    /// Find the best known configuration for a CPU model, profile, and PoW algorithm
    pub fn best_for_cpu_profile_algo(
        &self,
        cpu_model: &str,
        profile: &str,
        pow_algo: &str,
    ) -> Option<(PerfKey, PerfStats)> {
        self.entries
            .iter()
            .filter(|(k, _)| {
                k.cpu_model == cpu_model && k.profile == profile && k.pow_algo == pow_algo
            })
            .max_by(|(_, a), (_, b)| {
                a.best_hashrate
                    .partial_cmp(&b.best_hashrate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
    }

    /// Get all configurations for a CPU model and profile, sorted by performance
    pub fn all_for_cpu_and_profile(
        &self,
        cpu_model: &str,
        profile: &str,
    ) -> Vec<(PerfKey, PerfStats)> {
        let mut results: Vec<_> = self
            .entries
            .iter()
            .filter(|(k, _)| k.cpu_model == cpu_model && k.profile == profile)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        results.sort_by(|a, b| {
            b.1.best_hashrate
                .partial_cmp(&a.1.best_hashrate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get all configurations for a CPU model, profile, and algorithm, sorted by performance
    pub fn all_for_cpu_profile_algo(
        &self,
        cpu_model: &str,
        profile: &str,
        pow_algo: &str,
    ) -> Vec<(PerfKey, PerfStats)> {
        let mut results: Vec<_> = self
            .entries
            .iter()
            .filter(|(k, _)| {
                k.cpu_model == cpu_model && k.profile == profile && k.pow_algo == pow_algo
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        results.sort_by(|a, b| {
            b.1.best_hashrate
                .partial_cmp(&a.1.best_hashrate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get stats for a specific configuration
    pub fn get(&self, key: &PerfKey) -> Option<&PerfStats> {
        self.entries.get(key)
    }

    /// Get number of configurations stored
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Save performance store to disk
    pub fn save(&self) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).context("Failed to create miner perf store directory")?;
        }

        let json = serde_json::to_string_pretty(&self.entries)
            .context("Failed to serialize miner performance store")?;

        fs::write(&self.path, json).context("Failed to write miner performance store")?;

        Ok(())
    }

    /// Clear old entries (older than days_to_keep)
    pub fn cleanup_old_entries(&mut self, days_to_keep: i64) {
        let cutoff = chrono::Utc::now().timestamp() - (days_to_keep * 86400);
        self.entries.retain(|_, stats| stats.last_update > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_store_record_sample() {
        let mut store = MinerPerfStore {
            path: PathBuf::from("test.json"),
            entries: HashMap::new(),
        };

        let key = PerfKey {
            cpu_model: "Test CPU".to_string(),
            profile: "balanced".to_string(),
            pow_algo: "vision-pow-v1".to_string(),
            threads: 8,
            batch_size: 4,
        };

        store.record_sample(&key, 1000.0, 12345);
        assert_eq!(store.entries.len(), 1);

        let stats = store.get(&key).unwrap();
        assert_eq!(stats.sample_count, 1);
        assert_eq!(stats.avg_hashrate, 1000.0);
        assert_eq!(stats.best_hashrate, 1000.0);

        store.record_sample(&key, 1200.0, 12346);
        let stats = store.get(&key).unwrap();
        assert_eq!(stats.sample_count, 2);
        assert_eq!(stats.best_hashrate, 1200.0);
    }

    #[test]
    fn test_best_for_cpu_and_profile() {
        let mut store = MinerPerfStore {
            path: PathBuf::from("test.json"),
            entries: HashMap::new(),
        };

        let key1 = PerfKey {
            cpu_model: "Test CPU".to_string(),
            profile: "balanced".to_string(),
            pow_algo: "vision-pow-v1".to_string(),
            threads: 8,
            batch_size: 4,
        };

        let key2 = PerfKey {
            cpu_model: "Test CPU".to_string(),
            profile: "balanced".to_string(),
            pow_algo: "vision-pow-v1".to_string(),
            threads: 16,
            batch_size: 8,
        };

        store.record_sample(&key1, 1000.0, 12345);
        store.record_sample(&key2, 1500.0, 12346);

        let best = store.best_for_cpu_and_profile("Test CPU", "balanced");
        assert!(best.is_some());
        let (best_key, best_stats) = best.unwrap();
        assert_eq!(best_key.threads, 16);
        assert_eq!(best_stats.best_hashrate, 1500.0);
    }
}
