#![allow(dead_code)]
//! TCP P2P connection management with persistent bidirectional connections
//!
//! This module implements a proper P2P network layer where:
//! - Each peer has a persistent TCP connection (not HTTP request/response)
//! - Connections are bidirectional (both nodes can send/receive on same socket)
//! - Messages are length-prefixed for streaming
//! - Handshake establishes peer identity and chain height
//! - Connection failures automatically cleanup and remove dead peers
//!
//! TODO: Ensure every successful P2P handshake updates the Vision Peer Book.
//!
//! Requirements:
//! 1. After a peer passes all handshake validation (genesis, ticket, etc.),
//!    create or update a VisionPeer in sled using functions from peer_store.rs.
//!
//! 2. The VisionPeer should include:
//!      - node_id
//!      - node_tag
//!      - vision_address
//!      - role (guardian/constellation)
//!      - last_seen (unix timestamp)
//!      - admission_ticket_fingerprint (if available)
//!
//! 3. Log:
//!    [PEERBOOK] Saved peer {node_tag} ({vision_address}) as trusted peer.
//!
//! 4. Ensure this runs for BOTH directions:
//!      - When _we_ connect out to them
//!      - When they connect inbound to us
//!
//! Implement this in the handshake success path where we already have HandshakeMessage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::globals::{EBID_MANAGER, GUARDIAN_ROLE, P2P_MANAGER};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

static CONNECTION_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

fn is_public_seed_addr(addr: &std::net::SocketAddr) -> bool {
    // Local test mode: allow any local/loopback/RFC1918 address (any port)
    if crate::p2p::ip_filter::local_test_mode() {
        return crate::p2p::ip_filter::is_local_allowed(addr);
    }

    // Handshake seed peers MUST be public routable (no LAN/loopback/link-local).
    // Stay conservative: accept IPv4 only.
    let std::net::SocketAddr::V4(v4) = addr else {
        return false;
    };

    let ip = v4.ip();
    if crate::p2p::ip_filter::is_private_ipv4(ip) {
        return false;
    }

    // Must be the P2P port.
    v4.port() == 7072
}

fn normalize_seed_endpoint(s: &str) -> Option<std::net::SocketAddr> {
    // Accept "ip", "ip:port", normalize to ip:7072.
    if let Ok(sock) = s.parse::<std::net::SocketAddr>() {
        let ip = sock.ip();
        return Some(std::net::SocketAddr::new(ip, 7072));
    }

    if let Ok(ip) = s.parse::<std::net::IpAddr>() {
        return Some(std::net::SocketAddr::new(ip, 7072));
    }

    None
}

async fn build_handshake_seed_peers(limit: usize) -> Vec<String> {
    use std::collections::HashSet;

    let mut out: Vec<String> = Vec::new();
    let mut seen_ip: HashSet<std::net::IpAddr> = HashSet::new();

    // Self P2P address (drop self from seeds)
    let self_addr: Option<String> = {
        let g = crate::ADVERTISED_P2P_ADDRESS.lock();
        g.clone()
    };

    // 1) Start with peer book samples
    let peer_book_samples = {
        let chain = crate::CHAIN.lock();
        crate::p2p::peer_store::PeerStore::new(&chain.db)
            .ok()
            .map(|store| store.sample_public_peers(limit))
            .unwrap_or_default()
    };

    for s in peer_book_samples {
        if out.len() >= limit {
            break;
        }
        if let Some(ref sa) = self_addr {
            if s == sa.as_str() {
                continue;
            }
        }
        let Some(sock) = normalize_seed_endpoint(&s) else {
            continue;
        };
        if !is_public_seed_addr(&sock) {
            continue;
        }
        if !seen_ip.insert(sock.ip()) {
            continue;
        }
        out.push(format!("{}:7072", sock.ip()));
    }

    // 2) Add currently connected peers (live view)
    let connected = crate::P2P_MANAGER.get_peer_addresses().await;
    for s in connected {
        if out.len() >= limit {
            break;
        }
        let Some(sock) = normalize_seed_endpoint(&s) else {
            continue;
        };
        if !is_public_seed_addr(&sock) {
            continue;
        }
        if let Some(ref sa) = self_addr {
            if format!("{}:7072", sock.ip()) == *sa {
                continue;
            }
        }
        if !seen_ip.insert(sock.ip()) {
            continue;
        }
        out.push(format!("{}:7072", sock.ip()));
    }

    out
}

fn ingest_handshake_seed_peers(handshake: &HandshakeMessage, from_peer: &std::net::SocketAddr) {
    use std::collections::HashSet;

    if handshake.seed_peers.is_empty() {
        return;
    }

    let mut seen_ip: HashSet<std::net::IpAddr> = HashSet::new();
    let mut ingested = 0usize;

    let store = {
        let chain = crate::CHAIN.lock();
        crate::p2p::peer_store::PeerStore::new(&chain.db).ok()
    };

    let Some(store) = store else {
        return;
    };

    for s in handshake.seed_peers.iter().take(64) {
        let Some(sock) = normalize_seed_endpoint(s) else {
            continue;
        };
        if !is_public_seed_addr(&sock) {
            continue;
        }
        if !seen_ip.insert(sock.ip()) {
            continue;
        }

        if store
            .upsert_peer_from_http(sock.ip().to_string(), 7072, false)
            .is_ok()
        {
            ingested += 1;
        }
    }

    if ingested > 0 {
        tracing::info!(target: "p2p",
            "[HANDSHAKE] Ingested {} seed peers from {}",
            ingested,
            from_peer
        );
    }
}

/// ⭐ P2P Protocol Version for Constellation
/// Protocol 2 = Ed25519 identity + Genesis launch + Block-height sunset
pub const VISION_P2P_PROTOCOL_VERSION: u32 = 2;

/// ⭐ Minimum supported protocol version (strict: must be 2)
pub const MIN_SUPPORTED_PROTOCOL_VERSION: u32 = 2;

/// ⭐ Node Version for MAINNET v1.0.0 (100 = v1.0.0)
pub const VISION_NODE_VERSION: u32 = 100;

/// ⭐ Node Build Tag - Must match for P2P compatibility
pub const NODE_BUILD_TAG: &str = crate::vision_constants::VISION_VERSION;

/// P2P protocol version - MUST match between nodes
#[cfg(feature = "full")]
const P2P_PROTOCOL_VERSION: u32 = crate::vision_constants::PROTOCOL_VERSION_FULL;
#[cfg(not(feature = "full"))]
const P2P_PROTOCOL_VERSION: u32 = crate::vision_constants::PROTOCOL_VERSION_LITE;

/// Minimum acceptable protocol version for backwards compatibility
const MIN_PROTOCOL_VERSION: u32 = P2P_PROTOCOL_VERSION;

/// Network identifier - MUST match between peers (MAINNET)
const NETWORK_ID: &str = crate::vision_constants::VISION_NETWORK_ID;

/// Maximum handshake message size (10KB should be plenty)
const MAX_HANDSHAKE_SIZE: u32 = 10_000;

/// Handshake protocol magic header - identifies Vision Network P2P protocol (9 bytes)
const P2P_HANDSHAKE_MAGIC: &[u8] = b"VISION-P2";

/// Handshake protocol version byte
///
/// IMPORTANT: When the bincode wire struct changes, this MUST be bumped.
pub const P2P_HANDSHAKE_VERSION: u8 = 3;

/// Maximum handshake payload size (u16 max = 65535)
const MAX_HANDSHAKE_PAYLOAD: u16 = 65535;

/// Handshake timeout in milliseconds (reject slow/scanning peers)
// Fix 4: Separate handshake timeout (12s) from normal message timeout (5s)
const HANDSHAKE_TIMEOUT_MS: u64 = 12000; // 12 seconds (handshakes can be slower)

/// Helper to sort IPv4 addresses before IPv6 for connection priority
fn sort_ipv4_first(peers: &mut Vec<SocketAddr>) {
    peers.sort_by(|a, b| {
        let av4 = a.is_ipv4();
        let bv4 = b.is_ipv4();
        match (av4, bv4) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        }
    });
}

/// Handshake message - sent first on connection (uses bincode for compact binary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    pub protocol_version: u32,  // Must be 2 for v2.2+
    pub chain_id: [u8; 32],     // Chain identifier hash
    pub genesis_hash: [u8; 32], // Genesis block hash (must match)
    pub node_nonce: u64,        // Random nonce to detect self-connections
    pub chain_height: u64,      // Current blockchain height
    pub node_version: u32,      // Software version (1.1.0 = 110)
    #[serde(default)]
    pub network_id: String, // Network identifier (e.g., "mainnet")
    #[serde(default)]
    pub node_build: String, // Build tag (e.g., "v2.2-constellation") for version gating
    // Vision Identity fields (Phase 6: Node passport system)
    pub node_tag: String, // Human-readable node identifier (e.g., "VNODE-ABCD-1234")
    pub admission_ticket: String, // Cryptographic admission proof (JWT-style signed token)
    #[serde(default)]
    pub passport: Option<crate::passport::NodePassport>, // Guardian-issued passport (Phase 6: decentralized P2P verification)
    // Vision Peer Book fields (Phase 7: Identity-based addressing)
    pub vision_address: String, // vision://node_tag@pubkey_hash
    pub node_id: String,        // Unique node identifier
    pub public_key: String,     // Hex-encoded public key
    pub role: String,           // "constellation" or "guardian"
    // Phase 6: Immortality fields
    #[serde(default)]
    pub ebid: String, // Eternal Broadcast ID - stable across restarts
    #[serde(default)]
    pub is_guardian: bool, // Currently elected guardian
    #[serde(default)]
    pub is_guardian_candidate: bool, // Eligible for guardian role
    #[serde(default)]
    pub http_api_port: Option<u16>, // HTTP API port for compact block fallback
    // Phase 10: P2P Reachability & Advertisement
    #[serde(default)]
    pub advertised_ip: Option<String>, // External IP for inbound connections
    #[serde(default)]
    pub advertised_port: Option<u16>, // P2P port for inbound connections
    // Phase 11: Bootstrap Checkpoint - Baked-in prefix for network quarantine
    #[serde(default)]
    pub bootstrap_checkpoint_height: u64, // Last baked-in block height (9 = 10 blocks)
    #[serde(default)]
    pub bootstrap_checkpoint_hash: String, // Hash at checkpoint height for prefix validation
    #[serde(default)]
    pub bootstrap_prefix: String, // Bootstrap identifier ("vision-constellation-bootstrap-1" for mainnet)

    /// Curated seed peers (public routable P2P endpoints, ip:7072)
    #[serde(default)]
    pub seed_peers: Vec<String>,
    
    /// Economics fingerprint - cryptographic hash of vault addresses and reward splits
    /// This MUST match across all nodes to prevent vault address tampering
    /// Reject connections from peers with mismatched econ_hash
    #[serde(default)]
    pub econ_hash: String, // Hex string (32 bytes) - canonical economics fingerprint
}

/// Handshake v3 wire format (framed via `P2P_HANDSHAKE_VERSION = 3`).
///
/// This keeps the on-wire handshake human-inspectable for chain identity fields,
/// while we still convert into the internal `HandshakeMessage` (byte arrays)
/// for strict validation and downstream logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandshakeWireV3 {
    pub protocol_version: u32,
    pub chain_id: String,     // hex string (32 bytes)
    pub genesis_hash: String, // hex string (32 bytes)
    pub node_nonce: u64,
    pub chain_height: u64,
    #[serde(default)]
    pub tip_height: Option<u64>,
    pub node_version: u32,
    #[serde(default)]
    pub network_id: String,
    #[serde(default)]
    pub node_build: String,

    pub node_tag: String,
    pub admission_ticket: String,
    #[serde(default)]
    pub passport: Option<crate::passport::NodePassport>,

    pub vision_address: String,
    pub node_id: String,
    pub public_key: String,
    pub role: String,

    #[serde(default)]
    pub ebid: String,
    #[serde(default)]
    pub is_guardian: bool,
    #[serde(default)]
    pub is_guardian_candidate: bool,
    #[serde(default)]
    pub http_api_port: Option<u16>,

    #[serde(default)]
    pub advertised_ip: Option<String>,
    #[serde(default)]
    pub advertised_port: Option<u16>,

    #[serde(default)]
    pub bootstrap_checkpoint_height: u64,
    #[serde(default)]
    pub bootstrap_checkpoint_hash: String,
    #[serde(default)]
    pub bootstrap_prefix: String,

    /// Curated seed peers (public routable P2P endpoints, ip:7072)
    #[serde(default)]
    pub seed_peers: Vec<String>,
    
    /// Economics fingerprint (vault addresses + splits consensus lock)
    #[serde(default)]
    pub econ_hash: Option<String>,
}

impl HandshakeWireV3 {
    fn from_internal(h: &HandshakeMessage) -> Self {
        HandshakeWireV3 {
            protocol_version: h.protocol_version,
            chain_id: hex::encode(h.chain_id),
            genesis_hash: hex::encode(h.genesis_hash),
            node_nonce: h.node_nonce,
            chain_height: h.chain_height,
            tip_height: Some(h.chain_height),
            node_version: h.node_version,
            network_id: h.network_id.clone(),
            node_build: h.node_build.clone(),

            node_tag: h.node_tag.clone(),
            admission_ticket: h.admission_ticket.clone(),
            passport: h.passport.clone(),

            vision_address: h.vision_address.clone(),
            node_id: h.node_id.clone(),
            public_key: h.public_key.clone(),
            role: h.role.clone(),

            ebid: h.ebid.clone(),
            is_guardian: h.is_guardian,
            is_guardian_candidate: h.is_guardian_candidate,
            http_api_port: h.http_api_port,

            advertised_ip: h.advertised_ip.clone(),
            advertised_port: h.advertised_port,

            bootstrap_checkpoint_height: h.bootstrap_checkpoint_height,
            bootstrap_checkpoint_hash: h.bootstrap_checkpoint_hash.clone(),
            bootstrap_prefix: h.bootstrap_prefix.clone(),

            seed_peers: h.seed_peers.clone(),
            econ_hash: if h.econ_hash.is_empty() { None } else { Some(h.econ_hash.clone()) },
        }
    }

    fn try_into_internal(self) -> Result<HandshakeMessage, String> {
        fn decode_32_hex(label: &str, s: &str) -> Result<[u8; 32], String> {
            let trimmed = s.trim().trim_start_matches("0x");
            let bytes =
                hex::decode(trimmed).map_err(|e| format!("Invalid {} hex: {}", label, e))?;
            if bytes.len() != 32 {
                return Err(format!(
                    "{} wrong length: {} (expected 32)",
                    label,
                    bytes.len()
                ));
            }
            let mut out = [0u8; 32];
            out.copy_from_slice(&bytes);
            Ok(out)
        }

        let chain_id = decode_32_hex("chain_id", &self.chain_id)?;
        let genesis_hash = decode_32_hex("genesis_hash", &self.genesis_hash)?;
        let chain_height = self.tip_height.unwrap_or(self.chain_height);

        Ok(HandshakeMessage {
            protocol_version: self.protocol_version,
            chain_id,
            genesis_hash,
            node_nonce: self.node_nonce,
            chain_height,
            node_version: self.node_version,
            network_id: self.network_id,
            node_build: self.node_build,
            node_tag: self.node_tag,
            admission_ticket: self.admission_ticket,
            passport: self.passport,
            vision_address: self.vision_address,
            node_id: self.node_id,
            public_key: self.public_key,
            role: self.role,
            ebid: self.ebid,
            is_guardian: self.is_guardian,
            is_guardian_candidate: self.is_guardian_candidate,
            http_api_port: self.http_api_port,
            advertised_ip: self.advertised_ip,
            advertised_port: self.advertised_port,
            bootstrap_checkpoint_height: self.bootstrap_checkpoint_height,
            bootstrap_checkpoint_hash: self.bootstrap_checkpoint_hash,
            bootstrap_prefix: self.bootstrap_prefix,
            seed_peers: self.seed_peers,
            econ_hash: self.econ_hash.clone().unwrap_or_default(), // Economics fingerprint for vault consensus
        })
    }
}

