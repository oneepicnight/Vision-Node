//! Constellation Configuration
//!
//! Manages guardian base URL and constellation network settings.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationConfig {
    /// Guardian base URL for passport issuing and network entry (no path/query)
    /// Example: "https://visionworld.tech"
    pub guardian_base_url: String,

    /// Enable beacon-based peer discovery
    #[serde(default = "default_enable_beacon")]
    pub enable_beacon: bool,
}

fn default_enable_beacon() -> bool {
    true
}

impl Default for ConstellationConfig {
    fn default() -> Self {
        Self {
            guardian_base_url: "https://visionworld.tech".to_string(),
            enable_beacon: true,
        }
    }
}

impl ConstellationConfig {
    /// Load constellation config from file, creating default if missing
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read constellation config: {}", e))?;
            let mut config: ConstellationConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse constellation config: {}", e))?;

            // Normalize guardian URL
            config.guardian_base_url = normalize_guardian_base_url(&config.guardian_base_url)?;

            Ok(config)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Save constellation config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize constellation config: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write constellation config: {}", e))?;
        Ok(())
    }

    /// Load from environment variable or use default
    pub fn from_env_or_default() -> Self {
        // Check for BEACON_ENDPOINT env var
        if let Ok(raw) = std::env::var("BEACON_ENDPOINT") {
            let trimmed = raw.trim();

            // Handle standalone mode
            if trimmed.eq_ignore_ascii_case("standalone") || trimmed.eq_ignore_ascii_case("off") {
                let mut config = Self::default();
                config.enable_beacon = false;
                return config;
            }

            // Protect against localhost
            if trimmed.contains("127.0.0.1") || trimmed.contains("localhost") {
                tracing::warn!(
                    "BEACON_ENDPOINT points to localhost ({}). Using default: https://visionworld.tech",
                    trimmed
                );
                return Self::default();
            }

            // Use provided URL
            if !trimmed.is_empty() {
                match normalize_guardian_base_url(trimmed) {
                    Ok(normalized) => {
                        if trimmed != normalized {
                            tracing::warn!(
                                "Normalized guardian URL from '{}' to '{}'",
                                trimmed,
                                normalized
                            );
                        }
                        return ConstellationConfig {
                            guardian_base_url: normalized,
                            enable_beacon: true,
                        };
                    }
                    Err(e) => {
                        tracing::error!(
                            "Invalid BEACON_ENDPOINT URL '{}': {}. Using default.",
                            trimmed,
                            e
                        );
                    }
                }
            }
        }

        // Default
        Self::default()
    }

    /// Build API endpoint URL for guardian requests
    pub fn build_guardian_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.guardian_base_url, path)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Warn if not HTTPS
        if !self.guardian_base_url.starts_with("https://") {
            tracing::warn!("Guardian base URL is not HTTPS: {}", self.guardian_base_url);
        }

        Ok(())
    }
}

/// Normalize guardian base URL to strip path, query, and fragment
fn normalize_guardian_base_url(raw: &str) -> Result<String, String> {
    use url::Url;

    let url = Url::parse(raw).map_err(|e| format!("Invalid URL: {}", e))?;

    let host = url
        .host_str()
        .ok_or_else(|| "Guardian URL missing host".to_string())?;

    let mut normalized = format!("{}://{}", url.scheme(), host);

    if let Some(port) = url.port() {
        normalized.push(':');
        normalized.push_str(&port.to_string());
    }

    // No path, no query, no fragment
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_guardian_url() {
        // Clean URL stays the same
        assert_eq!(
            normalize_guardian_base_url("https://visionworld.tech").unwrap(),
            "https://visionworld.tech"
        );

        // Strip path
        assert_eq!(
            normalize_guardian_base_url("https://visionworld.tech/api/beacon").unwrap(),
            "https://visionworld.tech"
        );

        // Strip query
        assert_eq!(
            normalize_guardian_base_url("https://visionworld.tech?foo=bar").unwrap(),
            "https://visionworld.tech"
        );

        // Preserve port
        assert_eq!(
            normalize_guardian_base_url("http://localhost:7070").unwrap(),
            "http://localhost:7070"
        );

        // Strip everything but scheme://host:port
        assert_eq!(
            normalize_guardian_base_url("https://example.com:8080/path?query#fragment").unwrap(),
            "https://example.com:8080"
        );
    }

    #[test]
    fn test_build_guardian_url() {
        let config = ConstellationConfig {
            guardian_base_url: "https://visionworld.tech".to_string(),
            enable_beacon: true,
        };

        assert_eq!(
            config.build_guardian_url("api/beacon"),
            "https://visionworld.tech/api/beacon"
        );

        assert_eq!(
            config.build_guardian_url("/api/passport"),
            "https://visionworld.tech/api/passport"
        );
    }
}
