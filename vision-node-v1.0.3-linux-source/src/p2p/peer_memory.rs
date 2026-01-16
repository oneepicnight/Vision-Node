//! Constellation Memory Layer (CML)
//!
//! Provides persistent memory of all peers the node has ever connected to,
//! enabling autonomous recovery and self-healing after network disruptions.

use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Memory record for a single peer in the constellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationPeerMemory {
    /// Unique peer identifier
    pub peer_id: String,

    /// Eternal Broadcast ID - stable across restarts
    pub ebid: String,

    /// Last known IP address
    pub last_ip: String,

    /// Last known port (P2P)
    pub last_port: u16,

    /// HTTP API port (for compact block fallback)
    #[serde(default)]
    pub http_api_port: Option<u16>,

    /// Unix timestamp of last successful connection
    pub last_seen: u64,

    /// Whether this peer is eligible to become guardian
    pub is_guardian_candidate: bool,

    /// Uptime reliability score (0.0 - 1.0)
    pub uptime_score: f64,

    /// Total successful connections
    pub connection_count: u64,

    /// Failed connection attempts (recent window)
    pub failed_attempts: u32,

    /// Whether this peer has been promoted to anchor status (3+ successful connections)
    pub is_anchor: bool,

    /// Unix timestamp of last connection failure (for cooldown)
    #[serde(default)]
    pub last_fail_at: Option<u64>,

    /// Consecutive failure count (resets on success)
    #[serde(default)]
    pub fail_count: u32,
}

impl ConstellationPeerMemory {
    /// Create a new peer memory from handshake data
    pub fn from_handshake(
        peer_id: String,
        ebid: String,
        ip: String,
        port: u16,
        http_api_port: Option<u16>,
        now: u64,
        is_guardian: bool,
    ) -> Self {
        Self {
            peer_id,
            ebid,
            last_ip: ip,
            last_port: port,
            http_api_port,
            last_seen: now,
            is_guardian_candidate: is_guardian,
            uptime_score: 0.5, // Start neutral
            connection_count: 1,
            failed_attempts: 0,
            is_anchor: false, // Start as leaf, promote after 3+ connections
            last_fail_at: None,
            fail_count: 0,
        }
    }

    /// Update timestamps and location on successful connection
    pub fn touch(&mut self, now: u64, ip: &str, port: u16) {
        self.last_seen = now;
        self.last_ip = ip.to_string();
        self.last_port = port;
        self.connection_count += 1;
        self.failed_attempts = 0; // Reset on success
        self.fail_count = 0; // Reset consecutive failures
        self.last_fail_at = None; // Clear cooldown

        // Anchor promotion: after 3+ successful connections, mark as anchor
        if !self.is_anchor && self.connection_count >= 3 {
            self.is_anchor = true;
            info!(
                target: "vision_node::p2p::memory",
                "[CONSTELLATION_MEMORY] ⚓ Promoted peer {} to ANCHOR status ({} connections)",
                self.peer_id,
                self.connection_count
            );
        }

        // Gradually improve uptime score
        self.uptime_score = (self.uptime_score * 0.9 + 0.1).min(1.0);
    }

    /// Mark this peer as guardian candidate
    pub fn mark_candidate(&mut self) {
        self.is_guardian_candidate = true;
    }

    /// Increment uptime score by delta (capped at 1.0)
    pub fn increment_uptime(&mut self, delta: f64) {
        self.uptime_score = (self.uptime_score + delta).min(1.0);
    }

    /// Record a failed connection attempt with timestamp
    pub fn record_failure(&mut self, now: u64) {
        self.failed_attempts += 1;
        self.fail_count += 1;
        self.last_fail_at = Some(now);
        // Degrade score on failures
        self.uptime_score = (self.uptime_score * 0.95).max(0.0);
    }

    /// Check if peer should be skipped due to recent failures (cooldown logic)
    pub fn should_skip_temporarily(&self, now: u64) -> bool {
        // Skip if 3+ consecutive failures AND last failure was less than 5 minutes ago
        if self.fail_count >= 3 {
            if let Some(last_fail) = self.last_fail_at {
                let cooldown_seconds = 300; // 5 minutes
                return now.saturating_sub(last_fail) < cooldown_seconds;
            }
        }
        false
    }

    /// Reset failure counter (for periodic cleanup)
    pub fn reset_failures(&mut self) {
        self.failed_attempts = 0;
    }

    /// Decay fail_count over time (reduce by 1 per hour elapsed)
    pub fn decay_fail_count(&mut self, now: u64) {
        if let Some(last_fail) = self.last_fail_at {
            let elapsed_hours = now.saturating_sub(last_fail) / 3600;
            if elapsed_hours > 0 && self.fail_count > 0 {
                self.fail_count = self.fail_count.saturating_sub(elapsed_hours as u32);
                if self.fail_count == 0 {
                    self.last_fail_at = None; // Clear cooldown if fully decayed
                }
            }
        }
    }

