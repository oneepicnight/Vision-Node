#![allow(dead_code)]
//! Dial failure tracking for P2P debugging
//!
//! Tracks connection failures with reasons, timestamps, and sources
//! to help diagnose why peers aren't connecting.

use once_cell::sync::Lazy;
use std::collections::VecDeque;
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

/// Global dial failure tracker
pub static DIAL_TRACKER: Lazy<Arc<Mutex<DialTracker>>> =
    Lazy::new(|| Arc::new(Mutex::new(DialTracker::new())));

/// Dial failure tracker
pub struct DialTracker {
    failures: VecDeque<DialFailure>,
}

impl DialTracker {
    pub fn new() -> Self {
        Self {
            failures: VecDeque::with_capacity(MAX_DIAL_FAILURES),
        }
    }

    /// Record a dial failure
    pub fn record(&mut self, addr: String, reason: String, source: String) {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let failure = DialFailure {
            addr,
            reason,
            timestamp_unix,
            source,
        };

        self.failures.push_back(failure);

        // Keep only the most recent failures
        if self.failures.len() > MAX_DIAL_FAILURES {
            self.failures.pop_front();
        }
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

    /// Clear all failures (for testing)
    pub fn clear(&mut self) {
        self.failures.clear();
    }
}

/// Record a dial failure (convenience function)
pub fn record_dial_failure(addr: String, reason: String, source: String) {
    if let Ok(mut tracker) = DIAL_TRACKER.lock() {
        tracker.record(addr, reason, source);
    }
}

/// Get recent dial failures (convenience function)
pub fn get_dial_failures() -> Vec<DialFailure> {
    DIAL_TRACKER
        .lock()
        .ok()
        .map(|tracker| tracker.get_failures())
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
