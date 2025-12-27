//! P2P Configuration Module
//!
//! Handles loading and validation of P2P network configuration,
//! including seed peers for initial network bootstrap.

use serde::{Deserialize, Serialize};
use std::fs;

/// Configuration for seed peers used for initial network bootstrap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedPeersConfig {
    /// List of seed peer addresses (host:port format)
    pub seed_peers: Vec<String>,
    /// Minimum number of outbound connections to maintain
    pub min_outbound_connections: usize,
    /// Maximum number of outbound connections allowed
    pub max_outbound_connections: usize,
    /// Interval in seconds between connection maintenance checks
    pub reconnection_interval_seconds: u64,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Optional bootstrap URL to fetch dynamic seed peers (e.g., "https://visionworld.tech/api/bootstrap")
    #[serde(default)]
    pub bootstrap_url: Option<String>,
    /// Prefer IPv4 over IPv6 for peer connections (default: true)
    /// IPv4 is more stable and widely deployed, especially on Windows/home networks
    #[serde(default = "default_prefer_ipv4")]
    pub prefer_ipv4: bool,
    /// Whether this node acts as a public anchor (accepts inbound connections)
    /// Anchors need public IPv4 and open P2P port 7072
    #[serde(default = "default_is_anchor")]
    pub is_anchor: bool,
    /// Enable IPv6 support (default: false for v1 - IPv4 only)
    #[serde(default = "default_enable_ipv6")]
    pub enable_ipv6: bool,

    /// Retry backoff strategy in seconds (default: 60)
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff: u64,

    /// Discovery mode: "static", "dynamic", "hybrid"
    #[serde(default = "default_discovery_mode")]
    pub discovery_mode: DiscoveryMode,

    /// List of permanent seed peers that should never be removed
    #[serde(default)]
    pub permanent_seeds: Vec<String>,

    /// Minimum peer store population before stopping discovery (default: 20)
    #[serde(default = "default_min_peer_population")]
    pub min_peer_store_population: usize,
}

fn default_min_peer_population() -> usize {
    20
}

/// Discovery mode for peer finding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DiscoveryMode {
    /// Use only static seed peers from config
    Static,
    /// Use only dynamic discovery (bootstrap URL, gossip)
    Dynamic,
    /// Use both static and dynamic (recommended)
    #[default]
    Hybrid,
}

fn default_prefer_ipv4() -> bool {
    true
}

fn default_is_anchor() -> bool {
    false
}

fn default_enable_ipv6() -> bool {
    false
}

fn default_retry_backoff() -> u64 {
    60
}

fn default_discovery_mode() -> DiscoveryMode {
    DiscoveryMode::Hybrid
}

impl Default for SeedPeersConfig {
    fn default() -> Self {
        Self {
            seed_peers: vec![
                "127.0.0.1:7072".to_string(), // Local P2P testing
            ],
            min_outbound_connections: 1,
            max_outbound_connections: 8,
            reconnection_interval_seconds: 30,
            connection_timeout_seconds: 10,
            bootstrap_url: Some("https://visionworld.tech/api/bootstrap".to_string()),
            prefer_ipv4: true,
            is_anchor: false,                      // Default to leaf mode
            enable_ipv6: false,                    // v1 is IPv4-only
            retry_backoff: 60,                     // Retry failed connections after 60s
            discovery_mode: DiscoveryMode::Hybrid, // Use both static and dynamic
            permanent_seeds: vec![],               // No permanent seeds by default
            min_peer_store_population: 20,         // Default to 20 peers in store
        }
    }
}

impl SeedPeersConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Seed peers may be empty if dynamic discovery is enabled.
        // (e.g., bootstrap_url + peer book expansion)
        if self.seed_peers.is_empty()
            && self.discovery_mode == DiscoveryMode::Static
            && self.bootstrap_url.as_deref().unwrap_or("").is_empty()
        {
            return Err("seed_peers cannot be empty when discovery_mode=static and bootstrap_url is not set".to_string());
        }

        for peer in &self.seed_peers {
            if !peer.contains(':') {
                return Err(format!(
                    "Invalid peer address format (missing port): {}",
                    peer
                ));
            }
        }

        if self.min_outbound_connections > self.max_outbound_connections {
            return Err(
                "min_outbound_connections cannot be greater than max_outbound_connections"
                    .to_string(),
            );
        }

        if self.min_outbound_connections == 0 {
            return Err("min_outbound_connections must be at least 1".to_string());
        }

        Ok(())
    }
}

/// Load seed peers configuration from TOML file
pub fn load_seed_peers_config(path: &str) -> Result<SeedPeersConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file {}: {}", path, e))?;

    let mut config: SeedPeersConfig =
        toml::from_str(&content).map_err(|e| format!("Failed to parse TOML config: {}", e))?;

    // Treat empty-string bootstrap_url as "unset".
    if config
        .bootstrap_url
        .as_deref()
        .is_some_and(|s| s.trim().is_empty())
    {
        config.bootstrap_url = None;
    }

    // Override from environment variables if present
    if let Ok(val) = std::env::var("P2P_IS_ANCHOR") {
        config.is_anchor = val.to_lowercase() == "true" || val == "1";
    }
    if let Ok(val) = std::env::var("P2P_ENABLE_IPV6") {
        config.enable_ipv6 = val.to_lowercase() == "true" || val == "1";
    }

    config.validate()?;

    tracing::info!(
        seed_peers = config.seed_peers.len(),
        min_connections = config.min_outbound_connections,
        max_connections = config.max_outbound_connections,
        "Loaded seed peers configuration"
    );

    Ok(config)
}