/// P2P message types sent over TCP (uses JSON for flexibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    /// Initial handshake when connection established (DEPRECATED - use HandshakeMessage)
    Handshake {
        protocol_version: u32,
        chain_id: String,
        genesis_hash: String,
        chain_height: u64,
        peer_id: String,
        node_version: String,
    },
    /// Ping for keepalive
    Ping { timestamp: u64 },
    /// Pong response
    Pong { timestamp: u64 },
    /// Compact block announcement
    CompactBlock {
        compact: super::compact::CompactBlock,
    },
    /// Full block (fallback if compact reconstruction fails)
    FullBlock { block: crate::Block },
    /// Transaction relay
    Transaction { tx: crate::Tx },
    /// Request blocks by height range
    GetBlocks { start_height: u64, end_height: u64 },
    /// Disconnect notification
    Disconnect { reason: String },
    /// Peer list exchange for discovery
    PeerList { peers: Vec<String> },
    /// Request peer list from peer
    GetPeers,
    /// Peer exchange request (P2P Robustness #3)
    PeerExchangeRequest,
    /// Peer exchange response with peer list (P2P Robustness #3)
    PeerExchangeResponse { peers: Vec<PeerExchangeInfo> },
    /// Miner tuning hint (P2P distributed learning)
    MinerTuningHint {
        hint: crate::miner::tuning_hint::MinerTuningHint,
    },
    /// Peer gossip for discovery (Phase 10)
    PeerGossip(super::peer_gossip::PeerGossipMessage),
    
    // ===== CHAIN SYNC MESSAGES (P2P-based sync) =====
    /// Request tip information from peer
    GetTip,
    /// Response with current chain tip
    Tip {
        height: u64,
        hash: String,
    },
    /// Request headers starting from a locator
    GetHeaders {
        locator_hashes: Vec<String>,
        max: u32,
    },
    /// Response with block headers
    Headers {
        headers: Vec<crate::BlockHeader>,
    },
    /// Request a specific block by hash
    GetBlock {
        hash: String,
    },
    /// Response with full block
    Block {
        block: crate::Block,
    },
    /// Request blocks by height range (convenience method)
    GetBlocksByRange {
        start_height: u64,
        max: u32,
    },
    /// Response with multiple blocks
    Blocks {
        blocks: Vec<crate::Block>,
    },
    /// Request block hash at specific height (for fork detection)
    GetBlockHash {
        height: u64,
    },
    /// Response with block hash at height
    BlockHash {
        height: u64,
        hash: Option<String>, // None if height doesn't exist
    },
}

/// Peer information for exchange (P2P Robustness #3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerExchangeInfo {
    pub addr: String,
    pub last_seen: u64,
    pub score: f64,
}

/// Direction of peer connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDirection {
    Inbound,  // Peer connected to us
    Outbound, // We connected to peer
}

/// Active peer connection with TCP stream
pub struct PeerConnection {
    /// Peer address (host:port)
    pub address: String,
    /// Peer's reported chain height
    pub height: u64,
    /// Peer identifier (public key or node ID)
    pub peer_id: String,
    /// Eternal Broadcast ID for PeerManager tracking
    pub ebid: String,
    /// Connection direction
    pub direction: ConnectionDirection,
    /// Unique connection instance id (guards against stale loops removing the winner)
    pub connection_id: u64,
    /// True only after handshake chain identity is validated
    pub validated_chain: bool,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// TCP write half (for sending messages)
    writer: Arc<Mutex<tokio::io::WriteHalf<TcpStream>>>,
}

impl PeerConnection {
    /// Send a message to this peer
    pub async fn send_message(&self, msg: P2PMessage) -> Result<(), String> {
        // Serialize message
        let data =
            serde_json::to_vec(&msg).map_err(|e| format!("Failed to serialize message: {}", e))?;

        // Length-prefixed format: [4 bytes length][message data]
        let len = data.len() as u32;
        let len_bytes = len.to_be_bytes();

        // Fix D: Enforce 5-second timeout on send operations to prevent frozen sends
        let send_result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let mut writer = self.writer.lock().await;
            writer
                .write_all(&len_bytes)
                .await
                .map_err(|e| format!("Failed to write length: {}", e))?;
            writer
                .write_all(&data)
                .await
                .map_err(|e| format!("Failed to write message: {}", e))?;
            writer
                .flush()
                .await
                .map_err(|e| format!("Failed to flush: {}", e))?;
            Ok::<(), String>(())
        })
        .await;

        match send_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("Send timeout (5s) - peer likely frozen or network issue".to_string()),
        }
    }
}

/// Global P2P connection manager
pub struct P2PConnectionManager {
    /// Active peer connections indexed by address
    peers: Arc<Mutex<HashMap<String, Arc<Mutex<PeerConnection>>>>>,
    /// Our node's peer ID
    node_id: String,
}

impl HandshakeMessage {
    /// Create handshake message from local chain state
    fn new() -> Result<Self, String> {
        let g = crate::CHAIN.lock();
        if g.blocks.is_empty() {
            return Err("No genesis block".to_string());
        }

        let genesis_hash_str = g.blocks[0].header.pow_hash.clone();

        // Use node_id as node_tag (Vision Identity removed)
        let node_id_for_tag = crate::P2P_MANAGER.get_node_id().to_string();
        let node_tag = format!("VNODE-{}", &node_id_for_tag[..8]);
        let admission_ticket = ""; // No longer using admission tickets
        drop(g);

        // Parse genesis hash from hex string to [u8; 32]
        let genesis_bytes =
            hex::decode(&genesis_hash_str).map_err(|e| format!("Invalid genesis hash: {}", e))?;
        if genesis_bytes.len() != 32 {
            return Err(format!(
                "Genesis hash wrong length: {}",
                genesis_bytes.len()
            ));
        }
        let mut genesis_hash = [0u8; 32];
        genesis_hash.copy_from_slice(&genesis_bytes);

        // Deterministic chain id (drop-specific). MUST be identical across machines.
        let chain_id = crate::vision_constants::expected_chain_id_bytes();

        let g = crate::CHAIN.lock();
        let chain_height = g.blocks.len() as u64;
        drop(g);

        // Generate random nonce
        let node_nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        // Vision address and public key no longer needed
        let vision_address = ""; // Vision Identity removed
        let public_key = ""; // Public key verification removed

        // Get node ID from P2P manager
        let node_id = P2P_MANAGER.get_node_id().to_string();

        // Determine role
        let is_guardian_mode = std::env::var("VISION_GUARDIAN_MODE")
            .unwrap_or_default()
            .to_lowercase()
            == "true";

        let role = if is_guardian_mode {
            "guardian".to_string()
        } else {
            "constellation".to_string()
        };

        // Phase 6: Get EBID from global manager
        let ebid = {
            let mgr = EBID_MANAGER.lock();
            mgr.get_ebid().to_string()
        };

        // Phase 6: Check if we're the current guardian
        let is_guardian = {
            let role_mgr = GUARDIAN_ROLE.lock();
            if let Some(current_guardian) = role_mgr.get_current_guardian() {
                current_guardian == ebid
            } else {
                is_guardian_mode // Fallback to mode setting if no guardian elected
            }
        };

        // Phase 6: Guardian candidate status (based on config flag)
        let is_guardian_candidate = std::env::var("VISION_GUARDIAN_CANDIDATE")
            .unwrap_or_default()
            .to_lowercase()
            == "true"
            || is_guardian_mode;

        // Get HTTP API port for compact block fallback
        let http_api_port = std::env::var("VISION_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .or(Some(7070)); // Default to 7070 if not specified

        Ok(HandshakeMessage {
            protocol_version: VISION_P2P_PROTOCOL_VERSION, // ⭐ Unified protocol version
            chain_id,
            genesis_hash,
            node_nonce,
            chain_height,
            node_version: VISION_NODE_VERSION, // ⭐ Unified node version
            network_id: NETWORK_ID.to_string(),
            node_build: NODE_BUILD_TAG.to_string(), // ⭐ Build tag for version gating
            node_tag,
            admission_ticket: admission_ticket.to_string(),
            vision_address: vision_address.to_string(),
            node_id,
            public_key: public_key.to_string(),
            role,
            ebid,
            is_guardian,
            is_guardian_candidate,
            http_api_port,
            passport: load_node_passport(),
            // Phase 10: Include our advertised P2P address
            advertised_ip: {
                let addr_guard = crate::ADVERTISED_P2P_ADDRESS.lock();
                addr_guard
                    .as_ref()
                    .and_then(|addr| addr.split(':').next().map(|s| s.to_string()))
            },
            advertised_port: {
                let addr_guard = crate::ADVERTISED_P2P_ADDRESS.lock();
                addr_guard
                    .as_ref()
                    .and_then(|addr| addr.split(':').nth(1).and_then(|p| p.parse().ok()))
            },
            // Phase 11: Bootstrap checkpoint for network quarantine
            bootstrap_checkpoint_height: crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT,
            bootstrap_checkpoint_hash: crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH
                .to_string(),
            bootstrap_prefix: crate::vision_constants::VISION_BOOTSTRAP_PREFIX.to_string(),

            seed_peers: Vec::new(),
            
            // Economics fingerprint - CRITICAL for vault consensus
            econ_hash: crate::genesis::ECON_HASH.to_string(),
        })
    }

    /// Validate this handshake against our local chain
    ///
    /// # Relaxed P2P Handshake (v2.7.0+)
    ///
    /// P2P handshakes are now RELAXED by default to allow nodes on different
    /// versions, prefixes, and protocols to connect freely. Chain consensus
    /// safety is provided by HTTP anchor truth (port 7070), so P2P connections
    /// (port 7072) can be permissive.
    ///
    /// ## Hard Safety Checks (always enforced):
    ///
    /// 1. **chain_id** - Must match configured chain
    ///    - Prevents connecting to completely different chains
    ///    - Computed deterministically from genesis and bootstrap constants
    ///
    /// 2. **genesis_hash** - Must match locked genesis
    ///    - Prevents chain forks from connecting
    ///    - Uses "Genesis Door" pattern for new nodes
    ///
    /// ## Soft Checks (warnings only unless strict mode):
    ///
    /// 1. **bootstrap_prefix** - Expected to match `VISION_BOOTSTRAP_PREFIX`
    ///    - Mismatch logs warning but allows connection
    ///    - Incompatible builds are quarantined
    ///
    /// 2. **protocol_version** - Expected in range [MIN, MAX]
    ///    - Out-of-range logs warning but allows connection
    ///    - Forward/backward compatibility enabled
    ///
    /// 3. **node_version** - Expected >= `VISION_MIN_NODE_VERSION`
    ///    - Old version logs warning but allows connection
    ///    - Legacy nodes can participate
    ///
    /// ## Strict Mode (opt-in)
    ///
    /// Set env var `VISION_P2P_STRICT=1` to enable old strict behavior.
    /// This will reject connections from:
    /// - Mismatched bootstrap prefix
    /// - Out-of-range protocol versions
    /// - Old node versions
    ///
    /// Use strict mode only if you want to enforce homogeneous network.
    ///
    /// ## Architecture
    ///
    /// - Port 7070 (HTTP): Anchors provide canonical chain truth, peer lists
    /// - Port 7072 (P2P): Permissive transport for blocks and transactions
    /// - Safety comes from HTTP/anchor consensus, not P2P validation
    fn validate(&self) -> Result<(), String> {
        use crate::vision_constants::{
            VISION_BOOTSTRAP_PREFIX, VISION_MAX_PROTOCOL_VERSION, VISION_MIN_PROTOCOL_VERSION,
            VISION_VERSION,
        };

        // 1) Bootstrap/drop prefix MUST match exactly (quarantine across incompatible builds).
        if self.bootstrap_prefix != VISION_BOOTSTRAP_PREFIX {
            return Err(format!(
                "❌ HANDSHAKE REJECT: BOOTSTRAP_PREFIX mismatch. expected={} got={}",
                VISION_BOOTSTRAP_PREFIX, self.bootstrap_prefix
            ));
        }

        // 2) Protocol version window check - STRICT (always enforced for mainnet v1.0.0)
        if self.protocol_version < VISION_MIN_PROTOCOL_VERSION
            || self.protocol_version > VISION_MAX_PROTOCOL_VERSION
        {
            return Err(format!(
                "❌ HANDSHAKE REJECT: PROTOCOL_VERSION mismatch. expected_range={}-{} got={} build={}",
                VISION_MIN_PROTOCOL_VERSION,
                VISION_MAX_PROTOCOL_VERSION,
                self.protocol_version,
                self.node_build
            ));
        }

        // 3) Exact software version enforcement: reject mismatch
        // Prefer strict build tag equality to avoid mixed builds talking.
        if self.node_build.trim() != VISION_VERSION {
            return Err(format!(
                "[P2P] Version mismatch: local={} remote={}",
                VISION_VERSION,
                self.node_build.trim()
            ));
        }
        // Additionally ensure numeric version matches expected (defense in depth)
        if self.node_version != VISION_NODE_VERSION {
            let remote_version_str = format!(
                "v{}.{}.{}",
                self.node_version / 100,
                (self.node_version / 10) % 10,
                self.node_version % 10
            );
            return Err(format!(
                "[P2P] Version mismatch: local={} remote={}",
                VISION_VERSION, remote_version_str
            ));
        }

        // Reject too-old protocol versions (defensive, in addition to the window check above).
        if self.protocol_version < MIN_SUPPORTED_PROTOCOL_VERSION {
            return Err(format!(
                "❌ HANDSHAKE REJECT: PROTOCOL_VERSION too old. min_supported={} got={} build={}",
                MIN_SUPPORTED_PROTOCOL_VERSION, self.protocol_version, self.node_build
            ));
        }

        // Build tag already strictly enforced above.

        // Check network ID
        if !self.network_id.is_empty() && self.network_id != NETWORK_ID {
            return Err(format!(
                "Handshake rejected: network mismatch (local='{}' remote='{}')",
                NETWORK_ID, self.network_id
            ));
        }

        // Chain id MUST match deterministic expected chain id for this drop.
        let expected_chain_id = crate::vision_constants::expected_chain_id_bytes();
        if self.chain_id != expected_chain_id {
            return Err(format!(
                "❌ HANDSHAKE REJECT: CHAIN_ID mismatch. expected={} got={}",
                crate::vision_constants::expected_chain_id(),
                hex::encode(self.chain_id)
            ));
        }

        // Genesis MUST match canonical genesis for this drop.
        let peer_genesis = self.genesis_hash;
        let expected_genesis_hex = crate::vision_constants::expected_genesis_hash_hex();
        let expected_genesis_bytes = hex::decode(expected_genesis_hex)
            .map_err(|e| format!("❌ HANDSHAKE REJECT: local GENESIS_HASH hex invalid: {}", e))?;
        if expected_genesis_bytes.len() != 32 {
            return Err("❌ HANDSHAKE REJECT: local GENESIS_HASH wrong length".to_string());
        }
        let mut expected_genesis = [0u8; 32];
        expected_genesis.copy_from_slice(&expected_genesis_bytes);

        // Local genesis should already be locked by DB init; refuse if not.
        let local_genesis = {
            let chain = crate::CHAIN.lock();
            chain.get_genesis_hash()
        };
        match local_genesis {
            Some(local) if local == expected_genesis => {}
            Some(local) => {
                return Err(format!(
                    "❌ HANDSHAKE REJECT: local GENESIS_HASH mismatch. expected={} got={}",
                    expected_genesis_hex,
                    hex::encode(local)
                ));
            }
            None => {
                return Err("❌ HANDSHAKE REJECT: local GENESIS_HASH uninitialized".to_string());
            }
        }

        if peer_genesis != expected_genesis {
            return Err(format!(
                "❌ HANDSHAKE REJECT: GENESIS_HASH mismatch. expected={} got={}",
                expected_genesis_hex,
                hex::encode(peer_genesis)
            ));
        }

        // **CRITICAL: Economics fingerprint validation**
        // Reject peers with mismatched vault addresses or reward splits
        // This prevents nodes with tampered token_accounts.toml from participating
        if !self.econ_hash.is_empty() {
            let expected_econ_hash = crate::genesis::ECON_HASH;
            if self.econ_hash != expected_econ_hash {
                return Err(format!(
                    "❌ HANDSHAKE REJECT: ECON_HASH mismatch. expected={} got={}\n\
                     This peer is using different vault addresses or reward splits.\n\
                     Vault address tampering detected - REJECTING CONNECTION.",
                    expected_econ_hash,
                    self.econ_hash
                ));
            }
        } else {
            // Peer didn't send econ_hash (old version) - warn but allow for compatibility
            warn!(
                "[P2P] Peer {} did not send econ_hash - old version? Allowing connection but cannot verify vault addresses",
                self.node_tag
            );
        }

        // Bootstrap checkpoint MUST match (first N blocks quarantine).
        if self.bootstrap_checkpoint_height == crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT
        {
            if self.bootstrap_checkpoint_hash != crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH
            {
                return Err(format!(
                    "❌ HANDSHAKE REJECT: BOOTSTRAP_CHECKPOINT mismatch. expected={} got={} height={} build={}",
                    crate::vision_constants::BOOTSTRAP_CHECKPOINT_HASH,
                    self.bootstrap_checkpoint_hash,
                    self.bootstrap_checkpoint_height,
                    self.node_build
                ));
            }
        } else if self.bootstrap_checkpoint_height > 0 {
            return Err(format!(
                "❌ HANDSHAKE REJECT: BOOTSTRAP_CHECKPOINT_HEIGHT mismatch. expected={} got={} build={}",
                crate::vision_constants::BOOTSTRAP_CHECKPOINT_HEIGHT,
                self.bootstrap_checkpoint_height,
                self.node_build
            ));
        }

        // NOTE: ONE-CHAIN rule: genesis is hardcoded and already enforced above.

        // ⭐ Change 5: Fix chain height checks - allow early network nodes
        // If both nodes are at height <= 3, allow connection (early network)
        let my_height = {
            let chain = crate::CHAIN.lock();
            chain.blocks.len().saturating_sub(1) as u64
        };
        if my_height <= 3 && self.chain_height <= 3 {
            debug!(
                "[P2P] Early network connection: local_height={} remote_height={} - allowing",
                my_height, self.chain_height
            );
        }

        // NOTE: Offline-first handshake: guardian not required
        // ⭐ OFFLINE-FIRST PASSPORT VERIFICATION (optional, non-blocking metadata only)
        // Verify passport locally if present - NEVER call guardian, NEVER reject on failure
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let _trust_level = crate::passport::verify_passport_local(
            self.passport.as_ref(),
            &self.node_tag,
            &self.network_id,
            self.node_version,
            now,
        );
        // Note: trust_level used for governance/scoring but DOES NOT affect connectivity
        // Untrusted peers are still fully connected for P2P/mining/block propagation

        // ⭐ NO GUARDIAN TICKETS REQUIRED FOR P2P
        // All nodes accepted if protocol_version, genesis_hash, and chain_id match
        // Admission tickets, passports, and guardian validation are metadata-only

        // Log successful connection (no ticket/passport validation)
        if !self.node_tag.is_empty() {
            info!(
                "[P2P] Handshake validation successful: node_tag={}, role={}, height={}",
                self.node_tag, self.role, self.chain_height
            );
        }

        Ok(())
    }

    /// Serialize handshake to bytes using bincode
    fn serialize(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("Failed to serialize handshake: {}", e))
    }

