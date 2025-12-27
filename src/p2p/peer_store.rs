//! Vision Peer Book - Identity-based peer discovery system
//!
//! Instead of IP-based addressing, Vision uses a decentralized peer book where
//! nodes are identified by their Vision Identity (node_tag + public_key hash).
//!
//! **Vision Address Format:**
//! `vision://<node_tag>@<public_key_hash>`
//!
//! Example: `vision://VNODE-J4K8-99AZ@8a4fbd91c2337f0a83`
//!
//! **How it works:**
//! 1. Nodes generate Vision addresses after bootstrap handshake
//! 2. P2P handshakes exchange peer book entries
//! 3. Nodes gossip their known peers (like Bitcoin's addr messages)
//! 4. IP addresses are only used internally to open sockets
//! 5. All UI, logs, and network operations use Vision addresses

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use tracing::{debug, info, warn};

/// Helper function to determine PeerBook scope
/// This is re-exported from the parent module
fn peerbook_scope() -> String {
    crate::p2p::peerbook_scope()
}

// ============================================================================
// HTTP SEED PEER EXPORT (v2.7.0+)
// ============================================================================

/// Public peer information for HTTP seed distribution
/// Served via GET /api/p2p/seed_peers on port 7070
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicPeerInfo {
    /// P2P address in "ip:port" format (e.g., "69.173.206.211:7072")
    pub address: String,
    /// Anchor flag - true for backbone nodes, false for regular peers
    pub is_anchor: bool,
}

// ============================================================================
// UNLIMITED PEER MESH - NO CAPACITY LIMITS
// ============================================================================

/// NO LIMIT on peer book size - network grows organically
/// Every discovered peer is kept forever (unless manually pruned)
/// This creates a true full mesh where everyone knows everyone
// pub const MAX_PEERS: usize = 1000;  // REMOVED - unlimited growth

/// Minimum health score to keep a peer (0-100 scale)
pub const MIN_HEALTH_TO_KEEP: i32 = 10;

/// Maximum consecutive failures before hard eviction
pub const MAX_FAIL_COUNT: u32 = 8;

/// Node mood information - tracks peer health and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMoodInfo {
    /// Mood label: "calm" | "warning" | "storm" | "wounded" | "celebration"
    pub label: String,
    /// Mood score from 0.0 (critical) to 1.0 (perfect)
    pub score: f32,
    /// Human-readable reason for current mood
    pub reason: String,
    /// Last updated timestamp
    pub last_updated: i64,
}

/// Vision Peer Entry - represents a known peer in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionPeer {
    /// Unique node identifier
    pub node_id: String,

    /// Human-readable node tag (e.g., "VNODE-J4K8-99AZ")
    pub node_tag: String,

    /// Node's public key (hex encoded)
    pub public_key: String,

    /// Vision address (vision://node_tag@pubkey_hash)
    pub vision_address: String,

    /// Blake3 fingerprint of admission ticket (first 6 bytes hex)
    pub admission_ticket_fingerprint: String,

    /// Node role: "constellation" or "guardian"
    pub role: String,

    /// Last seen timestamp (Unix epoch seconds)
    pub last_seen: i64,

    /// Trusted peer flag - promoted based on mood and behavior
    pub trusted: bool,

    /// Current mood/health status of this peer
    pub mood: Option<NodeMoodInfo>,

    /// Optional IP address for direct connection (ephemeral, not stored long-term)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,

    // ========================================================================
    // ROLLING MESH HEALTH FIELDS
    // ========================================================================
    /// Health score (0-100): tracks reliability for eviction ranking
    #[serde(default = "default_health_score")]
    pub health_score: i32,

    /// Last successful handshake/block/interaction (unix timestamp)
    #[serde(default)]
    pub last_success: u64,

    /// Last failed connection attempt (unix timestamp)
    #[serde(default)]
    pub last_failure: u64,

    /// Consecutive failure count (resets on success)
    #[serde(default)]
    pub fail_count: u32,

    /// Seed peer flag (guardian/genesis seeds, protected from eviction)
    #[serde(default)]
    pub is_seed: bool,

    /// Anchor node flag (backbone/truth keepers, prioritized for routing)
    #[serde(default)]
    pub is_anchor: bool,

    /// Connection status: "connected", "disconnected", "connecting", "failed"
    #[serde(default = "default_connection_status")]
    pub connection_status: String,

    // ========================================================================
    // LATENCY & ROUTING INTELLIGENCE (Phase 3.5)
    // ========================================================================
    /// Last measured round-trip time in milliseconds
    #[serde(default)]
    pub last_rtt_ms: Option<u32>,

    /// Exponential moving average of RTT in milliseconds
    #[serde(default)]
    pub avg_rtt_ms: Option<u32>,

    /// Latency bucket classification for routing decisions
    #[serde(default)]
    pub latency_bucket: Option<LatencyBucket>,

    /// Combined reliability score (0.0 = unreliable, 1.0 = perfect)
    #[serde(default = "default_reliability_score")]
    pub reliability_score: f32,

    /// Success count for reliability calculation
    #[serde(default)]
    pub success_count: u32,

    /// Geographic region for cluster routing (e.g., "North America > United States")
    #[serde(default)]
    pub region: Option<String>,

    // ========================================================================
    // ADVERSARIAL RESILIENCE & REPUTATION (Phase 4)
    // ========================================================================
    /// Trust level classification for adversarial defense
    #[serde(default)]
    pub trust_level: PeerTrustLevel,

    /// Reputation score (0.0 = worst, 100.0 = perfect)
    #[serde(default = "default_reputation")]
    pub reputation: f32,

    /// Accumulated misbehavior score (higher = worse)
    #[serde(default)]
    pub misbehavior_score: f32,

    /// Graylisted until this timestamp (None = not graylisted)
    #[serde(default)]
    pub graylisted_until: Option<i64>,

    /// Banned until this timestamp (None = not banned)
    #[serde(default)]
    pub banned_until: Option<i64>,

    /// Total count of invalid messages received from this peer
    #[serde(default)]
    pub total_invalid_msgs: u32,

    /// Total count of protocol violations
    #[serde(default)]
    pub total_protocol_violations: u32,

    /// Total count of spam events
    #[serde(default)]
    pub total_spam_events: u32,

    // ========================================================================
    // ROUTE LEARNING & EFFECTIVENESS (Phase 4)
    // ========================================================================
    /// Total times this peer was used as a relay target
    #[serde(default)]
    pub route_uses: u32,

    /// Successful message deliveries via this peer
    #[serde(default)]
    pub route_successes: u32,

    /// Failed message deliveries via this peer
    #[serde(default)]
    pub route_failures: u32,

    /// Average delivery time in milliseconds (EMA)
    #[serde(default)]
    pub avg_delivery_ms: Option<u32>,

    // ========================================================================
    // PEER HIERARCHY & SYNC PROMOTION (Sync Stagnation Fix)
    // ========================================================================
    /// Peer tier for sync hierarchy: hot (default) -> warm -> anchor
    #[serde(default)]
    pub peer_tier: PeerTier,

    /// Timestamp when peer was last promoted (for stability tracking)
    #[serde(default)]
    pub last_promotion: Option<i64>,

    /// Public reachability status (for anchor promotion eligibility)
    #[serde(default)]
    pub public_reachable: bool,
}

