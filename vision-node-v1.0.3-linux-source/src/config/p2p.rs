//! P2P Configuration
//!
//! Manages P2P network settings including IPv4-only mode.

use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pConfig {
    /// Force IPv4-only connections (default: true for stability)
    #[serde(default = "default_force_ipv4")]
    pub force_ipv4: bool,

    /// P2P listen port (default: 7072)
    #[serde(default = "default_p2p_port")]
    pub listen_port: u16,

    /// Maximum concurrent peer connections
    #[serde(default = "default_max_peers")]
    pub max_peers: usize,

    /// Minimum peers before allowing mining
    #[serde(default = "default_min_peers_for_mining")]
    pub min_peers_for_mining: usize,
}

fn default_force_ipv4() -> bool {
    true
}

fn default_p2p_port() -> u16 {
    7072
}

fn default_max_peers() -> usize {
    50
}

fn default_min_peers_for_mining() -> usize {
    2
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            force_ipv4: default_force_ipv4(),
            listen_port: default_p2p_port(),
            max_peers: default_max_peers(),
            min_peers_for_mining: default_min_peers_for_mining(),
        }
    }
}

impl P2pConfig {
    /// Load P2P config from file, creating default if missing
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read P2P config: {}", e))?;
            let config: P2pConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse P2P config: {}", e))?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Save P2P config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize P2P config: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write P2P config: {}", e))?;
        Ok(())
    }

    /// Get bind address for P2P listener based on IPv4 mode
    pub fn get_bind_address(&self) -> SocketAddr {
        if self.force_ipv4 {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.listen_port)
        } else {
            SocketAddr::new(
                IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
                self.listen_port,
            )
        }
    }

    /// Filter peer address based on IPv4 mode
    pub fn should_connect_to_peer(&self, addr: &str) -> bool {
        if !self.force_ipv4 {
            return true; // Allow all addresses
        }

        // Parse address and check if IPv4
        if let Ok(socket_addr) = addr.parse::<SocketAddr>() {
            return socket_addr.is_ipv4();
        }

        // Try parsing as IP without port
        if let Some(host) = addr.split(':').next() {
            if let Ok(ip) = host.parse::<IpAddr>() {
                return ip.is_ipv4();
            }
        }

        // If can't parse, allow (might be hostname that resolves to IPv4)
        true
    }

    /// Filter a list of peer addresses
    pub fn filter_peer_addresses(&self, addresses: Vec<String>) -> Vec<String> {
        if !self.force_ipv4 {
            return addresses;
        }

        addresses
            .into_iter()
            .filter(|addr| self.should_connect_to_peer(addr))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_filtering() {
        let config = P2pConfig {
            force_ipv4: true,
            ..Default::default()
        };

        // IPv4 addresses should pass
        assert!(config.should_connect_to_peer("192.168.1.1:7072"));
        assert!(config.should_connect_to_peer("10.0.0.1:7072"));

        // IPv6 addresses should be filtered
        assert!(!config.should_connect_to_peer("[2001:db8::1]:7072"));
        assert!(!config.should_connect_to_peer("[::1]:7072"));
    }

    #[test]
    fn test_bind_address() {
        let ipv4_config = P2pConfig {
            force_ipv4: true,
            listen_port: 7072,
            ..Default::default()
        };

        let addr = ipv4_config.get_bind_address();
        assert!(addr.is_ipv4());
        assert_eq!(addr.port(), 7072);
    }
}
