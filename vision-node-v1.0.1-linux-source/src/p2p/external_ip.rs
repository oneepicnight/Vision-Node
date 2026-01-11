#![allow(dead_code)]
//! External IP Detection and Caching
//!
//! Automatically detects the node's external IP address for P2P advertisement.
//! Uses ipify.org as primary source with STUN fallback.
//! Results are cached for 30 minutes to minimize external requests.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Cached external IP with timestamp
#[derive(Clone)]
pub struct CachedExternalIp {
    pub ip: String,
    pub detected_at: Instant,
}

/// External IP detector with 30-minute cache
pub struct ExternalIpDetector {
    cache: Arc<Mutex<Option<CachedExternalIp>>>,
    cache_duration: Duration,
}

impl ExternalIpDetector {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(None)),
            cache_duration: Duration::from_secs(30 * 60), // 30 minutes
        }
    }

    /// Get external IP, using cache if valid
    pub async fn get_external_ip(&self) -> Option<String> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.as_ref() {
                if cached.detected_at.elapsed() < self.cache_duration {
                    info!(
                        target: "p2p::external_ip",
                        "Using cached external IP: {} (age: {}s)",
                        cached.ip,
                        cached.detected_at.elapsed().as_secs()
                    );
                    return Some(cached.ip.clone());
                }
            }
        }

        // Cache expired or missing, detect new IP
        info!(target: "p2p::external_ip", "Detecting external IP address...");

        // Try ipify.org first
        if let Some(ip) = self.detect_via_ipify().await {
            self.update_cache(ip.clone());
            return Some(ip);
        }

        // Fallback to STUN
        if let Some(ip) = self.detect_via_stun().await {
            self.update_cache(ip.clone());
            return Some(ip);
        }

        warn!(target: "p2p::external_ip", "Failed to detect external IP");
        None
    }

    /// Detect IP via ipify.org API
    async fn detect_via_ipify(&self) -> Option<String> {
        match tokio::time::timeout(
            Duration::from_secs(5),
            reqwest::get("https://api.ipify.org?format=text"),
        )
        .await
        {
            Ok(Ok(response)) => {
                match response.text().await {
                    Ok(ip) => {
                        let ip = ip.trim().to_string();
                        if Self::is_valid_ipv4(&ip) {
                            // Fix: Validate external IP is not private
                            if let Some(validated_ip) =
                                crate::p2p::ip_filter::validate_external_ip(&ip)
                            {
                                info!(
                                    target: "p2p::external_ip",
                                    "✓ Detected external IP via ipify: {}", validated_ip
                                );
                                return Some(validated_ip);
                            } else {
                                warn!(
                                    target: "p2p::external_ip",
                                    "ipify returned invalid/private IP: {}", ip
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!(target: "p2p::external_ip", "ipify response error: {}", e);
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(target: "p2p::external_ip", "ipify request failed: {}", e);
            }
            Err(_) => {
                warn!(target: "p2p::external_ip", "ipify request timeout");
            }
        }
        None
    }

    /// Detect IP via STUN server
    async fn detect_via_stun(&self) -> Option<String> {
        info!(target: "p2p::external_ip", "Attempting STUN detection...");

        // Use public STUN server
        let stun_server = "stun.l.google.com:19302";

        match tokio::time::timeout(Duration::from_secs(5), Self::stun_probe(stun_server)).await {
            Ok(Some(ip)) => {
                // Fix: Validate STUN result is not private
                if let Some(validated_ip) = crate::p2p::ip_filter::validate_external_ip(&ip) {
                    info!(
                        target: "p2p::external_ip",
                        "✓ Detected external IP via STUN: {}", validated_ip
                    );
                    Some(validated_ip)
                } else {
                    warn!(
                        target: "p2p::external_ip",
                        "STUN returned invalid/private IP: {}", ip
                    );
                    None
                }
            }
            Ok(None) => {
                warn!(target: "p2p::external_ip", "STUN probe returned no IP");
                None
            }
            Err(_) => {
                warn!(target: "p2p::external_ip", "STUN probe timeout");
                None
            }
        }
    }

    /// Simple STUN probe (basic implementation)
    async fn stun_probe(server: &str) -> Option<String> {
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
        socket.connect(server).await.ok()?;

        // STUN Binding Request (RFC 5389)
        // Simple implementation - just header
        let mut request = vec![0u8; 20];
        request[0] = 0x00; // Message Type: Binding Request
        request[1] = 0x01;
        request[2] = 0x00; // Message Length
        request[3] = 0x00;
        // Magic Cookie
        request[4] = 0x21;
        request[5] = 0x12;
        request[6] = 0xa4;
        request[7] = 0x42;
        // Transaction ID (random)
        for i in 8..20 {
            request[i] = rand::random();
        }

        socket.send(&request).await.ok()?;

        let mut response = vec![0u8; 512];
        match tokio::time::timeout(Duration::from_secs(2), socket.recv(&mut response)).await {
            Ok(Ok(len)) => {
                // Parse STUN response for XOR-MAPPED-ADDRESS
                Self::parse_stun_response(&response[..len])
            }
            _ => None,
        }
    }

    /// Parse STUN response for mapped address
    fn parse_stun_response(data: &[u8]) -> Option<String> {
        if data.len() < 20 {
            return None;
        }

        // Very basic parsing - look for XOR-MAPPED-ADDRESS (0x0020)
        let mut pos = 20; // Skip header
        while pos + 4 < data.len() {
            let attr_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
            let attr_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;

            if attr_type == 0x0020 && pos + 4 + attr_len <= data.len() {
                // XOR-MAPPED-ADDRESS found
                if attr_len >= 8 {
                    let family = data[pos + 5];
                    if family == 0x01 {
                        // IPv4
                        let _port_xor = u16::from_be_bytes([data[pos + 6], data[pos + 7]]);
                        let ip_xor = u32::from_be_bytes([
                            data[pos + 8],
                            data[pos + 9],
                            data[pos + 10],
                            data[pos + 11],
                        ]);

                        // XOR with magic cookie (0x2112A442)
                        let ip = ip_xor ^ 0x2112A442;

                        let ip_str = format!(
                            "{}.{}.{}.{}",
                            (ip >> 24) & 0xFF,
                            (ip >> 16) & 0xFF,
                            (ip >> 8) & 0xFF,
                            ip & 0xFF
                        );

                        return Some(ip_str);
                    }
                }
            }

            pos += 4 + attr_len;
            // Align to 4-byte boundary
            if !attr_len.is_multiple_of(4) {
                pos += 4 - (attr_len % 4);
            }
        }

        None
    }

    /// Validate IPv4 address format
    fn is_valid_ipv4(ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }

        for part in parts {
            if let Ok(_num) = part.parse::<u8>() {
                // Valid byte
                continue;
            } else {
                return false;
            }
        }

        true
    }

    /// Update cache with new IP
    fn update_cache(&self, ip: String) {
        let mut cache = self.cache.lock().unwrap();
        *cache = Some(CachedExternalIp {
            ip,
            detected_at: Instant::now(),
        });
    }

    /// Force cache refresh
    pub async fn refresh(&self) -> Option<String> {
        {
            let mut cache = self.cache.lock().unwrap();
            *cache = None;
        }
        self.get_external_ip().await
    }
}

impl Default for ExternalIpDetector {
    fn default() -> Self {
        Self::new()
    }
}