    /// Deserialize handshake from bytes using bincode
    fn deserialize(data: &[u8]) -> Result<Self, String> {
        bincode::deserialize(data).map_err(|e| format!("Failed to deserialize handshake: {}", e))
    }
}

/// Save peer from successful handshake to Vision Peer Book
///
/// This ensures every successful P2P handshake updates the peer book,
/// allowing nodes to remember each other even if the Guardian disappears.
fn save_peer_from_handshake(handshake: &HandshakeMessage, peer_addr: &SocketAddr) {
    use super::peer_store::{PeerStore, VisionPeer};

    // Skip if no Vision Identity
    if handshake.node_tag.is_empty() || handshake.vision_address.is_empty() {
        return;
    }

    // Use CHAIN db for unified peer storage (same as API uses)
    let db = {
        let chain = crate::CHAIN.lock();
        chain.db.clone()
    };

    // Create peer entry
    let mut peer = VisionPeer::new(
        handshake.node_id.clone(),
        handshake.node_tag.clone(),
        handshake.public_key.clone(),
        handshake.vision_address.clone(),
        Some(&handshake.admission_ticket),
        handshake.role.clone(),
    );

    // Peer identity is always "IP:7072" (never ephemeral ports)
    let normalized_addr = format!("{}:7072", peer_addr.ip());

    // Step 4: Never save private/LAN IPs unless explicitly allowed
    if !crate::p2p::ip_filter::validate_ip_for_storage(&normalized_addr) {
        return;
    }
    peer.ip_address = Some(normalized_addr.clone());

    // Save to peer book
    match PeerStore::new(&db) {
        Ok(store) => {
            if let Err(e) = store.upsert(peer) {
                warn!(
                    target: "vision_node::p2p::connection",
                    "[PEERBOOK] Failed to save peer {} ({}): {}",
                    handshake.node_tag,
                    handshake.vision_address,
                    e
                );
            } else {
                info!(
                    target: "vision_node::p2p::connection",
                    "[PEERBOOK] Saved peer {} ({}) as trusted peer",
                    handshake.node_tag,
                    handshake.vision_address
                );
            }
        }
        Err(e) => {
            warn!(
                target: "vision_node::p2p::connection",
                "[PEERBOOK] Failed to create peer store: {}",
                e
            );
        }
    }

    // Phase 6: Update Constellation Memory Layer
    {
        let mut memory = crate::CONSTELLATION_MEMORY.lock();
        memory.update_from_handshake(
            handshake.node_id.clone(),
            handshake.ebid.clone(),
            normalized_addr,
            7072,
            handshake.http_api_port,
            handshake.is_guardian,
            handshake.is_guardian_candidate,
        );

        debug!(
            target: "vision_node::p2p::connection",
            "[CONSTELLATION_MEMORY] Updated peer {} (EBID: {}, HTTP port: {:?})",
            handshake.node_tag,
            handshake.ebid,
            handshake.http_api_port
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CHAIN;
    use hex;
    use std::sync::Arc;

    #[tokio::test]
    async fn handshake_rejects_different_genesis() {
        // Mutate global CHAIN genesis to a known value
        {
            let mut g = CHAIN.lock();
            if g.blocks.is_empty() {
                // Should not happen in this test harness, but bail if it does
                panic!("chain uninitialized");
            }
            g.blocks[0].header.pow_hash =
                "aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000aaaa0000".to_string();
        }

        // Create handshake with a different genesis
        let genesis_bytes =
            hex::decode("bbbb0000bbbb0000bbbb0000bbbb0000bbbb0000bbbb0000bbbb0000bbbb0000")
                .unwrap();
        let mut gh = [0u8; 32];
        gh.copy_from_slice(&genesis_bytes);
        let h = HandshakeMessage {
            protocol_version: P2P_PROTOCOL_VERSION,
            chain_id: [0u8; 32],
            genesis_hash: gh,
            node_nonce: 1234,
            chain_height: 1,
            node_version: 100,
            network_id: NETWORK_ID.to_string(),
            node_build: "".to_string(),
            node_tag: "TEST-NODE-1".to_string(),
            admission_ticket: "test_ticket_1".to_string(),
            passport: None,
            vision_address: "vision://TEST-NODE-1@abc123".to_string(),
            node_id: "test-node-1".to_string(),
            public_key: "testpubkey1".to_string(),
            role: "constellation".to_string(),
            ebid: "test-ebid-1".to_string(),
            is_guardian: false,
            is_guardian_candidate: false,
            http_api_port: Some(7070),
            advertised_ip: None,
            advertised_port: None,
            bootstrap_checkpoint_height: 0,
            bootstrap_checkpoint_hash: "".to_string(),
            bootstrap_prefix: "".to_string(),
            seed_peers: Vec::new(),
            econ_hash: String::new(), // Empty for test (old version compatibility)
        };
        let res = h.validate();
        assert!(res.is_err());
    }
}

impl P2PConnectionManager {
    /// Create new connection manager
    pub fn new(node_id: String) -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            node_id,
        }
    }

    /// Get our node ID
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// Get list of connected peer addresses
    pub async fn get_peer_addresses(&self) -> Vec<String> {
        let peers = self.peers.lock().await;
        peers.keys().cloned().collect()
    }

    /// Get number of connected peers
    pub async fn get_peer_count(&self) -> usize {
        let peers = self.peers.lock().await;
        peers.len()
    }

    /// Get connected peer count using try_lock (non-blocking for telemetry)
    pub fn try_get_peer_count(&self) -> usize {
        self.peers.try_lock().map(|peers| peers.len()).unwrap_or(0)
    }

    /// Fix 1: Get live connected peer addresses (truth source for peer count)
    pub async fn connected_peer_addrs(&self) -> Vec<String> {
        let peers = self.peers.lock().await;
        peers.keys().cloned().collect()
    }

    /// Fix 1: Get live connected peer count (truth source)
    pub async fn connected_peer_count(&self) -> usize {
        let peers = self.peers.lock().await;
        peers.len()
    }

    /// Fix 2: Get live connected peer IDs for deterministic peer ID pool
    /// Returns peer addresses (can be enhanced with node_id mapping later)
    pub async fn connected_peer_ids(&self) -> Vec<String> {
        let peers = self.peers.lock().await;
        // Use the actual connected socket endpoints as IDs.
        // This allows multiple peers on the same IP with different ports (e.g. localhost testing).
        peers
            .keys()
            .map(|addr| {
                addr.parse::<std::net::SocketAddr>()
                    .map(|sa| sa.to_string())
                    .unwrap_or_else(|_| addr.clone())
            })
            .collect()
    }

    /// PATCH 1: Check if a peer is currently connected by address
    pub async fn is_peer_connected(&self, peer_addr: &str) -> bool {
        let peers = self.peers.lock().await;
        peers.contains_key(peer_addr)
    }

    /// Find lowest-scoring peer for eviction (P2P Robustness #4)
    async fn find_lowest_scoring_peer(
        &self,
        peers: &std::collections::HashMap<String, Arc<Mutex<PeerConnection>>>,
    ) -> Option<String> {
        let mut lowest_score = f64::MAX;
        let mut lowest_addr = None;

        for (addr, peer_arc) in peers.iter() {
            let peer = peer_arc.lock().await;
            let memory = crate::CONSTELLATION_MEMORY.lock();
            if let Some(peer_mem) = memory.get_peer(&peer.peer_id) {
                if peer_mem.uptime_score < lowest_score {
                    lowest_score = peer_mem.uptime_score;
                    lowest_addr = Some(addr.clone());
                }
            }
            drop(memory);
        }

        lowest_addr
    }

    /// Get peer list for PEX exchange (P2P Robustness #3)
    async fn get_pex_peer_list(&self) -> Vec<PeerExchangeInfo> {
        let memory = crate::CONSTELLATION_MEMORY.lock();
        let peers = memory.get_best_peers(16);

        peers
            .into_iter()
            .filter(|p| {
                // Only share IPv4, non-loopback peers
                if let Ok(addr) = p.last_ip.parse::<std::net::IpAddr>() {
                    match addr {
                        std::net::IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_unspecified(),
                        _ => false,
                    }
                } else {
                    false
                }
            })
            .map(|p| PeerExchangeInfo {
                addr: format!("{}:7072", p.last_ip),
                last_seen: p.last_seen,
                score: p.uptime_score,
            })
            .collect()
    }

    /// Merge PEX peers into constellation memory (P2P Robustness #3)
    async fn merge_pex_peers(&self, peers: Vec<PeerExchangeInfo>) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for peer_info in peers {
            // Validate IPv4-only
            if let Ok(socket_addr) = peer_info.addr.parse::<SocketAddr>() {
                if !crate::p2p::is_valid_ipv4_endpoint(&socket_addr) {
                    continue;
                }

                // Only add recent peers (within 24 hours)
                if now.saturating_sub(peer_info.last_seen) < 86400 {
                    debug!(
                        target: "p2p::connection",
                        addr = %peer_info.addr,
                        "Discovered peer from PEX exchange"
                    );
                }
            }
        }
    }

    /// Get peer connection info for API
    pub async fn get_peer_info(&self) -> Vec<PeerInfo> {
        let peers = self.peers.lock().await;
        let mut info = Vec::new();

        for (addr, peer_arc) in peers.iter() {
            let peer = peer_arc.lock().await;
            info.push(PeerInfo {
                address: addr.clone(),
                peer_id: peer.peer_id.clone(),
                height: peer.height,
                direction: peer.direction,
                last_activity_secs: peer.last_activity.elapsed().as_secs(),
            });
        }

        info
    }

    /// Register a new peer connection
    pub async fn register_peer(
        &self,
        address: String,
        peer_id: String,
        ebid: String,
        height: u64,
        direction: ConnectionDirection,
        writer: tokio::io::WriteHalf<TcpStream>,
    ) -> (Arc<Mutex<PeerConnection>>, bool) {
        // Use the provided address as the stable peer key.
        // Callers should pass a non-ephemeral identity (e.g. advertised ip:port).
        let normalized_key = address
            .parse::<std::net::SocketAddr>()
            .map(|sa| sa.to_string())
            .unwrap_or_else(|_| address.clone());

        let peer = Arc::new(Mutex::new(PeerConnection {
            address: address.clone(), // Keep original for logging
            height,
            peer_id: peer_id.clone(),
            ebid: ebid.clone(),
            direction,
            connection_id: CONNECTION_INSTANCE_ID.fetch_add(1, Ordering::Relaxed),
            validated_chain: true,
            last_activity: Instant::now(),
            writer: Arc::new(Mutex::new(writer)),
        }));

        let mut peers = self.peers.lock().await;

        // Duplicate handling: choose a deterministic "winner" so both sides converge.
        // Rule: compare local node_id with remote peer_id (expected to be the peer node_id).
        // - if local < remote: keep Outbound, drop Inbound
        // - if local > remote: keep Inbound, drop Outbound
        // Fallback: first-wins if IDs unavailable.
        if let Some(existing_arc) = peers.get(&normalized_key).cloned() {
            let local_node_id = crate::identity::node_id::NODE_IDENTITY
                .get()
                .map(|arc| arc.read().node_id.clone())
                .unwrap_or_default();

            let preferred_direction = if !local_node_id.is_empty() && !peer_id.is_empty() {
                match local_node_id.cmp(&peer_id) {
                    std::cmp::Ordering::Less => Some(ConnectionDirection::Outbound),
                    std::cmp::Ordering::Greater => Some(ConnectionDirection::Inbound),
                    std::cmp::Ordering::Equal => None,
                }
            } else {
                None
            };

            let existing_direction = { existing_arc.lock().await.direction };
            
            // COLLISION HANDLING: Deterministic tie-breaking to avoid simultaneous dial issues
            // Rule: Lower node_id keeps outbound, higher keeps inbound (prevents both sides from closing)
            let local_node_id = crate::vision_constants::VISION_NETWORK_ID;
            let remote_node_id = peer_id.as_str(); // Use peer_id string for comparison
            
            let should_keep_existing = if existing_direction == direction {
                // Same direction duplicate - always keep existing
                true
            } else if local_node_id < remote_node_id {
                // Lower ID keeps outbound
                existing_direction == ConnectionDirection::Outbound
            } else {
                // Higher ID keeps inbound  
                existing_direction == ConnectionDirection::Inbound
            };

            if should_keep_existing {
                info!(
                    "[P2P] COLLISION: keeping {:?}, dropping {:?} for {} (tie-break: local_id vs remote)",
                    existing_direction, direction, normalized_key
                );
                // Return existing peer; caller must NOT start a second message loop.
                return (existing_arc, false);
            }

            info!(
                "[P2P] COLLISION: replacing {:?} -> {:?} for {} (tie-break decided)",
                existing_direction, direction, normalized_key
            );
        }

        // P2P Robustness #4: Reserve slots for new peers
        if peers.len() >= crate::p2p::MAX_PEERS_TOTAL {
            // Check if this is a new peer (not in constellation memory)
            let is_new_peer = {
                let memory = crate::CONSTELLATION_MEMORY.lock();
                !memory.has_peer(&peer_id)
            };

            if is_new_peer {
                // Count anchor peers
                let anchor_count = {
                    let memory = crate::CONSTELLATION_MEMORY.lock();
                    memory.get_anchor_peers().len()
                };

                // Drop lowest-scoring peer if we have too many anchors
                if anchor_count >= crate::p2p::MAX_ANCHOR_PEERS {
                    if let Some(lowest_peer_addr) = self.find_lowest_scoring_peer(&peers).await {
                        debug!(
                            target: "p2p::connection",
                            "Dropping lowest-scoring peer {} to make room for new peer",
                            lowest_peer_addr
                        );
                        peers.remove(&lowest_peer_addr);
                    }
                }
            }
        }

        peers.insert(normalized_key.clone(), peer.clone());

        info!(
            address = %address,
            normalized_key = %normalized_key,
            peer_id = %peer_id,
            height = height,
            direction = ?direction,
            "Registered new peer connection"
        );

        // Phase 3: Guardian welcomes new star 🛡️✨
        let region = detect_peer_region(&address).await;
        crate::guardian::guardian()
            .welcome_star(&peer_id, Some(&address), region.as_deref())
            .await;

        // Persist discovered peer to database (only for outbound connections)
        if direction == ConnectionDirection::Outbound {
            self.persist_peer_address(&address).await;
        }

        (peer, true)
    }

    /// Remove peer connection
    pub async fn remove_peer(&self, address: &str) {
        let normalized_key = address
            .parse::<std::net::SocketAddr>()
            .map(|sa| sa.to_string())
            .unwrap_or_else(|_| address.to_string());

        let mut peers = self.peers.lock().await;
        if let Some(peer_arc) = peers.remove(&normalized_key) {
            // Phase 3: Guardian announces farewell 🛡️
            let peer = peer_arc.lock().await;
            let peer_ebid = peer.ebid.clone();
            crate::guardian::guardian()
                .farewell_star(&peer.peer_id, Some(&normalized_key))
                .await;
            drop(peer);

            // Update PEER_MANAGER - peer disconnected
            crate::PEER_MANAGER
                .update_peer_state(&peer_ebid, crate::p2p::PeerState::Disconnected)
                .await;

            info!(address = %normalized_key, "Removed peer connection");
            // Remove from persisted peers when connection is lost
            self.remove_persisted_peer_address(&normalized_key).await;
        }
    }

    async fn remove_peer_if_connection_id(&self, address: &str, connection_id: u64) {
        let normalized_key = address
            .parse::<std::net::SocketAddr>()
            .map(|sa| sa.to_string())
            .unwrap_or_else(|_| address.to_string());

        let mut peers = self.peers.lock().await;
        let should_remove = match peers.get(&normalized_key) {
            Some(peer_arc) => peer_arc.lock().await.connection_id == connection_id,
            None => false,
        };

        if !should_remove {
            debug!(
                address = %normalized_key,
                connection_id,
                "Skip remove: peer entry replaced"
            );
            return;
        }

        if let Some(peer_arc) = peers.remove(&normalized_key) {
            let peer = peer_arc.lock().await;
            let peer_ebid = peer.ebid.clone();
            crate::guardian::guardian()
                .farewell_star(&peer.peer_id, Some(&normalized_key))
                .await;
            drop(peer);

            crate::PEER_MANAGER
                .update_peer_state(&peer_ebid, crate::p2p::PeerState::Disconnected)
                .await;

            info!(address = %normalized_key, "Removed peer connection");
            self.remove_persisted_peer_address(&normalized_key).await;
        }
    }

    /// Get all persisted peer addresses from database
    pub fn load_persisted_peers() -> Vec<String> {
        let g = crate::CHAIN.lock();
        let mut peers = Vec::new();
        for (k, _v) in g.db.scan_prefix(crate::PEER_PREFIX.as_bytes()).flatten() {
            if let Ok(key) = String::from_utf8(k.to_vec()) {
                let url = key[crate::PEER_PREFIX.len()..].to_string();
                if !url.is_empty() {
                    peers.push(url);
                }
            }
        }
        info!("Loaded {} persisted peers from database", peers.len());
        peers
    }

    /// Persist peer address to database for reconnection on startup
    async fn persist_peer_address(&self, address: &str) {
        // Step 4: Never persist private/LAN peers unless explicitly allowed
        if !crate::p2p::ip_filter::validate_ip_for_storage(address) {
            debug!(address = %address, "Skipping persist of non-public peer address");
            return;
        }
        let g = crate::CHAIN.lock();
        let key = format!("{}{}", crate::PEER_PREFIX, address);
        let _ = g.db.insert(key.as_bytes(), crate::IVec::from(&b"1"[..]));
        let _ = g.db.flush();
        debug!(address = %address, "Persisted peer address to database");
    }

    /// Remove persisted peer address from database
    async fn remove_persisted_peer_address(&self, address: &str) {
        let g = crate::CHAIN.lock();
        let key = format!("{}{}", crate::PEER_PREFIX, address);
        let _ = g.db.remove(key.as_bytes());
        let _ = g.db.flush();
        debug!(address = %address, "Removed persisted peer address from database");
    }

    /// Maintain minimum outbound connections by connecting to persisted or seed peers
    pub async fn maintain_outbound_connections(self: Arc<Self>) {
        // Load seed peers config
        let seed_config =
            match crate::p2p::p2p_config::load_seed_peers_config("config/seed_peers.toml") {
                Ok(config) => config,
                Err(e) => {
                    warn!(
                        "Failed to load seed peers config for connection maintenance: {}",
                        e
                    );
                    return;
                }
            };

        let min_connections = seed_config.min_outbound_connections;
        let max_connections = seed_config.max_outbound_connections;

        loop {
            // ⭐ Change 8: Upgrade outbound connection loop - every 10s attempt 3 new peers
            tokio::time::sleep(Duration::from_secs(10)).await;

            let current_peers = self.get_peer_addresses().await;
            let outbound_count = current_peers.len(); // For now, assume all are outbound

            debug!(
                current_outbound = outbound_count,
                min_required = min_connections,
                max_allowed = max_connections,
                "Checking outbound connection requirements (10s interval)"
            );

            if outbound_count >= min_connections {
                continue; // Enough connections
            }

            // Need more connections - build candidate list in priority order:
            // 1) Persisted peers from DB
            // 2) Bootstrap peers from website (if configured)
            // 3) Static seed peers from config

            let persisted_peers = Self::load_persisted_peers();
            let mut candidate_peers: Vec<String> = persisted_peers
                .iter()
                .filter(|peer| !current_peers.contains(peer))
                .cloned()
                .collect();

            // Try bootstrap URL if we need more peers
            if candidate_peers.len() < (min_connections - outbound_count) {
                if let Some(ref bootstrap_url) = seed_config.bootstrap_url {
                    info!(url = %bootstrap_url, "Fetching bootstrap peers from website");
                    let bootstrap_peers =
                        crate::p2p::bootstrap::fetch_bootstrap_peers(bootstrap_url).await;
                    for peer in bootstrap_peers {
                        if !current_peers.contains(&peer) && !candidate_peers.contains(&peer) {
                            candidate_peers.push(peer);
                        }
                    }
                }
            }

            // If still not enough, add static seed peers
            if candidate_peers.len() < (min_connections - outbound_count) {
                info!("Adding static seed peers from configuration");
                for seed in &seed_config.seed_peers {
                    if !current_peers.contains(seed) && !candidate_peers.contains(seed) {
                        candidate_peers.push(seed.clone());
                    }
                }
            }

            // ⭐ Change 8: Take 3 peers and randomize order
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            let mut peers_to_try: Vec<String> = candidate_peers
                .into_iter()
                .take(max_connections.saturating_sub(outbound_count).max(3)) // Try at least 3
                .collect();
            peers_to_try.shuffle(&mut rng); // Randomize order

            if peers_to_try.is_empty() {
                debug!("No available peers to connect to");
                continue;
            }

            info!(
                attempting_connections = peers_to_try.len(),
                "Attempting to establish outbound connections (randomized, 3s timeout)"
            );

            for peer_addr in peers_to_try {
                let manager = self.clone();
                tokio::spawn(async move {
                    // ⭐ Change 8: Use 3s timeout instead of default
                    match tokio::time::timeout(
                        Duration::from_secs(3),
                        manager.connect_to_peer(peer_addr.clone()),
                    )
                    .await
                    {
                        Ok(Ok(())) => {
                            // Success logged inside connect_to_peer after handshake
                        }
                        Ok(Err(e)) => {
                            debug!("[P2P] Seed connection failed ({}): {}", peer_addr, e);
                        }
                        Err(_) => {
                            debug!("[P2P] Seed connection timed out: {}", peer_addr);
                        }
                    }
                });
            }
        }
    }

    /// Update peer height
    pub async fn update_peer_height(&self, address: &str, height: u64) {
        let peers = self.peers.lock().await;
        if let Some(peer_arc) = peers.get(address) {
            let mut peer = peer_arc.lock().await;
            peer.height = height;
            peer.last_activity = Instant::now();
        }
    }

    /// Broadcast message to all peers
    /// NOTE: Sends to ALL connected peers regardless of trust/score/reputation
    pub async fn broadcast_message(&self, msg: P2PMessage) -> (usize, usize) {
        let peers = self.peers.lock().await;
        let peer_list: Vec<_> = peers.values().cloned().collect();
        drop(peers);

        let mut success = 0;
        let mut failure = 0;

        // Send to all peers - no filtering by trust or reputation
        for peer_arc in peer_list {
            let peer = peer_arc.lock().await;
            let address = peer.address.clone();
            drop(peer);

            match peer_arc.lock().await.send_message(msg.clone()).await {
                Ok(()) => {
                    success += 1;
                    debug!(address = %address, "Sent message to peer");
                }
                Err(e) => {
                    failure += 1;
                    error!(address = %address, error = %e, "Failed to send message to peer");
                    // Remove dead peer
                    self.remove_peer(&address).await;
                }
            }
        }

        (success, failure)
    }

    /// Send message to specific peer
    pub async fn send_to_peer(&self, address: &str, msg: P2PMessage) -> Result<(), String> {
        let peers = self.peers.lock().await;
        let peer_arc = peers
            .get(address)
            .ok_or_else(|| format!("Peer {} not connected", address))?
            .clone();
        drop(peers);

        let result = peer_arc.lock().await.send_message(msg).await;

        if result.is_err() {
            // Remove dead peer on failure
            self.remove_peer(address).await;
        }

        result
    }

    /// ⭐ Handshake retry with exponential backoff (5 attempts) + timeout
    async fn perform_handshake_with_retry(
        &self,
        reader: &mut tokio::io::ReadHalf<TcpStream>,
        writer: &mut tokio::io::WriteHalf<TcpStream>,
        peer_addr: &SocketAddr,
        is_outbound: bool,
    ) -> Result<HandshakeMessage, String> {
        use tokio::time::{timeout, Duration};
        use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

        let backoff_ms = [0, 150, 300, 450, 600, 750]; // 5 retries: 0, 150ms, 300ms, 450ms, 600ms, 750ms
        let mut last_err = String::new();
        // PATCH 1: Track if handshake succeeded to guard against false timeout logging
        let handshake_done = Arc::new(AtomicBool::new(false));

        for (attempt, &delay_ms) in backoff_ms.iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            // Wrap handshake in timeout to reject slow/scanning peers
            let handshake_future = async {
                if is_outbound {
                    // Outbound: Send handshake first, then receive
                    match self.send_handshake(writer).await {
                        Ok(_) => self.receive_handshake(reader).await,
                        Err(e) => Err(e),
                    }
                } else {
                    // Inbound: Receive handshake first, then send
                    match self.receive_handshake(reader).await {
                        Ok(hs) => {
                            self.send_handshake(writer).await?;
                            Ok(hs)
                        }
                        Err(e) => Err(e),
                    }
                }
            };

            let done_flag = Arc::clone(&handshake_done);
            let result = match timeout(
                Duration::from_millis(HANDSHAKE_TIMEOUT_MS),
                handshake_future,
            )
            .await
            {
                Ok(inner_result) => inner_result,
                Err(_) => {
                    // PATCH 1: Guard against false timeout logging after successful handshake
                    // Only log timeout if handshake hasn't succeeded yet
                    if !done_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        warn!(
                            "[P2P] ⏱️  Handshake timeout from {} (exceeded {}ms)",
                            peer_addr, HANDSHAKE_TIMEOUT_MS
                        );

                        // ⭐ ROLLING MESH: Mark failure for timeout (if we can identify the peer)
                        // Note: We don't have node_id yet since handshake failed, so mark by IP if peer exists
                        {
                            let chain = crate::CHAIN.lock();
                            if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                                let now = chrono::Utc::now().timestamp() as u64;
                                // Try to find peer by IP address
                                for peer in peer_store.all() {
                                    if let Some(ip) = &peer.ip_address {
                                        if ip.starts_with(&peer_addr.ip().to_string()) {
                                            let _ = peer_store.mark_peer_failure(&peer.node_id, now);
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        return Err(
                            "Handshake timeout (non-Vision peer or slow connection)".to_string()
                        );
                    } else {
                        // Handshake already succeeded - this is a spurious timeout, ignore it
                        debug!(
                            "[P2P] Ignoring spurious timeout from {} (handshake already completed)",
                            peer_addr
                        );
                        return Err("Handshake timeout (non-Vision peer or slow connection)".to_string());
                    }
                }
            };

            match result {
                Ok(handshake) => {
                    // PATCH 1: Mark handshake as done to prevent false timeouts
                    handshake_done.store(true, std::sync::atomic::Ordering::SeqCst);
                    info!(
                        "[P2P] ✅ Handshake success with peer {} at {} (protocol={}, chain_id={}, height={})",
                        if !handshake.node_tag.is_empty() { &handshake.node_tag } else { "unknown" },
                        peer_addr,
                        handshake.protocol_version,
                        hex::encode(&handshake.chain_id[..8]),
                        handshake.chain_height
                    );
                    return Ok(handshake);
                }
                Err(e) => {
                    let e_str = e.to_string();

                    // Classify error for cleaner logging
                    let is_non_vision = e_str.contains("Non-Vision")
                        || e_str.contains("Incomplete handshake")
                        || e_str.contains("Invalid handshake magic");

                    if attempt < 5 {
                        if is_non_vision {
                            // Downgrade to debug for non-Vision connections to reduce spam
                            debug!(
                                "[P2P] Dropping non-Vision connection from {} ({})",
                                peer_addr, e_str
                            );
                        } else {
                            warn!(
                                "[P2P] Retry {}/5 connecting to {}: {}",
                                attempt + 1,
                                peer_addr,
                                e_str
                            );
                        }
                    }
                    last_err = e;

                    // ⭐ ROLLING MESH: Mark failure on last retry attempt
                    if attempt == 5 {
                        let chain = crate::CHAIN.lock();
                        if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                            let now = chrono::Utc::now().timestamp() as u64;
                            // Try to find peer by IP address
                            for peer in peer_store.all() {
                                if let Some(ip) = &peer.ip_address {
                                    if ip.starts_with(&peer_addr.ip().to_string()) {
                                        let _ = peer_store.mark_peer_failure(&peer.node_id, now);
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // If this was not a transient error, break early
                    if is_non_vision
                        || e_str.contains("timeout")
                        || e_str.contains("version mismatch")
                        || e_str.contains("network mismatch")
                        || e_str.contains("HANDSHAKE REJECT")
                    {
                        break;
                    }
                }
            }
        }

        Err(last_err)
    }

    /// Start TCP listener for incoming peer connections
    pub async fn start_listener(self: Arc<Self>, bind_addr: SocketAddr) -> Result<(), String> {
        // NOTE: IPv4-only P2P for initial testnet ignition
        // Force IPv4 binding - reject IPv6 or dual-stack
        if !bind_addr.is_ipv4() {
            return Err(format!(
                "IPv6 binding not supported: {}. Use IPv4 address (0.0.0.0)",
                bind_addr
            ));
        }

        let listener = TcpListener::bind(bind_addr)
            .await
            .map_err(|e| format!("Failed to bind to {}: {}", bind_addr, e))?;

        info!("[P2P] Listener started on {} (IPv4-only mode)", bind_addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!(
                        target: "vision_node::p2p::connection",
                        "[P2P] 👋 Inbound TCP accepted from {}",
                        peer_addr
                    );
                    let manager = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = manager.handle_inbound_connection(stream, peer_addr).await {
                            // Convert to string to avoid Send issues
                            let peer_str = peer_addr.to_string();
                            let err_str = e.to_string();

                            // Classify error for appropriate logging level
                            if err_str.contains("Non-Vision")
                                || err_str.contains("Incomplete handshake")
                                || err_str.contains("Invalid handshake magic")
                                || err_str.contains("timeout")
                            {
                                // Non-Vision connections are expected occasionally - use debug level
                                debug!(
                                    "[P2P] Dropping non-Vision connection from {}: {}",
                                    peer_str, err_str
                                );
                            } else {
                                // Unexpected errors from Vision nodes - use warn level
                                warn!(
                                    "[P2P] Failed to handle inbound connection from {}: {}",
                                    peer_str, err_str
                                );
                            }
                        }
                    });
                }
                Err(e) => {
                    let err_str = e.to_string();
                    error!(error = err_str, "Failed to accept connection");
                }
            }
        }
    }

    /// Send handshake message with proper framing: magic + version + length + payload
    async fn send_handshake(
        &self,
        writer: &mut tokio::io::WriteHalf<TcpStream>,
    ) -> Result<(), String> {
        // Create handshake
        let mut handshake = HandshakeMessage::new()?;

        // Populate curated seed peers (public routable ip:7072 only)
        handshake.seed_peers = build_handshake_seed_peers(64).await;

        info!(
            protocol_version = handshake.protocol_version,
            chain_height = handshake.chain_height,
            node_version = handshake.node_version,
            "Sending handshake"
        );

        // Serialize to binary using bincode (versioned wire format)
        let data = match P2P_HANDSHAKE_VERSION {
            3 => {
                let wire = HandshakeWireV3::from_internal(&handshake);
                bincode::serialize(&wire)
                    .map_err(|e| format!("Failed to serialize handshake v3: {}", e))?
            }
            _ => handshake.serialize()?,
        };
        let len = data.len();

        // Validate length fits in u16
        let len_u16: u16 = len.try_into().map_err(|_| {
            format!(
                "Handshake payload too large: {} bytes (max {})",
                len, MAX_HANDSHAKE_PAYLOAD
            )
        })?;

        info!(serialized_length = len, "Handshake serialized");

        // Send magic header (9 bytes: "VISION-P2")
        writer
            .write_all(P2P_HANDSHAKE_MAGIC)
            .await
            .map_err(|e| format!("Failed to write handshake magic: {}", e))?;

        // Send version byte (1 byte)
        writer
            .write_all(&[P2P_HANDSHAKE_VERSION])
            .await
            .map_err(|e| format!("Failed to write handshake version: {}", e))?;

        // Send length prefix (2 bytes u16, big-endian)
        let len_bytes = len_u16.to_be_bytes();
        writer
            .write_all(&len_bytes)
            .await
            .map_err(|e| format!("Failed to write handshake length: {}", e))?;

        // Send handshake payload
        writer
            .write_all(&data)
            .await
            .map_err(|e| format!("Failed to write handshake data: {}", e))?;

        writer
            .flush()
            .await
            .map_err(|e| format!("Failed to flush handshake: {}", e))?;

        debug!(
            "Handshake sent successfully (magic + version + {} bytes payload)",
            len
        );

        Ok(())
    }

    /// Receive handshake message with proper framing: magic + version + length + payload
    async fn receive_handshake(
        &self,
        reader: &mut tokio::io::ReadHalf<TcpStream>,
    ) -> Result<HandshakeMessage, String> {
        // Read magic header (9 bytes: "VISION-P2")
        let mut magic_bytes = [0u8; 9];
        reader.read_exact(&mut magic_bytes).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                // Simplified message - not an error, just a non-Vision connection
                return "Non-Vision connection (incompatible protocol)".to_string();
            }
            format!("Failed to read handshake magic: {}", e)
        })?;

        // Detect HTTP verbs hitting P2P port (quick check)
        if &magic_bytes[0..4] == b"GET "
            || &magic_bytes[0..4] == b"POST"
            || &magic_bytes[0..4] == b"HEAD"
            || &magic_bytes[0..4] == b"PUT "
        {
            tracing::debug!(
                "[P2P] HTTP request detected on P2P port (use HTTP API port 7070 instead)"
            );
            return Err("HTTP request on P2P port".to_string());
        }

        // Verify magic header
        if magic_bytes != P2P_HANDSHAKE_MAGIC {
            tracing::warn!(
                "[P2P] ❌ Invalid handshake magic: expected 'VISION-P2', got {:?}",
                String::from_utf8_lossy(&magic_bytes)
            );
            return Err("Invalid handshake magic header".to_string());
        }

        // Read version byte
        let mut version_byte = [0u8; 1];
        reader.read_exact(&mut version_byte).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return "Incomplete handshake (non-Vision peer)".to_string();
            }
            format!("Failed to read handshake version: {}", e)
        })?;

        let version = version_byte[0];
        if version != P2P_HANDSHAKE_VERSION {
            tracing::warn!(
                "[P2P] ❌ Handshake version mismatch: expected v{}, got v{}",
                P2P_HANDSHAKE_VERSION,
                version
            );
            return Err(format!(
                "Handshake version mismatch (expected v{}, got v{})",
                P2P_HANDSHAKE_VERSION, version
            ));
        }

        // Read 2-byte length prefix (u16, big-endian)
        let mut len_bytes = [0u8; 2];
        reader.read_exact(&mut len_bytes).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return "Incomplete handshake (non-Vision peer)".to_string();
            }
            format!("Failed to read handshake length: {}", e)
        })?;

        let len = u16::from_be_bytes(len_bytes) as usize;

        // ⭐ Change 3: Normalize header lengths - allow 32-96 bytes (flexible)
        if len == 0 {
            tracing::debug!("[P2P] Invalid handshake: zero-length payload");
            return Err("Garbage handshake packet".to_string());
        }

        // Accept variable-length handshakes (32-96 bytes typical range)
        if !(32..=16384).contains(&len) {
            // Max 16KB for safety
            tracing::warn!(
                "[P2P] Unusual handshake length: {} bytes (expected 32-96), allowing",
                len
            );
        }

        debug!(
            received_length = len,
            version = version,
            "Received handshake framing (magic + version + length)"
        );

        // Read handshake payload
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return "Incomplete handshake payload (non-Vision peer)".to_string();
            }
            format!("Failed to read handshake payload: {}", e)
        })?;

        debug!(
            data_length = data.len(),
            first_bytes = format!(
                "{:02x} {:02x} {:02x} {:02x}",
                data.first().unwrap_or(&0),
                data.get(1).unwrap_or(&0),
                data.get(2).unwrap_or(&0),
                data.get(3).unwrap_or(&0)
            ),
            "Received handshake data"
        );

        // Deserialize from bincode (versioned wire format)
        let handshake = match version {
            3 => {
                let wire: HandshakeWireV3 = bincode::deserialize(&data)
                    .map_err(|e| format!("Failed to deserialize handshake v3: {}", e))?;
                wire.try_into_internal()?
            }
            _ => HandshakeMessage::deserialize(&data)?,
        };

        info!(
            protocol_version = handshake.protocol_version,
            chain_height = handshake.chain_height,
            node_version = handshake.node_version,
            "Handshake deserialized"
        );

        // Validate handshake (log warning on failure with peer info)
        if let Err(e) = handshake.validate() {
            warn!("{}", e);
            return Err(e);
        }

        Ok(handshake)
    }

    /// Handle inbound peer connection
    async fn handle_inbound_connection(
        &self,
        stream: TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<(), String> {
        let allow_private_peers = crate::p2p::ip_filter::allow_private_peers();

        // Phase 1: IPv4-only validation - reject IPv6, loopback, and unspecified addresses
        // Step 4: Allow private/loopback only if explicitly enabled for LAN testing.
        if !crate::p2p::is_valid_ipv4_endpoint(&peer_addr) && !allow_private_peers {
            debug!(
                target: "p2p::connection",
                peer = %peer_addr,
                "Dropping inbound non-IPv4 or loopback peer"
            );
            return Ok(()); // Silently drop without error spam
        }

        // ⭐ Change 9: Fast-path for known beacon peers
        let is_known_beacon = {
            let peers = crate::beacon::get_peers();
            peers.iter().any(|p| p.ip == peer_addr.ip().to_string())
        };

        if is_known_beacon {
            info!(
                peer = %peer_addr,
                "Accepted inbound from known beacon peer (fast-path)"
            );
        } else {
            info!(peer = %peer_addr, "Accepted inbound IPv4 connection");
        }

        // Split stream
        let (mut reader, mut writer) = tokio::io::split(stream);

        // ⭐ Change 4: Use handshake retry wrapper for inbound connections
        info!(peer = %peer_addr, "Waiting to receive handshake with retry logic...");
        let handshake = self
            .perform_handshake_with_retry(&mut reader, &mut writer, &peer_addr, false)
            .await?;

        // Stable inbound identity: IP from the TCP peer + advertised port (fallback 7072)
        let peer_port = handshake.advertised_port.unwrap_or(7072);
        let normalized_addr = format!("{}:{}", peer_addr.ip(), peer_port);

        // Step 4: Reject private/LAN peers unless explicitly allowed
        if !crate::p2p::ip_filter::validate_ip_for_storage(&normalized_addr) {
            info!("[P2P] SKIP inbound non-public peer {}", peer_addr);
            return Ok(());
        }

        // Do not pre-reject duplicates here; `register_peer` applies deterministic tie-break.

        info!(
            "[P2P] ✅ Inbound peer connected: {} | protocol={} | build={} | height={}",
            handshake.node_tag,
            handshake.protocol_version,
            if !handshake.node_build.is_empty() {
                &handshake.node_build
            } else {
                "unknown"
            },
            handshake.chain_height
        );

        // Handshake seeding: ingest peers only after validation passed
        ingest_handshake_seed_peers(&handshake, &peer_addr);

        // Save peer to Vision Peer Book
        save_peer_from_handshake(&handshake, &peer_addr);

        // Register HTTP peer for autosync immediately after connect
        {
            let http_port = handshake.http_api_port.unwrap_or(7070);
            let peer_url = format!("http://{}:{}", peer_addr.ip(), http_port);
            let mut g = crate::CHAIN.lock();
            if g.peers.insert(peer_url.clone()) {
                let key = format!("{}{}", crate::PEER_PREFIX, peer_url);
                let _ = g.db.insert(key.as_bytes(), sled::IVec::from(&b"1"[..]));
                let _ = g.db.flush();
            }
        }

        // Phase 10: Register peer with normalized P2P identity
        let peer_p2p_addr = normalized_addr.clone();

        {
            let chain = crate::CHAIN.lock();
            if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                // Get existing peer or create new one
                let mut vp = peer_store.get(&handshake.node_id).unwrap_or_else(|| {
                    crate::p2p::peer_store::VisionPeer::new(
                        handshake.node_id.clone(),
                        handshake.node_tag.clone(),
                        handshake.public_key.clone(),
                        handshake.vision_address.clone(),
                        None, // admission_ticket (no longer used)
                        handshake.role.clone(),
                    )
                });

                // Update with current connection info
                vp.ip_address = Some(peer_p2p_addr.clone());
                vp.role = handshake.role.clone();
                vp.last_seen = chrono::Utc::now().timestamp();
                vp.connection_status = "connected".to_string();

                // Save to peer store
                let _ = peer_store.save(&vp);

                info!(
                    "[PEER BOOK] Registered peer {} ({}) at {}",
                    handshake.node_tag, handshake.node_id, peer_p2p_addr
                );
            }
        }

        // ⭐ ROLLING MESH: Mark handshake success for health tracking
        {
            let chain = crate::CHAIN.lock();
            if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                let now = chrono::Utc::now().timestamp() as u64;
                let _ = peer_store.mark_peer_success(&handshake.node_id, now);
            }
        }
        
        // 🔄 AUTO-SYNC TRIGGER: Check if we need to sync based on peer height
        // Commented out temporarily due to Send trait issues - will fix in next iteration
        /*
        {
            let chain = crate::CHAIN.lock();
            let local_height = chain.blocks.len().saturating_sub(1) as u64;
            let local_hash = chain.blocks.last()
                .map(|b| b.header.pow_hash.clone())
                .unwrap_or_else(|| String::from("0000000000000000"));
            drop(chain); // Release lock immediately
            
            let connected_peers = self.connected_peer_count().await;
            let peer_height = handshake.chain_height;
            let peer_id_short = if handshake.node_tag.len() >= 8 {
                &handshake.node_tag[..8]
            } else {
                &handshake.node_tag
            };
            
            info!(
                "[SYNC-CHECK] 🔍 Handshake complete: local_height={} local_hash={} peer_height={} peer={} connected={}",
                local_height,
                &local_hash[..16.min(local_hash.len())],
                peer_height,
                peer_id_short,
                connected_peers
            );
            
            // Call sync trigger decision
            crate::maybe_trigger_sync(
                "handshake",
                peer_height,
                &handshake.node_tag,
                local_height,
                connected_peers,
            );
        }
        */

        // Duplicate connection handling is performed in `register_peer`.
        let address = normalized_addr.clone();
        let peer_id = handshake.node_id.clone();
        let ebid = handshake.ebid.clone();

        // Phase 10: Test peer reachability if they advertised an address
        if let Some(advertised_ip) = handshake.advertised_ip.as_ref() {
            if let Some(advertised_port) = handshake.advertised_port {
                if crate::p2p::ip_filter::is_private_ip(advertised_ip) {
                    debug!(
                        "[P2P REACHABILITY] Skipping reachability probe for private peer {}:{}",
                        advertised_ip, advertised_port
                    );
                } else {
                    // Reset retry counters since peer provided valid advertised address
                    let peer_store_opt = {
                        let chain = crate::CHAIN.lock();
                        crate::p2p::peer_store::PeerStore::new(&chain.db).ok()
                    };
                    if let Some(peer_store) = peer_store_opt {
                        let ps = std::sync::Arc::new(peer_store);
                        crate::p2p::retry_worker::reset_peer_retry(ps, &ebid).await;
                    }

                    let ebid_clone = ebid.clone();
                    let advertised_ip_clone = advertised_ip.clone();
                    tokio::spawn(async move {
                        use crate::p2p::reachability::ReachabilityTester;
                        let tester = ReachabilityTester::new();
                        let token =
                            crate::p2p::reachability::ReachabilityHandshake::generate_token();

                        let result = tester
                            .test_reachability(&advertised_ip_clone, advertised_port, &token)
                            .await;

                        // Update peer with reachability results
                        let nat_type = result.nat_type.clone();
                        crate::PEER_MANAGER
                            .update_peer_reachability(
                                &ebid_clone,
                                result.public_reachable,
                                result.nat_type,
                                result.tested_at,
                            )
                            .await;

                        if result.public_reachable {
                            info!(
                            "[P2P REACHABILITY] ✅ Peer {} is publicly reachable at {}:{} (NAT: {})",
                            ebid_clone, advertised_ip_clone, advertised_port, nat_type
                        );
                        } else {
                            debug!(
                                "[P2P REACHABILITY] ⚠️  Peer {} not reachable at {}:{} (NAT: {})",
                                ebid_clone, advertised_ip_clone, advertised_port, nat_type
                            );
                        }
                    });
                }
            }
        }

        // Register with P2P connection manager
        let (peer_arc, registered) = self
            .register_peer(
                address.clone(),
                peer_id,
                ebid.clone(),
                handshake.chain_height,
                ConnectionDirection::Inbound,
                writer,
            )
            .await;

        // If this was a duplicate, do not start a second message loop and do not
        // mutate peer state (the existing connection remains authoritative).
        if !registered {
            // FIX: Explicitly close the duplicate connection to prevent half-open sockets
            // This is critical to prevent OS error 10053 on the remote peer
            drop(reader);  // Explicitly drop reader to ensure socket is closed
            info!(peer = %address, "Duplicate inbound connection rejected and closed");
            return Ok(());
        }

        // Only the winning connection should update the PeerManager entry.
        // Otherwise, the losing duplicate connection can overwrite a Connected peer
        // back to KnownOnly and break mining readiness gating.
        let new_peer = crate::p2p::Peer::new(peer_addr.ip().to_string(), peer_port, ebid.clone());
        crate::PEER_MANAGER.add_peer(new_peer).await;

        let connection_id = { peer_arc.lock().await.connection_id };

        // Update state to Connected and set height (use ebid, not socket address!)
        crate::PEER_MANAGER
            .update_peer_state(&ebid, crate::p2p::PeerState::Connected)
            .await;
        crate::PEER_MANAGER
            .update_peer_height(&ebid, handshake.chain_height)
            .await;

        // Update chain identity fields from handshake
        let node_version_str = format!(
            "{}.{}.{}",
            handshake.node_version / 100,
            (handshake.node_version / 10) % 10,
            handshake.node_version % 10
        );
        crate::PEER_MANAGER
            .update_peer_chain_identity(
                &ebid,
                hex::encode(handshake.chain_id),
                handshake.bootstrap_prefix.clone(),
                handshake.protocol_version,
                node_version_str,
            )
            .await;

        // Log successful constellation handshake
        info!(
            "[P2P] ✅ Connected to constellation peer {} at {} (direction: inbound)",
            if !handshake.node_tag.is_empty() {
                &handshake.node_tag
            } else {
                &ebid
            },
            peer_addr
        );

        info!(peer = %peer_addr, "Peer registered, starting message loop");

        // Start message loop for regular messages (blocks, txs, etc.)
        self.peer_message_loop(address, connection_id, reader).await;

        Ok(())
    }

    /// Connect to outbound peer
    pub async fn connect_to_peer(self: Arc<Self>, address: String) -> Result<(), String> {
        let allow_private_peers = crate::p2p::ip_filter::allow_private_peers();

        // Load P2P config for IPv4-only mode
        let p2p_config =
            crate::config::p2p::P2pConfig::load_or_create("p2p.json").unwrap_or_default();

        // Filter IPv6 addresses if IPv4-only mode is enabled
        if p2p_config.force_ipv4 && !p2p_config.should_connect_to_peer(&address) {
            debug!(
                target: "p2p::connection",
                peer = %address,
                "Skipping IPv6 peer (IPv4-only mode enabled)"
            );
            return Err("IPv6 address blocked by IPv4-only mode".to_string());
        }

        // ⭐ Change 2: Strip IPv6 from ALL outbound attempts - strict enforcement
        if let Ok(sock_addr) = address.parse::<SocketAddr>() {
            // Reject IPv6 immediately
            if sock_addr.is_ipv6() {
                info!("[DIAL] SKIP ipv6 ip={}", address);
                crate::p2p::dial_tracker::record_dial_failure(
                    address.to_string(),
                    "ipv6_blocked".to_string(),
                    "direct".to_string(),
                );
                return Err("IPv6 not supported (IPv4-only mode)".to_string());
            }
            if !crate::p2p::is_valid_ipv4_endpoint(&sock_addr) && !allow_private_peers {
                info!("[DIAL] SKIP non-ipv4-or-loopback ip={}", address);
                crate::p2p::dial_tracker::record_dial_failure(
                    address.to_string(),
                    "invalid_ipv4_endpoint".to_string(),
                    "direct".to_string(),
                );
                return Err("Unsupported address (IPv6/loopback)".to_string());
            }
        }

        // Normalize dial address:
        // - accept "ip:port" as-is
        // - accept "ip" and default to :7072
        let dial_addr = if address.parse::<std::net::SocketAddr>().is_ok() {
            address.clone()
        } else if let Ok(ip) = address.parse::<std::net::IpAddr>() {
            format!("{}:7072", ip)
        } else {
            address.clone()
        };

        // Peer identity defaults to the dial address (callers may later switch to advertised ip:port)
        let _normalized_addr = dial_addr.clone();

        // Step 4: Never dial private/LAN peers unless explicitly allowed
        {
            let local_ips = crate::p2p::ip_filter::get_local_ips();
            if let Some(reason) =
                crate::p2p::ip_filter::validate_ip_for_dial(&dial_addr, &local_ips)
            {
                info!("[DIAL] SKIP {} ip={}", reason, dial_addr);
                crate::p2p::dial_tracker::record_dial_failure(
                    dial_addr.clone(),
                    reason.clone(),
                    "direct".to_string(),
                );
                return Ok(());
            }
        }

        // Do not pre-skip duplicates here; `register_peer` applies deterministic tie-break.

        info!(peer = %address, normalized = %dial_addr, "Connecting to peer");

        // PATCH 5: Enhanced connection failure logging with detailed reasons
        // PATCH 6: Increase connection timeout from 5s to 10s
        // Home NAT + busy routers + cold sockets need more time than speed dating
        use tokio::time::{timeout, Duration};
        let connect_result = timeout(Duration::from_secs(10), TcpStream::connect(&dial_addr)).await;
        let stream = match connect_result {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                // Connection attempt failed (not timeout)
                let reason = match e.kind() {
                    std::io::ErrorKind::ConnectionRefused => "connection_refused",
                    std::io::ErrorKind::TimedOut => "timeout",
                    std::io::ErrorKind::ConnectionReset => "connection_reset",
                    std::io::ErrorKind::ConnectionAborted => "connection_aborted",
                    std::io::ErrorKind::NotConnected => "not_connected",
                    std::io::ErrorKind::AddrNotAvailable => "addr_not_available",
                    std::io::ErrorKind::NetworkUnreachable => "network_unreachable",
                    _ => "other",
                };

                warn!(
                    target: "p2p::connect",
                    peer = %address,
                    reason = %reason,
                    error = %e,
                    "Connection attempt failed"
                );

                // Track dial failure
                crate::p2p::dial_tracker::record_dial_failure(
                    address.to_string(),
                    reason.to_string(),
                    "direct".to_string(),
                );

                return Err(format!("Connection failed ({}): {}", reason, e));
            }
            Err(_) => {
                // Connection timeout (10 seconds elapsed)
                warn!(
                    target: "p2p::connect",
                    peer = %address,
                    "Connection attempt timed out after 10 seconds"
                );

                crate::p2p::dial_tracker::record_dial_failure(
                    address.to_string(),
                    "connection_timeout_10s".to_string(),
                    "direct".to_string(),
                );

                return Err(format!("Connection timeout after 10 seconds"));
            }
        };

        // Log which protocol successfully connected and store peer_addr
        let sock_addr = stream
            .peer_addr()
            .map_err(|e| format!("Failed to get peer address: {}", e))?;
        let proto = if sock_addr.is_ipv4() { "IPv4" } else { "IPv6" };
        info!(peer = %address, protocol = %proto, "Connected via {}", proto);

        let (mut reader, mut writer) = tokio::io::split(stream);

        // ⭐ Change 4: Use handshake retry wrapper for outbound connections
        // Fix 4: Handshake timeout (12s per attempt, 36s total for 3 attempts)
        info!(peer = %address, "Initiating handshake with retry logic...");
        let peer_handshake = tokio::time::timeout(
            Duration::from_secs(36), // 12s per attempt * 3 attempts
            self.perform_handshake_with_retry(&mut reader, &mut writer, &sock_addr, true),
        )
        .await
        .map_err(|_| {
            // PATCH 5: Enhanced handshake timeout logging
            warn!(
                target: "p2p::connect",
                peer = %address,
                reason = "handshake_timeout",
                "Handshake with retries timed out (36s)"
            );

            // Track dial failure
            crate::p2p::dial_tracker::record_dial_failure(
                address.to_string(),
                "handshake_timeout".to_string(),
                "direct".to_string(),
            );

            "Handshake with retries timed out (36s)".to_string()
        })?
        .map_err(|e| {
            // PATCH 5: Enhanced handshake failure logging
            let reason = if e.contains("protocol") {
                "version_mismatch"
            } else if e.contains("genesis") {
                "genesis_mismatch"
            } else if e.contains("banned") {
                "banned"
            } else if e.contains("ticket") {
                "ticket_invalid"
            } else {
                "handshake_error"
            };

            warn!(
                target: "p2p::connect",
                peer = %address,
                reason = %reason,
                error = %e,
                "Handshake failed after retries"
            );

            // Track dial failure
            crate::p2p::dial_tracker::record_dial_failure(
                address.to_string(),
                reason.to_string(),
                "direct".to_string(),
            );

            debug!(peer = %address, error = %e, "Failed handshake after retries");
            e
        })?;

        info!(
            peer = %address,
            chain_height = peer_handshake.chain_height,
            node_tag = %peer_handshake.node_tag,
            "Peer handshake received and validated"
        );

        // Handshake seeding: ingest peers only after validation passed
        ingest_handshake_seed_peers(&peer_handshake, &sock_addr);

        // Save peer to Vision Peer Book (normalized inside helper)
        save_peer_from_handshake(&peer_handshake, &sock_addr);

        // Register HTTP peer for autosync immediately after connect
        {
            let http_port = peer_handshake.http_api_port.unwrap_or(7070);
            let peer_url = format!("http://{}:{}", sock_addr.ip(), http_port);
            let mut g = crate::CHAIN.lock();
            if g.peers.insert(peer_url.clone()) {
                let key = format!("{}{}", crate::PEER_PREFIX, peer_url);
                let _ = g.db.insert(key.as_bytes(), sled::IVec::from(&b"1"[..]));
                let _ = g.db.flush();
            }
        }

        // Phase 10: Register peer with stable P2P identity (prefer advertised port)
        let dial_port = dial_addr
            .parse::<std::net::SocketAddr>()
            .map(|sa| sa.port())
            .unwrap_or(7072);
        let peer_port = peer_handshake.advertised_port.unwrap_or(dial_port);
        let peer_p2p_addr = format!("{}:{}", sock_addr.ip(), peer_port);

        {
            let chain = crate::CHAIN.lock();
            if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                // Get existing peer or create new one
                let mut vp = peer_store.get(&peer_handshake.node_id).unwrap_or_else(|| {
                    crate::p2p::peer_store::VisionPeer::new(
                        peer_handshake.node_id.clone(),
                        peer_handshake.node_tag.clone(),
                        peer_handshake.public_key.clone(),
                        peer_handshake.vision_address.clone(),
                        None, // admission_ticket (no longer used)
                        peer_handshake.role.clone(),
                    )
                });

                // Update with current connection info
                vp.ip_address = Some(peer_p2p_addr.clone());
                vp.role = peer_handshake.role.clone();
                vp.last_seen = chrono::Utc::now().timestamp();
                vp.connection_status = "connected".to_string();

                // Save to peer store
                let _ = peer_store.save(&vp);

                info!(
                    "[PEER BOOK] Registered peer {} ({}) at {}",
                    peer_handshake.node_tag, peer_handshake.node_id, peer_p2p_addr
                );
            }
        }

        // ⭐ ROLLING MESH: Mark handshake success for health tracking
        {
            let chain = crate::CHAIN.lock();
            if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&chain.db) {
                let now = chrono::Utc::now().timestamp() as u64;
                let _ = peer_store.mark_peer_success(&peer_handshake.node_id, now);
            }
        }
        
        // 🔄 AUTO-SYNC TRIGGER: Check if we need to sync based on peer height (outbound)
        // Commented out temporarily due to Send trait issues - will fix in next iteration
        /*
        {
            let chain = crate::CHAIN.lock();
            let local_height = chain.blocks.len().saturating_sub(1) as u64;
            let local_hash = chain.blocks.last()
                .map(|b| b.header.pow_hash.clone())
                .unwrap_or_else(|| String::from("0000000000000000"));
            drop(chain);
            
            let connected_peers = self.connected_peer_count().await;
            let peer_height = peer_handshake.chain_height;
            let peer_id_short = if peer_handshake.node_tag.len() >= 8 {
                &peer_handshake.node_tag[..8]
            } else {
                &peer_handshake.node_tag
            };
            
            info!(
                "[SYNC-CHECK] 🔍 Outbound handshake complete: local_height={} local_hash={} peer_height={} peer={} connected={}",
                local_height,
                &local_hash[..16.min(local_hash.len())],
                peer_height,
                peer_id_short,
                connected_peers
            );
            
            crate::maybe_trigger_sync(
                "handshake_outbound",
                peer_height,
                &peer_handshake.node_tag,
                local_height,
                connected_peers,
            );
        }
        */

        // Phase 10: Test peer reachability if they advertised an address
        if let Some(advertised_ip) = peer_handshake.advertised_ip.as_ref() {
            if let Some(advertised_port) = peer_handshake.advertised_port {
                if crate::p2p::ip_filter::is_private_ip(advertised_ip) {
                    debug!(
                        "[P2P REACHABILITY] Skipping reachability probe for private peer {}:{}",
                        advertised_ip, advertised_port
                    );
                } else {
                    // Reset retry counters since peer provided valid advertised address
                    let peer_store_opt = {
                        let chain = crate::CHAIN.lock();
                        crate::p2p::peer_store::PeerStore::new(&chain.db).ok()
                    };
                    if let Some(peer_store) = peer_store_opt {
                        let ps = std::sync::Arc::new(peer_store);
                        crate::p2p::retry_worker::reset_peer_retry(ps, &peer_handshake.ebid).await;
                    }

                    let ebid_clone = peer_handshake.ebid.clone();
                    let advertised_ip_clone = advertised_ip.clone();
                    tokio::spawn(async move {
                        use crate::p2p::reachability::ReachabilityTester;
                        let tester = ReachabilityTester::new();
                        let token =
                            crate::p2p::reachability::ReachabilityHandshake::generate_token();

                        let result = tester
                            .test_reachability(&advertised_ip_clone, advertised_port, &token)
                            .await;

                        // Update peer with reachability results
                        let nat_type = result.nat_type.clone();
                        crate::PEER_MANAGER
                            .update_peer_reachability(
                                &ebid_clone,
                                result.public_reachable,
                                result.nat_type,
                                result.tested_at,
                            )
                            .await;

                        if result.public_reachable {
                            info!(
                            "[P2P REACHABILITY] ✅ Peer {} is publicly reachable at {}:{} (NAT: {})",
                            ebid_clone, advertised_ip_clone, advertised_port, nat_type
                        );
                        } else {
                            debug!(
                                "[P2P REACHABILITY] ⚠️  Peer {} not reachable at {}:{} (NAT: {})",
                                ebid_clone, advertised_ip_clone, advertised_port, nat_type
                            );
                        }
                    });
                }
            }
        }

        // Register peer (use node_nonce as peer_id)
        let peer_id = peer_handshake.node_id.clone();
        let peer_height = peer_handshake.chain_height;
        let ebid = peer_handshake.ebid.clone();
        let normalized_addr = peer_p2p_addr.clone();
        let (peer_arc, registered) = self
            .register_peer(
                normalized_addr.clone(),
                peer_id,
                ebid.clone(),
                peer_height,
                ConnectionDirection::Outbound,
                writer,
            )
            .await;

        // If this was a duplicate, do not start a second message loop and do not
        // mutate peer state (the existing connection remains authoritative).
        if !registered {
            // FIX: Explicitly close the duplicate connection to prevent half-open sockets
            // This is critical to prevent OS error 10053 on the remote peer
            drop(reader);  // Explicitly drop reader to ensure socket is closed
            info!(peer = %normalized_addr, "Duplicate outbound connection rejected and closed");
            return Ok(());
        }

        // Only the winning connection should update the PeerManager entry.
        // Otherwise, the losing duplicate connection can overwrite a Connected peer
        // back to KnownOnly and break mining readiness gating.
        let new_peer = crate::p2p::Peer::new(sock_addr.ip().to_string(), peer_port, ebid.clone());
        crate::PEER_MANAGER.add_peer(new_peer).await;

        let connection_id = { peer_arc.lock().await.connection_id };

        // Update PEER_MANAGER with successful connection (use ebid, not socket address!)
        crate::PEER_MANAGER
            .update_peer_state(&ebid, crate::p2p::PeerState::Connected)
            .await;
        crate::PEER_MANAGER
            .update_peer_height(&ebid, peer_height)
            .await;

        // Update chain identity fields from handshake
        let node_version_str = format!(
            "{}.{}.{}",
            peer_handshake.node_version / 100,
            (peer_handshake.node_version / 10) % 10,
            peer_handshake.node_version % 10
        );
        crate::PEER_MANAGER
            .update_peer_chain_identity(
                &ebid,
                hex::encode(peer_handshake.chain_id),
                peer_handshake.bootstrap_prefix.clone(),
                peer_handshake.protocol_version,
                node_version_str,
            )
            .await;

        // Log successful constellation handshake with version info
        info!(
            "[P2P] ✅ Connected to peer: {} | protocol={} | build={} | height={}",
            if !peer_handshake.node_tag.is_empty() {
                &peer_handshake.node_tag
            } else {
                &ebid
            },
            peer_handshake.protocol_version,
            if !peer_handshake.node_build.is_empty() {
                &peer_handshake.node_build
            } else {
                "unknown"
            },
            peer_height
        );

        // GOSSIP MOVED TO PERIODIC LOOP (30s interval in connection_maintainer.rs)
        // This prevents thundering herd when multiple peers connect simultaneously

        // Request peer list for discovery
        if let Err(e) = self
            .send_to_peer(&normalized_addr, P2PMessage::GetPeers)
            .await
        {
            warn!(peer = %normalized_addr, error = %e, "Failed to request peer list");
        }

        // P2P Robustness #3: Send peer exchange request
        if let Err(e) = self
            .send_to_peer(&normalized_addr, P2PMessage::PeerExchangeRequest)
            .await
        {
            debug!(peer = %normalized_addr, error = %e, "Failed to send PEX request");
        }

        // Start message loop
        let manager = self.clone();
        tokio::spawn(async move {
            manager
                .peer_message_loop(normalized_addr, connection_id, reader)
                .await;
        });

        Ok(())
    }

    /// Message receive loop for a peer connection
    async fn peer_message_loop(
        &self,
        address: String,
        connection_id: u64,
        mut reader: tokio::io::ReadHalf<TcpStream>,
    ) {
        loop {
            match self.receive_message(&mut reader).await {
                Ok(msg) => {
                    if let Err(e) = self.handle_peer_message(&address, msg).await {
                        error!(peer = %address, error = %e, "Error handling message");
                    }
                }
                Err(e) => {
                    info!(peer = %address, error = %e, "Peer disconnected");
                    self.remove_peer_if_connection_id(&address, connection_id)
                        .await;

                    // Update PEER_MANAGER - peer disconnected
                    // Note: We don't have EBID here, so we'll update by finding the peer by address
                    // This is a limitation - ideally we'd store EBID in PeerConnection struct

                    break;
                }
            }
        }
    }

    /// Receive a length-prefixed message from stream
    async fn receive_message(
        &self,
        reader: &mut tokio::io::ReadHalf<TcpStream>,
    ) -> Result<P2PMessage, String> {
        // Read 4-byte length prefix
        let mut len_bytes = [0u8; 4];
        reader
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| format!("Failed to read length: {}", e))?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        debug!("Received message length prefix: {} bytes", len);

        // Sanity check
        if len > 100 * 1024 * 1024 {
            // 100MB max
            return Err(format!("Message too large: {} bytes", len));
        }

        if len == 0 {
            return Err("Message length is 0".to_string());
        }

        // Read message data
        let mut data = vec![0u8; len];
        reader
            .read_exact(&mut data)
            .await
            .map_err(|e| format!("Failed to read message: {}", e))?;

        // Log first 200 bytes for debugging
        let preview = if data.len() > 200 {
            format!(
                "{}... ({} total bytes)",
                String::from_utf8_lossy(&data[..200]),
                data.len()
            )
        } else {
            String::from_utf8_lossy(&data).to_string()
        };
        debug!("Received message data: {}", preview);

        // Deserialize
        serde_json::from_slice(&data).map_err(|e| {
            error!("Deserialization failed: {}", e);
            error!("Raw bytes (first 500): {:?}", &data[..data.len().min(500)]);
            format!("Failed to deserialize message: {}", e)
        })
    }

    /// Handle received message from peer
    async fn handle_peer_message(&self, address: &str, msg: P2PMessage) -> Result<(), String> {
        // Safety gate: do not process any non-handshake messages from peers
        // until handshake validation has completed.
        let validated_chain = {
            let peers = self.peers.lock().await;
            if let Some(peer_arc) = peers.get(address) {
                peer_arc.lock().await.validated_chain
            } else {
                // If we don't have a registered peer entry, treat as unvalidated.
                false
            }
        };

        if !validated_chain {
            match msg {
                P2PMessage::Handshake { .. } => {
                    warn!(peer = %address, "Dropping handshake message from unvalidated peer (unexpected)");
                }
                _ => {
                    warn!(peer = %address, "Dropping message from unvalidated peer");
                }
            }
            return Ok(());
        }

        match msg {
            P2PMessage::Ping { timestamp } => {
                // Respond with pong
                self.send_to_peer(address, P2PMessage::Pong { timestamp })
                    .await?;
            }
            P2PMessage::Pong { .. } => {
                // Update last activity
                self.update_peer_height(address, 0).await; // Just update timestamp
            }
            P2PMessage::CompactBlock { compact } => {
                let height = compact.header.height;
                let hash = compact.header.hash.clone();

                info!(
                    peer = %address,
                    hash = %hash,
                    height = height,
                    "Received compact block from peer"
                );

                // Handle compact block with peer tracking
                if let Err(e) = super::routes::handle_compact_block_direct(compact, Some(address.to_string())).await {
                    // Treat certain errors as hard incompatibility and fast-disconnect.
                    let is_incompatible = e.contains("No common ancestor")
                        || e.contains("Attempted reorg past bootstrap checkpoint")
                        || e.contains("bootstrap checkpoint")
                        || e.contains("wrong genesis")
                        || e.contains("incompatible chain");
                    if is_incompatible {
                        warn!(peer = %address, reason = %e, "Incompatible chain detected via compact block; disconnecting");
                        let _ = self
                            .send_to_peer(
                                address,
                                P2PMessage::Disconnect {
                                    reason: format!("INCOMPATIBLE_CHAIN: {}", e),
                                },
                            )
                            .await;
                        self.remove_peer(address).await;
                        return Err(format!("INCOMPATIBLE_CHAIN: {}", e));
                    }

                    return Err(e);
                }

                // Update peer height
                self.update_peer_height(address, height).await;
            }
            P2PMessage::FullBlock { block } => {
                info!(
                    peer = %address,
                    hash = %block.header.pow_hash,
                    height = block.header.number,
                    "Received full block from peer"
                );

                // Enterprise block integration with full validation
                match self.integrate_received_block(block.clone(), address).await {
                    Ok(integrated) => {
                        if integrated {
                            // Check if block is ACTUALLY in main chain (not just accepted/orphaned)
                            let is_in_main_chain = {
                                let g = crate::CHAIN.lock();
                                g.blocks.iter().any(|b| &b.header.pow_hash == &block.header.pow_hash)
                            };
                            
                            if is_in_main_chain {
                                info!(
                                    peer = %address,
                                    hash = %block.header.pow_hash,
                                    height = block.header.number,
                                    "✅ Block INTEGRATED into main chain"
                                );
                                self.update_peer_height(address, block.header.number).await;
                            } else {
                                debug!(
                                    peer = %address,
                                    hash = %block.header.pow_hash,
                                    "Block accepted but orphaned (waiting for parents)"
                                );
                            }
                        } else {
                            debug!(
                                peer = %address,
                                hash = %block.header.pow_hash,
                                "Block already known or in orphan pool"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            peer = %address,
                            hash = %block.header.pow_hash,
                            error = %e,
                            "Failed to integrate received block"
                        );

                        // Phase 4: Apply misbehavior penalty for invalid blocks
                        if let Ok(peer_store) =
                            crate::p2p::peer_store::PeerStore::new(&crate::CHAIN.lock().db)
                        {
                            use crate::p2p::reputation::{
                                apply_misbehavior, MisbehaviorKind, ReputationConfig,
                            };

                            // Find peer by vision_address
                            if let Some(mut peer) = peer_store
                                .all()
                                .into_iter()
                                .find(|p| p.vision_address == address)
                            {
                                let config = ReputationConfig::default();
                                apply_misbehavior(
                                    &mut peer,
                                    MisbehaviorKind::InvalidBlock,
                                    &config,
                                );
                                let _ = peer_store.upsert(peer);
                            }
                        }
                    }
                }
            }
            P2PMessage::GetPeers => {
                // Send our peer list
                let peers = self.get_peer_addresses().await;
                let _ = self
                    .send_to_peer(address, P2PMessage::PeerList { peers })
                    .await;
            }
            P2PMessage::PeerList { peers } => {
                let received_len = peers.len();
                // Fix 5: Safety cap reduced from 256 to 100 to prevent bandwidth explosion
                let capped_peers: Vec<String> = if received_len > 100 {
                    warn!(
                        peer = %address,
                        "Received {} peers, capping to 100 for safety (Fix 5)",
                        received_len
                    );
                    peers.into_iter().take(100).collect()
                } else {
                    peers
                };

                info!(
                    peer = %address,
                    received_len = received_len,
                    processing_count = capped_peers.len(),
                    "Received peer list from peer"
                );

                // Deduplicate and persist discovered peers
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                for peer_addr in capped_peers {
                    if peer_addr != address
                        && !peer_addr.is_empty()
                        && seen.insert(peer_addr.clone())
                    {
                        self.persist_peer_address(&peer_addr).await;
                    }
                }
            }
            P2PMessage::PeerExchangeRequest => {
                debug!(peer = %address, "Received PEX request");
                // P2P Robustness #3: Respond with top peers
                let peer_list = self.get_pex_peer_list().await;
                let _ = self
                    .send_to_peer(
                        address,
                        P2PMessage::PeerExchangeResponse { peers: peer_list },
                    )
                    .await;
            }
            P2PMessage::PeerExchangeResponse { peers } => {
                debug!(peer = %address, peer_count = peers.len(), "Received PEX response");
                // P2P Robustness #3: Merge received peers
                self.merge_pex_peers(peers).await;
            }
            P2PMessage::MinerTuningHint { hint } => {
                debug!(
                    peer = %address,
                    algo = %hint.pow_algo,
                    threads = hint.threads,
                    batch = hint.batch_size,
                    gain = format!("{:.1}%", hint.gain_ratio * 100.0),
                    "Received P2P tuning hint"
                );
                // Forward to miner intelligence system (handled by mining manager)
                // For now, just log - full integration requires miner handle access
            }
            P2PMessage::PeerGossip(gossip) => {
                info!(
                    peer = %address,
                    from_node = %gossip.from_node,
                    peer_count = gossip.peers.len(),
                    "Received peer gossip"
                );

                // Process gossip and get new peers to connect to
                let (peer_store, our_node_id) = {
                    let chain = crate::CHAIN.lock();
                    let ps = crate::p2p::peer_store::PeerStore::new(&chain.db).ok();
                    let node_id = crate::P2P_MANAGER.get_node_id().to_string();
                    (ps, node_id)
                };

                if let Some(peer_store) = peer_store {
                    let new_peers = crate::p2p::peer_gossip::process_gossip_message(
                        gossip,
                        std::sync::Arc::new(peer_store),
                        &our_node_id,
                    )
                    .await;

                    // Log discovered peers (actual connection handled by periodic discovery)
                    if !new_peers.is_empty() {
                        info!(
                            peer = %address,
                            new_peer_count = new_peers.len(),
                            peers = ?new_peers,
                            "Discovered new peers from gossip, saved to peer book"
                        );
                    }
                }
            }
            P2PMessage::Transaction { tx } => {
                let tx_hash = hex::encode(crate::tx_hash(&tx));
                debug!(
                    peer = %address,
                    tx_hash = %tx_hash,
                    "Received transaction from peer"
                );

                // Enterprise mempool integration with validation
                match self.add_transaction_to_mempool(tx.clone(), address).await {
                    Ok(added) => {
                        if added {
                            info!(
                                peer = %address,
                                tx_hash = %tx_hash,
                                "Transaction added to mempool"
                            );
                        } else {
                            debug!(
                                peer = %address,
                                tx_hash = %tx_hash,
                                "Transaction already in mempool or invalid"
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            peer = %address,
                            tx_hash = %tx_hash,
                            error = %e,
                            "Failed to add transaction to mempool"
                        );
                    }
                }
            }
            P2PMessage::GetBlocks {
                start_height,
                end_height,
            } => {
                info!(
                    peer = %address,
                    start = start_height,
                    end = end_height,
                    "Peer requested blocks"
                );

                // Enterprise block serving with rate limiting
                match self
                    .serve_blocks_to_peer(address, start_height, end_height)
                    .await
                {
                    Ok(sent_count) => {
                        info!(
                            peer = %address,
                            start = start_height,
                            end = end_height,
                            sent = sent_count,
                            "Sent blocks to peer"
                        );
                    }
                    Err(e) => {
                        warn!(
                            peer = %address,
                            error = %e,
                            "Failed to serve blocks to peer"
                        );
                    }
                }
            }
            P2PMessage::Disconnect { reason } => {
                info!(peer = %address, reason = %reason, "Peer disconnecting");
                self.remove_peer(address).await;
            }
            P2PMessage::Handshake { .. } => {
                warn!(peer = %address, "Received unexpected handshake after connection established");
            }
            // ===== CHAIN SYNC MESSAGE HANDLERS =====
            P2PMessage::GetTip => {
                debug!(peer = %address, "[SYNC] -> GetTip received");
                let (height, hash) = {
                    let chain = crate::CHAIN.lock();
                    let height = chain.blocks.len() as u64;
                    let hash = chain.blocks.last()
                        .map(|b| b.header.pow_hash.clone())
                        .unwrap_or_default();
                    (height, hash)
                };
                self.send_to_peer(address, P2PMessage::Tip { height, hash }).await?;
            }
            P2PMessage::Tip { height, hash } => {
                debug!(peer = %address, height = height, hash = %hash, "[SYNC] <- Tip received");
                // Update peer height for sync selection
                self.update_peer_height(address, height).await;
            }
            P2PMessage::GetHeaders { locator_hashes, max } => {
                debug!(peer = %address, locators = locator_hashes.len(), max = max, "[SYNC] -> GetHeaders received");
                // Find common ancestor and send headers
                let headers = {
                    let chain = crate::CHAIN.lock();
                    let mut result = Vec::new();
                    
                    // Find first locator that matches our chain
                    let start_idx = locator_hashes.iter()
                        .find_map(|hash| {
                            chain.blocks.iter().position(|b| &b.header.pow_hash == hash)
                        })
                        .unwrap_or(0);
                    
                    // Send up to 'max' headers starting from common ancestor + 1
                    for block in chain.blocks.iter().skip(start_idx + 1).take(max as usize) {
                        result.push(block.header.clone());
                    }
                    result
                };
                self.send_to_peer(address, P2PMessage::Headers { headers }).await?;
            }
            P2PMessage::Headers { headers } => {
                debug!(peer = %address, count = headers.len(), "[SYNC] <- Headers received");
                // TODO: Validate header chain and request missing blocks
                // For now, just log (full implementation requires sync state machine)
            }
            P2PMessage::GetBlock { hash } => {
                debug!(peer = %address, hash = %hash, "[SYNC] -> GetBlock received");
                let block = {
                    let chain = crate::CHAIN.lock();
                    chain.blocks.iter()
                        .find(|b| b.header.pow_hash == hash)
                        .cloned()
                };
                if let Some(block) = block {
                    self.send_to_peer(address, P2PMessage::Block { block }).await?;
                } else {
                    debug!(peer = %address, hash = %hash, "[SYNC] Block not found");
                }
            }
            P2PMessage::Block { block } => {
                debug!(peer = %address, hash = %block.header.pow_hash, height = block.header.number, "[SYNC] <- Block received");
                // Integrate block (same as FullBlock handler)
                match self.integrate_received_block(block.clone(), address).await {
                    Ok(integrated) => {
                        if integrated {
                            info!(
                                peer = %address,
                                hash = %block.header.pow_hash,
                                height = block.header.number,
                                "Sync block successfully integrated"
                            );
                            self.update_peer_height(address, block.header.number).await;
                        }
                    }
                    Err(e) => {
                        warn!(
                            peer = %address,
                            hash = %block.header.pow_hash,
                            error = %e,
                            "Failed to integrate sync block"
                        );
                    }
                }
            }
            P2PMessage::GetBlocksByRange { start_height, max } => {
                debug!(peer = %address, start = start_height, max = max, "[SYNC] -> GetBlocksByRange received");
                let blocks = {
                    let chain = crate::CHAIN.lock();
                    let start_idx = start_height.saturating_sub(1) as usize;
                    chain.blocks.iter()
                        .skip(start_idx)
                        .take(max as usize)
                        .cloned()
                        .collect()
                };
                self.send_to_peer(address, P2PMessage::Blocks { blocks }).await?;
            }
            P2PMessage::Blocks { blocks } => {
                debug!(peer = %address, count = blocks.len(), "[SYNC] <- Blocks received");
                // Integrate blocks sequentially
                for block in blocks {
                    if let Err(e) = self.integrate_received_block(block.clone(), address).await {
                        warn!(
                            peer = %address,
                            hash = %block.header.pow_hash,
                            error = %e,
                            "Failed to integrate bulk sync block"
                        );
                        break; // Stop on first failure
                    }
                }
            }
            P2PMessage::GetBlockHash { height } => {
                info!(peer = %address, height = height, "[SYNC-FORK] -> GetBlockHash REQUEST received from peer");
                let hash = {
                    let chain = crate::CHAIN.lock();
                    if height == 0 || height as usize > chain.blocks.len() {
                        warn!(peer = %address, height = height, "[SYNC-FORK] Height not in our chain (have {} blocks)", chain.blocks.len());
                        None
                    } else {
                        let idx = (height - 1) as usize;
                        chain.blocks.get(idx).map(|b| {
                            crate::canon_hash(&b.header.pow_hash)
                        })
                    }
                };
                info!(peer = %address, height = height, hash = ?hash, "[SYNC-FORK] -> Sending BlockHash RESPONSE");
                self.send_to_peer(address, P2PMessage::BlockHash { height, hash }).await?;
            }
            P2PMessage::BlockHash { height, hash } => {
                info!(peer = %address, height = height, hash = ?hash, "[SYNC-FORK] <- BlockHash RESPONSE received, storing in cache");
                // This is handled by the sync logic in auto_sync.rs
                // Store in a temporary cache for retrieval
                if let Some(ref h) = hash {
                    let mut cache = crate::SYNC_HASH_CACHE.lock();
                    let key = (address.to_string(), height);
                    cache.insert(key, h.clone());
                    info!(peer = %address, height = height, "[SYNC-FORK] Cached hash for peer (cache size: {})", cache.len());
                } else {
                    warn!(peer = %address, height = height, "[SYNC-FORK] Peer sent None hash (height doesn't exist on their chain)");
                }
            }
        }


        Ok(())
    }

    /// Start keepalive ping loop
    pub async fn start_keepalive(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;

                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let (success, failure) =
                    self.broadcast_message(P2PMessage::Ping { timestamp }).await;

                if success > 0 || failure > 0 {
                    debug!(success = success, failure = failure, "Keepalive ping sent");
                }
            }
        });
    }
}