/// Peer tier classification for sync hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerTier {
    Hot,    // Default tier - normal peer
    Warm,   // Promoted sync provider - reliable source
    Anchor, // Publicly reachable warm peer - serves the swarm
}

impl Default for PeerTier {
    fn default() -> Self {
        Self::Hot
    }
}

/// Latency bucket classification for intelligent routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyBucket {
    UltraLow, // < 25ms
    Low,      // 25-75ms
    Medium,   // 75-150ms
    High,     // 150-300ms
    Extreme,  // > 300ms
}

/// Trust level for adversarial resilience
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerTrustLevel {
    Trusted,    // High reputation, no recent issues (reputation >= 80.0)
    Normal,     // Standard peer (reputation 40.0-79.9)
    Probation,  // Slight issues, being watched (reputation 20.0-39.9)
    Graylisted, // Temporary ban for misbehavior (misbehavior >= 30.0)
    Banned,     // Severe misbehavior, long-term ban (misbehavior >= 80.0)
}

impl Default for PeerTrustLevel {
    fn default() -> Self {
        Self::Normal
    }
}

fn default_health_score() -> i32 {
    50 // Default mid-range health for new peers
}

fn default_reputation() -> f32 {
    50.0 // Default mid-range reputation for new peers
}

fn default_reliability_score() -> f32 {
    0.5 // Default mid-range reliability for new peers
}

fn default_connection_status() -> String {
    "disconnected".to_string()
}

impl VisionPeer {
    /// Create a new Vision peer entry
    pub fn new(
        node_id: String,
        node_tag: String,
        public_key: String,
        vision_address: String,
        admission_ticket: Option<&str>,
        role: String,
    ) -> Self {
        let admission_ticket_fingerprint = admission_ticket
            .map(compute_ticket_fingerprint)
            .unwrap_or_default();

        let now = chrono::Utc::now().timestamp() as u64;

        Self {
            node_id,
            node_tag,
            public_key,
            vision_address,
            admission_ticket_fingerprint,
            role,
            last_seen: now as i64,
            trusted: false,
            mood: None,
            ip_address: None,
            // Rolling mesh defaults
            health_score: 50,
            last_success: now,
            last_failure: 0,
            fail_count: 0,
            is_seed: false,
            is_anchor: false,
            connection_status: "disconnected".to_string(),
            // Latency & routing defaults
            last_rtt_ms: None,
            avg_rtt_ms: None,
            latency_bucket: None,
            reliability_score: 0.5,
            success_count: 0,
            region: None,
            // Reputation defaults
            trust_level: PeerTrustLevel::Normal,
            reputation: 50.0,
            misbehavior_score: 0.0,
            graylisted_until: None,
            banned_until: None,
            total_invalid_msgs: 0,
            total_protocol_violations: 0,
            total_spam_events: 0,
            // Route learning defaults
            route_uses: 0,
            route_successes: 0,
            route_failures: 0,
            avg_delivery_ms: None,
            // Peer hierarchy defaults
            peer_tier: PeerTier::Hot,
            last_promotion: None,
            public_reachable: false,
        }
    }

