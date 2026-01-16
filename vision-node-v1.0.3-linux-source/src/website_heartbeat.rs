//! Website Heartbeat Module
//!
//! Sends periodic heartbeats to visionworld.tech with node status
//! Non-blocking - failures only log warnings, never block node operation
#![allow(dead_code)]

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Website heartbeat configuration
const WEBSITE_API_BASE: &str = "https://visionworld.tech/api/net";
const HEARTBEAT_INTERVAL_SECS: u64 = 30;
const HEARTBEAT_TIMEOUT_SECS: u64 = 8;

/// Global website connection status
pub static WEBSITE_STATUS: Lazy<Arc<RwLock<WebsiteStatus>>> =
    Lazy::new(|| Arc::new(RwLock::new(WebsiteStatus::default())));

/// Website connection status
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebsiteStatus {
    /// Website is reachable
    pub reachable: bool,

    /// Last successful heartbeat timestamp (Unix seconds)
    pub last_heartbeat_unix: Option<u64>,

    /// Last HTTP response status code
    pub last_response_status: Option<u16>,

    /// Last error message (if any)
    pub last_error: Option<String>,

    /// Total heartbeats sent
    pub total_sent: u64,

    /// Total successful heartbeats
    pub total_success: u64,
}

impl WebsiteStatus {
    /// Get human-readable status
    pub fn status_text(&self) -> String {
        if self.reachable {
            "✅ Connected".to_string()
        } else if self.last_error.is_some() {
            "⚠️ Error".to_string()
        } else {
            "⏳ Connecting...".to_string()
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_sent == 0 {
            0.0
        } else {
            (self.total_success as f64 / self.total_sent as f64) * 100.0
        }
    }
}

/// Heartbeat payload sent to website
#[derive(Debug, Serialize)]
struct HeartbeatPayload {
    /// Node ID (Ed25519-derived)
    node_id: String,

    /// Ed25519 public key (base64)
    node_pubkey: String,

    /// Pubkey fingerprint (XXXX-XXXX-XXXX-XXXX)
    pubkey_fingerprint: String,

    /// Node role (Anchor/Edge)
    node_role: String,

    /// Chain height
    chain_height: u64,

    /// Latest block hash
    tip_hash: Option<String>,

    /// Unix timestamp
    timestamp: u64,

    /// Node version
    version: String,

    /// Wallet address (if configured)
    wallet_address: Option<String>,

    /// Node approved by wallet
    approved: bool,
}

/// Get current website status
pub fn get_website_status() -> WebsiteStatus {
    WEBSITE_STATUS.read().clone()
}

/// Update website status
fn update_status<F>(updater: F)
where
    F: FnOnce(&mut WebsiteStatus),
{
    let mut status = WEBSITE_STATUS.write();
    updater(&mut status);
}

/// Start website heartbeat background task
pub fn start_website_heartbeat() {
    tokio::spawn(async move {
        info!(
            "🌐 Starting website heartbeat task ({}s interval)",
            HEARTBEAT_INTERVAL_SECS
        );
        info!("   Target: {}/hello", WEBSITE_API_BASE);

        let mut interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));

        loop {
            interval.tick().await;

            // Send heartbeat (don't await to avoid blocking)
            tokio::spawn(send_heartbeat());
        }
    });
}

/// Send a single heartbeat to website
async fn send_heartbeat() {
    // Collect node info
    let payload = match build_heartbeat_payload() {
        Ok(p) => p,
        Err(e) => {
            warn!("⚠️  Failed to build heartbeat payload: {}", e);
            update_status(|s| {
                s.last_error = Some(format!("Payload build error: {}", e));
            });
            return;
        }
    };

    update_status(|s| {
        s.total_sent += 1;
    });

    // Send HTTP POST
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(HEARTBEAT_TIMEOUT_SECS))
        .build()
        .expect("Failed to create HTTP client");

    let url = format!("{}/hello", WEBSITE_API_BASE);

    match client.post(&url).json(&payload).send().await {
        Ok(response) => {
            let status_code = response.status().as_u16();

            if response.status().is_success() {
                debug!("🌐 Website heartbeat sent successfully ({})", status_code);

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                update_status(|s| {
                    s.reachable = true;
                    s.last_heartbeat_unix = Some(now);
                    s.last_response_status = Some(status_code);
                    s.last_error = None;
                    s.total_success += 1;
                });
            } else {
                warn!(
                    "⚠️  Website heartbeat returned error status: {} (operating in offline mode)",
                    status_code
                );

                update_status(|s| {
                    s.reachable = false;
                    s.last_response_status = Some(status_code);
                    // Add informative error for status endpoints
                    s.last_error = Some(format!(
                        "HTTP {} - origin/network issue (offline mode active)",
                        status_code
                    ));
                });
            }
        }
        Err(e) => {
            // Only log warning, never block node operation
            warn!("⚠️  Website heartbeat failed: {} (non-blocking)", e);

            update_status(|s| {
                s.reachable = false;
                s.last_error = Some(format!("{}", e));
            });
        }
    }
}

/// Build heartbeat payload from current node state
fn build_heartbeat_payload() -> anyhow::Result<HeartbeatPayload> {
    use crate::identity::{local_fingerprint, local_node_id, local_pubkey_b64};
    use crate::CHAIN;

    // Get node identity
    let node_id = local_node_id();
    let node_pubkey = local_pubkey_b64();
    let pubkey_fingerprint = local_fingerprint();

    // Get node role
    let node_role = crate::role::current_node_role().as_str().to_string();

    // Get chain info
    let chain = CHAIN.lock();
    let chain_height = chain.blocks.len().saturating_sub(1) as u64;
    let tip_hash = chain.blocks.last().map(|b| b.header.pow_hash.clone());

    // Get wallet address
    let wallet_address = chain
        .db
        .get(b"primary_wallet_address")?
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    drop(chain);

    // Check approval status
    let approved = match crate::node_approval::NodeApproval::load()? {
        Some(approval) => approval.verify(&node_id, &node_pubkey).is_ok(),
        None => false,
    };

    // Get current timestamp
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    Ok(HeartbeatPayload {
        node_id,
        node_pubkey,
        pubkey_fingerprint,
        node_role,
        chain_height,
        tip_hash,
        timestamp,
        version: crate::vision_constants::VISION_VERSION.to_string(),
        wallet_address,
        approved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_website_status_default() {
        let status = WebsiteStatus::default();
        assert!(!status.reachable);
        assert_eq!(status.total_sent, 0);
        assert_eq!(status.success_rate(), 0.0);
    }

    #[test]
    fn test_success_rate_calculation() {
        let mut status = WebsiteStatus::default();
        status.total_sent = 10;
        status.total_success = 8;
        assert_eq!(status.success_rate(), 80.0);
    }

    #[test]
    fn test_status_text() {
        let mut status = WebsiteStatus::default();

        // Initial state
        assert!(status.status_text().contains("Connecting"));

        // Reachable
        status.reachable = true;
        assert!(status.status_text().contains("Connected"));

        // Error
        status.reachable = false;
        status.last_error = Some("test error".to_string());
        assert!(status.status_text().contains("Error"));
    }
}
