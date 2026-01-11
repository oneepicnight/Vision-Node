// Peer Connection Diagnostics Report
//
// Tracks why peers are/aren't connecting during each maintainer cycle
// and outputs JSON files for debugging/monitoring.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use chrono::Utc;

/// Reasons why a peer might not be connected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PeerConnectReason {
    AlreadyConnected,
    AlreadyConnecting,
    CooldownActive,
    MaxPeersReached,
    PeerBanned,
    PeerUnhealthy,
    InvalidAddress,
    FilteredByPolicy,
    DialRefused,
    DialTimeout,
    DialError,
    HandshakeTimeout,
    HandshakeFailed_ChainId,
    HandshakeFailed_Version,
    HandshakeFailed_IncompatibleChain,
    HandshakeFailed_Other,
    NoRouteToHost,
}

impl PeerConnectReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlreadyConnected => "AlreadyConnected",
            Self::AlreadyConnecting => "AlreadyConnecting",
            Self::CooldownActive => "CooldownActive",
            Self::MaxPeersReached => "MaxPeersReached",
            Self::PeerBanned => "PeerBanned",
            Self::PeerUnhealthy => "PeerUnhealthy",
            Self::InvalidAddress => "InvalidAddress",
            Self::FilteredByPolicy => "FilteredByPolicy",
            Self::DialRefused => "DialRefused",
            Self::DialTimeout => "DialTimeout",
            Self::DialError => "DialError",
            Self::HandshakeTimeout => "HandshakeTimeout",
            Self::HandshakeFailed_ChainId => "HandshakeFailed_ChainId",
            Self::HandshakeFailed_Version => "HandshakeFailed_Version",
            Self::HandshakeFailed_IncompatibleChain => "HandshakeFailed_IncompatibleChain",
            Self::HandshakeFailed_Other => "HandshakeFailed_Other",
            Self::NoRouteToHost => "NoRouteToHost",
        }
    }
}

/// Bucket for tracking a specific reason
#[derive(Debug, Serialize, Deserialize)]
pub struct ReasonBucket {
    pub reason: String,
    pub count: usize,
    pub samples: Vec<String>,
}

/// Cycle statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct CycleStats {
    pub attempted: usize,
    pub connected: usize,
    pub final_connected: usize,
    pub target: usize,
}

/// Full peer connection report
#[derive(Debug)]
pub struct PeerConnectReport {
    scope: String,
    cycle: CycleStats,
    reasons: HashMap<String, ReasonBucket>,
    max_samples_per_reason: usize,
    redact_ips: bool,
}

impl PeerConnectReport {
    pub fn new(scope: String, target_peers: usize) -> Self {
        Self {
            scope,
            cycle: CycleStats {
                attempted: 0,
                connected: 0,
                final_connected: 0,
                target: target_peers,
            },
            reasons: HashMap::new(),
            max_samples_per_reason: 5,
            redact_ips: std::env::var("VISION_DEBUG_PUBLIC_PEERS").is_err(),
        }
    }

    /// Add a reason for why a peer wasn't connected
    pub fn add_reason(&mut self, reason: PeerConnectReason, addr: &SocketAddr) {
        let reason_str = reason.as_str().to_string();
        let addr_str = if self.redact_ips {
            Self::redact_ip(addr)
        } else {
            addr.to_string()
        };

        let bucket = self.reasons.entry(reason_str.clone()).or_insert(ReasonBucket {
            reason: reason_str,
            count: 0,
            samples: Vec::new(),
        });

        bucket.count += 1;
        if bucket.samples.len() < self.max_samples_per_reason {
            bucket.samples.push(addr_str);
        }
    }

    /// Increment attempt counter
    pub fn record_attempt(&mut self) {
        self.cycle.attempted += 1;
    }

    /// Increment connected counter
    pub fn record_connected(&mut self) {
        self.cycle.connected += 1;
    }

    /// Set final connected count
    pub fn set_final_connected(&mut self, count: usize) {
        self.cycle.final_connected = count;
    }

