// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

//! Control Plane Client - All HTTP 7070 communication goes through here
//!
//! The control plane (port 7070) is the nervous system:
//! - Peer discovery and identity
//! - Cluster membership and health
//! - Chain tip and sync status
//! - Exchange readiness signals
//!
//! The data plane (port 7072) is optional muscle for block/tx streaming.
#![allow(dead_code)]

use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime};
use tracing::{debug, info, warn};

// ============================================================================
// BACKBONE STATE (Global Truth)
// ============================================================================

/// Backbone state - the single source of truth for cluster health
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BackboneState {
    pub connected: bool,
    pub best_anchor: Option<String>, // "http://IP:7070"
    pub latency_ms: Option<u64>,
    pub last_ok: Option<SystemTime>,
    pub observed_tip_height: u64,
    pub observed_tip_hash: Option<String>,
    pub peerbook_count: usize,
    pub cluster_size_estimate: usize,
    pub exchange_ready: bool,
    pub last_error: Option<String>,
}

/// Global backbone state (shared by probe loop and public APIs)
static BACKBONE_STATE: Lazy<RwLock<BackboneState>> =
    Lazy::new(|| RwLock::new(BackboneState::default()));

/// Read-only snapshot of backbone state
pub fn get_backbone_state() -> BackboneState {
    BACKBONE_STATE.read().unwrap().clone()
}

/// Mutate backbone state atomically
pub fn update_backbone_state<F: FnOnce(&mut BackboneState)>(f: F) {
    if let Ok(mut guard) = BACKBONE_STATE.write() {
        f(&mut *guard);
    }
}

// ============================================================================
// CONTROL PLANE CLIENT
// ============================================================================

pub struct ControlPlaneClient {
    client: Client,
    timeout: Duration,
}

impl ControlPlaneClient {
    pub fn new() -> Self {
        let timeout = Duration::from_secs(5);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("control plane client");

        Self { client, timeout }
    }

    /// GET /api/status from an anchor/control-plane peer
    pub async fn fetch_status(&self, http_url: &str) -> Result<StatusResponse> {
        let url = format!("{}/api/status", http_url.trim_end_matches('/'));
        let response = self.client.get(&url).timeout(self.timeout).send().await?;
        let status: StatusResponse = response.json().await?;
        Ok(status)
    }

    /// GET /api/p2p/seed_peers from an anchor/control-plane peer
    pub async fn fetch_seed_peers(&self, http_url: &str) -> Result<Vec<PublicPeerInfo>> {
        let url = format!("{}/api/p2p/seed_peers", http_url.trim_end_matches('/'));
        let response = self.client.get(&url).timeout(self.timeout).send().await?;
        let peers: Vec<PublicPeerInfo> = response.json().await?;
        Ok(peers)
    }

    /// Post heartbeat (optional telemetry)
    pub async fn post_heartbeat(&self, http_url: &str, payload: HeartbeatPayload) -> Result<()> {
        let url = format!("{}/api/p2p/heartbeat", http_url.trim_end_matches('/'));

        self.client
            .post(&url)
            .json(&payload)
            .timeout(self.timeout)
            .send()
            .await?;

        Ok(())
    }