/// Peer info for API responses
#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub address: String,
    pub peer_id: String,
    pub height: u64,
    pub direction: ConnectionDirection,
    pub last_activity_secs: u64,
}

impl Serialize for ConnectionDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            ConnectionDirection::Inbound => "inbound",
            ConnectionDirection::Outbound => "outbound",
        })
    }
}

// ==================== ENTERPRISE P2P INTEGRATION HELPERS ====================

impl P2PConnectionManager {
    /// Enterprise-grade block integration with full validation
    async fn integrate_received_block(
        &self,
        block: crate::Block,
        peer_address: &str,
    ) -> Result<bool, String> {
        use tracing::{debug, info, warn};

        let block_hash = block.header.pow_hash.clone();
        let block_height = block.header.number;

        // 1. Check if we already have this block
        {
            let g = crate::CHAIN.lock();
            if g.blocks.iter().any(|b| b.header.pow_hash == block_hash) {
                debug!(
                    hash = %block_hash,
                    height = block_height,
                    "Block already in chain"
                );
                return Ok(false);
            }
        }

        // 2. Check if block extends current tip (validation happens in apply_block)
        let current_tip = {
            let g = crate::CHAIN.lock();
            g.blocks.last().map(|b| b.header.pow_hash.clone())
        };

        match current_tip {
            Some(tip_hash) if block.header.parent_hash == tip_hash => {
                // Block extends tip - add it
                let mut g = crate::CHAIN.lock();
                match crate::chain::accept::apply_block(&mut g, &block, Some(peer_address)) {
                    Ok(()) => {
                        info!(
                            hash = %block_hash,
                            height = block_height,
                            "Block integrated at tip"
                        );
                        Ok(true)
                    }
                    Err(e) => {
                        warn!(
                            hash = %block_hash,
                            height = block_height,
                            peer = %peer_address,
                            error = %e,
                            "Block rejected by apply_block"
                        );

                        // Apply misbehavior penalty for invalid blocks
                        if let Ok(peer_store) = crate::p2p::peer_store::PeerStore::new(&g.db) {
                            use crate::p2p::reputation::{
                                apply_misbehavior, MisbehaviorKind, ReputationConfig,
                            };

                            if let Some(mut peer) = peer_store
                                .all()
                                .into_iter()
                                .find(|p| p.vision_address == peer_address)
                            {
                                let config = ReputationConfig::default();
                                apply_misbehavior(
                                    &mut peer,
                                    MisbehaviorKind::InvalidBlock,
                                    &config,
                                );
                                let _ = peer_store.upsert(peer);
                            }
                        }

                        Err(format!("Block validation failed: {}", e))
                    }
                }
            }
            Some(_) => {
                // Block doesn't extend tip - let apply_block handle reorg if needed
                debug!(
                    hash = %block_hash,
                    height = block_height,
                    "Block doesn't extend tip - unified acceptance will handle reorg"
                );

                let result = {
                    let mut g = crate::CHAIN.lock();
                    crate::chain::accept::apply_block(&mut g, &block, Some(peer_address))
                };

                match result {
                    Ok(()) => {
                        // Check if block is ACTUALLY in main chain (not just orphaned)
                        let is_in_main_chain = {
                            let g = crate::CHAIN.lock();
                            g.blocks.iter().any(|b| crate::canon_hash(&b.header.pow_hash) == block_hash)
                        };
                        
                        if is_in_main_chain {
                            info!(
                                target = "p2p::connection",
                                peer = %peer_address,
                                hash = %block_hash,
                                height = block_height,
                                "✅ Block INTEGRATED into main chain"
                            );
                        } else {
                            debug!(
                                target = "p2p::connection",
                                peer = %peer_address,
                                hash = %block_hash,
                                "Block accepted but orphaned (waiting for parents)"
                            );
                        }
                        
                        // Drain orphan pool with safety cap (max 512 per insert)
                        let orphans_resolved = {
                            let mut g = crate::CHAIN.lock();
                            let mut total_processed = 0;
                            const MAX_ORPHAN_DRAIN: usize = 512;
                            
                            // Start with the block we just inserted
                            let mut to_check = vec![block_hash.clone()];
                            
                            while !to_check.is_empty() && total_processed < MAX_ORPHAN_DRAIN {
                                let parent_hash = to_check.pop().unwrap();
                                let processed = crate::chain::accept::process_orphans(&mut g, &parent_hash);
                                
                                if processed > 0 {
                                    tracing::info!(
                                        parent_hash = %parent_hash,
                                        processed = processed,
                                        total = total_processed + processed,
                                        "[ORPHAN-DRAIN] resolved children"
                                    );
                                    total_processed += processed;
                                }
                            }
                            
                            if total_processed >= MAX_ORPHAN_DRAIN {
                                tracing::warn!(
                                    "[ORPHAN-DRAIN] hit safety cap of {} orphans per insert",
                                    MAX_ORPHAN_DRAIN
                                );
                            }
                            
                            total_processed
                        };
                        
                        if orphans_resolved > 0 {
                            info!(
                                orphans_resolved = orphans_resolved,
                                "[ORPHAN-DRAIN] cascade integration complete"
                            );
                        }
                        
                        Ok(true)
                    }
                    Err(e) => {
                        // Only disconnect for truly incompatible chains (wrong network or huge reorg)
                        // Don't disconnect for "missing parent" - that's normal during sync (orphan pool handles it)
                        let is_incompatible = e.contains("bootstrap checkpoint")
                            || e.contains("reorg too large");
                        if is_incompatible {
                            warn!(peer = %peer_address, error = %e, "Incompatible chain detected; disconnecting peer");
                            let _ = self
                                .send_to_peer(
                                    peer_address,
                                    P2PMessage::Disconnect {
                                        reason: format!("INCOMPATIBLE_CHAIN: {}", e),
                                    },
                                )
                                .await;
                            self.remove_peer(peer_address).await;
                            return Err(format!("INCOMPATIBLE_CHAIN: {}", e));
                        }
                        Err(format!("Block acceptance failed: {}", e))
                    }
                }
            }
            None => {
                // Genesis case
                let mut g = crate::CHAIN.lock();
                match crate::chain::accept::apply_block(&mut g, &block, Some(peer_address)) {
                    Ok(()) => {
                        info!(hash = %block_hash, "Genesis block integrated");
                        Ok(true)
                    }
                    Err(e) => {
                        warn!(
                            hash = %block_hash,
                            peer = %peer_address,
                            error = %e,
                            "Genesis block rejected by apply_block"
                        );
                        Err(format!("Genesis block validation failed: {}", e))
                    }
                }
            }
        }
    }

