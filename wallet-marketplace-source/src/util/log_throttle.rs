use log::warn;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// In-process cache of last-seen warning keys ⇒ last time logged.
static LAST: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Only warn once per `period` for the same `key`.
pub fn warn_throttled(key: impl Into<String>, period: Duration, msg: impl AsRef<str>) {
    let key = key.into();
    let now = Instant::now();
    let mut map = LAST.lock();
    match map.get(&key) {
        Some(&prev) if now.duration_since(prev) < period => {
            // within throttle window: skip
        }
        _ => {
            map.insert(key, now);
            warn!("{}", msg.as_ref());
        }
    }
}