    /// Probe peer with exponential backoff retry
    pub async fn probe_with_retry(
        &self,
        http_url: &str,
        max_retries: u32,
    ) -> Result<StatusResponse> {
        let mut attempt = 0;
        let mut last_error = None;

        while attempt < max_retries {
            match self.fetch_status(http_url).await {
                Ok(status) => return Ok(status),
                Err(e) => {
                    last_error = Some(e);
                    attempt += 1;

                    if attempt < max_retries {
                        let backoff = Duration::from_millis(100 * 2_u64.pow(attempt));
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }
}

impl Default for ControlPlaneClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Global control plane client instance
pub static CONTROL_PLANE: Lazy<ControlPlaneClient> = Lazy::new(ControlPlaneClient::new);

// ============================================================================
// RESPONSE TYPES (Control Plane Protocol)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub node_id: String,
    pub pubkey_b64: Option<String>, // Ed25519 public key for node_id verification
    pub chain_id: String,
    pub genesis_hash: String,
    pub protocol_version: u32,
    pub node_version: String,
    pub is_anchor: bool,
    pub advertised_ip: Option<String>,
    pub advertised_port: Option<u16>,
    pub peers: Option<Vec<PublicPeerInfo>>,
}

impl HelloResponse {
    /// Verify that the claimed node_id matches the provided public key
    pub fn verify_node_id(&self) -> bool {
        use base64::Engine as _;

        if let Some(ref pubkey_b64) = self.pubkey_b64 {
            if let Ok(pubkey_bytes) = base64::engine::general_purpose::STANDARD.decode(pubkey_b64) {
                if pubkey_bytes.len() == 32 {
                    let derived_id = crate::identity::node_id_from_pubkey(&pubkey_bytes);
                    return derived_id == self.node_id;
                }
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicPeerInfo {
    pub address: String,              // "ip:port" for P2P (7072)
    pub http_address: Option<String>, // "ip:port" for HTTP (7070)
    pub is_anchor: bool,
    pub last_seen: Option<u64>, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub chain_height: u64,
    pub sync_height: u64,
    pub network_estimated_height: u64,
    pub connected_peers: usize,
    pub sync_status: String,
    pub can_mine: bool,
    pub node_role: String,
    pub exchange_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub node_id: String,
    pub height: u64,
    pub peers: usize,
    pub timestamp: u64,
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Robust peer address parser (Fix B)
/// Handles: http://ip:port, https://ip:port, ip:port, [ipv6]:port
fn parse_peer_address(addr: &str) -> Option<(String, u16)> {
    let mut clean_addr = addr.trim();

    // Strip scheme if present
    if clean_addr.starts_with("http://") {
        clean_addr = &clean_addr[7..];
    } else if clean_addr.starts_with("https://") {
        clean_addr = &clean_addr[8..];
    }

    // Strip path if present
    if let Some(slash_pos) = clean_addr.find('/') {
        clean_addr = &clean_addr[..slash_pos];
    }

    // Try parsing as SocketAddr first (handles both IPv4 and [IPv6]:port)
    if let Ok(sock_addr) = clean_addr.parse::<SocketAddr>() {
        return Some((sock_addr.ip().to_string(), sock_addr.port()));
    }

    // Fallback: split on last colon (for cases like "ip:port")
    if let Some(colon_pos) = clean_addr.rfind(':') {
        let ip = &clean_addr[..colon_pos];
        let port_str = &clean_addr[colon_pos + 1..];

        if let Ok(port) = port_str.parse::<u16>() {
            return Some((ip.to_string(), port));
        }
    }

    None
}

// ============================================================================
// CONTROL PLANE PROBE LOOP (Backbone Health Monitor)
// ============================================================================

/// Start the control plane backbone probe loop (polite for pure swarm)
/// Only runs when: pure_swarm=false OR explicit anchor seeds provided
pub fn start_backbone_probe_loop() {
    let summary_interval = Duration::from_secs(60);

    tokio::spawn(async move {
        // Check if pure swarm mode is enabled
        if crate::vision_constants::pure_swarm_mode() {
            // Check if we have explicit anchor seeds from environment
            let explicit_seeds = std::env::var("VISION_ANCHOR_SEEDS")
                .or_else(|_| std::env::var("VISION_ANCHORS"))
                .unwrap_or_default();

            if explicit_seeds.is_empty() {
                tracing::info!(target: "vision_node::control_plane", "[BACKBONE] ✅ Pure swarm mode - 7070 probe disabled (no explicit anchors)");
                return; // Exit task entirely
            } else {
                tracing::info!(target: "vision_node::control_plane", "[BACKBONE] 🌐 Pure swarm mode with explicit anchors - enabling 7070 probe");
            }
        }

        tracing::debug!(target: "vision_node::control_plane", "[BACKBONE] 🌐 Starting 7070 probe loop (HTTP fallback enabled)");

        let mut was_all_unreachable = false;
        let mut last_summary = Instant::now();

        loop {
            // Get anchors from environment (or fallback to defaults)
            let anchors = parse_anchor_seeds();

            if anchors.is_empty() {
                // Should never happen with fallback, but handle gracefully
                warn!(target: "vision_node::control_plane", "[BACKBONE] ⚠️ No anchors available - retrying in 10s");
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            // Probe all anchors and pick best one
            let mut best_result: Option<(String, StatusResponse, u64)> = None;
            let mut reachable = 0usize;

            for anchor in &anchors {
                let http_url = format!("http://{}:7070", anchor);
                let start = std::time::Instant::now();

                match CONTROL_PLANE.fetch_status(&http_url).await {
                    Ok(status) => {
                        reachable += 1;
                        let latency = start.elapsed().as_millis() as u64;

                        // Prefer higher height, then lower latency
                        let score = status.chain_height as i64 - (latency as i64 / 10);

                        if best_result
                            .as_ref()
                            .map(|(_, s, l)| score > s.chain_height as i64 - (*l as i64 / 10))
                            .unwrap_or(true)
                        {
                            best_result = Some((http_url.clone(), status, latency));
                        }
                    }
                    Err(_e) => {
                        // Failures are silent in probe loop - only state changes WARN
                        tracing::trace!(
                            target: "vision_node::control_plane",
                            "[BACKBONE] probe failed: {} (ignored)",
                            http_url
                        );
                    }
                }
            }

            // Update backbone state with best result
            if let Some((anchor_url, status, latency)) = best_result {
                update_backbone_state(|state| {
                    state.connected = true;
                    state.best_anchor = Some(anchor_url.clone());
                    state.latency_ms = Some(latency);
                    state.last_ok = Some(SystemTime::now());
                    state.observed_tip_height = status.chain_height.max(status.sync_height);
                    state.observed_tip_hash = None; // TODO: Add to status response
                    state.cluster_size_estimate = status.connected_peers;
                    state.exchange_ready = status.exchange_ready;
                    state.last_error = None;
                });

                // WARN only on state transition: unreachable → reachable
                if was_all_unreachable {
                    warn!(
                        target: "vision_node::control_plane",
                        "[BACKBONE] ✅ Backbone restored - connected to {} ({}ms) tip={} peers={}",
                        anchor_url, latency, status.chain_height, status.connected_peers
                    );
                }

                was_all_unreachable = false;
            } else {
                // All anchors failed
                update_backbone_state(|state| {
                    state.connected = false;
                    state.last_error = Some("All anchors unreachable".to_string());
                });

                // WARN only on state transition: reachable → unreachable
                if !was_all_unreachable {
                    warn!(
                        target: "vision_node::control_plane",
                        "[BACKBONE] ⚠️  All anchors unreachable (0/{} reachable) - HTTP fallback unavailable",
                        anchors.len()
                    );
                }

                was_all_unreachable = true;
            }

            // INFO summary every 60s: anchors_ok=N/7, last_tip=H, mode=pure_swarm_http_fallback
            if last_summary.elapsed() >= summary_interval {
                let s = get_backbone_state();
                info!(
                    target: "vision_node::control_plane",
                    "[BACKBONE] anchors_ok={}/{}, last_tip={}, mode=pure_swarm_http_fallback",
                    reachable,
                    anchors.len(),
                    s.observed_tip_height,
                );
                last_summary = std::time::Instant::now();
            }

            // Probe every 5 seconds
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

/// Parse VISION_ANCHOR_SEEDS environment variable
/// Falls back to genesis seed IPs if not set
fn parse_anchor_seeds() -> Vec<String> {
    if let Ok(seeds_env) = std::env::var("VISION_ANCHOR_SEEDS") {
        let anchors: Vec<String> = seeds_env
            .split(',')
            .filter_map(|raw| {
                let mut s = raw.trim();
                if s.is_empty() {
                    return None;
                }
                if let Some(rest) = s.strip_prefix("http://") {
                    s = rest;
                } else if let Some(rest) = s.strip_prefix("https://") {
                    s = rest;
                }
                if let Some((host_port, _path)) = s.split_once('/') {
                    s = host_port;
                }
                let host = if let Some((h, _port)) = s.rsplit_once(':') {
                    h
                } else {
                    s
                };
                let host = host.trim();
                if host.is_empty() {
                    None
                } else {
                    Some(host.to_string())
                }
            })
            .collect();

        if !anchors.is_empty() {
            info!(
                target: "vision_node::control_plane",
                "[BACKBONE] Using {} anchors from VISION_ANCHOR_SEEDS (probing HTTP 7070): {}",
                anchors.len(),
                anchors.join(", ")
            );
            return anchors;
        }
    }

    // Fallback to genesis seed IPs
    let defaults = crate::p2p::seed_peers::default_anchor_seeds();
    info!(
        target: "vision_node::control_plane",
        "[BACKBONE] Using {} default anchor seeds (probing HTTP 7070): {}",
        defaults.len(),
        defaults.join(", ")
    );
    defaults
}

// ============================================================================
// PEER HEALING LOOP (HTTP-Based Peer Discovery)
// ============================================================================

/// Start peer healing loop (discovers and maintains peers via HTTP 7070)
pub fn start_peer_healing_loop() {
    tokio::spawn(async move {
        info!(target: "vision_node::control_plane", "[HEALING] 🔄 Starting HTTP-based peer healing loop");

        // Wait for initial backbone connection
        tokio::time::sleep(Duration::from_secs(10)).await;

        loop {
            let state = get_backbone_state();

            if !state.connected {
                // No backbone - wait for connection
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            // Fetch seed peers from best anchor
            if let Some(anchor_url) = state.best_anchor {
                match CONTROL_PLANE.fetch_seed_peers(&anchor_url).await {
                    Ok(peers) => {
                        info!(
                            target: "vision_node::control_plane",
                            "[HEALING] 📥 Fetched {} peers from anchor",
                            peers.len()
                        );

                        // Update peer store with HTTP-discovered peers
                        if let Some(chain) = crate::CHAIN.try_lock() {
                            if let Ok(peer_store) =
                                crate::p2p::peer_store::PeerStore::new(&chain.db)
                            {
                                for peer in &peers {
                                    // Robust address parsing (Fix B)
                                    match parse_peer_address(&peer.address) {
                                        Some((ip, port)) => {
                                            let _ = peer_store.upsert_peer_from_http(
                                                ip,
                                                port,
                                                peer.is_anchor,
                                            );
                                        }
                                        None => {
                                            debug!(
                                                target: "vision_node::control_plane",
                                                "[HEALING] Skipping invalid peer address: {}",
                                                peer.address
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // Update backbone peerbook count
                        update_backbone_state(|s| {
                            s.peerbook_count = peers.len();
                        });
                    }
                    Err(e) => {
                        warn!(
                            target: "vision_node::control_plane",
                            "[HEALING] ⚠️  Failed to fetch peers: {:?}", e
                        );
                    }
                }
            }

            // Heal every 30 seconds
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}