    /// Update last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = chrono::Utc::now().timestamp();
    }

    /// Check if peer was seen recently (within last 24 hours)
    pub fn is_recent(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        (now - self.last_seen) < 86400 // 24 hours
    }

    // ========================================================================
    // ROLLING MESH HEALTH SCORING
    // ========================================================================

    /// Mark successful interaction (handshake, block, etc.)
    pub fn mark_success(&mut self, now: u64) {
        self.last_success = now;
        self.last_seen = now as i64;
        self.fail_count = 0;
        self.health_score = (self.health_score + 5).min(100);
    }

    /// Mark failed interaction (timeout, connection refused, etc.)
    ///
    /// PATCH 3: Seeds are ultra-forgiving - failures don't accumulate in testnet swarm mode
    pub fn mark_failure(&mut self, now: u64) {
        self.last_failure = now;

        // PATCH 3: If this is a seed peer, log but never penalize health or accumulate failures
        if self.is_seed {
            debug!(
                target: "p2p::seed_protection",
                peer = %self.node_tag,
                "Seed peer connection failed - no penalty applied (seed protection active)"
            );
            // Reset failure count to 0 so seeds are never blacklisted
            self.fail_count = 0;
            // Don't reduce health score for seeds
            return;
        }

        // Normal peers: accumulate failures and reduce health
        self.fail_count = self.fail_count.saturating_add(1);
        self.health_score = (self.health_score - 10).max(0);
    }

    /// Calculate eviction rank for sorting (lower = worse, evict first)
    /// Returns: (is_not_seed, -health_score, -last_success)
    /// Non-seeds evicted before seeds, lowest health first, oldest success first
    pub fn eviction_rank(&self) -> (bool, i32, u64) {
        let is_not_seed = !self.is_seed;
        let neg_health = -self.health_score; // invert for ascending sort
        let neg_last_success = if self.last_success > 0 {
            -(self.last_success as i64)
        } else {
            0
        };
        (is_not_seed, neg_health, neg_last_success as u64)
    }

    /// Check if peer should be hard evicted (health too low or too many failures)
    pub fn should_hard_evict(&self) -> bool {
        !self.is_seed
            && (self.health_score <= MIN_HEALTH_TO_KEEP || self.fail_count >= MAX_FAIL_COUNT)
    }

    // ========================================================================
    // LATENCY & RELIABILITY TRACKING (Phase 3.5)
    // ========================================================================

    /// Update latency measurement with exponential moving average
    pub fn update_latency(&mut self, rtt_ms: u32) {
        self.last_rtt_ms = Some(rtt_ms);

        // Exponential moving average (EMA) with alpha = 0.3
        let alpha = 0.3_f32;
        self.avg_rtt_ms = Some(match self.avg_rtt_ms {
            Some(old) => (alpha * rtt_ms as f32 + (1.0 - alpha) * old as f32).round() as u32,
            None => rtt_ms,
        });

        // Classify into latency bucket
        let avg = self.avg_rtt_ms.unwrap_or(rtt_ms);
        self.latency_bucket = Some(if avg < 25 {
            LatencyBucket::UltraLow
        } else if avg < 75 {
            LatencyBucket::Low
        } else if avg < 150 {
            LatencyBucket::Medium
        } else if avg < 300 {
            LatencyBucket::High
        } else {
            LatencyBucket::Extreme
        });
    }

    /// Update reliability score based on success/failure
    pub fn update_reliability(&mut self, ok: bool) {
        if ok {
            self.success_count = self.success_count.saturating_add(1);
        } else {
            // failure already tracked by fail_count in mark_failure
        }

        let total = self.success_count + self.fail_count;
        self.reliability_score = if total == 0 {
            0.5
        } else {
            self.success_count as f32 / total as f32
        };
    }
}

/// Persistent storage for Vision Peer Book
/// Scoped storage prevents peer data from mixing across networks/test runs
pub struct PeerStore {
    db: sled::Tree,
    scope: String,
}

impl PeerStore {
    /// Create or open the Vision Peer Book database with automatic scope
    pub fn new(db: &sled::Db) -> Result<Self> {
        let scope = peerbook_scope();
        Self::new_scoped(db, scope)
    }

    /// Create or open the Vision Peer Book database with explicit scope
    pub fn new_scoped(db: &sled::Db, scope: impl Into<String>) -> Result<Self> {
        let tree = db.open_tree("vision_peer_book")?;
        let scope = scope.into();
        info!(
            "[PEER BOOK] Opened Vision Peer Book database with scope='{}'",
            scope
        );
        Ok(Self { db: tree, scope })
    }

    /// Generate scope-prefixed key for peer storage
    fn scoped_key(&self, key: &str) -> String {
        format!("{}|{}", self.scope, key)
    }

    /// Save or update a peer entry
    pub fn save(&self, peer: &VisionPeer) -> Result<()> {
        // Fix 2: Never save private IPs to PeerBook
        if let Some(ref ip_addr) = peer.ip_address {
            if !crate::p2p::ip_filter::validate_ip_for_storage(ip_addr) {
                debug!(
                    "[PEER BOOK] Rejecting peer {} with private IP: {}",
                    peer.node_id, ip_addr
                );
                return Err(anyhow::anyhow!("Private IP not allowed in peer book"));
            }
        }

        let key = self.scoped_key(&peer.node_id);
        let val = serde_json::to_vec(peer)?;
        self.db.insert(key.as_bytes(), val)?;

        debug!(
            "[PEER BOOK] Saved peer: {} ({})",
            peer.vision_address, peer.node_id
        );

        Ok(())
    }

    /// Get a peer by node_id
    pub fn get(&self, node_id: &str) -> Option<VisionPeer> {
        let key = self.scoped_key(node_id);
        let val = self.db.get(key.as_bytes()).ok()??;
        serde_json::from_slice(&val).ok()
    }

    /// Get all known peers in this scope
    pub fn all(&self) -> Vec<VisionPeer> {
        let prefix = format!("{}|", self.scope);
        self.db
            .iter()
            .filter_map(|v| v.ok())
            .filter(|(k, _)| {
                // Only include entries with matching scope prefix
                String::from_utf8_lossy(k).starts_with(&prefix)
            })
            .filter_map(|(_, val)| serde_json::from_slice(&val).ok())
            .collect()
    }

