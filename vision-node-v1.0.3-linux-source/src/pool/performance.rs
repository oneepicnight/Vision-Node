#![allow(dead_code)]
//! Performance enhancements for the pool system
//!
//! Includes:
//! - Rate limiting for share submissions
//! - Job caching to reduce block template rebuilds
//! - Worker banning logic for bad actors
//! - Performance metrics

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Rate limiter for share submissions
pub struct ShareRateLimiter {
    // worker_id -> (submit_count, window_start)
    workers: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    max_per_second: u32,
    window: Duration,
}

impl ShareRateLimiter {
    pub fn new(max_per_second: u32) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            max_per_second,
            window: Duration::from_secs(1),
        }
    }

    /// Check if worker can submit a share
    pub fn allow_share(&self, worker_id: &str) -> bool {
        let mut workers = self.workers.lock().unwrap();
        let now = Instant::now();

        let entry = workers.entry(worker_id.to_string()).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(entry.1) >= self.window {
            entry.0 = 0;
            entry.1 = now;
        }

        // Check rate limit
        if entry.0 >= self.max_per_second {
            return false;
        }

        entry.0 += 1;
        true
    }

    /// Cleanup stale entries
    pub fn cleanup(&self) {
        let mut workers = self.workers.lock().unwrap();
        let now = Instant::now();
        let stale_threshold = Duration::from_secs(60);

        workers.retain(|_, (_, last_seen)| now.duration_since(*last_seen) < stale_threshold);
    }
}

/// Job cache to avoid rebuilding block templates
pub struct JobCache {
    cached_job: Arc<Mutex<Option<(u64, crate::pool::protocol::PoolJob, Instant)>>>,
    cache_duration: Duration,
}

impl JobCache {
    pub fn new() -> Self {
        Self {
            cached_job: Arc::new(Mutex::new(None)),
            cache_duration: Duration::from_secs(30),
        }
    }

    /// Get cached job if still valid for this height
    pub fn get(&self, height: u64) -> Option<crate::pool::protocol::PoolJob> {
        let cache = self.cached_job.lock().unwrap();

        if let Some((cached_height, ref job, cached_at)) = *cache {
            // Check if cache is still valid
            if cached_height == height && cached_at.elapsed() < self.cache_duration {
                return Some(job.clone());
            }
        }

        None
    }

    /// Store job in cache
    pub fn set(&self, height: u64, job: crate::pool::protocol::PoolJob) {
        let mut cache = self.cached_job.lock().unwrap();
        *cache = Some((height, job, Instant::now()));
    }

    /// Invalidate cache (call when new block arrives)
    pub fn invalidate(&self) {
        let mut cache = self.cached_job.lock().unwrap();
        *cache = None;
    }
}

/// Worker ban manager for bad actors
pub struct WorkerBanManager {
    // worker_id -> BanRecord
    bans: Arc<Mutex<HashMap<String, BanRecord>>>,
    invalid_share_threshold: f64, // Ban if invalid ratio exceeds this (e.g., 0.1 = 10%)
    min_shares_for_ban: u64,      // Minimum total shares before considering ban
}

struct BanRecord {
    banned_until: Option<Instant>,
    invalid_shares: u64,
    valid_shares: u64,
}

impl WorkerBanManager {
    pub fn new(invalid_share_threshold: f64, min_shares_for_ban: u64) -> Self {
        Self {
            bans: Arc::new(Mutex::new(HashMap::new())),
            invalid_share_threshold,
            min_shares_for_ban,
        }
    }

    /// Record a valid share
    pub fn record_valid(&self, worker_id: &str) {
        let mut bans = self.bans.lock().unwrap();
        let record = bans.entry(worker_id.to_string()).or_insert(BanRecord {
            banned_until: None,
            invalid_shares: 0,
            valid_shares: 0,
        });

        record.valid_shares += 1;
    }

    /// Record an invalid share and check if ban needed
    pub fn record_invalid(&self, worker_id: &str) -> bool {
        let mut bans = self.bans.lock().unwrap();
        let record = bans.entry(worker_id.to_string()).or_insert(BanRecord {
            banned_until: None,
            invalid_shares: 0,
            valid_shares: 0,
        });

        record.invalid_shares += 1;

        // Check if should be banned
        let total_shares = record.valid_shares + record.invalid_shares;
        if total_shares >= self.min_shares_for_ban {
            let invalid_ratio = record.invalid_shares as f64 / total_shares as f64;
            if invalid_ratio > self.invalid_share_threshold {
                // Ban for 1 hour
                record.banned_until = Some(Instant::now() + Duration::from_secs(3600));
                tracing::warn!(
                    "🚫 Banned worker {} for high invalid share rate: {:.1}% ({}/{})",
                    worker_id,
                    invalid_ratio * 100.0,
                    record.invalid_shares,
                    total_shares
                );
                return true;
            }
        }

        false
    }

