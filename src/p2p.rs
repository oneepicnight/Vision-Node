//! p2p.rs — tiny rate-limit helper (per-peer URL key)
use std::collections::HashMap;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use once_cell::sync::Lazy;

static RL: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Returns true if the peer should be throttled (called too soon).
pub fn should_throttle(peer_key: &str, min_interval_ms: u64) -> bool {
    let now = Instant::now();
    let mut map = RL.lock();
    if let Some(last) = map.get(peer_key) {
        if now.duration_since(*last) < Duration::from_millis(min_interval_ms) {
            return true;
        }
    }
    map.insert(peer_key.to_string(), now);
    false
}