    /// Remove a peer by node_id (for network healing)
    pub fn remove(&self, node_id: &str) -> Result<()> {
        let key = self.scoped_key(node_id);
        self.db.remove(key.as_bytes())?;
        debug!(
            "[PEER BOOK] Removed peer: {} (scope={})",
            node_id, self.scope
        );
        Ok(())
    }

    /// Get recently seen peers in this scope (within last 24 hours)
    pub fn recent(&self) -> Vec<VisionPeer> {
        self.all().into_iter().filter(|p| p.is_recent()).collect()
    }

    /// Get peers by role in this scope
    pub fn by_role(&self, role: &str) -> Vec<VisionPeer> {
        self.all().into_iter().filter(|p| p.role == role).collect()
    }

    /// Merge peer book entries from another node (gossip protocol)
    pub fn merge(&self, peers: Vec<VisionPeer>) -> Result<usize> {
        let mut merged = 0;

        for peer in peers {
            // Check if we already have this peer
            if let Some(mut existing) = self.get(&peer.node_id) {
                // Update if newer
                if peer.last_seen > existing.last_seen {
                    existing.last_seen = peer.last_seen;
                    existing.ip_address = peer.ip_address.or(existing.ip_address);
                    self.save(&existing)?;
                    merged += 1;
                }
            } else {
                // New peer - add to book
                self.save(&peer)?;
                merged += 1;
            }
        }

        if merged > 0 {
            info!("[PEER BOOK] Merged {} peer entries from gossip", merged);
        }

        Ok(merged)
    }

    /// Prune old peers in this scope (not seen in 7 days)
    pub fn prune_old(&self, max_age_days: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let max_age_seconds = max_age_days * 86400;
        let mut pruned = 0;
        let prefix = format!("{}|", self.scope);

        for (key, val) in self.db.iter().flatten() {
            // Only prune entries in this scope
            if !String::from_utf8_lossy(&key).starts_with(&prefix) {
                continue;
            }

            if let Ok(peer) = serde_json::from_slice::<VisionPeer>(&val) {
                if (now - peer.last_seen) > max_age_seconds {
                    self.db.remove(&key)?;
                    pruned += 1;
                }
            }
        }

        if pruned > 0 {
            info!(
                "[PEER BOOK] Pruned {} stale peers in scope '{}' (>{}d old)",
                pruned, self.scope, max_age_days
            );
        }

        Ok(pruned)
    }

    /// Get peer count in this scope
    pub fn count(&self) -> usize {
        self.all().len()
    }

    /// Upsert peer with updated timestamp
    pub fn upsert(&self, mut peer: VisionPeer) -> Result<()> {
        peer.last_seen = chrono::Utc::now().timestamp();
        self.save(&peer)
    }

    /// Get all peers (alias for compatibility)
    pub fn get_all(&self) -> Vec<VisionPeer> {
        self.all()
    }

    /// Mark a peer as trusted or untrusted by node_tag
    pub fn mark_trusted(&self, node_tag: &str, trusted: bool) -> Result<usize> {
        let peers = self.all();
        let mut updated = 0;

        for mut peer in peers {
            if peer.node_tag == node_tag {
                peer.trusted = trusted;
                self.save(&peer)?;
                updated += 1;
            }
        }

        if updated > 0 {
            info!(
                "[PEER BOOK] Marked {} peer(s) '{}' as trusted={}",
                updated, node_tag, trusted
            );
        }

        Ok(updated)
    }

    /// Get all trusted peers
    pub fn get_trusted(&self) -> Vec<VisionPeer> {
        self.all().into_iter().filter(|p| p.trusted).collect()
    }

    /// Auto-promote peers to trusted based on a predicate function
    pub fn auto_mark_trusted<F>(&self, mut is_good: F) -> Result<usize>
    where
        F: FnMut(&VisionPeer) -> bool,
    {
        let peers = self.all();
        let mut promoted = 0;

        for mut peer in peers {
            if is_good(&peer) && !peer.trusted {
                peer.trusted = true;
                self.save(&peer)?;
                promoted += 1;
                info!(
                    "[PEER BOOK] Auto-promoted peer to trusted: {} ({})",
                    peer.node_tag, peer.vision_address
                );
            }
        }

        Ok(promoted)
    }

    // ========================================================================
    // HTTP SEED PEER EXPORT (v2.7.0+)
    // ========================================================================

