#![allow(dead_code)]
//! P2P Block Synchronization Module
//!
//! Implements headers-first sync with pipelined block fetching
//!
//! Phase 2 Extensions:
//! - Compact blocks (BIP-152 style)
//! - INV/GETDATA transaction relay
//! - Mempool synchronization
//! - Reorg handling

use std::net::{IpAddr, SocketAddr};

/// Maximum allowed handshake packet size (10KB) to prevent garbage/scanner packets
pub const MAX_HANDSHAKE_LEN: u32 = 10_000;

/// P2P Connection Limits (P2P Robustness #4)
pub const MAX_PEERS_TOTAL: usize = 64;
pub const MAX_ANCHOR_PEERS: usize = 48;
pub const RESERVED_NEW_PEERS: usize = 16;

/// P2P Timeouts (P2P Robustness #6)
pub const DIAL_TIMEOUT_SECS: u64 = 5;
pub const HANDSHAKE_TIMEOUT_SECS: u64 = 5;
pub const IDLE_CONNECTION_TIMEOUT_SECS: u64 = 120;

/// IPv4-only validation: rejects IPv6, loopback (127.0.0.1), and unspecified (0.0.0.0)
/// This ensures only real, publicly routable IPv4 addresses are used for P2P connections
pub fn is_valid_ipv4_endpoint(addr: &SocketAddr) -> bool {
    match addr.ip() {
        IpAddr::V4(v4) => {
            // Filter out 0.0.0.0 and 127.0.0.1
            !v4.is_unspecified() && !v4.is_loopback()
        }
        IpAddr::V6(_) => false,
    }
}

pub mod orphans;
pub mod p2p_config;
pub mod protocol;
pub mod routes;
pub mod sync;

// Phase 2 modules
pub mod compact;
pub mod mempool_sync;
pub mod reorg;
pub mod tx_relay;

// Phase 3 modules
pub mod beacon_bootstrap;

// Phase 3: TCP P2P with persistent connections
pub mod connection;

// Bootstrap module for dynamic peer discovery
pub mod bootstrap;

// Phase 6: Constellation Memory Layer (immortality)
pub mod peer_memory;
pub mod peer_recovery;

// Phase 7: Vision Peer Book - Identity-based addressing
pub mod peer_store;

// Phase 8: P2P API and intelligent peer management
pub mod api;
pub mod health_monitor;
pub mod peer_manager;
pub mod routing_helpers;

// Genesis seed peers for offline-first bootstrap
pub mod seed_peers;

// Connection maintainer - ensures minimum peer connections (PATCH 2)
pub mod connection_maintainer;

// UPnP port forwarding for automatic public reachability
pub mod upnp;

// Phase 9: Advanced P2P Intelligence (Enterprise-Grade - ALL WIRED UP)
pub mod anchor_election; // ✅ Anchor node election and promotion system
pub mod backoff; // ✅ Exponential backoff for peer reconnection
pub mod network_healing; // ✅ Self-healing network topology
pub mod network_readiness; // ✅ Network readiness and health checks
pub mod node_id; // ✅ Node identity and reputation management
pub mod peer_gossip; // ✅ Peer gossip protocol for discovery
pub mod retry_worker; // ✅ Retry logic for failed operations
pub mod swarm_bootstrap;
pub mod swarm_intelligence; // ✅ Intelligent peer selection and routing // ✅ Swarm-based peer discovery

// Phase 3.5: Latency-Based Routing Intelligence (Adaptive Swarm)
pub mod latency; // ✅ Latency monitoring and RTT measurement
pub mod routing; // ✅ Auto-clustering and intelligent peer selection

// Phase 4: Adversarial Resilience & Reputation System
pub mod reputation; // ✅ Trust scoring and misbehavior tracking

// Phase 10: External IP Detection & Reachability
pub mod external_ip; // ✅ External IP detection with caching
pub mod ip_filter; // ✅ IP validation and private IP filtering (Guardrails)

// Phase 11: Anchor HTTP Backbone - Real-time 7070 connectivity proof
pub mod anchor_http; // ✅ Background HTTP probe for anchor nodes
pub mod reachability; // ✅ NAT detection and reverse connection testing

// Dial failure tracking for debugging P2P issues
pub mod dial_tracker; // ✅ Track and report connection failures

// Test utilities for isolated unit testing
#[cfg(test)]
pub mod test_utils;

/// Determine the PeerBook scope for this network/test run
///
/// Scope prevents peer data from mixing across networks or test runs.
/// Priority order:
/// 1. VISION_PEERBOOK_SCOPE env var (if set and non-empty)
/// 2. First 8 characters of drop_prefix (network-specific)
/// 3. "default" fallback
pub fn peerbook_scope() -> String {
    // Check environment variable first
    if let Ok(v) = std::env::var("VISION_PEERBOOK_SCOPE") {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }

    // Fall back to drop_prefix for network-specific separation
    let prefix = crate::vision_constants::VISION_BOOTSTRAP_PREFIX;
    if !prefix.is_empty() && prefix.len() >= 8 {
        format!("pb_{}", &prefix[..8])
    } else {
        "pb_default".to_string()
    }
}

// Public re-exports used by other modules (keep minimal to avoid unused_imports)
pub use connection::P2PConnectionManager;
pub use node_id::{ensure_node_wallet_consistency, load_or_create_node_id};
pub use peer_manager::{Peer, PeerBucket, PeerManager, PeerState};
