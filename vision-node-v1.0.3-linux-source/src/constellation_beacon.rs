//! Constellation Beacon - Guardian's network heartbeat broadcaster
//!
//! The Constellation Beacon is the Guardian's radio tower - broadcasting heartbeats
//! to tell all nodes in the network that the beacon is alive and operational.
//!
//! **Guardian Nodes**: Run beacon in ACTIVE mode, broadcasting heartbeats
//! **Constellation Nodes**: Connect to beacon endpoint for network discovery
//!
//! **CRITICAL**: Beacon is READ-ONLY for chain state
//! - Only reads chain height/peer count for metrics
//! - Never modifies consensus or chain state
//! - Used for peer discovery, NOT chain authority
//!
//! Environment Variables:
//! - BEACON_PORT: Port for beacon HTTP endpoint (default: 7070, same as main API)
//! - BEACON_INTERVAL_SECS: Seconds between heartbeat broadcasts (default: 30)
//! - BEACON_ENABLED: Set to "false" to disable beacon (Guardian always overrides to true)
//! - BEACON_MODE: "active" (broadcast) or "passive" (receive only) - Guardian forces "active"
#![allow(dead_code)]

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Default public beacon endpoint for constellation nodes
const DEFAULT_PUBLIC_BEACON: &str = "https://visionworld.tech";

/// Helper function to normalize beacon URL
/// Strips trailing slashes and accidentally included /api/beacon paths
fn normalize_beacon_url(url: &str) -> String {
    // Strip trailing slash
    let trimmed = url.trim_end_matches('/');

    // If user accidentally includes `/api/beacon` in env var, strip it
    let cleaned = trimmed
        .strip_suffix("/api/beacon")
        .unwrap_or(trimmed)
        .to_string();

    cleaned
}

/// Get beacon base URL from environment or default to public beacon
/// Returns None only if user explicitly sets BEACON_ENDPOINT=standalone or off
fn beacon_base_url() -> Option<String> {
    use std::env;

    // 1) Check env override
    if let Ok(raw) = env::var("BEACON_ENDPOINT") {
        let trimmed = raw.trim();

        // Explicit opt-out to standalone mode
        if trimmed.eq_ignore_ascii_case("standalone") || trimmed.eq_ignore_ascii_case("off") {
            return None;
        }

        // User provided custom beacon URL
        if !trimmed.is_empty() {
            return Some(normalize_beacon_url(trimmed));
        }
    }

    // 2) Default to public beacon
    Some(normalize_beacon_url(DEFAULT_PUBLIC_BEACON))
}

/// Global beacon instance
static BEACON: OnceCell<Arc<ConstellationBeacon>> = OnceCell::new();

/// Beacon operational mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeaconMode {
    /// Active broadcasting - Guardian nodes only
    Active,
    /// Passive listening - Constellation nodes
    Passive,
}

impl BeaconMode {
    pub fn from_env() -> Self {
        match std::env::var("BEACON_MODE").ok().as_deref() {
            Some("passive") => Self::Passive,
            _ => {
                // Default to active if Guardian, passive otherwise
                if crate::is_guardian_mode() {
                    Self::Active
                } else {
                    Self::Passive
                }
            }
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Passive => "passive",
        }
    }
}

/// Beacon heartbeat packet (sent over UDP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconHeartbeat {
    /// Guardian node ID
    pub node_id: String,
    /// Network identifier
    pub network: String,
    /// Current block height
    pub height: u64,
    /// Number of active peers
    pub peer_count: usize,
    /// Unix timestamp
    pub timestamp: u64,
    /// Beacon version
    pub version: String,
}

/// Constellation Beacon - The Guardian's heartbeat broadcaster
pub struct ConstellationBeacon {
    /// Node identifier
    node_id: String,
    /// Beacon mode (active/passive)
    mode: BeaconMode,
    /// Beacon start time
    start_time: Instant,
    /// Total heartbeats broadcast
    heartbeat_count: AtomicU64,
    /// Running flag
    running: AtomicBool,
    /// Broadcast interval (seconds)
    interval_secs: u64,
}

impl ConstellationBeacon {
    /// Create a new beacon instance
    pub fn new(node_id: String) -> Self {
        let mode = BeaconMode::from_env();
        let interval_secs = std::env::var("BEACON_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30); // Default: 30 seconds

        Self {
            node_id,
            mode,
            start_time: Instant::now(),
            heartbeat_count: AtomicU64::new(0),
            running: AtomicBool::new(false),
            interval_secs,
        }
    }

    /// Get beacon mode
    pub fn mode(&self) -> BeaconMode {
        self.mode
    }

    /// Get beacon uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get total heartbeats sent
    pub fn heartbeat_count(&self) -> u64 {
        self.heartbeat_count.load(Ordering::Relaxed)
    }

    /// Check if beacon is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the beacon (spawns background task)
    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            warn!("[BEACON] Already running");
            return;
        }