    /// Enterprise-grade mempool transaction integration
    async fn add_transaction_to_mempool(
        &self,
        tx: crate::Tx,
        peer_address: &str,
    ) -> Result<bool, String> {
        use tracing::{debug, warn};

        let tx_hash = hex::encode(crate::tx_hash(&tx));

        // 1. Check if we already have this transaction
        {
            let g = crate::CHAIN.lock();
            if g.seen_txs.contains(&tx_hash) {
                debug!(tx_hash = %tx_hash, "Transaction already seen");
                return Ok(false);
            }
        }

        // 2. Validate transaction signature and structure
        if let Err(e) = crate::verify_tx(&tx) {
            warn!(
                tx_hash = %tx_hash,
                peer = %peer_address,
                error = ?e,
                "Transaction validation failed"
            );
            return Err(format!("Transaction validation failed: {:?}", e));
        }

        // 3. Add to mempool (bulk queue - lower priority for P2P received)
        let mut g = crate::CHAIN.lock();
        g.mempool_bulk.push_back(tx.clone());
        g.seen_txs.insert(tx_hash.clone());

        debug!(
            tx_hash = %tx_hash,
            peer = %peer_address,
            "Transaction added to mempool"
        );

        Ok(true)
    }

    /// Enterprise-grade block serving to peers
    async fn serve_blocks_to_peer(
        &self,
        peer_address: &str,
        start_height: u64,
        end_height: u64,
    ) -> Result<usize, String> {
        use tracing::{debug, warn};

        // Rate limiting: max 100 blocks per request
        const MAX_BLOCKS_PER_REQUEST: u64 = 100;
        let safe_end = std::cmp::min(end_height, start_height + MAX_BLOCKS_PER_REQUEST);

        // Fetch requested blocks
        let blocks: Vec<crate::Block> = {
            let g = crate::CHAIN.lock();
            g.blocks
                .iter()
                .filter(|b| b.header.number >= start_height && b.header.number <= safe_end)
                .cloned()
                .collect()
        };

        if blocks.is_empty() {
            debug!(
                peer = %peer_address,
                start = start_height,
                end = safe_end,
                "No blocks found in requested range"
            );
            return Ok(0);
        }

        // Send blocks one by one
        let mut sent_count = 0;
        for block in blocks {
            match self
                .send_to_peer(peer_address, P2PMessage::FullBlock { block })
                .await
            {
                Ok(_) => sent_count += 1,
                Err(e) => {
                    warn!(
                        peer = %peer_address,
                        error = %e,
                        sent = sent_count,
                        "Failed to send block to peer"
                    );
                    break;
                }
            }
        }

        Ok(sent_count)
    }
}

