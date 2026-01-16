//! FarmHand Configuration
//!
//! Configuration bundled with FarmHand agent executables.
//! Determines whether rig connects to local (LAN) or public (offsite) farm controller.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FarmHandEndpointMode {
    Local,
    Public,
}

impl std::fmt::Display for FarmHandEndpointMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FarmHandEndpointMode::Local => write!(f, "Local"),
            FarmHandEndpointMode::Public => write!(f, "Public"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FarmHandConfig {
    /// Unique rig identifier (UUID)
    pub rig_id: String,

    /// Human-readable rig name
    pub rig_name: String,

    /// WebSocket URL to connect to farm controller
    /// e.g. "ws://192.168.1.10:7070/farm/ws" or "wss://farm.visionworld.tech/farm/ws"
    pub controller_ws_url: String,

    /// Connection mode (Local LAN or Public offsite)
    pub endpoint_mode: FarmHandEndpointMode,

    /// Authentication token for this rig
    pub auth_token: String,

    /// Optional default wallet address for mining rewards
    pub wallet_address: Option<String>,

    /// Optional default thread count (if not specified, uses cpu_count - 1)
    pub default_threads: Option<u32>,
}

impl FarmHandConfig {
    /// Serialize to TOML format
    pub fn to_toml(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize to TOML: {}", e))
    }

    /// Serialize to JSON format
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }

    /// Parse from TOML format
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse TOML: {}", e))
    }

    /// Parse from JSON format
    pub fn from_json(content: &str) -> Result<Self, String> {
        serde_json::from_str(content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_farmhand_config_serialization() {
        let config = FarmHandConfig {
            rig_id: "test-rig-001".to_string(),
            rig_name: "TestRig".to_string(),
            controller_ws_url: "ws://localhost:7070/farm/ws".to_string(),
            endpoint_mode: FarmHandEndpointMode::Local,
            auth_token: "secret123".to_string(),
            wallet_address: Some("0xtest".to_string()),
            default_threads: Some(4),
        };

        // Test TOML round-trip
        let toml_str = config.to_toml().unwrap();
        let parsed = FarmHandConfig::from_toml(&toml_str).unwrap();
        assert_eq!(parsed.rig_id, config.rig_id);
        assert_eq!(parsed.endpoint_mode, config.endpoint_mode);

        // Test JSON round-trip
        let json_str = config.to_json().unwrap();
        let parsed = FarmHandConfig::from_json(&json_str).unwrap();
        assert_eq!(parsed.rig_id, config.rig_id);
        assert_eq!(parsed.endpoint_mode, config.endpoint_mode);
    }
}
