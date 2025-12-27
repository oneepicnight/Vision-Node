use serde::{Deserialize, Serialize};
use std::env;

/// Network configuration for P2P subsystem
/// Controls bootstrap behavior and guardian/beacon dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// If true, node operates in pure swarm mode with ZERO guardian/beacon usage
    /// - Never calls beacon/guardian URLs
    /// - Never uses guardian as relay
    /// - Never registers with guardian
    /// - Only uses: seeds + peer_store + swarm intelligence
    pub pure_swarm_mode: bool,

    /// If true (and pure_swarm_mode is false), allows beacon bootstrap as fallback
    /// Ignored when pure_swarm_mode is enabled
    pub enable_beacon_bootstrap: bool,

    /// If true (and pure_swarm_mode is false), allows guardian relay for NAT traversal
    /// Ignored when pure_swarm_mode is enabled
    pub enable_guardian_relay: bool,

    /// Maximum peers to maintain in active connections
    pub max_peers: usize,

    /// Minimum healthy connections before triggering aggressive discovery
    pub min_healthy_connections: usize,

    /// Enable swarm intelligence features (reputation, gossip, anchors, healing)
    pub enable_swarm_intelligence: bool,

    /// Interval for network self-healing cycles (seconds)
    pub healing_interval_secs: u64,

    /// Interval for retry worker cycles (seconds)
    pub retry_interval_secs: u64,

    // =================== Guardian Launch Sequence ===================

    /// Enable the Guardian launch rules.
    /// When enabled, Guardian mines blocks 1-3, then all other miners start.
    #[serde(default)]
    pub launch_guardian_enabled: bool,

    /// The miner address of the Guardian node.
    /// Only this address can mine blocks 1-3 on mainnet-full when launch enabled.
    #[serde(default)]
    pub guardian_address: String,

    /// If true, the Guardian is allowed to mine again in emergencies.
    /// Normally false - Guardian retires after block 3.
    #[serde(default)]
    pub allow_guardian_emergency_mining: bool,

    /// Whether this node itself is the Guardian node.
    #[serde(default)]
    pub is_guardian: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            // 🌌 DEFAULT: Pure swarm mode - guardian-less operation
            pure_swarm_mode: true,
            enable_beacon_bootstrap: false,
            enable_guardian_relay: false,
            
            // Swarm intelligence settings
            max_peers: 50,
            min_healthy_connections: 5,
            enable_swarm_intelligence: true,
            healing_interval_secs: 60,
            retry_interval_secs: 30,
            
            // Guardian launch defaults
            launch_guardian_enabled: false,
            guardian_address: String::new(),
            allow_guardian_emergency_mining: false,
            is_guardian: false,
        }
    }
}