/// Detect geographic region of peer based on IP address
/// Uses MaxMind GeoIP2 for production-grade accuracy (~99.8% country-level)
async fn detect_peer_region(address: &str) -> Option<String> {
    use tracing::debug;

    // Extract IP from "ip:port" format
    let ip_str = if let Some(colon_pos) = address.rfind(':') {
        &address[..colon_pos]
    } else {
        address
    };

    // Parse IP address
    let ip: std::net::IpAddr = match ip_str.parse() {
        Ok(ip) => ip,
        Err(_) => {
            debug!(
                target: "p2p::connection",
                address = address,
                "Could not parse IP for region detection"
            );
            return Some("Unknown".to_string());
        }
    };

    // Check for local/private IPs first (fast path)
    if ip.is_loopback() || ip.is_unspecified() {
        return Some("Local".to_string());
    }

    // Check for private network ranges (RFC 1918)
    if let std::net::IpAddr::V4(ipv4) = ip {
        let octets = ipv4.octets();
        if octets[0] == 10 // 10.0.0.0/8
            || (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) // 172.16.0.0/12
            || (octets[0] == 192 && octets[1] == 168)
        // 192.168.0.0/16
        {
            return Some("Private".to_string());
        }
    }

    // Try MaxMind GeoIP2 database (production-grade)
    // Database path: vision_data/GeoLite2-Country.mmdb or GeoLite2-City.mmdb
    if let Some(region) = lookup_geoip2_region(&ip).await {
        debug!(
            target: "p2p::connection",
            ip = %ip,
            region = %region,
            method = "GeoIP2",
            "Detected peer region (MaxMind)"
        );
        return Some(region);
    }

    // Fallback: Simple continent-level detection based on IP ranges
    // This is used when GeoIP2 database is not available
    // Accuracy: ~80-90% continent-level
    debug!(
        target: "p2p::connection",
        ip = %ip,
        "GeoIP2 database not available, using fallback detection"
    );

    if let std::net::IpAddr::V4(ipv4) = ip {
        let first_octet = ipv4.octets()[0];

        // Rough continent mapping based on IANA allocations
        let region = match first_octet {
            1..=2 | 6..=7 | 11..=12 | 13..=15 | 20..=24 | 50..=63 | 64..=127 | 199 | 206..=216 => {
                "North America"
            }
            128..=130 | 144..=146 | 147..=149 | 150..=152 | 153..=155 | 190..=191 | 200..=201 => {
                "South America"
            }
            // Keep this arm mutually exclusive with the two above.
            3..=5 | 31..=37 | 41..=45 | 46..=48 | 131..=134 | 141 | 176..=188 | 193..=195 => {
                "Europe"
            }
            196 | 197 | 217 => "Africa",
            // Keep this arm mutually exclusive with the arms above.
            27 | 39..=40 | 49 | 163 | 171 | 175 | 202..=203 | 218..=223 => "Asia",
            _ => "Unknown",
        };

        debug!(
            target: "p2p::connection",
            ip = %ip,
            region = region,
            method = "fallback",
            "Detected peer region (fallback)"
        );

        Some(region.to_string())
    } else {
        // IPv6 - attempt GeoIP2 lookup, fallback to generic label
        Some("IPv6".to_string())
    }
}