    /// Export healthy peers for HTTP seed distribution
    ///
    /// Returns up to `max` healthy peers, prioritizing anchors first.
    /// Used by GET /api/p2p/seed_peers endpoint on port 7070.
    ///
    /// # Peer Selection Criteria
    /// - Health score >= 30
    /// - Seen within last 7 days
    /// - Has valid IP address
    /// - Not banned or graylisted
    /// - Anchors sorted first, then by health score
    pub fn export_seed_peers(&self, max: usize) -> Vec<PublicPeerInfo> {
        let now = chrono::Utc::now().timestamp();
        let max_age = 7 * 86400; // 7 days in seconds

        let mut peers: Vec<PublicPeerInfo> = self
            .all()
            .into_iter()
            .filter(|p| {
                // Only export healthy, recent peers with IP addresses
                p.health_score >= 30
                    && (now - p.last_seen) < max_age
                    && p.ip_address.is_some()
                    && p.banned_until.is_none()
                    && p.graylisted_until.is_none()
            })
            .filter_map(|p| {
                // Extract IP and port from ip_address field
                p.ip_address.map(|ip_addr| {
                    // ip_addr format can be "ip:port" or just "ip"
                    // We need to construct P2P address with port 7072
                    let address = if ip_addr.contains(':') {
                        ip_addr // Already has port
                    } else {
                        format!("{}:7072", ip_addr) // Add default P2P port
                    };

                    PublicPeerInfo {
                        address,
                        is_anchor: p.is_anchor,
                    }
                })
            })
            .collect();

        // Sort: anchors first, then by health (implicit from healthy filter)
        peers.sort_by(|a, b| b.is_anchor.cmp(&a.is_anchor));

        // Limit to requested max
        peers.truncate(max);

        // Fallback: If no seeds qualify (early network bootstrap), return top healthy peers
        if peers.is_empty() {
            debug!("[PEER BOOK] No seeds meet strict criteria, falling back to top healthy peers");

            let fallback_peers: Vec<PublicPeerInfo> = self
                .all()
                .into_iter()
                .filter(|p| {
                    // Relaxed criteria: any peer with IP that's not banned
                    p.ip_address.is_some() && p.banned_until.is_none() && p.health_score > 0
                })
                .take(max)
                .filter_map(|p| {
                    p.ip_address.map(|ip_addr| {
                        let address = if ip_addr.contains(':') {
                            ip_addr
                        } else {
                            format!("{}:7072", ip_addr)
                        };
                        PublicPeerInfo {
                            address,
                            is_anchor: p.is_anchor,
                        }
                    })
                })
                .collect();

            debug!(
                "[PEER BOOK] Exported {} fallback seed peers ({} anchors)",
                fallback_peers.len(),
                fallback_peers.iter().filter(|p| p.is_anchor).count()
            );

            return fallback_peers;
        }

        debug!(
            "[PEER BOOK] Exported {} seed peers for HTTP distribution ({} anchors)",
            peers.len(),
            peers.iter().filter(|p| p.is_anchor).count()
        );

        peers
    }

    /// Sample a curated list of public P2P peers for handshake seeding.
    ///
    /// Rules:
    /// - Public routable only (no 192.168/10/172/loopback/link-local/multicast)
    /// - P2P endpoints only: ip:7072
    /// - No duplicates (dedupe by IP)
    /// - Deterministic-ish order (anchor first, then health/recency)
    pub fn sample_public_peers(&self, limit: usize) -> Vec<String> {
        use std::collections::HashSet;
        use std::net::{IpAddr, SocketAddr};

        if limit == 0 {
            return Vec::new();
        }

        let mut peers = self.all();

        // Sort: anchors first, then health, then success/recency.
        peers.sort_by_key(|p| {
            (
                Reverse(p.is_anchor),
                Reverse(p.health_score),
                Reverse(p.last_success),
                Reverse(p.last_seen),
            )
        });

        let mut out: Vec<String> = Vec::new();
        let mut seen: HashSet<IpAddr> = HashSet::new();

        for p in peers {
            if out.len() >= limit {
                break;
            }

            let Some(addr_str) = p.ip_address.as_deref() else {
                continue;
            };
            let Some(ip_str) = crate::p2p::ip_filter::extract_ip_from_addr(addr_str) else {
                continue;
            };
            let Ok(ip) = ip_str.parse::<IpAddr>() else {
                continue;
            };

            // Conservative: handshake seeds are IPv4-only.
            let IpAddr::V4(v4) = ip else {
                continue;
            };
            if crate::p2p::ip_filter::is_private_ipv4(&v4) {
                continue;
            }

            if !seen.insert(IpAddr::V4(v4)) {
                continue;
            }

            let sock = SocketAddr::new(IpAddr::V4(v4), 7072);
            out.push(sock.to_string());
        }

        out
    }

    /// Upsert peer from HTTP seed fetch (used during bootstrap)
    ///
    /// Creates or updates a peer entry based on HTTP-distributed seed data.
    /// Sets health score to 50 (neutral) for new HTTP-discovered peers.
    pub fn upsert_peer_from_http(&self, ip: String, port: u16, is_anchor: bool) -> Result<()> {
        let address = format!("{}:{}", ip, port);

        // Local test mode: validate IP is local-allowed
        if crate::p2p::ip_filter::local_test_mode() {
            let sock_addr: std::net::SocketAddr = match address.parse() {
                Ok(a) => a,
                Err(_) => return Err(anyhow::anyhow!("Invalid address format: {}", address)),
            };
            if !crate::p2p::ip_filter::is_local_allowed(&sock_addr) {
                debug!(
                    "[PEER BOOK] Local test mode: rejecting non-local HTTP seed peer: {}",
                    address
                );
                return Err(anyhow::anyhow!(
                    "Non-local peer rejected in local test mode"
                ));
            }
        }

        // Validate IP for storage (already enforces local test mode + private IP rules)
        if !crate::p2p::ip_filter::validate_ip_for_storage(&address) {
            return Err(anyhow::anyhow!("IP not valid for storage: {}", address));
        }

        let node_id = format!("http-seed-{}", address); // Temporary ID until handshake

        // Check if we already have this peer by IP
        let existing = self.all().into_iter().find(|p| {
            p.ip_address
                .as_ref()
                .map(|a| a.starts_with(&ip))
                .unwrap_or(false)
        });

        if let Some(mut peer) = existing {
            // Update existing peer
            peer.ip_address = Some(address);
            peer.is_anchor = is_anchor || peer.is_anchor; // Preserve anchor status
            peer.last_seen = chrono::Utc::now().timestamp();
            self.save(&peer)?;

            debug!(
                "[PEER BOOK] Updated peer from HTTP seed: {} (anchor={})",
                peer.node_tag, is_anchor
            );
        } else {
            // Create new peer entry (will be replaced during handshake)
            let now = chrono::Utc::now().timestamp() as u64;
            let mut new_peer = VisionPeer::new(
                node_id.clone(),
                format!("SEED-{}", &ip[ip.len().saturating_sub(12)..]), // Temp tag from IP suffix
                String::new(),                                          // No pubkey yet
                format!("vision://temp-seed@{}", node_id),
                None,
                "constellation".to_string(),
            );

            new_peer.ip_address = Some(address.clone());
            new_peer.is_anchor = is_anchor;
            new_peer.health_score = 50; // Neutral starting health
            new_peer.last_seen = now as i64;

            self.save(&new_peer)?;

            debug!(
                "[PEER BOOK] Added new peer from HTTP seed: {} (anchor={})",
                address, is_anchor
            );
        }

        Ok(())
    }

