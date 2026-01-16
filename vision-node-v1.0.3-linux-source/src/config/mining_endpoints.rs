use serde::{Deserialize, Serialize};

/// Configuration for mining endpoints (public pool URL vs local node URL vs public farm)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningEndpointConfig {
    /// Public pool URL for external miners (e.g., stratum+tcp://pool.visionworld.tech:4242)
    pub public_pool_url: Option<String>,
    /// Local node URL for farm rigs on LAN (e.g., http://192.168.1.10:7070)
    pub local_node_url: Option<String>,
    /// Public HTTP base URL for farm controller (e.g., https://farm.visionworld.tech)
    /// FarmHand offsite rigs will connect here (converted to ws/wss + "/farm/ws")
    pub public_farm_base_url: Option<String>,
}

impl Default for MiningEndpointConfig {
    fn default() -> Self {
        Self {
            public_pool_url: None,
            local_node_url: Some("http://127.0.0.1:7070".to_string()),
            public_farm_base_url: None,
        }
    }
}

impl MiningEndpointConfig {
    /// Convert HTTP base URL to WebSocket URL with /farm/ws path
    fn http_to_ws(base: &str) -> String {
        let ws_base = if base.starts_with("https://") {
            base.replace("https://", "wss://")
        } else if base.starts_with("http://") {
            base.replace("http://", "ws://")
        } else {
            format!("ws://{}", base)
        };

        // Remove trailing slash if present
        let ws_base = ws_base.trim_end_matches('/');

        format!("{}/farm/ws", ws_base)
    }

    /// Get local farm WebSocket URL (for LAN rigs)
    pub fn local_farm_ws_url(&self) -> Option<String> {
        self.local_node_url
            .as_ref()
            .map(|base| Self::http_to_ws(base))
    }

    /// Get public farm WebSocket URL (for offsite rigs)
    pub fn public_farm_ws_url(&self) -> Option<String> {
        self.public_farm_base_url
            .as_ref()
            .map(|base| Self::http_to_ws(base))
    }

    /// Load from sled database
    pub fn load(db: &sled::Db) -> Self {
        match db.get(b"mining_endpoints") {
            Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
            _ => Self::default(),
        }
    }

    /// Save to sled database
    pub fn save(&self, db: &sled::Db) -> Result<(), String> {
        let bytes = serde_json::to_vec(self).map_err(|e| format!("Failed to serialize: {}", e))?;

        db.insert(b"mining_endpoints", bytes)
            .map_err(|e| format!("Failed to save: {}", e))?;

        db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Create from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Validate URLs are properly formatted
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref url) = self.public_pool_url {
            if url.is_empty() {
                return Err("Public pool URL cannot be empty if set".to_string());
            }
            // Basic URL validation
            if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with("stratum+tcp://")
                && !url.starts_with("stratum+ssl://")
            {
                return Err("Public pool URL must start with http://, https://, stratum+tcp://, or stratum+ssl://".to_string());
            }
        }

        if let Some(ref url) = self.local_node_url {
            if url.is_empty() {
                return Err("Local node URL cannot be empty if set".to_string());
            }
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err("Local node URL must start with http:// or https://".to_string());
            }
        }

        Ok(())
    }

    /// Get the public pool URL for external miners
    pub fn get_public_pool_url(&self) -> Option<&str> {
        self.public_pool_url.as_deref()
    }

    /// Get the local node URL for farm rigs
    pub fn get_local_node_url(&self) -> Option<&str> {
        self.local_node_url.as_deref()
    }

    /// Set public pool URL with validation
    pub fn set_public_pool_url(&mut self, url: String) -> Result<(), String> {
        if url.is_empty() {
            self.public_pool_url = None;
            return Ok(());
        }

        // Validate
        if !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("stratum+tcp://")
            && !url.starts_with("stratum+ssl://")
        {
            return Err("Invalid URL scheme".to_string());
        }

        self.public_pool_url = Some(url);
        Ok(())
    }

    /// Set local node URL with validation
    pub fn set_local_node_url(&mut self, url: String) -> Result<(), String> {
        if url.is_empty() {
            self.local_node_url = None;
            return Ok(());
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Local URL must use http:// or https://".to_string());
        }

        self.local_node_url = Some(url);
        Ok(())
    }

    /// Set public farm base URL with validation
    pub fn set_public_farm_base_url(&mut self, url: String) -> Result<(), String> {
        if url.is_empty() {
            self.public_farm_base_url = None;
            return Ok(());
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Public farm URL must use http:// or https://".to_string());
        }

        self.public_farm_base_url = Some(url);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MiningEndpointConfig::default();
        assert_eq!(config.public_pool_url, None);
        assert_eq!(
            config.local_node_url,
            Some("http://127.0.0.1:7070".to_string())
        );
    }

    #[test]
    fn test_validation() {
        let mut config = MiningEndpointConfig::default();

        // Valid public pool URL
        config.public_pool_url = Some("stratum+tcp://pool.example.com:4242".to_string());
        assert!(config.validate().is_ok());

        // Invalid public pool URL
        config.public_pool_url = Some("ftp://invalid.com".to_string());
        assert!(config.validate().is_err());

        // Valid HTTP pool URL
        config.public_pool_url = Some("http://pool.example.com:7070".to_string());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_setters() {
        let mut config = MiningEndpointConfig::default();

        assert!(config
            .set_public_pool_url("http://pool.test.com:7070".to_string())
            .is_ok());
        assert_eq!(
            config.public_pool_url,
            Some("http://pool.test.com:7070".to_string())
        );

        assert!(config
            .set_local_node_url("http://192.168.1.10:7070".to_string())
            .is_ok());
        assert_eq!(
            config.local_node_url,
            Some("http://192.168.1.10:7070".to_string())
        );

        // Empty strings clear the fields
        assert!(config.set_public_pool_url("".to_string()).is_ok());
        assert_eq!(config.public_pool_url, None);
    }
}
