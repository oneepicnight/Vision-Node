//! Global Singletons Module
//! 
//! Centralized place for all global manager instances.
//! Managers are initialized lazily and support safe late-init patterns.
//! This module ensures:
//! - No fake placeholder structs in main.rs
//! - All globals defined and exported in one place
//! - Safe initialization before wallet/node startup
//! - No crashes if accessed before ready (return sensible defaults)

pub mod p2p_global;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;
use sled::Config;
use serde::{Deserialize, Serialize};
use tracing::{warn, info};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
const DEFAULT_P2P_PORT: u16 = 7072;
use crate::p2p::ip_filter::{get_local_ips, validate_ip_for_storage};

// ============================================================================
// P2P Manager (safe late-init wrapper)
// ============================================================================
pub static P2P_MANAGER: Lazy<p2p_global::GlobalP2PManager> =
    Lazy::new(p2p_global::GlobalP2PManager::new);

// ============================================================================
// PEER_MANAGER (real type from p2p module)
// ============================================================================
pub use crate::p2p::peer_manager::PeerManager;

pub static PEER_MANAGER: Lazy<PeerManager> =
    Lazy::new(PeerManager::new);

// ============================================================================
// CONSTELLATION_MEMORY (backed by sled; boots with temporary DB)
// ============================================================================
pub static CONSTELLATION_MEMORY: Lazy<Mutex<crate::p2p::peer_memory::ConstellationMemory>> =
    Lazy::new(|| {
        let db = Config::new()
            .temporary(true)
            .open()
            .expect("Failed to initialize temporary sled DB for constellation memory");

        let memory = match crate::p2p::peer_memory::ConstellationMemory::new(&db) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    target: "vision_node::globals",
                    "[GLOBALS] ConstellationMemory init failed: {} — using empty memory",
                    e
                );
                // Best-effort fallback: re-attempt with a fresh temporary DB
                let db2 = Config::new().temporary(true).open().expect(
                    "Failed to reinitialize temporary sled DB for constellation memory",
                );
                crate::p2p::peer_memory::ConstellationMemory::new(&db2)
                    .expect("ConstellationMemory fallback init failed")
            }
        };

        Mutex::new(memory)
    });

