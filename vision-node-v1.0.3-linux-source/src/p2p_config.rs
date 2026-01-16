//! P2P Configuration Module
//!
//! Handles loading seed peer configuration for network bootstrap

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SeedPeersConfig {
    /// List of seed peer addresses for initial network discovery
    pub seed_peers: Vec<String>,
    /// Minimum number of outbound connections to maintain
    pub min_outbound_connections: usize,
    /// Maximum number of outbound connections
    pub max_outbound_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Peer reconnection interval in seconds
    pub reconnection_interval_seconds: u64,
}

impl Default for SeedPeersConfig {
    fn default() -> Self {
        Self {
            seed_peers: vec![
                // Default seed peers - can be overridden in config file
                "localhost:7072".to_string(), // For local P2P testing
            ],
            min_outbound_connections: 8,
            max_outbound_connections: 16,
            connection_timeout_seconds: 10,
            reconnection_interval_seconds: 30,
        }
    }
}

impl SeedPeersConfig {
    pub fn validate(&self) -> Result<()> {
        if self.min_outbound_connections > self.max_outbound_connections {
            return Err(anyhow!(
                "min_outbound_connections ({}) cannot be greater than max_outbound_connections ({})",
                self.min_outbound_connections, self.max_outbound_connections
            ));
        }

        if self.min_outbound_connections == 0 {
            return Err(anyhow!("min_outbound_connections must be at least 1"));
        }

        if self.connection_timeout_seconds == 0 {
            return Err(anyhow!("connection_timeout_seconds must be greater than 0"));
        }

        if self.reconnection_interval_seconds == 0 {
            return Err(anyhow!("reconnection_interval_seconds must be greater than 0"));
        }

        // Validate peer addresses (basic format check)
        for peer in &self.seed_peers {
            if peer.trim().is_empty() {
                return Err(anyhow!("seed peer address cannot be empty"));
            }
            // Basic format validation - should contain colon for host:port
            if !peer.contains(':') {
                return Err(anyhow!("seed peer '{}' must be in host:port format", peer));
            }
        }

        Ok(())
    }
}

/// Load seed peers configuration from TOML file
pub fn load_seed_peers_config(path: &str) -> Result<SeedPeersConfig> {
    let p = Path::new(path);

    // If config file doesn't exist, return defaults
    if !p.exists() {
        let cfg = SeedPeersConfig::default();
        cfg.validate()?;
        return Ok(cfg);
    }

    let raw = fs::read_to_string(p)?;
    let cfg: SeedPeersConfig = toml::from_str(&raw)?;
    cfg.validate()?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = SeedPeersConfig::default();
        assert!(cfg.validate().is_ok());
        assert_eq!(cfg.min_outbound_connections, 8);
        assert_eq!(cfg.max_outbound_connections, 16);
    }

    #[test]
    fn test_invalid_config() {
        let cfg = SeedPeersConfig {
            min_outbound_connections: 10,
            max_outbound_connections: 5, // Invalid: min > max
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }
}