        match self.mode {
            BeaconMode::Active => {
                info!(
                    "[BEACON] Starting in ACTIVE mode - broadcasting every {} seconds",
                    self.interval_secs
                );
                self.start_active_beacon();
            }
            BeaconMode::Passive => {
                info!("[BEACON] Running in PASSIVE mode - will register with Guardian");
                self.start_passive_registration();
            }
        }
    }

    /// Start passive mode - register with Guardian beacon
    fn start_passive_registration(&self) {
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            // Get beacon base URL (defaults to public beacon unless explicitly disabled)
            let base = match beacon_base_url() {
                Some(url) => {
                    info!("[BEACON] Connecting to Guardian beacon at: {}", url);
                    url
                }
                None => {
                    info!("[BEACON] No beacon endpoint configured (BEACON_ENDPOINT=standalone/off) - running in standalone mode");
                    info!("[BEACON] To connect to public beacon, remove BEACON_ENDPOINT or set to a URL");
                    return;
                }
            };

            let register_url = format!("{}/api/beacon/register", base);

            info!(
                "[BEACON] Registering with Guardian beacon at: {}",
                register_url
            );

            // Prepare registration payload
            // Use P2P port for beacon registration (not HTTP API port)
            // P2P port is either VISION_P2P_PORT or VISION_PORT + 2
            let p2p_port: u16 = std::env::var("VISION_P2P_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .or_else(|| {
                    // Fallback: use VISION_PORT + 2
                    std::env::var("VISION_PORT")
                        .ok()
                        .and_then(|p| p.parse::<u16>().ok())
                        .map(|p: u16| p + 2)
                })
                .unwrap_or(7072);

            // Vision Identity removed - use node_id as node_tag
            let node_tag = node_id.clone();
            let admission_ticket = ""; // No longer using admission tickets

            // Check if this node is configured as an anchor
            let is_anchor = std::env::var("P2P_IS_ANCHOR")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(false);

            let mut payload = serde_json::json!({
                "node_id": node_id,
                "ip": "auto-detect", // Guardian will use request IP
                "port": p2p_port,     // P2P TCP port, not HTTP port
                "node_tag": node_tag,
                "admission_ticket": admission_ticket,
                "is_anchor": is_anchor,
            });

            // Include role if we can detect Guardian mode
            if let Ok(guardian_env) = std::env::var("VISION_GUARDIAN_MODE") {
                if guardian_env.to_lowercase() == "true" {
                    payload["role"] = serde_json::Value::String("guardian".to_string());
                }
            }

            // Send registration request with Bearer authentication
            let client = reqwest::Client::new();

            // Check if we have an admission ticket for authentication
            let mut request_builder = client
                .post(&register_url)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10));

            // Add Bearer token if admission ticket is available
            if !admission_ticket.is_empty() {
                info!("[BEACON] Adding Bearer authentication to registration");
                request_builder = request_builder.bearer_auth(admission_ticket);
            } else {
                warn!("[BEACON] No admission ticket available - registration may fail with 401");
            }

            match request_builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        info!("[BEACON] ✅ Successfully registered with Guardian beacon");
                        if let Ok(body) = response.json::<serde_json::Value>().await {
                            info!("[BEACON] Registration response: {:?}", body);
                        }
                    } else {
                        let body = response.text().await.unwrap_or_default();
                        warn!(
                            "[BEACON] Guardian registration failed: status={} body={}",
                            status, body
                        );
                    }
                }
                Err(e) => {
                    error!("[BEACON] Failed to register with Guardian: {}", e);
                }
            }
        });
    }

    /// Stop the beacon
    pub fn stop(&self) {
        if self.running.swap(false, Ordering::SeqCst) {
            info!("[BEACON] Stopped");
        }
    }

    /// Start active beacon broadcasting (Guardian mode)
    fn start_active_beacon(&self) {
        let node_id = self.node_id.clone();
        let interval = self.interval_secs;

        // Get references to the atomic counters (they're already behind Arc in the global)
        // We'll access them via the global BEACON instance
        let beacon_node_id = node_id.clone();

        tokio::spawn(async move {
            info!("[BEACON] Active broadcast loop started");

            // Get broadcast address from env or use default
            let broadcast_addr = std::env::var("BEACON_BROADCAST_ADDR")
                .unwrap_or_else(|_| "255.255.255.255:7072".to_string());

            // Create UDP socket for broadcasting
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(s) => {
                    if let Err(e) = s.set_broadcast(true) {
                        error!("[BEACON] Failed to enable broadcast: {}", e);
                        if let Some(b) = BEACON.get() {
                            b.running.store(false, Ordering::SeqCst);
                        }
                        return;
                    }
                    info!("[BEACON] UDP broadcast socket bound");
                    s
                }
                Err(e) => {
                    error!("[BEACON] Failed to bind UDP socket: {}", e);
                    if let Some(b) = BEACON.get() {
                        b.running.store(false, Ordering::SeqCst);
                    }
                    return;
                }
            };

            loop {
                // Check if still running via global beacon
                let should_continue = BEACON
                    .get()
                    .map(|b| b.running.load(Ordering::Relaxed))
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }

                let peer_count = crate::PEER_MANAGER.try_connected_validated_count();

                // Build heartbeat packet
                let heartbeat = BeaconHeartbeat {
                    node_id: beacon_node_id.clone(),
                    network: crate::vision_constants::VISION_NETWORK_ID.to_string(),
                    height: {
                        let chain = crate::CHAIN.lock();
                        chain.blocks.len() as u64
                    },
                    peer_count,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    version: crate::vision_constants::VISION_VERSION.to_string(),
                };

                // Serialize and broadcast
                match serde_json::to_vec(&heartbeat) {
                    Ok(data) => match socket.send_to(&data, &broadcast_addr) {
                        Ok(sent) => {
                            if let Some(b) = BEACON.get() {
                                let count = b.heartbeat_count.fetch_add(1, Ordering::Relaxed) + 1;
                                info!(
                                        "[BEACON] Broadcasting active | Heartbeat #{} | Height: {} | Peers: {} | {} bytes sent",
                                        count,
                                        heartbeat.height,
                                        heartbeat.peer_count,
                                        sent
                                    );
                            }
                        }
                        Err(e) => {
                            error!("[BEACON] Failed to send heartbeat: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("[BEACON] Failed to serialize heartbeat: {}", e);
                    }
                }

                // Wait for next interval
                sleep(Duration::from_secs(interval)).await;
            }

            info!("[BEACON] Broadcast loop stopped");
        });
    }
}