    // ========================================================================
    // ROLLING 1000-PEER MESH IMPLEMENTATION
    // ========================================================================

    /// Upsert peer with automatic capacity enforcement (rolling mesh)
    ///
    /// If peer exists: updates address, health stats (keeps best scores)
    /// If new: inserts (unlimited growth - no eviction)
    pub fn upsert_peer(&self, mut new_peer: VisionPeer) -> Result<()> {
        // Check if peer already exists
        if let Some(existing) = self.get(&new_peer.node_id) {
            // Merge: keep best health score, most recent timestamps
            new_peer.health_score = new_peer.health_score.max(existing.health_score);
            new_peer.last_success = new_peer.last_success.max(existing.last_success);
            new_peer.last_seen = new_peer.last_seen.max(existing.last_seen);

            // Keep seed status if already marked
            new_peer.is_seed = new_peer.is_seed || existing.is_seed;

            // Update IP if new one provided
            if new_peer.ip_address.is_none() {
                new_peer.ip_address = existing.ip_address;
            }

            debug!(
                "[PEER BOOK] Updating peer {} (health: {} -> {})",
                new_peer.node_tag, existing.health_score, new_peer.health_score
            );
        }

        // Save peer
        self.save(&new_peer)?;

        // Enforce capacity (evict worst peers if over limit)
        self.enforce_capacity()?;

        Ok(())
    }

    /// Capacity enforcement DISABLED - unlimited peer book growth
    /// Peers are never evicted automatically
    fn enforce_capacity(&self) -> Result<()> {
        // UNLIMITED GROWTH: No capacity enforcement
        // Peer book grows forever - true full mesh networking
        // Only manual pruning via admin API can remove peers
        Ok(())
    }

    /// Delete a peer by node_id
    fn delete_peer(&self, node_id: &str) -> Result<()> {
        let key = self.scoped_key(node_id);
        self.db.remove(key.as_bytes())?;
        Ok(())
    }

    /// Get best healthy peers for bootstrap (sorted by health + recency)
    ///
    /// Returns top N peers with health >= min_health, sorted by:
    /// 1. Health score (descending)
    /// 2. Last success timestamp (descending)
    pub fn get_best_peers(&self, limit: usize, min_health: i32) -> Vec<VisionPeer> {
        let mut peers = self
            .all()
            .into_iter()
            .filter(|p| p.health_score >= min_health)
            .collect::<Vec<_>>();

        // Sort by health (desc), then last_success (desc)
        peers.sort_by_key(|p| (Reverse(p.health_score), Reverse(p.last_success)));

        peers.into_iter().take(limit).collect()
    }

    /// Mark peer success by node_id
    pub fn mark_peer_success(&self, node_id: &str, now: u64) -> Result<()> {
        if let Some(mut peer) = self.get(node_id) {
            peer.mark_success(now);
            self.save(&peer)?;

            debug!(
                "[PEER BOOK] Success: {} (health={})",
                peer.node_tag, peer.health_score
            );
        }
        Ok(())
    }

    /// Mark peer failure by node_id
    pub fn mark_peer_failure(&self, node_id: &str, now: u64) -> Result<()> {
        if let Some(mut peer) = self.get(node_id) {
            peer.mark_failure(now);

            // Hard evict if health/failures exceed limits
            if peer.should_hard_evict() {
                warn!(
                    "[PEER BOOK] Hard evicting {} (health={}, fails={})",
                    peer.node_tag, peer.health_score, peer.fail_count
                );
                self.delete_peer(node_id)?;
            } else {
                self.save(&peer)?;
                debug!(
                    "[PEER BOOK] Failure: {} (health={}, fails={})",
                    peer.node_tag, peer.health_score, peer.fail_count
                );
            }
        }
        Ok(())
    }

    // ========================================================================
    // ROUTING INTELLIGENCE (Phase 3.5)
    // ========================================================================

    /// Update peer latency measurement
    pub fn update_peer_latency(&self, node_id: &str, rtt_ms: u32, ok: bool) -> Result<()> {
        if let Some(mut peer) = self.get(node_id) {
            peer.update_latency(rtt_ms);
            peer.update_reliability(ok);
            self.save(&peer)?;

            debug!(
                "[PEER BOOK] Latency: {} = {}ms (avg: {}ms, bucket: {:?}, reliability: {:.2})",
                peer.node_tag,
                rtt_ms,
                peer.avg_rtt_ms.unwrap_or(0),
                peer.latency_bucket,
                peer.reliability_score
            );
        }
        Ok(())
    }