// Swap the temporary memory with a persistent, DB-backed instance.
// Call this during bootstrap once the real sled::Db is opened.
pub fn swap_constellation_memory(db: sled::Db) -> Result<(), String> {
    // Open meta tree for one-time marker
    let meta = db
        .open_tree("constellation_meta")
        .map_err(|e| format!("Failed to open meta tree: {}", e))?;

    // If marker exists, skip migration and just load from DB
    if meta.get(b"predb_migrated").map_err(|e| format!("Meta get error: {}", e))?.is_some() {
        let new_mem = crate::p2p::peer_memory::ConstellationMemory::new(&db)?;
        {
            let mut guard = CONSTELLATION_MEMORY.lock();
            *guard = new_mem;
        }

        let when_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        crate::p2p::predb_migration_report::set_report(
            crate::p2p::predb_migration_report::PredbMigrationReport {
                ran: false,
                skipped: true,
                captured: 0,
                kept: 0,
                dropped: 0,
                cap: 0,
                when_ms,
                samples: Vec::new(),
                counts_wrong_port: 0,
                counts_invalid_ip: 0,
                counts_self_ip: 0,
                counts_banned: 0,
                counts_blocked_cidr: 0,
                counts_duplicate: 0,
                dropped_by_cap: 0,
            },
        );
        return Ok(());
    }

    // Snapshot existing (temporary) peers before swapping
    let mut guard = CONSTELLATION_MEMORY.lock();
    let snapshot = guard.all_peers();
    let captured = snapshot.len();

    // Determine P2P port
    let p2p_port: u16 = std::env::var("VISION_P2P_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(DEFAULT_P2P_PORT);

    // Local interfaces for self detection
    let local_ips = get_local_ips();

    // Load optional blocked CIDRs from env (IPv4 only)
    let blocked_cidrs = std::env::var("VISION_BLOCKED_CIDRS")
        .ok()
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
        .unwrap_or_default();

    // Helpers
    fn ip_in_blocked(ip_str: &str, cidrs: &[String]) -> bool {
        let ip: Ipv4Addr = match ip_str.parse() {
            Ok(IpAddr::V4(v4)) => v4,
            Ok(IpAddr::V6(_)) => return false, // Skip IPv6 in simple guard
            Err(_) => return false,
        };
        let ip_u32 = u32::from(ip);
        for cidr in cidrs {
            // Expect format: a.b.c.d/nn
            let (net, prefix) = match cidr.split_once('/') {
                Some((n, p)) => (n, p),
                None => continue,
            };
            let net_v4: Ipv4Addr = match net.parse() {
                Ok(IpAddr::V4(v4)) => v4,
                _ => continue,
            };
            let prefix_len: u32 = match prefix.parse::<u32>() {
                Ok(n) if n <= 32 => n,
                _ => continue,
            };
            let mask: u32 = if prefix_len == 0 { 0 } else { u32::MAX << (32 - prefix_len) };
            if (ip_u32 & mask) == (u32::from(net_v4) & mask) {
                return true;
            }
        }
        false
    }
    fn is_banned(db: &sled::Db, peer_id: &str) -> bool {
        let key = format!("banned_{}", peer_id);
        db.get(key.as_bytes()).ok().flatten().is_some()
    }

    // Minimal validation + dedupe set
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut validated: Vec<crate::p2p::peer_memory::ConstellationPeerMemory> = Vec::new();
    // Reporting counters and capped samples
    let mut counts_wrong_port = 0usize;
    let mut counts_invalid_ip = 0usize;
    let mut counts_self_ip = 0usize;
    let mut counts_banned = 0usize;
    let mut counts_blocked_cidr = 0usize;
    let mut counts_duplicate = 0usize;
    let mut samples: Vec<crate::p2p::predb_migration_report::DropSample> = Vec::new();
    const MAX_SAMPLES: usize = 50;
    for mut p in snapshot {
        // Port check
        if p.last_port != p2p_port {
            counts_wrong_port += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::WrongPort,
                });
            }
            continue;
        }
        // IP format + storage policy check
        let addr = format!("{}:{}", p.last_ip, p.last_port);
        if !validate_ip_for_storage(&addr) {
            counts_invalid_ip += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::InvalidIP,
                });
            }
            continue;
        }
        // Self detection (drop peers on local interfaces)
        if local_ips.iter().any(|ip| ip == &p.last_ip) {
            counts_self_ip += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::SelfIP,
                });
            }
            continue;
        }
        // Drop if banned or in blocked CIDRs
        if is_banned(&db, &p.peer_id) {
            counts_banned += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::Banned,
                });
            }
            continue;
        }
        if ip_in_blocked(&p.last_ip, &blocked_cidrs) {
            counts_blocked_cidr += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::BlockedCIDR,
                });
            }
            continue;
        }
        // Deduplicate by peer_id
        if seen_ids.insert(p.peer_id.clone()) {
            validated.push(p);
        } else {
            counts_duplicate += 1;
            if samples.len() < MAX_SAMPLES {
                samples.push(crate::p2p::predb_migration_report::DropSample {
                    peer_id: p.peer_id.clone(),
                    last_ip: p.last_ip.clone(),
                    last_port: p.last_port,
                    reason: crate::p2p::predb_migration_report::DropReason::Duplicate,
                });
            }
        }
    }

    // Prefer anchors, then higher uptime, then more recent last_seen
    validated.sort_by(|a, b| {
        use std::cmp::Ordering;
        match (b.is_anchor, a.is_anchor) { // anchors first
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                // uptime desc
                let sc = b.uptime_score.partial_cmp(&a.uptime_score).unwrap_or(Ordering::Equal);
                if sc != Ordering::Equal {
                    return sc;
                }
                // last_seen desc
                b.last_seen.cmp(&a.last_seen)
            }
        }
    });

    // Cap migration size
    let cap: usize = std::env::var("VISION_PREDB_MIGRATE_CAP")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(2048);
    let mut kept = 0usize;

    let mut new_mem = crate::p2p::peer_memory::ConstellationMemory::new(&db)?;
    let validated_len = validated.len();
    let capped_count = std::cmp::min(validated_len, cap);

    // Idempotent: skip peers already present in DB-loaded memory
    for p in validated.into_iter().take(cap) {
        if new_mem.has_peer(&p.peer_id) {
            continue;
        }
        new_mem.record_peer(p);
        kept += 1;
    }

    // Persist merged peers into the real DB
    new_mem.flush_to_db()?;

    let dropped = captured.saturating_sub(kept);
    let capped = captured > cap;
    info!(
        target: "vision_node::globals",
        "[P2P] Migrated pre-DB peers: captured={} kept={} dropped={} flushed=true{}",
        captured,
        kept,
        dropped,
        if capped { format!(" (cap={})", cap) } else { String::new() }
    );

    *guard = new_mem;

    // Write marker only after successful flush
    let when_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let marker = crate::p2p::predb_migration_report::PredbMigrationMarker {
        when_ms,
        captured,
        kept,
        dropped,
        cap,
    };
    let marker_bytes = serde_json::to_vec(&marker).map_err(|e| format!("Marker serialize error: {}", e))?;
    meta.insert(b"predb_migrated", marker_bytes)
        .map_err(|e| format!("Meta insert error: {}", e))?;
    meta.flush().map_err(|e| format!("Meta flush error: {}", e))?;

    // Store report in-memory for debug endpoint
    crate::p2p::predb_migration_report::set_report(
        crate::p2p::predb_migration_report::PredbMigrationReport {
            ran: true,
            skipped: false,
            captured,
            kept,
            dropped,
            cap,
            when_ms,
            samples,
            counts_wrong_port,
            counts_invalid_ip,
            counts_self_ip,
            counts_banned,
            counts_blocked_cidr,
            counts_duplicate,
            dropped_by_cap: (validated_len as u64).saturating_sub(capped_count as u64),
        },
    );

    Ok(())
}

