#![allow(dead_code)]
//! Anchor HTTP Backbone - Real-time 7070 connectivity proof
//!
//! Continuously probes anchor nodes over HTTP (port 7070) to demonstrate
//! network connectivity and provide visible proof that nodes are "drinking
//! from the 7070 firehose" for chain truth.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// Anchor HTTP connection state (global, updated by background probe)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AnchorHttpState {
    pub connected: bool,
    pub anchor: Option<String>, // "http://IP:7070"
    pub last_ok: Option<SystemTime>,
    pub last_error: Option<String>,
    pub tip_height: Option<u64>,
    pub tip_hash: Option<String>,
    pub latency_ms: Option<u64>,
}

/// Global anchor HTTP state
pub static ANCHOR_HTTP_STATE: Lazy<RwLock<AnchorHttpState>> =
    Lazy::new(|| RwLock::new(AnchorHttpState::default()));

/// Status response from anchor /api/status endpoint
#[derive(Debug, Deserialize)]
struct AnchorStatusResponse {
    #[serde(default)]
    pub chain_height: u64,
    #[serde(default)]
    pub sync_height: u64,
}

/// Fetch status from a single anchor
async fn fetch_anchor_status(base_url: &str) -> anyhow::Result<AnchorStatusResponse> {
    let url = format!("{}/api/status", base_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client.get(&url).send().await?;
    let status: AnchorStatusResponse = response.json().await?;

    Ok(status)
}

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
}

/// Get anchor candidates (HTTP 7070 URLs) from env, falling back to default anchors.
fn get_anchor_candidates() -> Vec<String> {
    let from_env = std::env::var("VISION_ANCHOR_SEEDS")
        .ok()
        .unwrap_or_default()
        .split(',')
        .filter_map(normalize_anchor_host)
        .collect::<Vec<_>>();

    let hosts = if !from_env.is_empty() {
        from_env
    } else {
        crate::p2p::seed_peers::default_anchor_seeds()
    };

    hosts
        .into_iter()
        .map(|h| format!("http://{}:7070", h))
        .collect()
}

/// Start background HTTP probe loop
pub fn start_anchor_http_probe() {
    tokio::spawn(async move {
        info!(target: "vision_node::p2p::anchor_http", "[ANCHOR_HTTP] 🌐 Starting 7070 backbone probe");

        loop {
            let anchors = get_anchor_candidates();

            if anchors.is_empty() {
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            let mut any_ok = false;

            for anchor_url in anchors {
                let start = std::time::Instant::now();

                match fetch_anchor_status(&anchor_url).await {
                    Ok(status) => {
                        let latency = start.elapsed().as_millis() as u64;

                        if let Ok(mut state) = ANCHOR_HTTP_STATE.write() {
                            state.connected = true;
                            state.anchor = Some(anchor_url.clone());
                            state.last_ok = Some(SystemTime::now());
                            state.last_error = None;
                            state.tip_height = Some(status.chain_height.max(status.sync_height));
                            state.tip_hash = None;
                            state.latency_ms = Some(latency);
                        }

                        info!(
                            target: "vision_node::p2p::anchor_http",
                            "[ANCHOR_HTTP] ✅ Connected to {} ({}ms) - tip height {}",
                            anchor_url,
                            latency,
                            status.chain_height
                        );

                        any_ok = true;
                        break;
                    }
                    Err(e) => {
                        warn!(
                            target: "vision_node::p2p::anchor_http",
                            "[ANCHOR_HTTP] ⚠️  Failed to reach {}: {:?}",
                            anchor_url,
                            e
                        );
                    }
                }
            }

            if !any_ok {
                if let Ok(mut state) = ANCHOR_HTTP_STATE.write() {
                    state.connected = false;
                    state.last_error = Some("All anchors unreachable".to_string());
                }
            }

            // Probe every 5 seconds
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

/// Get current anchor HTTP state (for API response)
pub fn get_anchor_http_state() -> AnchorHttpState {
    ANCHOR_HTTP_STATE
        .read()
        .map(|s| s.clone())
        .unwrap_or_default()
}