    /// Calculate routing score for a peer (0.0 - 100.0+)
    /// Higher score = better for routing
    pub fn routing_score(&self, peer: &VisionPeer, local_region: Option<&str>) -> f32 {
        // Phase 4: Exclude banned/graylisted peers immediately
        use crate::p2p::reputation::{
            is_excluded_from_routing, reputation_factor, route_performance_score,
        };

        if is_excluded_from_routing(peer) {
            return -1000.0; // Cannot route through banned/graylisted peers
        }

        let mut score = 0.0_f32;

        // 1) Base reliability (0-50 points)
        score += peer.reliability_score * 50.0;

        // 2) Latency contribution (0-30 points)
        if let Some(avg) = peer.avg_rtt_ms {
            let latency_score = (300.0_f32 - avg as f32).max(0.0) / 10.0;
            score += latency_score;
        }

        // 3) Region match bonus (0-15 points)
        if let (Some(local), Some(peer_region)) = (local_region, &peer.region) {
            if peer_region.starts_with(local) {
                score += 15.0; // Same region bonus
            }
        }

        // 4) Role-based routing bonus (0-20 points)
        match peer.role.as_str() {
            "guardian" => score += 20.0,
            "anchor" => score += 10.0,
            _ => {}
        }

        // 5) Health score contribution (0-10 points)
        score += (peer.health_score as f32 / 10.0).min(10.0);

        // 6) Penalty for excessive failures (-10 to 0 points)
        if peer.fail_count > 5 {
            score -= ((peer.fail_count - 5) as f32).min(10.0);
        }

        // 7) Trusted peer bonus (0-10 points)
        if peer.trusted {
            score += 10.0;
        }

        // ========================================================================
        // Phase 4: Reputation & Route Learning Integration
        // ========================================================================

        // 8) Route learning performance (0-25 points)
        score += route_performance_score(peer);

        // 9) Trust level penalties
        match peer.trust_level {
            PeerTrustLevel::Probation => score -= 10.0,
            PeerTrustLevel::Graylisted => score -= 40.0, // Should never reach here due to early exit
            PeerTrustLevel::Banned => score -= 1000.0, // Should never reach here due to early exit
            _ => {}
        }

        // 10) Apply reputation multiplier (0.1x to 1.2x)
        score *= reputation_factor(peer);

        score
    }

