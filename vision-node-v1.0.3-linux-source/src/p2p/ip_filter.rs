//! IP Address Validation and Filtering
//!
//! Guardrails to prevent private/invalid IPs from entering the peer system.
//! Fixes three critical issues:
//! 1. Never dial private IPs (unless LAN mode enabled)
//! 2. Never save private IPs to PeerBook or gossip
//! 3. Never allow self-connections

use std::net::{IpAddr, Ipv4Addr};
use tracing::warn;

/// Check if IP address is private/non-routable
pub fn is_private_ip(ip: &str) -> bool {
    // Try to parse as IP address
    let addr: IpAddr = match ip.parse() {
        Ok(a) => a,
        Err(_) => return false, // Not a valid IP
    };

    match addr {
        IpAddr::V4(ipv4) => is_private_ipv4(&ipv4),
        IpAddr::V6(_) => {
            // For now, consider all IPv6 as potentially valid
            // TODO: Add proper IPv6 private range checking
            false
        }
    }
}

/// Check if IPv4 address is private/non-routable
pub fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    // RFC 1918 private ranges
    if ip.octets()[0] == 10 {
        return true; // 10.0.0.0/8
    }
    if ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31) {
        return true; // 172.16.0.0/12
    }
    if ip.octets()[0] == 192 && ip.octets()[1] == 168 {
        return true; // 192.168.0.0/16
    }

    // Loopback
    if ip.octets()[0] == 127 {
        return true; // 127.0.0.0/8
    }

    // Link-local
    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
        return true; // 169.254.0.0/16
    }

    // Multicast
    if ip.octets()[0] >= 224 && ip.octets()[0] <= 239 {
        return true; // 224.0.0.0/4
    }

    // Reserved/broadcast
    if ip.octets()[0] == 0 {
        return true; // 0.0.0.0/8
    }
    if ip.octets()[0] == 255 {
        return true; // 255.0.0.0/8
    }

    // Not private
    false
}

/// Check if we should allow private IPs (for LAN testing)
pub fn allow_private_peers() -> bool {
    std::env::var("VISION_ALLOW_PRIVATE_PEERS")
        .ok()
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "true" || v == "1" || v == "yes" || v == "y"
        })
        .unwrap_or(false)
}

/// Check if we're in local test mode (strict localhost-only)
/// When enabled, ONLY loopback and RFC1918 private IPs are allowed
pub fn local_test_mode() -> bool {
    std::env::var("VISION_LOCAL_TEST")
        .ok()
        .map(|v| v.trim() == "1")
        .unwrap_or(false)
}

/// Check if an IP address is allowed in local test mode
/// Returns true for loopback, RFC1918 private ranges, and link-local
/// Returns false for public IPs (prevents WAN peer pollution in tests)
pub fn is_local_allowed(addr: &std::net::SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(ipv4) => is_local_allowed_ipv4(&ipv4),
        IpAddr::V6(ipv6) => {
            // Allow IPv6 loopback (::1)
            ipv6.is_loopback()
        }
    }
}

/// Check if IPv4 address is allowed in local test mode
/// Allows: loopback (127.x), RFC1918 (10.x, 172.16-31.x, 192.168.x), link-local (169.254.x)
pub fn is_local_allowed_ipv4(ip: &Ipv4Addr) -> bool {
    // Loopback: 127.0.0.0/8
    if ip.octets()[0] == 127 {
        return true;
    }

    // RFC1918 private ranges
    if ip.octets()[0] == 10 {
        return true; // 10.0.0.0/8
    }
    if ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31) {
        return true; // 172.16.0.0/12
    }
    if ip.octets()[0] == 192 && ip.octets()[1] == 168 {
        return true; // 192.168.0.0/16
    }

    // Link-local: 169.254.0.0/16 (for local network discovery)
    if ip.octets()[0] == 169 && ip.octets()[1] == 254 {
        return true;
    }

    // Everything else is public IP - not allowed in local test mode
    false
}

/// Extract IP from address string (removes port)
pub fn extract_ip_from_addr(addr: &str) -> Option<String> {
    // Handle formats: "IP:PORT", "IP", "[IPv6]:PORT"
    if let Some((ip, _port)) = addr.rsplit_once(':') {
        // Remove brackets for IPv6
        let ip = ip.trim_start_matches('[').trim_end_matches(']');
        Some(ip.to_string())
    } else {
        Some(addr.to_string())
    }
}

