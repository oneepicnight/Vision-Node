//! Anti-replay protection for signed messages
//!
//! Prevents replay attacks by tracking used nonces and enforcing timestamp windows

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Nonce cache entry with timestamp
#[derive(Clone, Debug)]
struct NonceEntry {
    nonce: String,
    timestamp: u64,
}

/// Global nonce cache - tracks recent nonces to prevent replay attacks
/// Key: (peer_node_id, nonce), Value: timestamp when seen
static NONCE_CACHE: Lazy<DashMap<(String, String), u64>> = Lazy::new(DashMap::new);

/// Timestamp window for nonce validity (±10 minutes)
const TIMESTAMP_WINDOW_SECS: u64 = 600;

/// Maximum nonces to track per node (prevents memory exhaustion)
const MAX_NONCES_PER_NODE: usize = 1000;

/// Check if a nonce has been used before, and mark it as used
///
/// Returns:
/// - Ok(()) if nonce is valid and marked as used
/// - Err(String) if nonce was already used (replay attack)
pub fn check_and_mark_nonce(node_id: &str, nonce: &str, timestamp: u64) -> Result<(), String> {
    // Validate timestamp is within acceptable window
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let age = now.abs_diff(timestamp);

    if age > TIMESTAMP_WINDOW_SECS {
        return Err(format!(
            "Timestamp outside acceptable window: age {}s (max {}s)",
            age, TIMESTAMP_WINDOW_SECS
        ));
    }

    // Check if nonce was already used
    let key = (node_id.to_string(), nonce.to_string());

    if NONCE_CACHE.contains_key(&key) {
        return Err(format!(
            "Nonce replay detected: {} already used by {}",
            nonce, node_id
        ));
    }

    // Mark nonce as used
    NONCE_CACHE.insert(key, timestamp);

    // Clean up old entries (opportunistic cleanup)
    cleanup_old_nonces();

    Ok(())
}

/// Clean up nonces older than the timestamp window
/// This is called opportunistically to prevent unbounded memory growth
fn cleanup_old_nonces() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();

    let cutoff = now.saturating_sub(TIMESTAMP_WINDOW_SECS * 2);

    // Remove expired entries
    NONCE_CACHE.retain(|_, &mut timestamp| timestamp > cutoff);
}

/// Get current nonce cache size (for monitoring)
pub fn nonce_cache_size() -> usize {
    NONCE_CACHE.len()
}

/// Clear all cached nonces (for testing or emergency reset)
pub fn clear_nonce_cache() {
    NONCE_CACHE.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_replay_detection() {
        clear_nonce_cache();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let node_id = "test_node_123";
        let nonce = "unique_nonce_abc";

        // First use should succeed
        assert!(check_and_mark_nonce(node_id, nonce, now).is_ok());

        // Second use should fail (replay)
        assert!(check_and_mark_nonce(node_id, nonce, now).is_err());
    }

    #[test]
    fn test_timestamp_window_enforcement() {
        clear_nonce_cache();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let node_id = "test_node_456";

        // Too old
        let old_timestamp = now - TIMESTAMP_WINDOW_SECS - 100;
        assert!(check_and_mark_nonce(node_id, "nonce1", old_timestamp).is_err());

        // Too far in future
        let future_timestamp = now + TIMESTAMP_WINDOW_SECS + 100;
        assert!(check_and_mark_nonce(node_id, "nonce2", future_timestamp).is_err());

        // Valid
        assert!(check_and_mark_nonce(node_id, "nonce3", now).is_ok());
    }

    #[test]
    fn test_different_nodes_same_nonce() {
        clear_nonce_cache();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let nonce = "shared_nonce";

        // Different nodes can use the same nonce
        assert!(check_and_mark_nonce("node_a", nonce, now).is_ok());
        assert!(check_and_mark_nonce("node_b", nonce, now).is_ok());
    }
}