/// Lookup geographic region using MaxMind GeoIP2 database
/// Returns continent/country for public IPs, None if database unavailable
async fn lookup_geoip2_region(ip: &std::net::IpAddr) -> Option<String> {
    use std::path::Path;
    use tracing::debug;

    // Check for GeoIP2 database files in vision_data/
    // Priority: City database (more detailed) > Country database
    let db_paths = [
        "vision_data/GeoLite2-City.mmdb",
        "vision_data/GeoLite2-Country.mmdb",
        "GeoLite2-City.mmdb",
        "GeoLite2-Country.mmdb",
    ];

    for db_path in &db_paths {
        let path = Path::new(db_path);
        if !path.exists() {
            continue;
        }

        // Try to open and query the database
        match maxminddb::Reader::open_readfile(path) {
            Ok(reader) => {
                // Try City database lookup first (more detailed)
                if db_path.contains("City") {
                    if let Ok(city_data) = reader.lookup::<maxminddb::geoip2::City>(*ip) {
                        // Build region string: "Continent > Country > City"
                        let mut region_parts = Vec::new();

                        if let Some(continent) = city_data.continent.and_then(|c| c.names) {
                            if let Some(name) = continent.get("en") {
                                region_parts.push(name.to_string());
                            }
                        }

                        if let Some(country) = city_data.country.and_then(|c| c.names) {
                            if let Some(name) = country.get("en") {
                                region_parts.push(name.to_string());
                            }
                        }

                        if let Some(city) = city_data.city.and_then(|c| c.names) {
                            if let Some(name) = city.get("en") {
                                region_parts.push(name.to_string());
                            }
                        }

                        if !region_parts.is_empty() {
                            let region = region_parts.join(" > ");
                            debug!(
                                target: "p2p::connection",
                                ip = %ip,
                                region = %region,
                                database = db_path,
                                "GeoIP2 City lookup successful"
                            );
                            return Some(region);
                        }
                    }
                } else {
                    // Country database lookup
                    if let Ok(country_data) = reader.lookup::<maxminddb::geoip2::Country>(*ip) {
                        let mut region_parts = Vec::new();

                        if let Some(continent) = country_data.continent.and_then(|c| c.names) {
                            if let Some(name) = continent.get("en") {
                                region_parts.push(name.to_string());
                            }
                        }

                        if let Some(country) = country_data.country.and_then(|c| c.names) {
                            if let Some(name) = country.get("en") {
                                region_parts.push(name.to_string());
                            }
                        }

                        if !region_parts.is_empty() {
                            let region = region_parts.join(" > ");
                            debug!(
                                target: "p2p::connection",
                                ip = %ip,
                                region = %region,
                                database = db_path,
                                "GeoIP2 Country lookup successful"
                            );
                            return Some(region);
                        }
                    }
                }
            }
            Err(e) => {
                debug!(
                    target: "p2p::connection",
                    error = %e,
                    database = db_path,
                    "Failed to open GeoIP2 database"
                );
                continue;
            }
        }
    }

    // No database found or lookup failed
    None
}

