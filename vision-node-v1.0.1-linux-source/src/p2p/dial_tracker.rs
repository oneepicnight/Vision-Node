#![allow(dead_code)]
//! Dial failure tracking for P2P debugging
//!
//! Tracks connection failures with reasons, timestamps, and sources
//! to help diagnose why peers aren't connecting.
//!
//! PRODUCTION BACKOFF SYSTEM:
//! - Per-peer failure tracking with escalating cooldowns
//! - Seed hygiene: prevent spam-dialing reliable nodes
//! - Smart backoff: connection_refused vs timeout have different schedules
//! - Quarantine: temporary ban for repeat offenders

use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Maximum number of dial failures to keep in memory
const MAX_DIAL_FAILURES: usize = 100;

/// A single dial failure event
#[derive(Debug, Clone)]
pub struct DialFailure {
    pub addr: String,
    pub reason: String,
    pub timestamp_unix: u64,
    pub source: String, // "seed", "gossip", "handshake", "bootstrap", "manual"
}

/// Per-peer backoff tracking
#[derive(Debug, Clone)]
pub struct DialBackoff {
    pub fail_streak: u32,        // Consecutive failures (resets on success)
    pub cooldown_until: u64,     // Unix timestamp when we can try again
    pub last_fail_reason: String, // "connection_refused", "timeout", etc.
    pub last_attempt_at: u64,    // Unix timestamp of last dial attempt
    pub last_success_at: u64,    // Unix timestamp of last successful connection
    pub total_attempts: u32,     // Total dial attempts (all time)
    pub total_successes: u32,    // Total successful connections (all time)
}

impl DialBackoff {
    pub fn new() -> Self {
        Self {
            fail_streak: 0,
            cooldown_until: 0,
            last_fail_reason: String::new(),
            last_attempt_at: 0,
            last_success_at: 0,
            total_attempts: 0,
            total_successes: 0,
        }
    }

    /// Check if this peer is currently in cooldown
    pub fn is_in_cooldown(&self, now: u64) -> bool {
        now < self.cooldown_until
    }

    /// Get time remaining in cooldown (seconds)
    pub fn cooldown_remaining(&self, now: u64) -> u64 {
        if now < self.cooldown_until {
            self.cooldown_until - now
        } else {
            0
        }
    }
}

/// Global dial failure tracker
pub static DIAL_TRACKER: Lazy<Arc<Mutex<DialTracker>>> =
    Lazy::new(|| Arc::new(Mutex::new(DialTracker::new())));

/// Dial failure tracker with comprehensive backoff
pub struct DialTracker {
    failures: VecDeque<DialFailure>,
    backoff: HashMap<String, DialBackoff>, // Key: socket address
}

impl DialTracker {
    pub fn new() -> Self {
        Self {
            failures: VecDeque::with_capacity(MAX_DIAL_FAILURES),
            backoff: HashMap::new(),
        }
    }

    /// Record a dial failure with smart backoff
    pub fn record(&mut self, addr: String, reason: String, source: String) {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let failure = DialFailure {
            addr: addr.clone(),
            reason: reason.clone(),
            timestamp_unix,
            source: source.clone(),
        };

        self.failures.push_back(failure);

        // Keep only the most recent failures
        if self.failures.len() > MAX_DIAL_FAILURES {
            self.failures.pop_front();
        }

        // Update backoff for this peer
        let backoff = self.backoff.entry(addr.clone()).or_insert_with(DialBackoff::new);
        backoff.fail_streak += 1;
        backoff.last_fail_reason = reason.clone();
        backoff.last_attempt_at = timestamp_unix;
        backoff.total_attempts += 1;

        // Calculate cooldown based on failure type and streak
        let cooldown_duration = calculate_cooldown(&reason, backoff.fail_streak, &source);
        backoff.cooldown_until = timestamp_unix + cooldown_duration;

        tracing::debug!(
            "[BACKOFF] {} failed (streak: {}, cooldown: {}s): {}",
            addr,
            backoff.fail_streak,
            cooldown_duration,
            reason
        );
    }

