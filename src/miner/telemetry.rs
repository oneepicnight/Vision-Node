//! Network-Wide Mining Telemetry
//!
//! Anonymous hashrate reporting and global tuning hints for new nodes.
//! Enables collective learning: "What works best on similar hardware?"

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Anonymous performance snapshot for network telemetry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemSnapshot {
    /// Normalized CPU model (redacted identifying info)
    pub cpu_model: String,
    /// Logical cores (hyperthreaded)
    pub logical_cores: u32,
    /// Physical cores
    pub physical_cores: u32,
    /// PoW algorithm identifier
    pub pow_algo: String,
    /// Mining profile: "laptop", "balanced", "beast"
    pub profile: String,
    /// Worker threads used
    pub threads: u32,
    /// Batch size for SIMD processing
    pub batch_size: u32,
    /// Average hashrate (H/s)
    pub avg_hashrate_hs: f64,
    /// Number of samples contributing to this average
    pub sample_count: u64,
    /// Client version (e.g., "v2.0.0")
    pub client_version: String,
}

/// Suggestion response from telemetry server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemSuggestion {
    pub threads: u32,
    pub batch_size: u32,
    pub expected_hashrate: f64,
    pub confidence: f64, // 0.0-1.0, based on sample count
}

/// Telemetry client for anonymous reporting
pub struct TelemetryClient {
    endpoint: String,
    client: reqwest::blocking::Client,
    enabled: bool,
}

impl TelemetryClient {
    /// Create new telemetry client
    pub fn new(endpoint: Option<String>, enabled: bool) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self {
            endpoint: endpoint.unwrap_or_else(|| "https://telemetry.visionnetwork.io".to_string()),
            client,
            enabled,
        }
    }

    /// Report performance snapshot to telemetry server
    pub fn report_snapshot(&self, snapshot: &TelemSnapshot) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let url = format!("{}/telem/miner-stats", self.endpoint);

        self.client
            .post(&url)
            .json(snapshot)
            .send()
            .context("Failed to send telemetry snapshot")?;

        Ok(())
    }

    /// Query suggestions for similar hardware
    pub fn fetch_suggestions(
        &self,
        cpu_model: &str,
        logical_cores: u32,
        pow_algo: &str,
        profile: &str,
    ) -> Result<Vec<TelemSuggestion>> {
        if !self.enabled {
            return Ok(Vec::new());
        }

        let url = format!(
            "{}/telem/suggestions?cpu_model={}&cores={}&pow_algo={}&profile={}",
            self.endpoint,
            urlencoding::encode(cpu_model),
            logical_cores,
            urlencoding::encode(pow_algo),
            profile
        );

        let response = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch telemetry suggestions")?;

        if !response.status().is_success() {
            return Ok(Vec::new()); // Server has no suggestions
        }

        let suggestions: Vec<TelemSuggestion> = response
            .json()
            .context("Failed to parse telemetry suggestions")?;

        Ok(suggestions)
    }

    /// Check if telemetry is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Normalize CPU model string for privacy
pub fn normalize_cpu_model(raw_model: &str) -> String {
    let model = raw_model.to_lowercase();

    // Remove serial numbers, specific stepping, etc.
    let normalized = model
        .replace("(r)", "")
        .replace("(tm)", "")
        .replace("cpu", "")
        .replace("processor", "");

    // Extract just the base model
    // Example: "AMD Ryzen 9 5900X 12-Core Processor" -> "amd ryzen 9 5900x"
    // Example: "Intel Core i7-10700K CPU @ 3.80GHz" -> "intel core i7-10700k"

    let parts: Vec<&str> = normalized
        .split_whitespace()
        .filter(|s| !s.is_empty() && !s.contains("@") && !s.contains("ghz"))
        .take(5) // First 5 meaningful tokens
        .collect();

    parts.join(" ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cpu_model() {
        assert_eq!(
            normalize_cpu_model("AMD Ryzen 9 5900X 12-Core Processor"),
            "amd ryzen 9 5900x 12-core"
        );

        assert_eq!(
            normalize_cpu_model("Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz"),
            "intel core i7-10700k"
        );

        assert_eq!(normalize_cpu_model("Apple M2 Pro"), "apple m2 pro");
    }

    #[test]
    fn test_telemetry_disabled() {
        let client = TelemetryClient::new(None, false);
        assert!(!client.is_enabled());

        let snapshot = TelemSnapshot {
            cpu_model: "test".to_string(),
            logical_cores: 8,
            physical_cores: 4,
            pow_algo: "vision-pow-v1".to_string(),
            profile: "balanced".to_string(),
            threads: 8,
            batch_size: 4,
            avg_hashrate_hs: 1000.0,
            sample_count: 10,
            client_version: "v2.0.0".to_string(),
        };

        // Should succeed but do nothing
        assert!(client.report_snapshot(&snapshot).is_ok());
    }
}
