#![allow(dead_code)]
//! Exponential Backoff Logic for P2P Connection Retries
//!
//! Implements progressive retry delays to prevent aggressive reconnection attempts
//! while allowing eventual recovery from transient network issues.
//!
//! Backoff Schedule:
//! - Attempt 0: 5 seconds
//! - Attempt 1: 15 seconds
//! - Attempt 2: 30 seconds
//! - Attempt 3: 60 seconds (1 minute)
//! - Attempt 4: 120 seconds (2 minutes)
//! - Attempt 5: 300 seconds (5 minutes)
//! - Attempt 6+: 900 seconds (15 minutes) - capped

/// Calculate backoff delay in seconds based on attempt count
///
/// Returns the number of seconds to wait before the next retry attempt.
/// Uses exponential backoff with a maximum cap of 15 minutes.
///
/// # Arguments
/// * `attempts` - Number of failed connection attempts (0-indexed)
///
/// # Returns
/// Number of seconds to wait before next retry
///
/// # Examples
/// ```
/// assert_eq!(backoff(0), 5);   // First retry: 5 seconds
/// assert_eq!(backoff(1), 15);  // Second retry: 15 seconds
/// assert_eq!(backoff(5), 300); // Sixth retry: 5 minutes
/// assert_eq!(backoff(10), 900); // 10th+ retry: 15 minutes (capped)
/// ```
pub fn backoff(attempts: u32) -> u64 {
    match attempts {
        0 => 5,   // 5 seconds
        1 => 15,  // 15 seconds
        2 => 30,  // 30 seconds
        3 => 60,  // 1 minute
        4 => 120, // 2 minutes
        5 => 300, // 5 minutes
        _ => 900, // 15 minutes (cap)
    }
}

/// Get current Unix timestamp in seconds
pub fn current_time() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_schedule() {
        assert_eq!(backoff(0), 5);
        assert_eq!(backoff(1), 15);
        assert_eq!(backoff(2), 30);
        assert_eq!(backoff(3), 60);
        assert_eq!(backoff(4), 120);
        assert_eq!(backoff(5), 300);
        assert_eq!(backoff(6), 900);
        assert_eq!(backoff(10), 900);
        assert_eq!(backoff(100), 900);
    }

    #[test]
    fn test_current_time() {
        let now = current_time();
        assert!(now > 1_600_000_000); // After 2020
    }
}