    /// Redact IP address for privacy (show only partial)
    fn redact_ip(addr: &SocketAddr) -> String {
        let ip = addr.ip();
        let port = addr.port();
        
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                format!("{}.{}.***:***:{}", octets[0], octets[1], port)
            }
            std::net::IpAddr::V6(_) => {
                format!("[redacted_ipv6]:{}", port)
            }
        }
    }

    /// Get top N blockers (most common reasons)
    fn top_blockers(&self, n: usize) -> Vec<String> {
        let mut reasons: Vec<_> = self.reasons.values().collect();
        reasons.sort_by(|a, b| b.count.cmp(&a.count));
        reasons.into_iter()
            .take(n)
            .map(|r| r.reason.clone())
            .collect()
    }

    /// Write stats file (counts only, safe to share)
    pub fn write_stats_file(&self, base_path: &Path) -> Result<(), std::io::Error> {
        let stats = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "scope": self.scope,
            "known_peers": self.total_peers(),
            "connected": self.cycle.final_connected,
            "connecting": 0, // TODO: track from P2P_MANAGER if needed
            "cooldown": self.reason_count("CooldownActive"),
            "banned": self.reason_count("PeerBanned"),
            "unhealthy": self.reason_count("PeerUnhealthy"),
            "attempted_last_cycle": self.cycle.attempted,
            "connected_last_cycle": self.cycle.connected,
        });

        let path = base_path.join("peer_store_stats.json");
        Self::write_atomic_json(&path, &stats)
    }

    /// Write reasons file (detailed with samples)
    pub fn write_reasons_file(&self, base_path: &Path) -> Result<(), std::io::Error> {
        let mut reasons: Vec<_> = self.reasons.values().collect();
        reasons.sort_by(|a, b| b.count.cmp(&a.count));

        let top_blockers = self.top_blockers(3);
        let primary_blocker = top_blockers.get(0).cloned();
        let secondary_blocker = top_blockers.get(1).cloned();
        let third_blocker = top_blockers.get(2).cloned();

        let report = serde_json::json!({
            "ts": Utc::now().to_rfc3339(),
            "scope": self.scope,
            "cycle": {
                "attempted": self.cycle.attempted,
                "connected": self.cycle.connected,
                "final_connected": self.cycle.final_connected,
                "target": self.cycle.target,
            },
            "top_blockers": {
                "primary": primary_blocker,
                "secondary": secondary_blocker,
                "third": third_blocker,
            },
            "reasons": reasons,
        });

        let path = base_path.join("peer_connect_reasons.json");
        Self::write_atomic_json(&path, &report)
    }

    /// Atomic write: write to .tmp, then rename
    fn write_atomic_json(path: &Path, data: &serde_json::Value) -> Result<(), std::io::Error> {
        use std::fs;
        use std::io::Write;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let tmp_path = path.with_extension("json.tmp");
        let json_str = serde_json::to_string_pretty(data)?;
        
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(json_str.as_bytes())?;
        file.sync_all()?;

        fs::rename(tmp_path, path)?;
        Ok(())
    }

    /// Get total known peers (sum of all reasons + connected)
    fn total_peers(&self) -> usize {
        let reasons_total: usize = self.reasons.values().map(|b| b.count).sum();
        reasons_total + self.cycle.final_connected
    }

    /// Get count for a specific reason
    fn reason_count(&self, reason: &str) -> usize {
        self.reasons.get(reason).map(|b| b.count).unwrap_or(0)
    }

    /// Write both files
    pub fn write_all(&self, base_path: &Path) -> Result<(), std::io::Error> {
        self.write_stats_file(base_path)?;
        self.write_reasons_file(base_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_ip() {
        let addr: SocketAddr = "192.168.1.100:7072".parse().unwrap();
        let redacted = PeerConnectReport::redact_ip(&addr);
        assert_eq!(redacted, "192.168.***:***:7072");
    }

    #[test]
    fn test_add_reason() {
        let mut report = PeerConnectReport::new("test".into(), 8);
        let addr: SocketAddr = "1.2.3.4:7072".parse().unwrap();
        
        report.add_reason(PeerConnectReason::CooldownActive, &addr);
        report.add_reason(PeerConnectReason::CooldownActive, &addr);
        
        assert_eq!(report.reason_count("CooldownActive"), 2);
    }

    #[test]
    fn test_top_blockers() {
        let mut report = PeerConnectReport::new("test".into(), 8);
        let addr: SocketAddr = "1.2.3.4:7072".parse().unwrap();
        
        for _ in 0..10 {
            report.add_reason(PeerConnectReason::CooldownActive, &addr);
        }
        for _ in 0..5 {
            report.add_reason(PeerConnectReason::DialTimeout, &addr);
        }
        for _ in 0..2 {
            report.add_reason(PeerConnectReason::PeerBanned, &addr);
        }
        
        let top = report.top_blockers(3);
        assert_eq!(top[0], "CooldownActive");
        assert_eq!(top[1], "DialTimeout");
        assert_eq!(top[2], "PeerBanned");
    }
}