// ============================================================================
// GUARDIAN_CONSCIOUSNESS (feature-gated string state)
// ============================================================================
#[cfg(not(feature = "staging"))]
pub static GUARDIAN_CONSCIOUSNESS: Lazy<String> =
    Lazy::new(|| "inactive".to_string());

#[cfg(feature = "staging")]
pub static GUARDIAN_CONSCIOUSNESS: Lazy<String> =
    Lazy::new(|| "active".to_string());

// ============================================================================
// Legacy/Placeholder Globals (referenced by old code, safe stubs)
// ============================================================================

/// Stub for EBID manager - referenced by legacy code
#[derive(Default)]
pub struct EBIDManager {
    ebid: String,
    created_at: u64,
    node_tag: Option<String>,
}

impl EBIDManager {
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            ebid: uuid::Uuid::new_v4().to_string(),
            created_at: now,
            node_tag: None,
        }
    }
    pub fn get_ebid(&self) -> &str {
        &self.ebid
    }
    pub fn age_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.created_at)
    }
    pub fn get_full(&self) -> EbidInfo {
        EbidInfo {
            ebid: self.ebid.clone(),
            created_at: self.created_at,
            node_tag: self.node_tag.clone(),
        }
    }
}

#[derive(Clone)]
pub struct EbidInfo {
    pub ebid: String,
    pub created_at: u64,
    pub node_tag: Option<String>,
}

pub static EBID_MANAGER: Lazy<Mutex<EBIDManager>> = Lazy::new(|| Mutex::new(EBIDManager::new()));

/// Peer store DB used by rolling mesh bootstrap
pub static PEER_STORE_DB: Lazy<sled::Db> = Lazy::new(|| {
    sled::Config::new()
        .temporary(true)
        .open()
        .expect("Failed to initialize temporary sled DB for peer store")
});

/// Stub for GUARDIAN_ROLE - synchronized role state
pub struct GuardianRoleState {
    pub current_guardian_ebid: String,
    pub last_guardian_change: u64,
    pub last_guardian_ping: u64,
}

impl GuardianRoleState {
    /// Returns the current guardian ID if set
    pub fn get_current_guardian(&self) -> Option<String> {
        if self.current_guardian_ebid.is_empty() {
            None
        } else {
            Some(self.current_guardian_ebid.clone())
        }
    }

    /// Expose state for read-only callers
    pub fn get_state(&self) -> Option<&GuardianRoleState> {
        Some(self)
    }

    /// Simple reachability check based on last ping
    pub fn is_guardian_reachable(&self, now: u64) -> bool {
        let time_since_ping = now.saturating_sub(self.last_guardian_ping);
        time_since_ping < 300
    }
}

pub static GUARDIAN_ROLE: Lazy<Mutex<GuardianRoleState>> = Lazy::new(|| {
    Mutex::new(GuardianRoleState {
        current_guardian_ebid: String::new(),
        last_guardian_change: 0,
        last_guardian_ping: 0,
    })
});

// ============================================================================
// Re-export for convenient access
// ============================================================================
pub use p2p_global::GlobalP2PManager;
pub use crate::p2p::peer_memory::ConstellationMemory;