    /// Calculate age in seconds since last seen
    pub fn age_seconds(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_seen)
    }
}

/// Manager for constellation memory - persistent peer database
pub struct ConstellationMemory {
    /// In-memory cache of all known peers
    peers: HashMap<String, ConstellationPeerMemory>,

    /// Persistent sled database tree
    db: Arc<sled::Tree>,
}

impl ConstellationMemory {
    /// Create a new constellation memory backed by sled
    pub fn new(db: &Db) -> Result<Self, String> {
        let tree = db
            .open_tree("constellation_memory")
            .map_err(|e| format!("Failed to open constellation_memory tree: {}", e))?;

        let mut memory = Self {
            peers: HashMap::new(),
            db: Arc::new(tree),
        };

        memory.load_all_from_db()?;

        info!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Loaded {} peers from persistent storage",
            memory.peers.len()
        );

        Ok(memory)
    }

    /// Load all peers from sled database into memory
    pub fn load_all_from_db(&mut self) -> Result<(), String> {
        for item in self.db.iter() {
            let (key, value) = item.map_err(|e| format!("DB iteration error: {}", e))?;

            let peer_id = String::from_utf8_lossy(&key).to_string();

            match bincode::deserialize::<ConstellationPeerMemory>(&value) {
                Ok(peer_memory) => {
                    self.peers.insert(peer_id, peer_memory);
                }
                Err(e) => {
                    warn!(
                        target: "vision_node::p2p::memory",
                        "[CONSTELLATION_MEMORY] Failed to deserialize peer {}: {}",
                        peer_id, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Record or update a peer in memory
    pub fn record_peer(&mut self, peer: ConstellationPeerMemory) {
        debug!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Recording peer {} (EBID: {})",
            peer.peer_id, peer.ebid
        );

        self.peers.insert(peer.peer_id.clone(), peer);
    }

    /// Update peer from handshake data
    pub fn update_from_handshake(
        &mut self,
        peer_id: String,
        ebid: String,
        ip: String,
        port: u16,
        http_api_port: Option<u16>,
        is_guardian: bool,
        is_candidate: bool,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(existing) = self.peers.get_mut(&peer_id) {
            // Update existing peer
            existing.touch(now, &ip, port);
            existing.http_api_port = http_api_port; // Update HTTP port

            if is_candidate {
                existing.mark_candidate();
            }
        } else {
            // Create new peer memory
            let mut peer = ConstellationPeerMemory::from_handshake(
                peer_id.clone(),
                ebid,
                ip,
                port,
                http_api_port,
                now,
                is_guardian || is_candidate,
            );

            if is_candidate {
                peer.mark_candidate();
            }

            self.record_peer(peer);
        }
    }

    /// Find peer by IP address (for HTTP API port lookup)
    pub fn find_peer_by_ip(&self, ip: &str) -> Option<ConstellationPeerMemory> {
        self.peers.values().find(|p| p.last_ip == ip).cloned()
    }

    /// Get the best peers for reconnection attempts
    pub fn get_best_peers(&self, limit: usize) -> Vec<ConstellationPeerMemory> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut peers: Vec<ConstellationPeerMemory> = self
            .peers
            .values()
            .filter(|p| !p.should_skip_temporarily(now))
            .cloned()
            .collect();

        // Sort by: guardian candidates first, then uptime score, then recency
        peers.sort_by(|a, b| {
            // Guardian candidates first
            match (a.is_guardian_candidate, b.is_guardian_candidate) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Then by uptime score
            let score_cmp = b.uptime_score.partial_cmp(&a.uptime_score).unwrap();
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }

            // Then by recency
            let age_a = a.age_seconds(now);
            let age_b = b.age_seconds(now);
            age_a.cmp(&age_b)
        });

        peers.into_iter().take(limit).collect()
    }

    /// Get anchor peers (3+ successful connections, persistent)
    pub fn get_anchor_peers(&self) -> Vec<ConstellationPeerMemory> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut anchors: Vec<ConstellationPeerMemory> = self
            .peers
            .values()
            .filter(|p| p.is_anchor && !p.should_skip_temporarily(now))
            .cloned()
            .collect();

        // Sort by most recently seen
        anchors.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

        info!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Found {} anchor peers for bootstrap",
            anchors.len()
        );

        anchors
    }

    /// Get leaf peers (recent connections, not yet anchored, within 72hr window)
    pub fn get_leaf_peers(&self) -> Vec<ConstellationPeerMemory> {
        const LEAF_RETENTION_SECS: u64 = 72 * 3600; // 72 hours

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cutoff = now - LEAF_RETENTION_SECS;

        let mut leaves: Vec<ConstellationPeerMemory> = self
            .peers
            .values()
            .filter(|p| {
                !p.is_anchor // Not an anchor yet
                && p.last_seen >= cutoff // Within 72hr window
                && !p.should_skip_temporarily(now) // Not temporarily failed
            })
            .cloned()
            .collect();

        // Sort by most recently seen
        leaves.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));

        info!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Found {} leaf peers (within 72hr window)",
            leaves.len()
        );

        leaves
    }

    /// Get top guardian candidates
    pub fn get_top_guardian_candidates(&self, limit: usize) -> Vec<ConstellationPeerMemory> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut candidates: Vec<ConstellationPeerMemory> = self
            .peers
            .values()
            .filter(|p| p.is_guardian_candidate)
            .cloned()
            .collect();

        // Sort by uptime score, then recency
        candidates.sort_by(|a, b| {
            let score_cmp = b.uptime_score.partial_cmp(&a.uptime_score).unwrap();
            if score_cmp != std::cmp::Ordering::Equal {
                return score_cmp;
            }

            let age_a = a.age_seconds(now);
            let age_b = b.age_seconds(now);
            age_a.cmp(&age_b)
        });

        candidates.into_iter().take(limit).collect()
    }

    /// Record a failed connection attempt for a peer
    pub fn record_failure(&mut self, peer_id: &str, now: u64) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.record_failure(now);
        }
    }

    /// Reset failure counters for all peers (periodic cleanup)
    pub fn reset_all_failures(&mut self) {
        for peer in self.peers.values_mut() {
            peer.reset_failures();
        }

        debug!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Reset failure counters for all peers"
        );
    }

    /// Decay fail_count for all peers (reduce by 1 per hour elapsed)
    pub fn decay_all_fail_counts(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut decayed_count = 0;
        for peer in self.peers.values_mut() {
            let old_fail_count = peer.fail_count;
            peer.decay_fail_count(now);
            if peer.fail_count < old_fail_count {
                decayed_count += 1;
            }
        }

        if decayed_count > 0 {
            debug!(
                target: "vision_node::p2p::memory",
                "[CONSTELLATION_MEMORY] Decayed fail_count for {} peers",
                decayed_count
            );
        }
    }

    /// Flush all peers to persistent storage
    pub fn flush_to_db(&self) -> Result<(), String> {
        for (peer_id, peer_memory) in &self.peers {
            let serialized = bincode::serialize(peer_memory)
                .map_err(|e| format!("Failed to serialize peer {}: {}", peer_id, e))?;

            self.db
                .insert(peer_id.as_bytes(), serialized)
                .map_err(|e| format!("Failed to insert peer {}: {}", peer_id, e))?;
        }

        self.db
            .flush()
            .map_err(|e| format!("Failed to flush db: {}", e))?;

        debug!(
            target: "vision_node::p2p::memory",
            "[CONSTELLATION_MEMORY] Flushed {} peers to persistent storage",
            self.peers.len()
        );

        Ok(())
    }

    /// Get a snapshot of all peers (cloned) for migration/inspection
    pub fn all_peers(&self) -> Vec<ConstellationPeerMemory> {
        self.peers.values().cloned().collect()
    }

    /// Get total count of known peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Check if peer exists in memory (P2P Robustness #4)
    pub fn has_peer(&self, peer_id: &str) -> bool {
        self.peers.contains_key(peer_id)
    }

    /// Get peer by ID (P2P Robustness #4)
    pub fn get_peer(&self, peer_id: &str) -> Option<ConstellationPeerMemory> {
        self.peers.get(peer_id).cloned()
    }

    /// Get peer by EBID
    pub fn get_by_ebid(&self, ebid: &str) -> Option<&ConstellationPeerMemory> {
        self.peers.values().find(|p| p.ebid == ebid)
    }

    /// Get summary statistics
    pub fn get_summary(&self) -> ConstellationMemorySummary {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let recent_threshold = now - 3600; // Last hour
        let recent_peers = self
            .peers
            .values()
            .filter(|p| p.last_seen > recent_threshold)
            .count();

        let guardian_candidates = self
            .peers
            .values()
            .filter(|p| p.is_guardian_candidate)
            .count();

        ConstellationMemorySummary {
            total_peers: self.peers.len(),
            recent_peers,
            guardian_candidates,
            avg_uptime_score: self.peers.values().map(|p| p.uptime_score).sum::<f64>()
                / self.peers.len().max(1) as f64,
        }
    }
}

/// Summary statistics for constellation memory
#[derive(Debug, Serialize)]
pub struct ConstellationMemorySummary {
    pub total_peers: usize,
    pub recent_peers: usize,
    pub guardian_candidates: usize,
    pub avg_uptime_score: f64,
}
