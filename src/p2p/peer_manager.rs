//! P2P Peer Manager with Hot/Warm/Cold Bucket System
//!
//! Implements intelligent peer management using bucket classification:
//! - **Hot**: Currently connected or recently seen with good metrics
//! - **Warm**: Known peers with mixed history
//! - **Cold**: Never connected or very stale/failed
//!
//! This prevents the "desperate Tinder user" problem of repeatedly
//! connecting to dead peers while ignoring good candidates.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Lightweight snapshot of active peer connections for API responses
#[derive(Clone)]
pub struct PeerSnapshot {
    pub active_peers: Vec<PeerSnapshotEntry>,
    pub inbound_count: usize,
    pub outbound_count: usize,
}

/// Single peer entry in snapshot
#[derive(Clone)]
pub struct PeerSnapshotEntry {
    pub vnode_tag: String,
    pub addr: SocketAddr,
    pub is_inbound: bool,
    pub remote_height: Option<u64>, // Peer's blockchain height for quorum detection
}

/// Peer state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerState {
    Connected,
    Connecting,
    Disconnected,
    Failed,
    KnownOnly, // in memory/db, not currently trying
}

/// Peer bucket classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerBucket {
    Hot,
    Warm,
    Cold,
}

/// Peer connection metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerMetrics {
    pub last_seen: Option<u64>,
    pub last_attempt: Option<u64>,
    pub success_count: u32,
    pub failure_count: u32,
    pub latency_ms: Option<u32>,
    pub score: f32, // 0.0 - 1.0
    /// Track if last successful connection was IPv4 (more stable than IPv6)
    #[serde(default)]
    pub last_success_ipv4: bool,
}

impl Default for PeerMetrics {
    fn default() -> Self {
        Self {
            last_seen: None,
            last_attempt: None,
            success_count: 0,
            failure_count: 0,
            latency_ms: None,
            score: 0.65, // ⭐ Change 7: Improved baseline (was 0.5, now 0.65)
            last_success_ipv4: false,
        }
    }
}

impl PeerMetrics {
    /// Compute peer score based on success/failure history and latency
    pub fn compute_score(&mut self) {
        let success_factor = (self.success_count as f32).min(10.0) / 10.0;
        let failure_penalty = (self.failure_count as f32).min(10.0) / 10.0;

        let latency_factor = match self.latency_ms {
            Some(ms) if ms < 50 => 1.0,
            Some(ms) if ms < 200 => 0.7,
            Some(ms) if ms < 500 => 0.4,
            _ => 0.2,
        };

        let mut score = 0.5 * success_factor + 0.3 * latency_factor - 0.2 * failure_penalty;

        score = score.clamp(0.0, 1.0);

        self.score = score;
    }
}

/// Peer information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peer {
    pub ip: String,
    pub port: u16,
    pub ebid: String,
    pub state: PeerState,
    pub bucket: PeerBucket,
    pub metrics: PeerMetrics,
    pub height: Option<u64>,               // blockchain height
    pub last_reported_height: Option<u64>, // Most recently reported height
    #[serde(skip)]
    pub last_height_updated_at: Option<std::time::Instant>, // When height was last updated

    // Phase 10: Reachability & Advertisement
    pub advertised_ip: Option<String>, // External IP advertised by peer
    pub advertised_port: Option<u16>,  // Port advertised (should be 7072)
    pub public_reachable: bool,        // Can we dial back to this peer?
    pub nat_type: String,              // "Open", "Restricted", "Symmetric", "Unknown"
    pub last_reachability_test: Option<u64>, // Unix timestamp of last test

    // Chain identity fields populated from handshake
    pub chain_id: Option<String>,
    pub bootstrap_prefix: Option<String>,
    pub protocol_version: Option<u32>,
    pub node_version: Option<String>,
}

impl Peer {
    /// Check if this peer uses IPv4
    pub fn is_ipv4(&self) -> bool {
        self.ip
            .parse::<std::net::IpAddr>()
            .map(|addr| addr.is_ipv4())
            .unwrap_or(false)
    }

    /// Check if this peer uses IPv6
    pub fn is_ipv6(&self) -> bool {
        self.ip
            .parse::<std::net::IpAddr>()
            .map(|addr| addr.is_ipv6())
            .unwrap_or(false)
    }

