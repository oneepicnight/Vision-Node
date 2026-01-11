//! UPnP Port Forwarding Module
//!
//! Automatically requests port forwarding from UPnP-enabled routers
//! to make the P2P port accessible from the internet.

use igd::{search_gateway, PortMappingProtocol, SearchOptions};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{error, info, warn};

/// Global UPnP success status tracker
/// Set to true when port forwarding succeeds, false on failure or no UPnP
pub static UPNP_SUCCESS: AtomicBool = AtomicBool::new(false);

/// Port lease duration (24 hours)
/// Note: We renew every 12 hours to stay ahead of expiration
const PORT_LEASE_DURATION_SECS: u32 = 86400;

/// Gateway search timeout
const GATEWAY_SEARCH_TIMEOUT_SECS: u64 = 5;

/// UPnP port mapping result
#[derive(Debug, Clone)]
pub struct UpnpMapping {
    pub external_ip: Ipv4Addr,
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: String,
}

/// Attempt to set up UPnP port forwarding for the P2P port
///
/// # Arguments
/// * `port` - The local port to forward (e.g., 7072)
/// * `description` - Service description for the router (e.g., "Vision Node P2P")
///
/// # Returns
/// * `Some(UpnpMapping)` if successful
/// * `None` if UPnP is unavailable or failed
pub async fn setup_port_forwarding(port: u16, description: &str) -> Option<UpnpMapping> {
    info!(
        "[UPnP] 🔍 Searching for UPnP-enabled gateway (timeout: {}s)...",
        GATEWAY_SEARCH_TIMEOUT_SECS
    );

    // Run UPnP search in blocking thread pool
    let desc = description.to_string();
    let result = tokio::task::spawn_blocking(move || {
        // Search for gateway with configurable timeout
        let search_options = SearchOptions {
            timeout: Some(Duration::from_secs(GATEWAY_SEARCH_TIMEOUT_SECS)),
            ..Default::default()
        };

        match search_gateway(search_options) {
            Ok(gateway) => {
                info!("[UPnP] ✅ Found UPnP gateway: {}", gateway.addr);

                // Get external IP
                let external_ip = match gateway.get_external_ip() {
                    Ok(ip) => {
                        info!("[UPnP] 🌍 External IP detected: {}", ip);
                        ip
                    }
                    Err(e) => {
                        error!("[UPnP] ❌ Failed to get external IP: {}", e);
                        warn!("[UPnP] 💡 Check router UPnP settings");
                        return None;
                    }
                };

                // Use 0.0.0.0 for local address (any local IP)
                let local_addr = Ipv4Addr::new(0, 0, 0, 0);
                let local_socket = SocketAddrV4::new(local_addr, port);

                // Request port forwarding
                info!("[UPnP] 📡 Requesting port forward: external:{} → internal:{}", port, port);
                info!("[UPnP] ⏱️  Lease duration: {} seconds (~{} hours)", PORT_LEASE_DURATION_SECS, PORT_LEASE_DURATION_SECS / 3600);

                match gateway.add_port(
                    PortMappingProtocol::TCP,
                    port,
                    local_socket,
                    PORT_LEASE_DURATION_SECS,
                    &desc,
                ) {
                    Ok(()) => {
                        info!("[UPnP] ✅ Port forwarding established successfully!");
                        info!("[UPnP] 🎯 Mapping: {}:{} (external) → {}:{} (internal)",
                            external_ip, port, local_addr, port);
                        info!("[UPnP] 🌐 Public P2P endpoint: {}:{}", external_ip, port);
                        info!("[UPnP] 🔄 Auto-renewal: Every 12 hours");

                        // Mark UPnP as successful
                        UPNP_SUCCESS.store(true, Ordering::SeqCst);

                        Some(UpnpMapping {
                            external_ip,
                            external_port: port,
                            internal_port: port,
                            protocol: "TCP".to_string(),
                        })
                    }
                    Err(e) => {
                        error!("[UPnP] ❌ Failed to add port mapping: {}", e);
                        warn!("[UPnP] 💡 Manual port forwarding required:");
                        warn!("[UPnP]    Router settings → Port Forwarding → Add rule:");
                        warn!("[UPnP]    External Port: {} → Internal IP: <this machine> → Internal Port: {}", port, port);

                        // Mark UPnP as failed
                        UPNP_SUCCESS.store(false, Ordering::SeqCst);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("[UPnP] ⚠️  No UPnP-enabled gateway found: {}", e);
                info!("[UPnP] ℹ️  This is normal if:");
                info!("[UPnP]    • Router doesn't support UPnP/IGD");
                info!("[UPnP]    • UPnP is disabled in router settings");
                info!("[UPnP]    • Running behind multiple NAT layers");
                warn!("[UPnP] 💡 Manual port forwarding required for internet P2P connectivity");

                // Mark UPnP as unavailable
                UPNP_SUCCESS.store(false, Ordering::SeqCst);
                None
            }
        }
    })
    .await;

    match result {
        Ok(mapping) => mapping,
        Err(e) => {
            error!("[UPnP] Task failed: {}", e);
            None
        }
    }
}

/// Remove UPnP port forwarding (called on shutdown)
pub async fn remove_port_forwarding(port: u16) {
    info!("[UPnP] 🧹 Removing port forwarding for port {}...", port);

    let result = tokio::task::spawn_blocking(move || {
        let search_options = SearchOptions {
            timeout: Some(Duration::from_secs(2)),
            ..Default::default()
        };

        match search_gateway(search_options) {
            Ok(gateway) => match gateway.remove_port(PortMappingProtocol::TCP, port) {
                Ok(()) => {
                    info!("[UPnP] ✅ Port {} forwarding removed cleanly", port);
                }
                Err(e) => {
                    warn!("[UPnP] ⚠️  Failed to remove port mapping: {}", e);
                    info!("[UPnP] ℹ️  Mapping may expire automatically (24h lease)");
                }
            },
            Err(e) => {
                warn!("[UPnP] ⚠️  Could not find gateway to remove mapping: {}", e);
                info!("[UPnP] ℹ️  Port mapping will expire automatically");
            }
        }
    })
    .await;

    if let Err(e) = result {
        error!("[UPnP] Task failed during cleanup: {}", e);
    }
}

/// Renew UPnP port mapping (called periodically, every 12 hours)
/// Returns true if renewal succeeded, false otherwise
pub async fn renew_port_forwarding(port: u16, description: &str) -> bool {
    info!(
        "[UPnP] 🔄 Renewing port forwarding for port {} (lease refresh)...",
        port
    );

    match setup_port_forwarding(port, description).await {
        Some(_) => {
            info!("[UPnP] ✅ Port forwarding renewed - lease extended for 24h");
            true
        }
        None => {
            warn!("[UPnP] ⚠️  Port forwarding renewal failed");
            warn!("[UPnP] 💡 Will retry at next interval (12h)");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Run manually with: cargo test --package vision-node --lib p2p::upnp::tests --features "" -- --ignored
    async fn test_upnp_setup() {
        // This test requires a UPnP-enabled router on the network
        let result = setup_port_forwarding(7072, "Vision Node P2P Test").await;

        if let Some(mapping) = result {
            println!("✅ UPnP successful!");
            println!(
                "   External: {}:{}",
                mapping.external_ip, mapping.external_port
            );
            println!("   Internal: {}", mapping.internal_port);

            // Clean up
            remove_port_forwarding(7072).await;
        } else {
            println!("⚠️  UPnP not available (this is OK for manual port forwarding)");
        }
    }
}