/// Validate IP for dialing (Fix 1 & 3)
/// Returns None if IP should not be dialed, Some(reason) if rejected
pub fn validate_ip_for_dial(addr: &str, local_ips: &[String]) -> Option<String> {
    // Extract IP (and port if present) from address
    let (ip, port_opt) = match addr.parse::<std::net::SocketAddr>() {
        Ok(sock) => (sock.ip().to_string(), Some(sock.port())),
        Err(_) => {
            let ip = match extract_ip_from_addr(addr) {
                Some(ip) => ip,
                None => return Some("Invalid address format".to_string()),
            };
            (ip, None)
        }
    };

    // Fix 3: Self-connect kill switch
    // Allow localhost/LAN multi-node testing on the same machine by only rejecting
    // *true* self-dials (same IP + same local P2P listen port).
    if local_ips.iter().any(|local| local == &ip) {
        // Best-effort: detect our local P2P listen port.
        let local_p2p_port = std::env::var("VISION_P2P_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .or_else(|| {
                std::env::var("VISION_PUBLIC_PORT")
                    .ok()
                    .and_then(|v| v.parse::<u16>().ok())
            });

        match (port_opt, local_p2p_port) {
            (Some(port), Some(local_port)) if port == local_port => {
                return Some(format!(
                    "Self-connection attempt: {}:{} matches local listener",
                    ip, port
                ));
            }
            // If we don't know ports, stay conservative and reject.
            (None, _) | (Some(_), None) => {
                return Some(format!(
                    "Self-connection attempt: {} matches local interface",
                    ip
                ));
            }
            // Same IP but different port: allow (multi-node on localhost)
            (Some(_), Some(_)) => {}
        }
    }

    // Check for common gateway IP (poison value)
    if ip == "192.168.1.1" || ip == "192.168.0.1" || ip == "10.0.0.1" {
        return Some(format!("Rejecting common gateway IP: {}", ip));
    }

    // Fix 1: Never dial private IPs (unless LAN mode)
    if is_private_ip(&ip) {
        if allow_private_peers() {
            warn!(
                target: "p2p::ip_filter",
                "Allowing private IP {} (VISION_ALLOW_PRIVATE_PEERS=true)", ip
            );
            return None; // Allow in LAN mode
        } else {
            return Some(format!("Skipping non-public peer address: {}", ip));
        }
    }

    None // Valid for dialing
}

/// Validate IP for storage (Fix 2 + Local Test Mode)
/// Returns true if IP should be saved to PeerBook/gossip
pub fn validate_ip_for_storage(addr: &str) -> bool {
    let ip = match extract_ip_from_addr(addr) {
        Some(ip) => ip,
        None => return false, // Invalid format
    };

    // Parse to SocketAddr for local test mode check
    let sock_addr: std::net::SocketAddr = match format!("{}:0", ip).parse() {
        Ok(a) => a,
        Err(_) => return false,
    };

    // Local test mode: ONLY allow loopback and RFC1918
    if local_test_mode() {
        if !is_local_allowed(&sock_addr) {
            warn!(
                target: "p2p::ip_filter",
                "Local test mode: rejecting non-local peer: {}", ip
            );
            return false;
        }
        return true; // Local IP in local test mode
    }

    // Fix 2: Never save private IPs (even in LAN mode)
    // Private IPs are useless for remote peers
    if is_private_ip(&ip) {
        if allow_private_peers() {
            // In LAN mode, allow storage for testing
            return true;
        }
        warn!(
            target: "p2p::ip_filter",
            "Filtering out private IP from storage: {}", ip
        );
        return false;
    }

    true
}

/// Validate external IP detection result
/// Returns None if IP is invalid for external advertisement
pub fn validate_external_ip(ip: &str) -> Option<String> {
    // Must be a valid IP
    let addr: IpAddr = match ip.parse() {
        Ok(a) => a,
        Err(_) => {
            warn!(
                target: "p2p::ip_filter",
                "Invalid external IP format: {}", ip
            );
            return None;
        }
    };

    // Must not be private
    if is_private_ip(ip) {
        warn!(
            target: "p2p::ip_filter",
            "External IP detection returned private IP: {} (rejecting)", ip
        );
        return None;
    }

    // Valid external IP
    Some(addr.to_string())
}

/// Get local interface IPs (for self-connect detection)
pub fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();

    // Always include localhost
    ips.push("127.0.0.1".to_string());
    ips.push("::1".to_string());
    ips.push("0.0.0.0".to_string());

    // Check environment variable for configured external IP
    if let Ok(external) = std::env::var("VISION_EXTERNAL_IP") {
        ips.push(external);
    }

    // Try to get local interface IPs using if-addrs crate (if available)
    #[cfg(feature = "full")]
    {
        // Use a simple approach - bind a UDP socket and get local addr
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            // Connect to a public IP (doesn't actually send data)
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(local_addr) = socket.local_addr() {
                    ips.push(local_addr.ip().to_string());
                }
            }
        }
    }

    ips
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_ip_detection() {
        // Private ranges
        assert!(is_private_ip("192.168.1.1"));
        assert!(is_private_ip("192.168.0.100"));
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("10.255.255.255"));
        assert!(is_private_ip("172.16.0.1"));
        assert!(is_private_ip("172.31.255.255"));
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("169.254.1.1"));
        assert!(is_private_ip("0.0.0.0"));

        // Public IPs
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("1.1.1.1"));
        assert!(!is_private_ip("172.15.0.1")); // Just outside private range
        assert!(!is_private_ip("172.32.0.1")); // Just outside private range
        assert!(!is_private_ip("192.167.1.1"));
        assert!(!is_private_ip("192.169.1.1"));
    }

    #[test]
    fn test_extract_ip() {
        assert_eq!(
            extract_ip_from_addr("192.168.1.1:7072"),
            Some("192.168.1.1".to_string())
        );
        assert_eq!(
            extract_ip_from_addr("8.8.8.8:443"),
            Some("8.8.8.8".to_string())
        );
        assert_eq!(extract_ip_from_addr("8.8.8.8"), Some("8.8.8.8".to_string()));
    }
}