    /// Get best routing peers sorted by score
    pub fn best_routing_peers(&self, max: usize, local_region: Option<&str>) -> Vec<VisionPeer> {
        let mut peers = self.all();

        // Sort by routing score descending
        peers.sort_by(|a, b| {
            let sa = self.routing_score(a, local_region);
            let sb = self.routing_score(b, local_region);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        peers.into_iter().take(max).collect()
    }

    /// Classify peers into routing rings (Inner/Middle/Outer)
    pub fn classify_peers_for_routing(&self, local_region: Option<&str>) -> Vec<ClassifiedPeer> {
        let mut result = Vec::new();

        for peer in self.all() {
            let score = self.routing_score(&peer, local_region);
            let ring = classify_ring(&peer, local_region);
            result.push(ClassifiedPeer { peer, ring, score });
        }

        // Sort by score descending
        result.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        result
    }

    /// Get peer book statistics for monitoring
    pub fn get_stats(&self) -> PeerBookStats {
        let peers = self.all();
        let total = peers.len();
        let seeds = peers.iter().filter(|p| p.is_seed).count();
        let avg_health = if total > 0 {
            peers.iter().map(|p| p.health_score as f32).sum::<f32>() / total as f32
        } else {
            0.0
        };

        // Get top 5 peers for sample
        let top_sample = self
            .get_best_peers(5, 0)
            .into_iter()
            .map(|p| PeerSample {
                addr: p.ip_address.unwrap_or_else(|| p.vision_address.clone()),
                health: p.health_score,
                node_tag: p.node_tag,
            })
            .collect();

        PeerBookStats {
            total,
            seeds,
            avg_health,
            top_sample,
        }
    }

    // ========================================================================
    // PEER HIERARCHY PROMOTION (Sync Stagnation Fix)
    // ========================================================================

    /// Promote peer to WARM tier (reliable sync provider)
    pub fn promote_to_warm(&self, peer_id: &str) -> Result<()> {
        if let Some(mut peer) = self.get(peer_id) {
            if peer.peer_tier == PeerTier::Hot {
                peer.peer_tier = PeerTier::Warm;
                peer.last_promotion = Some(chrono::Utc::now().timestamp());
                self.save(&peer)?;

                info!(
                    "🌡 Peer {} promoted to WARM (reliable sync provider)",
                    peer.node_tag
                );
            }
        }
        Ok(())
    }

    /// Promote peer to ANCHOR tier (publicly reachable warm peer - serves the swarm)
    pub fn promote_to_anchor(&self, peer_id: &str) -> Result<()> {
        if let Some(mut peer) = self.get(peer_id) {
            if peer.peer_tier == PeerTier::Warm && peer.public_reachable {
                peer.peer_tier = PeerTier::Anchor;
                peer.last_promotion = Some(chrono::Utc::now().timestamp());
                self.save(&peer)?;

                info!(
                    "🚀 Peer {} promoted to ANCHOR — now serving the swarm",
                    peer.node_tag
                );
            } else if peer.peer_tier != PeerTier::Warm {
                warn!(
                    "[PEER HIERARCHY] Cannot promote {} to anchor: must be warm first",
                    peer.node_tag
                );
            } else if !peer.public_reachable {
                warn!(
                    "[PEER HIERARCHY] Cannot promote {} to anchor: not publicly reachable",
                    peer.node_tag
                );
            }
        }
        Ok(())
    }

    /// Get best peer for sync (prefer anchor > warm > hot, then by height/latency)
    pub fn get_best_sync_peer(&self) -> Option<VisionPeer> {
        let mut peers = self.all();

        // Filter out banned/graylisted peers
        peers.retain(|p| {
            p.trust_level != PeerTrustLevel::Banned && p.trust_level != PeerTrustLevel::Graylisted
        });

        if peers.is_empty() {
            return None;
        }

        // Sort by: tier (anchor > warm > hot), then health, then latency
        peers.sort_by(|a, b| {
            // First compare tier (higher is better)
            let tier_cmp = match (a.peer_tier, b.peer_tier) {
                (PeerTier::Anchor, PeerTier::Anchor) => std::cmp::Ordering::Equal,
                (PeerTier::Anchor, _) => std::cmp::Ordering::Greater,
                (_, PeerTier::Anchor) => std::cmp::Ordering::Less,
                (PeerTier::Warm, PeerTier::Warm) => std::cmp::Ordering::Equal,
                (PeerTier::Warm, _) => std::cmp::Ordering::Greater,
                (_, PeerTier::Warm) => std::cmp::Ordering::Less,
                _ => std::cmp::Ordering::Equal,
            };

            if tier_cmp != std::cmp::Ordering::Equal {
                return tier_cmp;
            }

            // Then compare health score
            let health_cmp = b.health_score.cmp(&a.health_score);
            if health_cmp != std::cmp::Ordering::Equal {
                return health_cmp;
            }

            // Finally compare latency (lower is better)
            match (a.avg_rtt_ms, b.avg_rtt_ms) {
                (Some(a_lat), Some(b_lat)) => a_lat.cmp(&b_lat),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        peers.into_iter().next()
    }

    /// Elect best peer to warm if no warm/anchor peers exist (fallback)
    pub fn auto_elect_warm_peer(&self) -> Result<()> {
        // Check if we have any warm or anchor peers
        let has_elevated = self
            .all()
            .iter()
            .any(|p| p.peer_tier == PeerTier::Warm || p.peer_tier == PeerTier::Anchor);

        if has_elevated {
            return Ok(()); // Already have elevated peers
        }

        // Find best candidate: highest health + lowest latency
        if let Some(best) = self.get_best_sync_peer() {
            info!(
                "[PEER HIERARCHY] No warm/anchor peers found - auto-electing: {}",
                best.node_tag
            );
            self.promote_to_warm(&best.node_id)?;
        }

        Ok(())
    }
}

/// Peer book statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBookStats {
    pub total: usize,
    pub seeds: usize,
    pub avg_health: f32,
    pub top_sample: Vec<PeerSample>,
}

/// Sample peer for top peers list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSample {
    pub addr: String,
    pub health: i32,
    pub node_tag: String,
}

/// Compute Blake3 fingerprint of admission ticket (first 6 bytes)
pub fn compute_ticket_fingerprint(ticket: &str) -> String {
    let hash = blake3::hash(ticket.as_bytes());
    hex::encode(&hash.as_bytes()[0..6])
}

/// Generate Vision address from node tag and public key
pub fn generate_vision_address(node_tag: &str, public_key: &str) -> String {
    let public_key_hash = blake3::hash(public_key.as_bytes());
    let short_hash = hex::encode(&public_key_hash.as_bytes()[0..10]);
    format!("vision://{}@{}", node_tag, short_hash)
}

// ============================================================================
// ROUTING RING CLASSIFICATION (Phase 3.5)
// ============================================================================

/// Peer routing ring classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerRing {
    Inner,  // Local cluster: same region, low latency
    Middle, // Regional backup: same continent, higher latency
    Outer,  // Global backbone: cross-continent, guardians/anchors
}

/// Classified peer with routing metadata
#[derive(Debug, Clone)]
pub struct ClassifiedPeer {
    pub peer: VisionPeer,
    pub ring: PeerRing,
    pub score: f32,
}

/// Classify a peer into a routing ring
fn classify_ring(peer: &VisionPeer, local_region: Option<&str>) -> PeerRing {
    let same_region = match (local_region, &peer.region) {
        (Some(l), Some(r)) => r.starts_with(l),
        _ => false,
    };

    let avg = peer.avg_rtt_ms.unwrap_or(200);

    // Inner ring: same region AND low latency
    if same_region && avg <= 100 {
        PeerRing::Inner
    } else if same_region {
        // Middle ring: same region but higher latency
        PeerRing::Middle
    } else {
        // Outer ring: different region (global backbone)
        PeerRing::Outer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_address_generation() {
        let addr = generate_vision_address("VNODE-TEST-1234", "04abcdef1234567890");
        assert!(addr.starts_with("vision://VNODE-TEST-1234@"));
        assert_eq!(addr.len(), "vision://VNODE-TEST-1234@".len() + 20); // 10 bytes = 20 hex chars
    }

    #[test]
    fn test_ticket_fingerprint() {
        let ticket = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature";
        let fingerprint = compute_ticket_fingerprint(ticket);
        assert_eq!(fingerprint.len(), 12); // 6 bytes = 12 hex chars
    }

    #[test]
    fn test_peer_is_recent() {
        let mut peer = VisionPeer::new(
            "test-node-1".to_string(),
            "VNODE-TEST-1234".to_string(),
            "pubkey".to_string(),
            "vision://test@hash".to_string(),
            None,
            "constellation".to_string(),
        );

        assert!(peer.is_recent());

        // Set to 2 days ago
        peer.last_seen = chrono::Utc::now().timestamp() - (2 * 86400);
        assert!(!peer.is_recent());
    }
}