// Legacy validate_block_structure removed - validation now handled by chain::accept::apply_block

// Strict PoW hash parsing (accept optional 0x prefix, require 32 bytes)
fn parse_pow_hash_32(pow_hash: &str) -> Result<[u8; 32], String> {
    let s = pow_hash.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != 64 {
        return Err(format!(
            "Invalid PoW hash length: expected 64 hex chars (32 bytes), got {}",
            s.len()
        ));
    }
    let bytes = hex::decode(s).map_err(|_| "Invalid PoW hash format".to_string())?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Load node passport from identity storage
fn load_node_passport() -> Option<crate::passport::NodePassport> {
    use tracing::debug;

    // Check if passport feature is enabled
    if std::env::var("VISION_PASSPORT_ENABLED").unwrap_or_else(|_| "false".to_string()) != "true" {
        return None;
    }

    // Try to load passport from vision_data/identity.db
    let identity_path = std::path::Path::new("vision_data").join("identity.db");
    if !identity_path.exists() {
        debug!(
            target: "p2p::connection",
            "Identity database not found, passport loading disabled"
        );
        return None;
    }

    // Open the identity database and try to read passport
    match sled::open(&identity_path) {
        Ok(db) => {
            match db.get(b"node_passport") {
                Ok(Some(passport_bytes)) => {
                    // Deserialize passport from IVec
                    match serde_json::from_slice::<crate::passport::NodePassport>(&passport_bytes) {
                        Ok(passport) => {
                            debug!(
                                target: "p2p::connection",
                                node_tag = %passport.node_tag,
                                role = %passport.role,
                                "Loaded node passport from identity storage"
                            );
                            return Some(passport);
                        }
                        Err(e) => {
                            debug!(
                                target: "p2p::connection",
                                error = %e,
                                "Failed to deserialize passport"
                            );
                        }
                    }
                }
                Ok(None) => {
                    debug!(
                        target: "p2p::connection",
                        "No passport found in identity storage"
                    );
                }
                Err(e) => {
                    debug!(
                        target: "p2p::connection",
                        error = %e,
                        "Error reading from identity database"
                    );
                }
            }
        }
        Err(e) => {
            debug!(
                target: "p2p::connection",
                error = %e,
                "Failed to open identity database"
            );
        }
    }

    None
}