impl NetworkConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Pure swarm mode is hard-wired (release) with optional debug opt-out.
        config.pure_swarm_mode = crate::vision_constants::pure_swarm_mode();

        // VISION_BEACON_BOOTSTRAP: only used if pure_swarm_mode is false
        if let Ok(val) = env::var("VISION_BEACON_BOOTSTRAP") {
            config.enable_beacon_bootstrap = val.trim().eq_ignore_ascii_case("true");
        }

        // VISION_GUARDIAN_RELAY: only used if pure_swarm_mode is false
        if let Ok(val) = env::var("VISION_GUARDIAN_RELAY") {
            config.enable_guardian_relay = val.trim().eq_ignore_ascii_case("true");
        }

        // VISION_MAX_PEERS
        if let Ok(val) = env::var("VISION_MAX_PEERS") {
            if let Ok(num) = val.parse::<usize>() {
                config.max_peers = num;
            }
        }

        // VISION_MIN_HEALTHY_CONNECTIONS
        if let Ok(val) = env::var("VISION_MIN_HEALTHY_CONNECTIONS") {
            if let Ok(num) = val.parse::<usize>() {
                config.min_healthy_connections = num;
            }
        }

        // VISION_SWARM_INTELLIGENCE
        if let Ok(val) = env::var("VISION_SWARM_INTELLIGENCE") {
            config.enable_swarm_intelligence = val.trim().eq_ignore_ascii_case("true");
        }

        // VISION_HEALING_INTERVAL_SECS
        if let Ok(val) = env::var("VISION_HEALING_INTERVAL_SECS") {
            if let Ok(num) = val.parse::<u64>() {
                config.healing_interval_secs = num;
            }
        }

        // VISION_RETRY_INTERVAL_SECS
        if let Ok(val) = env::var("VISION_RETRY_INTERVAL_SECS") {
            if let Ok(num) = val.parse::<u64>() {
                config.retry_interval_secs = num;
            }
        }

        // VISION_GUARDIAN_ADDRESS
        if let Ok(val) = env::var("VISION_GUARDIAN_ADDRESS") {
            config.guardian_address = val.trim().to_string();
        }

        // VISION_LAUNCH_GUARDIAN_ENABLED
        if let Ok(val) = env::var("VISION_LAUNCH_GUARDIAN_ENABLED") {
            config.launch_guardian_enabled = val.trim().eq_ignore_ascii_case("true");
        }

        // VISION_IS_GUARDIAN
        if let Ok(val) = env::var("VISION_IS_GUARDIAN") {
            config.is_guardian = val.trim().eq_ignore_ascii_case("true");
        }

        // VISION_GUARDIAN_EMERGENCY_MINING
        if let Ok(val) = env::var("VISION_GUARDIAN_EMERGENCY_MINING") {
            config.allow_guardian_emergency_mining = val.trim().eq_ignore_ascii_case("true");
        }

        config
    }

    /// Returns human-readable mode description
    pub fn mode_description(&self) -> &'static str {
        if self.pure_swarm_mode {
            "PURE SWARM MODE (guardian-less)"
        } else {
            "HYBRID MODE (guardian fallback enabled)"
        }
    }

    /// Check if guardian features should be available
    pub fn guardian_available(&self) -> bool {
        !self.pure_swarm_mode
    }

    /// Check if beacon bootstrap should be attempted
    pub fn should_use_beacon(&self) -> bool {
        !self.pure_swarm_mode && self.enable_beacon_bootstrap
    }

    /// Check if guardian relay should be attempted
    pub fn should_use_guardian_relay(&self) -> bool {
        !self.pure_swarm_mode && self.enable_guardian_relay
    }

    // =================== Guardian Launch Helpers ===================

    /// Validate Guardian configuration
    pub fn validate_guardian_config(&self) -> Result<(), String> {
        // If launch is enabled, guardian_address must be set
        if self.launch_guardian_enabled {
            if self.guardian_address.is_empty() {
                return Err(
                    "Guardian address must be set when launch_guardian_enabled=true"
                        .to_string()
                );
            }

            // Validate guardian address is a LAND address
            if !self.guardian_address.starts_with("land1") {
                return Err(format!(
                    "Invalid Guardian address: {}. Must be a LAND address (starts with 'land1')",
                    self.guardian_address
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_pure_swarm() {
        let config = NetworkConfig::default();
        assert!(config.pure_swarm_mode);
        assert!(!config.enable_beacon_bootstrap);
        assert!(!config.enable_guardian_relay);
        assert!(config.enable_swarm_intelligence);
    }

    #[test]
    fn test_pure_swarm_blocks_guardian() {
        let config = NetworkConfig {
            pure_swarm_mode: true,
            enable_beacon_bootstrap: true, // ignored
            enable_guardian_relay: true,   // ignored
            ..Default::default()
        };

        assert!(!config.guardian_available());
        assert!(!config.should_use_beacon());
        assert!(!config.should_use_guardian_relay());
    }

    #[test]
    fn test_hybrid_mode_allows_guardian() {
        let config = NetworkConfig {
            pure_swarm_mode: false,
            enable_beacon_bootstrap: true,
            enable_guardian_relay: true,
            ..Default::default()
        };

        assert!(config.guardian_available());
        assert!(config.should_use_beacon());
        assert!(config.should_use_guardian_relay());
    }
}