/// Initialize the global beacon
pub fn init_beacon(node_id: String) {
    let beacon = Arc::new(ConstellationBeacon::new(node_id));

    if BEACON.set(beacon.clone()).is_err() {
        warn!("[BEACON] Already initialized");
        return;
    }

    info!(
        "[BEACON] Initialized | Mode: {} | Node: {}",
        beacon.mode().as_str(),
        beacon.node_id
    );
}

/// Get the global beacon instance
pub fn beacon() -> &'static Arc<ConstellationBeacon> {
    BEACON
        .get()
        .expect("Beacon not initialized - call init_beacon() first")
}

/// Beacon status for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconStatus {
    pub enabled: bool,
    pub mode: String,
    pub running: bool,
    pub uptime_secs: u64,
    pub heartbeat_count: u64,
    pub interval_secs: u64,
}

/// Get beacon status for API
pub fn get_beacon_status() -> BeaconStatus {
    if let Some(b) = BEACON.get() {
        BeaconStatus {
            enabled: true,
            mode: b.mode().as_str().to_string(),
            running: b.is_running(),
            uptime_secs: b.uptime().as_secs(),
            heartbeat_count: b.heartbeat_count(),
            interval_secs: b.interval_secs,
        }
    } else {
        BeaconStatus {
            enabled: false,
            mode: "disabled".to_string(),
            running: false,
            uptime_secs: 0,
            heartbeat_count: 0,
            interval_secs: 0,
        }
    }
}

/// Register a constellation node with the Guardian beacon
/// Called when a constellation node POSTs to /api/beacon/register
pub fn register_constellation_node(node_id: &str, ip: &str, port: u16) {
    // TODO: Store registered nodes in a persistent registry
    // For now, just log the registration
    info!(
        "[BEACON] Registered constellation node: {} at {}:{}",
        node_id, ip, port
    );

    // Future enhancement: Add to peer list, track node health, etc.
    // This could integrate with the existing peer discovery system
}

/// Check beacon configuration on node boot
pub fn check_beacon_config() {
    let is_guardian = crate::is_guardian_mode();

    // Check beacon configuration for constellation nodes
    if !is_guardian {
        match beacon_base_url() {
            Some(url) => {
                info!("[BEACON] Will connect to Guardian beacon at: {}", url);
            }
            None => {
                info!("[BEACON] Running in standalone mode (BEACON_ENDPOINT=standalone/off)");
                info!("[BEACON] To connect to public beacon at {}, remove BEACON_ENDPOINT or set to a URL", DEFAULT_PUBLIC_BEACON);
            }
        }
    }

    // Guardian nodes should always have beacon enabled
    if is_guardian {
        let mode = BeaconMode::from_env();
        if mode != BeaconMode::Active {
            error!("[BEACON] Guardian mode requires ACTIVE beacon mode!");
            error!("[BEACON] Forcing beacon mode to ACTIVE");
            std::env::set_var("BEACON_MODE", "active");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_mode() {
        // Test default mode detection
        std::env::remove_var("BEACON_MODE");
        std::env::remove_var("VISION_GUARDIAN_MODE");

        let mode = BeaconMode::from_env();
        assert_eq!(mode, BeaconMode::Passive);
    }

    #[test]
    fn test_beacon_creation() {
        let beacon = ConstellationBeacon::new("test-node-123".to_string());
        assert!(!beacon.is_running());
        assert_eq!(beacon.heartbeat_count(), 0);
    }
}
