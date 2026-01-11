/// Vision Genesis Seed Peers
///
/// Static seed peer list for offline-first P2P bootstrap
/// These peers form the genesis mesh and are marked as protected seeds
/// NOTE: IPv4-only P2P for initial testnet ignition
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};
use std::sync::Arc;
use crate::globals::P2P_MANAGER;

/// Hardcoded genesis seed peers - IPv4 only, pre-trusted
/// All seeds on port 7072 (P2P port)
/// v1.0.0 MAINNET LAUNCH - Decentralized Mainnet Seeds
pub const INITIAL_SEEDS: &[(&str, u16)] = &[
    ("35.151.236.81", 7072),  // Seed 1
    ("16.163.123.221", 7072), // Seed 2
    ("69.173.206.211", 7072), // Seed 3
    ("75.128.156.69", 7072),  // Seed 4
    ("98.97.137.74", 7072),   // Seed 5
    ("182.106.66.15", 7072),  // Seed 6
    ("69.173.206.46", 7072),  // Seed 7 (new)
    ("68.142.62.22", 7072),   // Seed 8 (new)
];

/// Get default anchor seeds for 7070 HTTP control plane
/// Returns the same IPs from INITIAL_SEEDS (without ports)
/// The control plane will add :7070 automatically
pub fn default_anchor_seeds() -> Vec<String> {
    INITIAL_SEEDS
        .iter()
        .map(|(ip, _port)| ip.to_string())
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedPeerConfig {
    pub version: u32,
    pub generated_at: String,
    pub description: String,
    pub peers: Vec<String>,
}

impl Default for SeedPeerConfig {
    fn default() -> Self {
        Self {
            version: 1,
            generated_at: chrono::Utc::now().to_rfc3339(),
            description: "Vision v1.0.0 Mainnet Seeds - Decentralized Launch".to_string(),
            // v1.0.0 MAINNET LAUNCH - Decentralized Mainnet Seeds
            peers: vec![
                "35.151.236.81:7072".to_string(),
                "16.163.123.221:7072".to_string(),
                "69.173.206.211:7072".to_string(),
                "69.173.207.135:7072".to_string(),
                "75.128.156.69:7072".to_string(),
                "98.97.137.74:7072".to_string(),
                "182.106.66.15:7072".to_string(),
            ],
        }
    }
}

impl SeedPeerConfig {
    fn home_seed_path() -> Option<PathBuf> {
        home_dir().map(|home| {
            home.join("vision-node")
                .join("vision_data")
                .join("seed_peers.json")
        })
    }

    /// Get the primary seed peers file path
    fn get_primary_path() -> PathBuf {
        // Try multiple locations in order
        let mut paths = vec![
            PathBuf::from("./vision_data/seed_peers.json"),
            PathBuf::from("./seed_peers.json"),
            PathBuf::from("/opt/vision-node/vision_data/seed_peers.json"),
        ];

        if let Some(home_path) = Self::home_seed_path() {
            paths.push(home_path);
        }

        for path in &paths {
            if path.exists() {
                return path.clone();
            }
        }

        // Default to local seed_peers.json if none exist
        PathBuf::from("./seed_peers.json")
    }

    /// Load seed peers from vision_data/seed_peers.json
    /// Falls back to hardcoded genesis peers if file not found
    pub fn load() -> Self {
        let mut paths = vec![
            PathBuf::from("./vision_data/seed_peers.json"),
            PathBuf::from("./seed_peers.json"),
            PathBuf::from("/opt/vision-node/vision_data/seed_peers.json"),
        ];

        if let Some(home_path) = Self::home_seed_path() {
            paths.push(home_path);
        }

        for path in paths {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<SeedPeerConfig>(&content) {
                            Ok(config) => {
                                if config.peers.is_empty() {
                                    warn!("[P2P] No seeds configured — swarm discovery will be limited.");
                                    if cfg!(feature = "full") {
                                        info!("[SEED_PEERS] Falling back to hardcoded seeds for mainnet build");
                                        return Self::default();
                                    }
                                }

                                info!(
                                    "[SEED_PEERS] ✅ Loaded {} seed peers from {}",
                                    config.peers.len(),
                                    path.display()
                                );
                                return config;
                            }
                            Err(e) => {
                                warn!("[SEED_PEERS] Failed to parse {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("[SEED_PEERS] Could not read {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!("[SEED_PEERS] Using hardcoded genesis seeds (seed_peers.json not found)");
        Self::default()
    }

    /// Save seed peers to seed_peers.json
    /// Merges with existing peers to avoid duplicates
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_primary_path();

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            }
        }

        // Serialize to JSON
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        // Write to file
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;

        debug!(
            "[SEED_PEERS] 💾 Saved {} peers to {}",
            self.peers.len(),
            path.display()
        );

        Ok(())
    }

    /// Add a new peer address to the seed list (if not duplicate)
    /// Returns true if peer was added, false if it already existed
    pub fn add_peer(&mut self, peer_addr: String) -> bool {
        // Check if peer already exists
        if self.peers.contains(&peer_addr) {
            return false;
        }

        // Add new peer
        self.peers.push(peer_addr.clone());
        self.generated_at = chrono::Utc::now().to_rfc3339();

        info!("[SEED_PEERS] 🌱 Added new peer to seed book: {}", peer_addr);

        true
    }

    /// Add multiple peers at once (deduplicates automatically)
    /// Returns count of new peers added
    pub fn add_peers(&mut self, new_peers: Vec<String>) -> usize {
        let mut added = 0;

        for peer_addr in new_peers {
            if self.add_peer(peer_addr) {
                added += 1;
            }
        }

        added
    }

    /// Get all seed peer addresses
    pub fn get_seeds(&self) -> Vec<String> {
        self.peers.clone()
    }

    /// Parse anchor seeds from VISION_ANCHOR_SEEDS environment variable.
    ///
    /// Anchors are HTTP-only (port 7070). This returns host/IP values only
    /// (no scheme, no port), suitable for constructing `http://<host>:7070`.
    ///
    /// Supported input formats per entry:
    /// - `ip`
    /// - `ip:7070`
    /// - `http://ip:7070` / `https://ip:7070`
    pub fn parse_anchor_seeds_from_env() -> Vec<String> {
        fn normalize_anchor_host(raw: &str) -> Option<String> {
            let mut s = raw.trim();
            if s.is_empty() {
                return None;
            }

            if let Some(rest) = s.strip_prefix("http://") {
                s = rest;
            } else if let Some(rest) = s.strip_prefix("https://") {
                s = rest;
            }

            // Trim any path/query fragment
            if let Some((host_port, _path)) = s.split_once('/') {
                s = host_port;
            }

            // If a port is present (e.g. :7070 or :7072), strip it.
            // Anchors are HTTP-only; callers will construct :7070.
            if let Some((host, _port)) = s.rsplit_once(':') {
                let host = host.trim();
                if !host.is_empty() {
                    return Some(host.to_string());
                }
                return None;
            }

            Some(s.to_string())
        }

        let Ok(raw) = std::env::var("VISION_ANCHOR_SEEDS") else {
            return Vec::new();
        };

        raw.split(',').filter_map(normalize_anchor_host).collect()
    }

    /// Get P2P seed peers.
    ///
    /// Note: `VISION_ANCHOR_SEEDS` is HTTP-only and must NOT be treated as P2P peers.
    pub fn get_seeds_with_anchors(&self) -> Vec<String> {
        self.peers.clone()
    }

    /// Add seed peers to peer book with is_seed=true (protected from eviction)
    pub fn bootstrap_to_peer_book(
        &self,
        peer_store: &crate::p2p::peer_store::PeerStore,
    ) -> Result<usize, String> {
        let now = chrono::Utc::now().timestamp();
        let mut added = 0;

        for (idx, addr) in self.peers.iter().enumerate() {
            // Parse address to validate format
            if let Ok(sock_addr) = addr.parse::<std::net::SocketAddr>() {
                // Only accept IPv4 seeds
                if !sock_addr.is_ipv4() {
                    warn!("[SEED_PEERS] Skipping IPv6 seed: {}", addr);
                    continue;
                }

                let node_id = format!("genesis-seed-{}", idx);
                let node_tag = format!("GENESIS-SEED-{}", idx);

                let seed_peer = crate::p2p::peer_store::VisionPeer {
                    node_id: node_id.clone(),
                    node_tag: node_tag.clone(),
                    public_key: String::new(),
                    vision_address: format!("vision://{}@genesis", node_tag),
                    ip_address: Some(addr.clone()),
                    role: "seed".to_string(),
                    last_seen: now,
                    trusted: true, // Genesis seeds are pre-trusted
                    admission_ticket_fingerprint: String::new(),
                    mood: None,
                    // Rolling mesh health fields - seeds start with high health
                    health_score: 100, // Maximum health
                    last_success: now as u64,
                    last_failure: 0,
                    fail_count: 0,
                    is_seed: true,    // 🔥 PROTECTED FROM EVICTION
                    is_anchor: false, // Genesis seeds are not necessarily anchors
                    connection_status: "disconnected".to_string(),
                    // Phase 3.5: Latency & routing defaults
                    last_rtt_ms: None,
                    avg_rtt_ms: None,
                    latency_bucket: None,
                    reliability_score: 0.5,
                    success_count: 0,
                    region: None,
                    // Phase 4: Reputation defaults (seeds start trusted)
                    trust_level: crate::p2p::peer_store::PeerTrustLevel::Trusted,
                    reputation: 100.0, // Maximum reputation for seeds
                    misbehavior_score: 0.0,
                    graylisted_until: None,
                    banned_until: None,
                    total_invalid_msgs: 0,
                    total_protocol_violations: 0,
                    total_spam_events: 0,
                    // Phase 4: Route learning defaults
                    route_uses: 0,
                    route_successes: 0,
                    route_failures: 0,
                    avg_delivery_ms: None,
                    // Phase 5: Peer hierarchy defaults (seeds start as Warm for stability)
                    peer_tier: crate::p2p::peer_store::PeerTier::Warm,
                    last_promotion: Some(now),
                    public_reachable: true, // Genesis seeds are assumed publicly reachable
                };

                match peer_store.upsert_peer(seed_peer) {
                    Ok(_) => {
                        debug!("[SEED_PEERS] Added genesis seed: {} ({})", node_tag, addr);
                        added += 1;
                    }
                    Err(e) => {
                        warn!("[SEED_PEERS] Failed to add seed {}: {}", addr, e);
                    }
                }
            } else {
                warn!("[SEED_PEERS] Invalid seed address: {}", addr);
            }
        }

        if added > 0 {
            info!(
                "[SEED_PEERS] 🌱 Bootstrapped {} genesis seeds to peer book (protected from eviction)",
                added
            );
        }

        Ok(added)
    }
}

/// Bootstrap P2P connections from genesis seed peers
pub async fn bootstrap_from_seeds() -> Result<usize, String> {
    // use crate::P2P_MANAGER;  // TODO: P2P_MANAGER not defined
    use std::sync::Arc;

    let config = SeedPeerConfig::load();
    let seeds = config.get_seeds();

    if seeds.is_empty() {
        return Err("No seed peers available".to_string());
    }

    info!(
        "[SEED_BOOTSTRAP] 🌱 Starting genesis seed bootstrap ({} seeds)",
        seeds.len()
    );

    // Also add to peer book for future reconnections
    if let Some(chain) = crate::CHAIN.try_lock() {
        if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
            let _ = config.bootstrap_to_peer_book(&peer_store);
        }
    }

    let mut connected = 0;

    // Try connecting to each seed peer
    for addr in seeds {
        let p2p = Arc::clone(&*P2P_MANAGER);
        let _addr_clone = addr.clone();

        info!("[SEED_BOOTSTRAP] Connecting to seed: {}", addr);

        // Attempt connection with timeout and track result
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            p2p.connect_to_peer(addr.clone()),
        )
        .await
        {
            Ok(Ok(_)) => {
                info!("[SEED_BOOTSTRAP] ✅ Connected to seed: {}", addr);
                connected += 1;
            }
            Ok(Err(e)) => {
                debug!("[SEED_BOOTSTRAP] Failed to connect to seed {}: {}", addr, e);
            }
            Err(_) => {
                debug!("[SEED_BOOTSTRAP] Timeout connecting to seed: {}", addr);
            }
        }

        // Small delay between connection attempts
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!(
        "[SEED_BOOTSTRAP] 🚀 Initiated connections to {} seeds",
        connected
    );

    Ok(connected)
}

/// Periodic task to sync peer book to seed_peers.json
/// This ensures all discovered peers are persisted for future runs
/// Creates a true "pure swarm" where the seed list grows organically
pub async fn spawn_seed_sync_loop() {
    use tokio::time::{sleep, Duration};

    info!("[SEED_SYNC] 🌱 Starting periodic seed sync (every 5 minutes)");

    // Wait a bit before first sync to let initial connections establish
    sleep(Duration::from_secs(60)).await;

    loop {
        // Sync every 5 minutes
        sleep(Duration::from_secs(300)).await;

        // Get all peers from peer book
        let peer_addresses = {
            if let Some(chain) = crate::CHAIN.try_lock() {
                if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                    // Get all peers that have IP addresses
                    peer_store
                        .get_all()
                        .into_iter()
                        .filter_map(|p| p.ip_address)
                        .collect::<Vec<String>>()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        };

        if peer_addresses.is_empty() {
            debug!("[SEED_SYNC] No peers to sync");
            continue;
        }

        // Sync to seed_peers.json in background thread
        tokio::task::spawn_blocking({
            let peers = peer_addresses;
            move || {
                let mut config = SeedPeerConfig::load();
                let initial_count = config.peers.len();
                let added = config.add_peers(peers);

                if added > 0 {
                    match config.save() {
                        Ok(_) => {
                            info!(
                                "[SEED_SYNC] 💾 Synced peer book: {} new peers added (total: {} → {})",
                                added,
                                initial_count,
                                config.peers.len()
                            );
                        }
                        Err(e) => {
                            warn!("[SEED_SYNC] Failed to save seed peers: {}", e);
                        }
                    }
                } else {
                    debug!(
                        "[SEED_SYNC] ✅ Seed book up to date ({} peers)",
                        config.peers.len()
                    );
                }
            }
        });
    }
}