    /// Get full socket address as string
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    /// Update the reported height from handshake or status message
    pub fn update_reported_height(&mut self, height: u64) {
        self.last_reported_height = Some(height);
        self.last_height_updated_at = Some(std::time::Instant::now());
        self.height = Some(height); // Also update legacy height field
    }

    /// Calculate how far behind this peer is from network height
    pub fn lag_blocks(&self, network_estimated_height: u64) -> u64 {
        if let Some(h) = self.last_reported_height {
            network_estimated_height.saturating_sub(h)
        } else {
            0
        }
    }

    /// Check if this peer is considered "slow" and needs help catching up
    pub fn is_slow(&self, network_estimated_height: u64) -> bool {
        self.lag_blocks(network_estimated_height) >= crate::vision_constants::SLOW_PEER_LAG_BLOCKS
    }
}

impl Peer {
    /// Create a new peer entry
    pub fn new(ip: String, port: u16, ebid: String) -> Self {
        Self {
            ip,
            port,
            ebid,
            state: PeerState::KnownOnly,
            bucket: PeerBucket::Cold, // Start in cold bucket
            metrics: PeerMetrics::default(),
            height: None,
            last_reported_height: None,
            last_height_updated_at: None,

            // Phase 10: Reachability defaults
            advertised_ip: None,
            advertised_port: None,
            public_reachable: false,
            nat_type: "Unknown".to_string(),
            last_reachability_test: None,

            // Chain identity defaults
            chain_id: None,
            bootstrap_prefix: None,
            protocol_version: None,
            node_version: None,
        }
    }