    /// Record a successful connection (resets backoff)
    pub fn record_success(&mut self, addr: String) {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backoff = self.backoff.entry(addr.clone()).or_insert_with(DialBackoff::new);
        
        // Reset failure tracking on success
        let had_failures = backoff.fail_streak > 0;
        backoff.fail_streak = 0;
        backoff.cooldown_until = 0;
        backoff.last_success_at = timestamp_unix;
        backoff.total_successes += 1;

        if had_failures {
            tracing::info!(
                "[BACKOFF] {} connected successfully! Backoff cleared (total: {}/{})",
                addr,
                backoff.total_successes,
                backoff.total_attempts
            );
        }
    }

    /// Check if a peer is in cooldown
    pub fn is_in_cooldown(&self, addr: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.backoff
            .get(addr)
            .map(|b| b.is_in_cooldown(now))
            .unwrap_or(false)
    }

    /// Get backoff info for a peer
    pub fn get_backoff(&self, addr: &str) -> Option<DialBackoff> {
        self.backoff.get(addr).cloned()
    }

    /// Check if peer should be quarantined (long-term failure)
    pub fn should_quarantine(&self, addr: &str) -> bool {
        if let Some(backoff) = self.backoff.get(addr) {
            // Quarantine if: never succeeded AND 3+ failures
            if backoff.total_successes == 0 && backoff.total_attempts >= 3 {
                return true;
            }
            
            // Quarantine if: fail streak >= 5
            if backoff.fail_streak >= 5 {
                return true;
            }
        }
        false
    }

    /// Get all recorded failures (most recent first)
    pub fn get_failures(&self) -> Vec<DialFailure> {
        self.failures.iter().rev().cloned().collect()
    }

    /// Get failures for a specific address
    pub fn get_failures_for_addr(&self, addr: &str) -> Vec<DialFailure> {
        self.failures
            .iter()
            .filter(|f| f.addr == addr)
            .rev()
            .cloned()
            .collect()
    }

    /// Get backoff statistics summary
    pub fn get_backoff_summary(&self) -> HashMap<String, DialBackoff> {
        self.backoff.clone()
    }

    /// Clear all failures (for testing)
    pub fn clear(&mut self) {
        self.failures.clear();
        self.backoff.clear();
    }
}

/// Calculate cooldown duration based on failure type and streak
/// 
/// EXPONENTIAL BACKOFF SCHEDULE:
/// 
/// Connection refused (host alive, port closed):
/// - 1st fail → 60s
/// - 2nd fail → 5 min
/// - 3rd fail → 30 min
/// - 4th+ fail → 1-6 hours (capped)
///
/// Timeout (firewall/NAT/unreachable):
/// - 1st fail → 30s
/// - 2nd fail → 2 min
/// - 3rd fail → 10 min
/// - 4th+ fail → 30 min - 1 hour (capped)
///
/// Handshake reject (wrong chain/version):
/// - Immediate 6 hour cooldown (essentially banned)
///
/// Seeds get 2x longer cooldowns (they should be reliable)
fn calculate_cooldown(reason: &str, fail_streak: u32, source: &str) -> u64 {
    let is_seed = source == "seed";
    let reason_lower = reason.to_lowercase();
    
    // Check failure type
    let is_refused = reason_lower.contains("refused") 
        || reason_lower.contains("connection refused")
        || reason_lower.contains("actively refused");

    let is_timeout = reason_lower.contains("timeout")
        || reason_lower.contains("timed out");
    
    let is_handshake_reject = reason_lower.contains("handshake")
        || reason_lower.contains("version")
        || reason_lower.contains("chain")
        || reason_lower.contains("banned");

    // Handshake reject = essentially banned (wrong network/version)
    if is_handshake_reject {
        return if is_seed { 21600 } else { 21600 }; // 6 hours
    }

    // Connection refused = host alive, port closed (exponential backoff)
    // FIX B: Heavy penalty for connection_refused - these peers are not accepting inbound
    if is_refused {
        match fail_streak {
            1 => if is_seed { 3600 } else { 3600 },    // 1st: 1hr (both) - HEAVY PENALTY
            2 => if is_seed { 7200 } else { 3600 },    // 2nd: 2hr (seed) / 1hr (peer)
            3 => if is_seed { 14400 } else { 7200 },   // 3rd: 4hr (seed) / 2hr (peer)
            4 => if is_seed { 21600 } else { 14400 },  // 4th: 6hr (seed) / 4hr (peer)
            _ => if is_seed { 86400 } else { 43200 },  // 5th+: 24hr (seed) / 12hr (peer) - ESSENTIALLY DEAD
        }
    } else if is_timeout {
        // Timeout = maybe firewall/NAT/slow handshake (SHORT backoff)
        // 5s dial timeout is too short for home NAT + busy routers + cold sockets
        // Most timeouts are legit but slow peers, not dead ones
        match fail_streak {
            1 => if is_seed { 20 } else { 10 },        // 1st: 20s (seed) / 10s (peer)
            2 => if is_seed { 60 } else { 30 },        // 2nd: 1min (seed) / 30s (peer)
            3 => if is_seed { 120 } else { 60 },       // 3rd: 2min (seed) / 60s (peer)
            4 => if is_seed { 240 } else { 120 },      // 4th: 4min (seed) / 2min (peer) - CAPPED
            _ => if is_seed { 240 } else { 120 },      // 5th+: 4min (seed) / 2min (peer) - CAPPED
        }
    } else {
        // Unknown error type - conservative exponential backoff
        match fail_streak {
            1 => 60,       // 1 min
            2 => 300,      // 5 min
            3 => 900,      // 15 min
            4 => 1800,     // 30 min
            _ => 3600,     // 1 hour - CAPPED
        }
    }
}