    /// Check if worker is currently banned
    pub fn is_banned(&self, worker_id: &str) -> bool {
        let bans = self.bans.lock().unwrap();

        if let Some(record) = bans.get(worker_id) {
            if let Some(banned_until) = record.banned_until {
                return Instant::now() < banned_until;
            }
        }

        false
    }

    /// Cleanup expired bans
    pub fn cleanup_expired(&self) {
        let mut bans = self.bans.lock().unwrap();
        let now = Instant::now();

        for record in bans.values_mut() {
            if let Some(banned_until) = record.banned_until {
                if now >= banned_until {
                    record.banned_until = None;
                    // Reset counters
                    record.valid_shares = 0;
                    record.invalid_shares = 0;
                }
            }
        }
    }
}

/// Pool performance metrics
#[derive(Debug, Clone)]
pub struct PoolMetrics {
    pub total_shares_received: u64,
    pub invalid_shares_received: u64,
    pub blocks_found: u64,
    pub total_payouts: u128,
    pub avg_job_response_time_ms: f64,
    pub avg_share_response_time_ms: f64,
    pub current_hashrate: u64,
}

impl PoolMetrics {
    pub fn new() -> Self {
        Self {
            total_shares_received: 0,
            invalid_shares_received: 0,
            blocks_found: 0,
            total_payouts: 0,
            avg_job_response_time_ms: 0.0,
            avg_share_response_time_ms: 0.0,
            current_hashrate: 0,
        }
    }

    pub fn invalid_share_rate(&self) -> f64 {
        let total = self.total_shares_received + self.invalid_shares_received;
        if total == 0 {
            0.0
        } else {
            self.invalid_shares_received as f64 / total as f64
        }
    }
}

/// Global performance components
pub static SHARE_RATE_LIMITER: Lazy<ShareRateLimiter> = Lazy::new(|| ShareRateLimiter::new(100)); // Max 100 shares/sec per worker

pub static JOB_CACHE: Lazy<JobCache> = Lazy::new(JobCache::new);

pub static BAN_MANAGER: Lazy<WorkerBanManager> = Lazy::new(|| WorkerBanManager::new(0.1, 100)); // Ban at 10% invalid, min 100 shares

pub static POOL_METRICS: Lazy<Mutex<PoolMetrics>> = Lazy::new(|| Mutex::new(PoolMetrics::new()));

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_rate_limiter() {
        let limiter = ShareRateLimiter::new(10);

        // Should allow first 10
        for _ in 0..10 {
            assert!(limiter.allow_share("worker1"));
        }

        // Should reject 11th
        assert!(!limiter.allow_share("worker1"));

        // After window expires, should allow again
        thread::sleep(Duration::from_secs(2));
        assert!(limiter.allow_share("worker1"));
    }

    #[test]
    fn test_job_cache() {
        let cache = JobCache::new();

        // Cache miss
        assert!(cache.get(100).is_none());

        // Store and retrieve
        let job = crate::pool::protocol::PoolJob {
            job_id: "test".to_string(),
            height: 100,
            prev_hash: "0x00".to_string(),
            merkle_root: "0x00".to_string(),
            target: "0xff".to_string(),
            share_target: "0xff".to_string(),
            extra_nonce_start: 0,
            extra_nonce_end: 1000,
            difficulty: 1000,
        };

        cache.set(100, job.clone());
        assert!(cache.get(100).is_some());

        // Wrong height = cache miss
        assert!(cache.get(101).is_none());

        // Invalidate
        cache.invalidate();
        assert!(cache.get(100).is_none());
    }

    #[test]
    fn test_ban_manager() {
        let manager = WorkerBanManager::new(0.2, 10); // 20% threshold, 10 min shares

        // Record 8 valid, 2 invalid = 20% (at threshold)
        for _ in 0..8 {
            manager.record_valid("worker1");
        }

        assert!(!manager.is_banned("worker1"));

        manager.record_invalid("worker1"); // 11% invalid
        assert!(!manager.is_banned("worker1"));

        manager.record_invalid("worker1"); // 20% invalid
                                           // Should trigger ban
        let banned = manager.record_invalid("worker1"); // 27% invalid
        assert!(banned);
        assert!(manager.is_banned("worker1"));
    }
}