    /// Check if this peer is compatible with our chain
    pub fn is_chain_compatible(
        &self,
        chain_id: &str,
        bootstrap_prefix: &str,
        min_proto: u32,
        max_proto: u32,
        min_node_version: &str,
    ) -> bool {
        // Chain id and prefix must match exactly
        match (&self.chain_id, &self.bootstrap_prefix) {
            (Some(cid), Some(prefix)) if cid == chain_id && prefix == bootstrap_prefix => {}
            _ => return false,
        }

        // Protocol version must be present and within range
        if let Some(pv) = self.protocol_version {
            if pv < min_proto || pv > max_proto {
                return false;
            }
        } else {
            return false;
        }

        // Node version check (string compare is fine if you already use semver elsewhere)
        if let Some(ver) = &self.node_version {
            if ver.as_str() < min_node_version {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    /// Update bucket classification based on current metrics
    pub fn update_bucket(&mut self, now: u64) {
        // HOT: currently connected OR seen recently & low failures
        if self.state == PeerState::Connected {
            self.bucket = PeerBucket::Hot;
            return;
        }

        // Check if seen recently (within 5 minutes) with no failures
        if let Some(last_seen) = self.metrics.last_seen {
            if now.saturating_sub(last_seen) < 300 && self.metrics.failure_count == 0 {
                self.bucket = PeerBucket::Hot;
                return;
            }
        }

        // COLD: never successfully connected OR very stale
        if self.metrics.success_count == 0 && self.metrics.failure_count > 0 {
            self.bucket = PeerBucket::Cold;
            return;
        }

        if let Some(last_seen) = self.metrics.last_seen {
            if now.saturating_sub(last_seen) > 3600 {
                self.bucket = PeerBucket::Cold;
                return;
            }
        } else {
            // Never seen
            self.bucket = PeerBucket::Cold;
            return;
        }

        // WARM: everything else (mixed history, medium recency)
        self.bucket = PeerBucket::Warm;
    }

    /// Record successful connection
    pub fn record_success(&mut self, now: u64) {
        let is_v4 = self.is_ipv4();

        // Track that this peer succeeded over IPv4
        self.metrics.last_success_ipv4 = is_v4;

        // Double reward for IPv4 connections (more stable)
        if is_v4 {
            self.metrics.success_count += 2;
            debug!("[PEER MANAGER] IPv4 success bonus for {}", self.ebid);
        } else {
            self.metrics.success_count += 1;
        }

        self.metrics.last_seen = Some(now);
        self.metrics.last_attempt = Some(now);

        // Reset failure count on successful connection
        if self.metrics.failure_count > 0 {
            self.metrics.failure_count = self.metrics.failure_count.saturating_sub(1);
        }

        self.metrics.compute_score();
        self.update_bucket(now);

        let proto = if is_v4 { "IPv4" } else { "IPv6" };
        debug!(
            "[PEER MANAGER] Success via {}: {} (score: {:.2}, bucket: {:?}, ipv4_proven: {})",
            proto, self.ebid, self.metrics.score, self.bucket, self.metrics.last_success_ipv4
        );
    }

    /// Record failed connection attempt
    pub fn record_failure(&mut self, now: u64) {
        // IPv6 failures penalized more heavily (often misconfigured)
        if self.is_ipv6() {
            self.metrics.failure_count += 2;
            warn!("[PEER MANAGER] IPv6 failure penalty for {}", self.ebid);
        } else {
            self.metrics.failure_count += 1;
        }

        self.metrics.last_attempt = Some(now);

        self.metrics.compute_score();
        self.update_bucket(now);

        let proto = if self.is_ipv4() { "IPv4" } else { "IPv6" };
        warn!(
            "[PEER MANAGER] Failure via {}: {} (failures: {}, score: {:.2}, bucket: {:?})",
            proto, self.ebid, self.metrics.failure_count, self.metrics.score, self.bucket
        );
    }

    /// Update latency measurement
    pub fn update_latency(&mut self, latency_ms: u32, now: u64) {
        self.metrics.latency_ms = Some(latency_ms);
        self.metrics.compute_score();
        self.update_bucket(now);
    }
}

/// Summary of how many peers agree with our chain
#[derive(Debug, Clone)]
pub struct ConsensusQuorum {
    pub compatible_peers: usize,
    pub incompatible_peers: usize,
    pub min_compatible_height: Option<u64>,
    pub max_compatible_height: Option<u64>,
}

/// Peer manager handles all peer lifecycle and bucket management
pub struct PeerManager {
    peers: Arc<RwLock<HashMap<String, Peer>>>, // key: ebid
    last_guardian_check: Arc<RwLock<Option<u64>>>,
    guardian_status: Arc<RwLock<Option<(bool, Option<String>)>>>,
    last_peer_event: Arc<RwLock<Option<u64>>>,
    db: Option<sled::Tree>, // Persistent storage
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            last_guardian_check: Arc::new(RwLock::new(None)),
            guardian_status: Arc::new(RwLock::new(None)),
            last_peer_event: Arc::new(RwLock::new(None)),
            db: None,
        }
    }

    /// Create peer manager with persistent storage
    pub fn with_storage(db: sled::Db) -> Self {
        let tree = db
            .open_tree(b"peer_reputation")
            .expect("Failed to open peer_reputation tree");

        let mut manager = Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            last_guardian_check: Arc::new(RwLock::new(None)),
            guardian_status: Arc::new(RwLock::new(None)),
            last_peer_event: Arc::new(RwLock::new(None)),
            db: Some(tree.clone()),
        };

        // Load persisted peers from database
        manager.load_persisted_peers(&tree);

        info!("[PEER MANAGER] Initialized with persistent storage");
        manager
    }

    /// Load peers from persistent storage
    fn load_persisted_peers(&mut self, tree: &sled::Tree) {
        let mut loaded_count = 0;
        for (key, value) in tree.iter().flatten() {
            if let Ok(ebid) = String::from_utf8(key.to_vec()) {
                if let Ok(peer) = bincode::deserialize::<Peer>(&value) {
                    // Only load if seen within last 7 days
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let week_ago = now.saturating_sub(7 * 24 * 60 * 60);

                    if let Some(last_seen) = peer.metrics.last_seen {
                        if last_seen > week_ago {
                            if let Ok(mut peers) = self.peers.try_write() {
                                peers.insert(ebid.clone(), peer);
                                loaded_count += 1;
                            }
                        }
                    }
                }
            }
        }
        if loaded_count > 0 {
            info!(
                "[PEER MANAGER] Loaded {} peers from persistent storage",
                loaded_count
            );
        }
    }

    /// Add or update a peer
    pub async fn add_peer(&self, peer: Peer) {
        let mut peers = self.peers.write().await;
        let ebid = peer.ebid.clone();
        peers.insert(ebid.clone(), peer.clone());

        // Persist to database
        if let Some(db) = &self.db {
            if let Ok(encoded) = bincode::serialize(&peer) {
                let _ = db.insert(ebid.as_bytes(), encoded);
            }
        }

        // Update last event timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        *self.last_peer_event.write().await = Some(now);

        debug!("[PEER MANAGER] Added peer: {}", ebid);
    }