/// Record a dial failure (convenience function)
pub fn record_dial_failure(addr: String, reason: String, source: String) {
    if let Ok(mut tracker) = DIAL_TRACKER.lock() {
        tracker.record(addr, reason, source);
    }
}

/// Record a successful connection (convenience function)
pub fn record_dial_success(addr: String) {
    if let Ok(mut tracker) = DIAL_TRACKER.lock() {
        tracker.record_success(addr);
    }
}

/// Check if a peer is in cooldown (convenience function)
pub fn is_peer_in_cooldown(addr: &str) -> bool {
    DIAL_TRACKER
        .lock()
        .ok()
        .map(|tracker| tracker.is_in_cooldown(addr))
        .unwrap_or(false)
}

/// Get backoff info for a peer (convenience function)
pub fn get_peer_backoff(addr: &str) -> Option<DialBackoff> {
    DIAL_TRACKER
        .lock()
        .ok()
        .and_then(|tracker| tracker.get_backoff(addr))
}

/// Check if peer should be quarantined (convenience function)
pub fn should_quarantine_peer(addr: &str) -> bool {
    DIAL_TRACKER
        .lock()
        .ok()
        .map(|tracker| tracker.should_quarantine(addr))
        .unwrap_or(false)
}

/// Get recent dial failures (convenience function)
pub fn get_dial_failures() -> Vec<DialFailure> {
    DIAL_TRACKER
        .lock()
        .ok()
        .map(|tracker| tracker.get_failures())
        .unwrap_or_default()
}

/// Get backoff summary (convenience function)
pub fn get_backoff_summary() -> HashMap<String, DialBackoff> {
    DIAL_TRACKER
        .lock()
        .ok()
        .map(|tracker| tracker.get_backoff_summary())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dial_tracker_basic() {
        let mut tracker = DialTracker::new();

        tracker.record(
            "1.2.3.4:7072".to_string(),
            "connection_refused".to_string(),
            "seed".to_string(),
        );

        let failures = tracker.get_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].addr, "1.2.3.4:7072");
        assert_eq!(failures[0].reason, "connection_refused");
        assert_eq!(failures[0].source, "seed");
    }

    #[test]
    fn test_dial_tracker_limit() {
        let mut tracker = DialTracker::new();

        // Add more than MAX_DIAL_FAILURES
        for i in 0..150 {
            tracker.record(
                format!("1.2.3.{}:7072", i % 255),
                "timeout".to_string(),
                "gossip".to_string(),
            );
        }

        let failures = tracker.get_failures();
        assert_eq!(failures.len(), MAX_DIAL_FAILURES);
    }

    #[test]
    fn test_get_failures_for_addr() {
        let mut tracker = DialTracker::new();

        tracker.record(
            "1.2.3.4:7072".to_string(),
            "timeout".to_string(),
            "seed".to_string(),
        );
        tracker.record(
            "1.2.3.5:7072".to_string(),
            "refused".to_string(),
            "gossip".to_string(),
        );
        tracker.record(
            "1.2.3.4:7072".to_string(),
            "banned".to_string(),
            "handshake".to_string(),
        );

        let failures = tracker.get_failures_for_addr("1.2.3.4:7072");
        assert_eq!(failures.len(), 2);
    }
}
