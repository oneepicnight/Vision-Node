#![allow(dead_code)]
//! Block Synchronization Logic
//!
//! Handles headers-first sync with windowed block fetching

use std::collections::{HashSet, VecDeque};
use std::time::Instant;

/// Build block locator for headers sync
/// Returns: [tip, tip-1, tip-2, tip-4, tip-8, tip-16, ..., genesis]
pub fn build_block_locator(chain_blocks: &[crate::Block]) -> Vec<String> {
    let mut locator = Vec::new();
    let height = chain_blocks.len().saturating_sub(1);

    if height == 0 {
        // Only genesis
        if let Some(genesis) = chain_blocks.first() {
            locator.push(genesis.header.pow_hash.clone());
        }
        return locator;
    }

    // Start with tip
    let mut step = 1;
    let mut h = height;

    loop {
        if h < chain_blocks.len() {
            locator.push(chain_blocks[h].header.pow_hash.clone());
        }

        if h == 0 {
            break;
        }

        // Exponential backoff
        if locator.len() >= 10 {
            step *= 2;
        }

        h = h.saturating_sub(step);
    }

    // Always include genesis
    if let Some(genesis) = chain_blocks.first() {
        let genesis_hash = genesis.header.pow_hash.clone();
        if !locator.contains(&genesis_hash) {
            locator.push(genesis_hash);
        }
    }

    locator
}

/// Download queue for windowed block fetching
#[derive(Debug)]
pub struct DownloadQueue {
    /// Blocks we want to download
    pub want: VecDeque<String>,
    /// Blocks currently being fetched
    pub inflight: HashSet<String>,
    /// Maximum inflight requests (window size)
    pub window: usize,
}

impl DownloadQueue {
    pub fn new(window: usize) -> Self {
        Self {
            want: VecDeque::new(),
            inflight: HashSet::new(),
            window,
        }
    }

    /// Add blocks to download queue
    pub fn enqueue(&mut self, hashes: Vec<String>) {
        for hash in hashes {
            if !self.inflight.contains(&hash) {
                self.want.push_back(hash);
            }
        }
    }

    /// Get next batch to request (fills window)
    pub fn next_batch(&mut self) -> Vec<String> {
        let available = self.window.saturating_sub(self.inflight.len());
        let mut batch = Vec::new();

        for _ in 0..available {
            if let Some(hash) = self.want.pop_front() {
                self.inflight.insert(hash.clone());
                batch.push(hash);
            } else {
                break;
            }
        }

        batch
    }

    /// Mark block as received
    pub fn mark_received(&mut self, hash: &str) {
        self.inflight.remove(hash);
    }

    /// Check if download is complete
    pub fn is_complete(&self) -> bool {
        self.want.is_empty() && self.inflight.is_empty()
    }
}

/// Peer state for adaptive sync
#[derive(Debug, Clone)]
pub struct PeerState {
    pub peer_url: String,
    /// Number of inflight block requests
    pub inflight_blocks: usize,
    /// Exponentially weighted moving average RTT (ms)
    pub rtt_ewma: f64,
    /// Last activity timestamp
    pub last_seen: Instant,
    /// If set, peer is paused until this time
    pub paused_until: Option<Instant>,
    /// Consecutive failure count
    pub failures: u32,
    /// Current window size (adaptive)
    pub window: usize,
}

impl PeerState {
    pub fn new(peer_url: String) -> Self {
        Self {
            peer_url,
            inflight_blocks: 0,
            rtt_ewma: 100.0, // Start with 100ms estimate
            last_seen: Instant::now(),
            paused_until: None,
            failures: 0,
            window: 12, // Default window size
        }
    }

    /// Update RTT with exponential smoothing (alpha = 0.2)
    pub fn update_rtt(&mut self, rtt_ms: f64) {
        self.rtt_ewma = 0.8 * self.rtt_ewma + 0.2 * rtt_ms;
        self.last_seen = Instant::now();
        self.failures = 0;

        // Adapt window based on new RTT
        self.adapt_window();
    }

    /// Record a failure
    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.last_seen = Instant::now();

        // Halve window on failure
        self.window = (self.window / 2).max(4);

        // Pause peer after 3 failures
        if self.failures >= 3 {
            self.paused_until = Some(Instant::now() + std::time::Duration::from_secs(30));
            eprintln!(
                "⏸️ Pausing peer {} for 30s after {} failures",
                self.peer_url, self.failures
            );
        }
    }

    /// Check if peer is paused
    pub fn is_paused(&self) -> bool {
        if let Some(until) = self.paused_until {
            if Instant::now() < until {
                return true;
            }
        }
        false
    }

    /// Adapt window based on RTT (4..32 range)
    /// Formula: window = min(32, max(4, round(2 * 1000ms / rtt_ewma)))
    pub fn adapt_window(&mut self) {
        // Adaptive window sizing based on RTT
        // Target: keep ~2x RTT worth of blocks in flight
        let target_window = if self.rtt_ewma > 0.0 {
            let calculated = (2.0 * 1000.0 / self.rtt_ewma).round() as usize;
            calculated.clamp(4, 32)
        } else {
            12 // Default if no RTT data yet
        };

        // Gradually adjust towards target (prevents oscillation)
        if self.window < target_window {
            self.window = (self.window + 2).min(target_window).min(32);
        } else if self.window > target_window {
            self.window = (self.window.saturating_sub(1)).max(target_window).max(4);
        }
    }

    /// Get recommended timeout for this peer (ms)
    pub fn timeout_ms(&self) -> u64 {
        // Use 3x RTT or minimum timeout
        let adaptive = (self.rtt_ewma * 3.0) as u64;
        adaptive.max(3000) // At least 3 seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_queue() {
        let mut queue = DownloadQueue::new(8);

        queue.enqueue(vec!["h1".to_string(), "h2".to_string(), "h3".to_string()]);
        assert_eq!(queue.want.len(), 3);

        let batch = queue.next_batch();
        assert_eq!(batch.len(), 3);
        assert_eq!(queue.inflight.len(), 3);

        queue.mark_received("h1");
        assert_eq!(queue.inflight.len(), 2);
    }

    #[test]
    fn test_peer_state_rtt() {
        let mut peer = PeerState::new("http://peer1".to_string());
        assert_eq!(peer.rtt_ewma, 100.0);

        peer.update_rtt(50.0);
        assert!(peer.rtt_ewma < 100.0);
        assert!(peer.rtt_ewma > 50.0);
    }
}