    /// Get all peers
    pub async fn get_all_peers(&self) -> Vec<Peer> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Get peers in a specific bucket
    pub async fn peers_in_bucket(&self, bucket: PeerBucket) -> Vec<Peer> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| p.bucket == bucket)
            .cloned()
            .collect()
    }

    /// Get connected peers
    pub async fn connected_peers(&self) -> Vec<Peer> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| p.state == PeerState::Connected)
            .cloned()
            .collect()
    }

    /// Compute gossip weight for a peer based on lag and health
    /// Slow peers get boosted weight (2x) to help them catch up faster
    pub fn compute_peer_weight(peer: &Peer, network_estimated_height: u64) -> f32 {
        let mut weight = 1.0_f32;

        if let Some(h) = peer.last_reported_height {
            let lag = network_estimated_height.saturating_sub(h);

            if lag >= crate::vision_constants::SLOW_PEER_LAG_BLOCKS {
                // Help slow peers catch up with extra gossip priority
                weight *= 2.0;

                tracing::debug!(
                    target: "vision_node::p2p::gossip",
                    peer_ebid = %peer.ebid,
                    lag_blocks = lag,
                    weight = weight,
                    "Boosting slow peer with extra gossip weight"
                );
            }
        }

        // Optional: reduce weight for flaky connections
        // (Currently p2p_health is in SyncHealthSnapshot, not Peer)
        // If we add per-peer health metrics later, could do:
        // if peer.connection_quality == "weak" { weight *= 0.5; }

        weight
    }

    /// Select peers with weighted random sampling
    /// Slow/lagging peers get higher probability to help them catch up
    pub async fn select_weighted_peers(
        &self,
        count: usize,
        network_estimated_height: u64,
    ) -> Vec<String> {
        use rand::Rng;

        let peers = self.connected_peers().await;
        if peers.is_empty() {
            return Vec::new();
        }

        let mut rng = rand::thread_rng();
        let mut selected = Vec::new();

        // Calculate weights for all peers
        let weights: Vec<f32> = peers
            .iter()
            .map(|p| Self::compute_peer_weight(p, network_estimated_height))
            .collect();

        let mut total_weight: f32 = weights.iter().sum();
        if total_weight <= 0.0 {
            // Fallback to random selection if all weights are zero
            return peers.iter().take(count).map(|p| p.ebid.clone()).collect();
        }

        // Build (ebid, weight) pairs
        let mut available: Vec<(String, f32)> = peers
            .iter()
            .map(|p| p.ebid.clone())
            .zip(weights.into_iter())
            .collect();

        // Weighted random selection without replacement
        while selected.len() < count && !available.is_empty() {
            let roll = rng.gen::<f32>() * total_weight;
            let mut acc = 0.0;
            let mut idx = 0;

            for (i, (_, w)) in available.iter().enumerate() {
                acc += *w;
                if acc >= roll {
                    idx = i;
                    break;
                }
            }

            let (peer_ebid, w) = available.remove(idx);
            selected.push(peer_ebid);
            total_weight -= w;
        }

        tracing::debug!(
            target: "vision_node::p2p::gossip",
            selected_count = selected.len(),
            total_peers = peers.len(),
            "Selected weighted peers for gossip"
        );

        selected
    }

    /// Create snapshot of active peer connections for API
    ///
    /// This returns a lightweight view of connected peers with their
    /// node tags, addresses, and connection direction (inbound/outbound).
    /// Used by /api/constellation/peers endpoint.
    pub async fn snapshot(&self) -> PeerSnapshot {
        let peers_guard = self.peers.read().await;

        let mut entries = Vec::new();
        let mut inbound = 0usize;
        let mut outbound = 0usize;

        for peer in peers_guard.values() {
            // Only include actively connected peers
            if peer.state != PeerState::Connected {
                continue;
            }

            // Parse socket address from peer ip:port
            let addr_str = format!("{}:{}", peer.ip, peer.port);
            let addr = match addr_str.parse::<SocketAddr>() {
                Ok(a) => a,
                Err(_) => {
                    warn!("[PEER MANAGER] Invalid peer address: {}", addr_str);
                    continue;
                }
            };

            // Generate vnode tag from EBID (use first 12 chars for compact display)
            let vnode_tag = if peer.ebid.len() >= 12 {
                format!("VNODE-{}", &peer.ebid[..12].to_uppercase())
            } else {
                format!("VNODE-{}", peer.ebid.to_uppercase())
            };

            // Determine direction (we don't currently track this explicitly,
            // so we'll infer: peers we discovered via bootstrap are likely outbound)
            // For now, treat all as outbound until we add proper direction tracking
            let is_inbound = false; // TODO: Track connection direction in Peer struct

            if is_inbound {
                inbound += 1;
            } else {
                outbound += 1;
            }

            entries.push(PeerSnapshotEntry {
                vnode_tag,
                addr,
                is_inbound,
                remote_height: peer.height,
            });
        }

        PeerSnapshot {
            active_peers: entries,
            inbound_count: inbound,
            outbound_count: outbound,
        }
    }

    /// Update peer state
    pub async fn update_peer_state(&self, ebid: &str, state: PeerState) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(ebid) {
            peer.state = state;

            match state {
                PeerState::Connected => peer.record_success(now),
                PeerState::Failed => peer.record_failure(now),
                _ => peer.update_bucket(now),
            }

            // Persist updated peer to database
            if let Some(db) = &self.db {
                if let Ok(encoded) = bincode::serialize(&*peer) {
                    let _ = db.insert(ebid.as_bytes(), encoded);
                }
            }

            *self.last_peer_event.write().await = Some(now);
        }
    }

    /// Update peer latency
    pub async fn update_peer_latency(&self, ebid: &str, latency_ms: u32) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(ebid) {
            peer.update_latency(latency_ms, now);
        }
    }

    /// Update peer height
    pub async fn update_peer_height(&self, ebid: &str, height: u64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(ebid) {
            peer.height = Some(height);
        }
    }

    /// Update peer chain identity fields from handshake
    pub async fn update_peer_chain_identity(
        &self,
        ebid: &str,
        chain_id: String,
        bootstrap_prefix: String,
        protocol_version: u32,
        node_version: String,
    ) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(ebid) {
            peer.chain_id = Some(chain_id);
            peer.bootstrap_prefix = Some(bootstrap_prefix);
            peer.protocol_version = Some(protocol_version);
            peer.node_version = Some(node_version);

            // Persist updated peer to database
            if let Some(db) = &self.db {
                if let Ok(encoded) = bincode::serialize(&*peer) {
                    let _ = db.insert(ebid.as_bytes(), encoded);
                }
            }
        }
    }

    /// Find the best height quorum from the current snapshot.
    /// Returns (height, count) of the dominant height band if any.
    /// This helps determine if the network has converged on a height consensus.
    pub async fn best_height_quorum(&self, max_delta: u64) -> Option<(u64, usize)> {
        let snapshot = self.snapshot().await;

        // Collect only peers that have a known height
        let mut buckets: HashMap<u64, usize> = HashMap::new();

        for entry in snapshot.active_peers.iter() {
            if let Some(h) = entry.remote_height {
                *buckets.entry(h).or_insert(0) += 1;
            }
        }

        if buckets.is_empty() {
            return None;
        }

        // Find the "dominant" height band:
        // for every height, count how many peers are within +/- max_delta
        let mut best_height = 0;
        let mut best_count = 0;

        for (&base_height, _) in buckets.iter() {
            let mut band_count = 0;
            for (&h, &c) in buckets.iter() {
                if h >= base_height.saturating_sub(max_delta)
                    && h <= base_height.saturating_add(max_delta)
                {
                    band_count += c;
                }
            }

            if band_count > best_count {
                best_count = band_count;
                best_height = base_height;
            }
        }

        if best_count == 0 {
            None
        } else {
            Some((best_height, best_count))
        }
    }

    /// Get consensus quorum snapshot - how many peers are on the same chain
    pub async fn consensus_quorum(&self) -> ConsensusQuorum {
        use crate::vision_constants::{
            expected_chain_id, VISION_BOOTSTRAP_PREFIX, VISION_MAX_PROTOCOL_VERSION,
            VISION_MIN_NODE_VERSION, VISION_MIN_PROTOCOL_VERSION,
        };

        let expected_chain_id = expected_chain_id();

        let peers = self.peers.read().await;

        let mut compatible_peers = 0;
        let mut incompatible_peers = 0;
        let mut min_h: Option<u64> = None;
        let mut max_h: Option<u64> = None;

        for p in peers.values() {
            // Only consider connected peers
            if p.state != PeerState::Connected {
                continue;
            }

            if p.is_chain_compatible(
                &expected_chain_id,
                VISION_BOOTSTRAP_PREFIX,
                VISION_MIN_PROTOCOL_VERSION,
                VISION_MAX_PROTOCOL_VERSION,
                VISION_MIN_NODE_VERSION,
            ) {
                compatible_peers += 1;

                if let Some(h) = p.height {
                    min_h = Some(min_h.map(|m| m.min(h)).unwrap_or(h));
                    max_h = Some(max_h.map(|m| m.max(h)).unwrap_or(h));
                }
            } else {
                incompatible_peers += 1;
            }
        }

        ConsensusQuorum {
            compatible_peers,
            incompatible_peers,
            min_compatible_height: min_h,
            max_compatible_height: max_h,
        }
    }

    /// Find the highest known remote height among compatible peers.
    /// Used by auto-sync to determine the best chain to follow.
    /// This is simpler than best_height_quorum() - it just returns the max height seen.
    pub async fn best_remote_height(&self) -> Option<u64> {
        let snapshot = self.snapshot().await;

        let mut best: Option<u64> = None;
        for entry in snapshot.active_peers.iter() {
            if let Some(h) = entry.remote_height {
                best = match best {
                    Some(cur) if h > cur => Some(h),
                    None => Some(h),
                    Some(cur) => Some(cur),
                };
            }
        }

        best
    }

    /// Phase 10: Update peer reachability test results
    pub async fn update_peer_reachability(
        &self,
        ebid: &str,
        reachable: bool,
        nat_type: String,
        tested_at: u64,
    ) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(ebid) {
            peer.public_reachable = reachable;
            peer.nat_type = nat_type;
            peer.last_reachability_test = Some(tested_at);
        }
    }

    /// Phase 10: Get peer reachability info (returns tuple: reachable, nat_type)
    pub async fn get_peer_reachability(&self, ebid: &str) -> Option<(bool, String)> {
        let peers = self.peers.read().await;
        peers
            .get(ebid)
            .map(|peer| (peer.public_reachable, peer.nat_type.clone()))
    }

    /// Choose peers for reconnection attempts
    pub async fn choose_reconnect_candidates(&self, target: usize) -> Vec<Peer> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let peers = self.peers.read().await;
        let mut candidates = Vec::new();

        // Check if we have healthy IPv4 peers already
        let has_healthy_ipv4 = peers.values().any(|p| {
            p.metrics.last_success_ipv4
                && matches!(p.state, PeerState::Connected | PeerState::Connecting)
        });

        // Helper to filter candidates with backoff (IPv4-aware)
        let should_attempt = |peer: &Peer| -> bool {
            // Skip if already connecting or connected
            if matches!(peer.state, PeerState::Connecting | PeerState::Connected) {
                return false;
            }

            // Calculate exponential backoff based on failure count
            let base_delay: u64 = 10; // seconds
            let failures = peer.metrics.failure_count.max(1) as u64;
            let backoff = base_delay
                .saturating_mul(1u64.checked_shl((failures - 1).min(6) as u32).unwrap_or(64));

            if let Some(last_attempt) = peer.metrics.last_attempt {
                if now < last_attempt + backoff {
                    return false;
                }
            }

            // IPv6-only peers: be extra picky if we have healthy IPv4 peers
            if !peer.metrics.last_success_ipv4 && has_healthy_ipv4 {
                // Only try IPv6-only peers occasionally when severely under target
                if peer.metrics.failure_count > 3 {
                    debug!("[PEER MANAGER] Skipping IPv6-only peer {} (failures: {}, have healthy IPv4)",
                           peer.ebid, peer.metrics.failure_count);
                    return false;
                }
            }

            true
        };

        // Collect and sort peers from each bucket
        for bucket in [PeerBucket::Hot, PeerBucket::Warm, PeerBucket::Cold] {
            if candidates.len() >= target {
                break;
            }

            let mut bucket_peers: Vec<_> = peers
                .values()
                .filter(|p| p.bucket == bucket && should_attempt(p))
                .cloned()
                .collect();

            // Sort to prefer:
            // 1. Peers with proven IPv4 success (last_success_ipv4 = true)
            // 2. Then IPv4 addresses (even if unproven)
            // 3. Then everything else (IPv6)
            bucket_peers.sort_by(|a, b| {
                // First priority: proven IPv4 success
                match b
                    .metrics
                    .last_success_ipv4
                    .cmp(&a.metrics.last_success_ipv4)
                {
                    std::cmp::Ordering::Equal => {
                        // Second priority: IPv4 address
                        let a_v4 = a.is_ipv4();
                        let b_v4 = b.is_ipv4();
                        b_v4.cmp(&a_v4)
                    }
                    other => other,
                }
            });

            for peer in bucket_peers {
                if candidates.len() >= target {
                    break;
                }
                let proto = if peer.is_ipv4() { "IPv4" } else { "IPv6" };
                let proven = if peer.metrics.last_success_ipv4 {
                    "✓"
                } else {
                    "?"
                };
                debug!(
                    "[PEER MANAGER] Adding {:?} peer {} ({}{}, score: {:.2})",
                    bucket, peer.ebid, proto, proven, peer.metrics.score
                );
                candidates.push(peer);
            }
        }

        // Log recovery candidate details
        let summary: Vec<_> = candidates
            .iter()
            .map(|p| {
                let proto = if p.is_ipv4() { "IPv4" } else { "IPv6" };
                let proven = if p.metrics.last_success_ipv4 {
                    "proven"
                } else {
                    "unproven"
                };
                format!("{}({}/{})", &p.ebid[..8], proto, proven)
            })
            .collect();

        info!(
            "[PEER_RECOVERY] Selected {} reconnect candidates (target: {}): {:?}",
            candidates.len(),
            target,
            summary
        );

        candidates
    }

    /// Update guardian status
    pub async fn update_guardian_status(&self, reachable: bool, ebid: Option<String>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        *self.guardian_status.write().await = Some((reachable, ebid));
        *self.last_guardian_check.write().await = Some(now);
    }

    /// Get guardian status
    pub async fn get_guardian_status(&self) -> Option<(bool, Option<String>)> {
        self.guardian_status.read().await.clone()
    }

    /// Get last guardian check timestamp
    pub async fn get_last_guardian_check(&self) -> Option<u64> {
        *self.last_guardian_check.read().await
    }

    /// Get last peer event timestamp
    pub async fn get_last_peer_event(&self) -> Option<u64> {
        *self.last_peer_event.read().await
    }

    /// Check if we have healthy IPv4 peers connected or connecting
    pub async fn has_healthy_ipv4_peers(&self) -> bool {
        let peers = self.peers.read().await;
        peers.values().any(|p| {
            p.metrics.last_success_ipv4
                && matches!(p.state, PeerState::Connected | PeerState::Connecting)
        })
    }

    /// Get count of IPv4 vs IPv6 peers by state
    pub async fn get_peer_protocol_stats(&self) -> (usize, usize, usize, usize) {
        let peers = self.peers.read().await;
        let ipv4_connected = peers
            .values()
            .filter(|p| p.is_ipv4() && p.state == PeerState::Connected)
            .count();
        let ipv6_connected = peers
            .values()
            .filter(|p| p.is_ipv6() && p.state == PeerState::Connected)
            .count();
        let ipv4_proven = peers
            .values()
            .filter(|p| p.metrics.last_success_ipv4)
            .count();
        let ipv6_only = peers
            .values()
            .filter(|p| p.is_ipv6() && !p.metrics.last_success_ipv4)
            .count();

        (ipv4_connected, ipv6_connected, ipv4_proven, ipv6_only)
    }

    /// Count currently connected peers that have validated chain identity.
    ///
    /// Single source of truth for "validated connected peer" gating.
    pub async fn connected_validated_count(&self) -> usize {
        use crate::vision_constants::{
            expected_chain_id, VISION_BOOTSTRAP_PREFIX, VISION_MAX_PROTOCOL_VERSION,
            VISION_MIN_NODE_VERSION, VISION_MIN_PROTOCOL_VERSION,
        };

        let expected_chain_id = expected_chain_id();
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| {
                p.state == PeerState::Connected
                    && p.is_chain_compatible(
                        &expected_chain_id,
                        VISION_BOOTSTRAP_PREFIX,
                        VISION_MIN_PROTOCOL_VERSION,
                        VISION_MAX_PROTOCOL_VERSION,
                        VISION_MIN_NODE_VERSION,
                    )
            })
            .count()
    }

    /// Best-effort non-async variant for sync contexts.
    /// Returns 0 if the peer map lock is contended.
    pub fn try_connected_validated_count(&self) -> usize {
        use crate::vision_constants::{
            expected_chain_id, VISION_BOOTSTRAP_PREFIX, VISION_MAX_PROTOCOL_VERSION,
            VISION_MIN_NODE_VERSION, VISION_MIN_PROTOCOL_VERSION,
        };

        let expected_chain_id = expected_chain_id();
        let peers = match self.peers.try_read() {
            Ok(g) => g,
            Err(_) => return 0,
        };

        peers
            .values()
            .filter(|p| {
                p.state == PeerState::Connected
                    && p.is_chain_compatible(
                        &expected_chain_id,
                        VISION_BOOTSTRAP_PREFIX,
                        VISION_MIN_PROTOCOL_VERSION,
                        VISION_MAX_PROTOCOL_VERSION,
                        VISION_MIN_NODE_VERSION,
                    )
            })
            .count()
    }

    /// Best-effort non-async highest known remote height among compatible connected peers.
    /// Returns None if the lock is contended or no heights are known.
    pub fn try_best_remote_height(&self) -> Option<u64> {
        use crate::vision_constants::{
            expected_chain_id, VISION_BOOTSTRAP_PREFIX, VISION_MAX_PROTOCOL_VERSION,
            VISION_MIN_NODE_VERSION, VISION_MIN_PROTOCOL_VERSION,
        };

        let expected_chain_id = expected_chain_id();
        let peers = self.peers.try_read().ok()?;

        peers
            .values()
            .filter(|p| {
                p.state == PeerState::Connected
                    && p.is_chain_compatible(
                        &expected_chain_id,
                        VISION_BOOTSTRAP_PREFIX,
                        VISION_MIN_PROTOCOL_VERSION,
                        VISION_MAX_PROTOCOL_VERSION,
                        VISION_MIN_NODE_VERSION,
                    )
            })
            .filter_map(|p| p.height)
            .max()
    }

    /// Back-compat alias for connected validated peer count.
    pub async fn connected_peer_count(&self) -> usize {
        self.connected_validated_count().await
    }

    /// Check if we have a Connected peer with this IP (for duplicate detection)
    pub async fn has_connected_peer_with_ip(&self, ip: &str) -> bool {
        let peers = self.peers.read().await;
        peers
            .values()
            .any(|p| p.state == PeerState::Connected && p.ip == ip)
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_bucket_classification() {
        let mut peer = Peer::new("127.0.0.1".to_string(), 7072, "test_ebid".to_string());
        let now = 1000000;

        // New peer should be cold
        peer.update_bucket(now);
        assert_eq!(peer.bucket, PeerBucket::Cold);

        // Connected peer should be hot
        peer.state = PeerState::Connected;
        peer.update_bucket(now);
        assert_eq!(peer.bucket, PeerBucket::Hot);

        // Disconnected with recent success should be warm
        peer.state = PeerState::Disconnected;
        peer.metrics.last_seen = Some(now - 500); // 8 minutes ago
        peer.metrics.success_count = 1;
        peer.update_bucket(now);
        assert_eq!(peer.bucket, PeerBucket::Warm);

        // Very old peer should be cold
        peer.metrics.last_seen = Some(now - 5000); // ~83 minutes ago
        peer.update_bucket(now);
        assert_eq!(peer.bucket, PeerBucket::Cold);
    }

    #[test]
    fn test_peer_score_computation() {
        let mut metrics = PeerMetrics::default();

        // High success, low latency = high score
        metrics.success_count = 10;
        metrics.failure_count = 0;
        metrics.latency_ms = Some(30);
        metrics.compute_score();
        assert!(metrics.score > 0.8);

        // High failures = low score
        metrics.success_count = 1;
        metrics.failure_count = 10;
        metrics.latency_ms = Some(500);
        metrics.compute_score();
        assert!(metrics.score < 0.3);
    }
}
