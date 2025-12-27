#![allow(dead_code)]
//! Reachability Testing & NAT Type Detection
//!
//! Implements handshake-based reachability validation through reverse connections.
//! Determines NAT type (Open vs Restricted) for proper P2P advertisement.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Reachability test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityResult {
    pub public_reachable: bool,
    pub nat_type: String, // "Open", "Restricted", "Symmetric", "Unknown"
    pub tested_at: u64,
    pub attempts: u32,
    pub success_count: u32,
}

impl ReachabilityResult {
    pub fn unreachable() -> Self {
        Self {
            public_reachable: false,
            nat_type: "Restricted".to_string(),
            tested_at: Self::now(),
            attempts: 3,
            success_count: 0,
        }
    }

    pub fn reachable() -> Self {
        Self {
            public_reachable: true,
            nat_type: "Open".to_string(),
            tested_at: Self::now(),
            attempts: 1,
            success_count: 1,
        }
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Reachability tester for reverse connection validation
pub struct ReachabilityTester {
    /// Cache of test results by peer address
    results: Arc<Mutex<HashMap<String, ReachabilityResult>>>,
    /// Test timeout duration
    timeout_duration: Duration,
    /// Maximum retry attempts
    max_attempts: u32,
}

impl ReachabilityTester {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::new())),
            timeout_duration: Duration::from_secs(5),
            max_attempts: 3,
        }
    }

    /// Test if a peer is reachable by attempting reverse connection
    pub async fn test_reachability(
        &self,
        advertised_ip: &str,
        advertised_port: u16,
        token: &str,
    ) -> ReachabilityResult {
        let address = format!("{}:{}", advertised_ip, advertised_port);

        // Step 4: Never probe private/LAN IPs unless explicitly allowed
        {
            let local_ips = crate::p2p::ip_filter::get_local_ips();
            if let Some(reason) = crate::p2p::ip_filter::validate_ip_for_dial(&address, &local_ips)
            {
                warn!(
                    target: "p2p::reachability",
                    "Skipping reachability probe for {}: {}",
                    address,
                    reason
                );

                let result = ReachabilityResult {
                    public_reachable: false,
                    nat_type: "Unknown".to_string(),
                    tested_at: ReachabilityResult::now(),
                    attempts: 0,
                    success_count: 0,
                };
                self.cache_result(&address, result.clone());
                return result;
            }
        }

        info!(
            target: "p2p::reachability",
            "Testing reachability for {} (token: {}...)", address, &token[..8.min(token.len())]
        );

        let mut success_count = 0;

        for attempt in 1..=self.max_attempts {
            debug!(
                target: "p2p::reachability",
                "Attempt {}/{} for {}", attempt, self.max_attempts, address
            );

            match self.try_connect(&address).await {
                Ok(true) => {
                    success_count += 1;
                    info!(
                        target: "p2p::reachability",
                        "✓ Reverse connection successful to {} (attempt {}/{})",
                        address, attempt, self.max_attempts
                    );

                    // Continue testing to get full picture (2/3 threshold)
                    debug!(
                        target: "p2p::reachability",
                        "Success {}/{} for {}", success_count, self.max_attempts, address
                    );
                }
                Ok(false) => {
                    debug!(
                        target: "p2p::reachability",
                        "✗ Reverse connection failed to {} (attempt {}/{})",
                        address, attempt, self.max_attempts
                    );
                }
                Err(e) => {
                    warn!(
                        target: "p2p::reachability",
                        "Connection error to {}: {} (attempt {}/{})",
                        address, e, attempt, self.max_attempts
                    );
                }
            }

            // Brief delay between attempts
            if attempt < self.max_attempts {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        // Fix 1: Use 2/3 threshold for public_reachable (more forgiving)
        let is_reachable = success_count >= 2;

        if is_reachable {
            info!(
                target: "p2p::reachability",
                "✓ Peer {} IS reachable ({}/{} probes succeeded)",
                address, success_count, self.max_attempts
            );
        } else {
            warn!(
                target: "p2p::reachability",
                "✗ Peer {} NOT reachable ({}/{} probes succeeded, need 2/3)",
                address, success_count, self.max_attempts
            );
        }

        let result = ReachabilityResult {
            public_reachable: is_reachable,
            nat_type: Self::determine_nat_type(success_count, self.max_attempts),
            tested_at: ReachabilityResult::now(),
            attempts: self.max_attempts,
            success_count,
        };

        // Cache result
        self.cache_result(&address, result.clone());
        result
    }

    /// Attempt a single TCP connection
    async fn try_connect(&self, address: &str) -> Result<bool, String> {
        match timeout(self.timeout_duration, TcpStream::connect(address)).await {
            Ok(Ok(_stream)) => {
                // Connection successful
                Ok(true)
            }
            Ok(Err(e)) => {
                // Connection failed
                Err(format!("Connection failed: {}", e))
            }
            Err(_) => {
                // Timeout
                Err("Connection timeout".to_string())
            }
        }
    }

    /// Determine NAT type based on test results
    /// Fix 1: Updated for 2/3 threshold model
    fn determine_nat_type(success_count: u32, _total_attempts: u32) -> String {
        if success_count == 0 {
            "Restricted".to_string()
        } else if success_count >= 2 {
            // 2+ successes = reachable (Open or Cone NAT)
            "Open".to_string()
        } else {
            // 1 success only = intermittent (Symmetric NAT)
            "Symmetric".to_string()
        }
    }

    /// Get cached test result if available and recent
    pub fn get_cached_result(&self, address: &str) -> Option<ReachabilityResult> {
        let results = self.results.lock().unwrap();
        results.get(address).cloned()
    }

    /// Cache a test result
    fn cache_result(&self, address: &str, result: ReachabilityResult) {
        let mut results = self.results.lock().unwrap();
        results.insert(address.to_string(), result);
    }

    /// Clear old cache entries (older than 1 hour)
    pub fn cleanup_cache(&self) {
        let mut results = self.results.lock().unwrap();
        let now = ReachabilityResult::now();
        results.retain(|_, result| {
            now - result.tested_at < 3600 // Keep entries less than 1 hour old
        });
    }
}

impl Default for ReachabilityTester {
    fn default() -> Self {
        Self::new()
    }
}

/// Reachability handshake message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityHandshake {
    /// Unique token for this test
    pub token: String,
    /// Advertised IP address
    pub advertised_ip: String,
    /// Advertised port (should be 7072)
    pub advertised_port: u16,
    /// Timestamp of handshake
    pub timestamp: u64,
}

impl ReachabilityHandshake {
    pub fn new(advertised_ip: String, advertised_port: u16) -> Self {
        Self {
            token: Self::generate_token(),
            advertised_ip,
            advertised_port,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn generate_token() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }
}
