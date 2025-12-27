#![allow(dead_code)]
/// Node role auto-detection module
///
/// Automatically determines if a node should operate as:
/// - Anchor: Backbone infrastructure (publicly reachable, many peers, stable)
/// - Edge: Regular miner node (outbound-only OK, uses anchors for truth)
///
/// End users don't configure roles manually - the node self-organizes based on health.
use once_cell::sync::Lazy;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::auto_sync::SyncHealthSnapshot;
use crate::vision_constants::MAX_MINING_LAG_BLOCKS;

/// Node role in the network
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeRole {
    /// Backbone node: publicly reachable, many peers, provides canonical truth
    Anchor,
    /// Regular miner/user node: outbound-only is fine, queries anchors for truth
    Edge,
}

impl NodeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeRole::Anchor => "Anchor",
            NodeRole::Edge => "Edge",
        }
    }
}

/// Current role state with change tracking
#[derive(Debug)]
pub struct RoleState {
    pub current: NodeRole,
    pub last_change: Instant,
}

/// Global node role singleton
pub static NODE_ROLE: Lazy<RwLock<RoleState>> = Lazy::new(|| {
    RwLock::new(RoleState {
        current: NodeRole::Edge, // Start as Edge, promote to Anchor if qualified
        last_change: Instant::now(),
    })
});

/// Minimum peers required for Anchor promotion
const MIN_ANCHOR_PEERS: usize = 3;

/// Minimum seconds before allowing role change (prevents flapping)
const MIN_ROLE_STABILITY_SECS: u64 = 60;

/// Detect node role based on health snapshot
///
/// Anchor criteria:
/// - Publicly reachable on P2P
/// - Has 3+ peer connections
/// - Within sync window (2 blocks of tip)
///
/// Otherwise: Edge (regular miner)
fn detect_node_role(snapshot: &SyncHealthSnapshot) -> NodeRole {
    // Hidden overrides for ops/dev (not documented for end users)
    if crate::vision_constants::is_env_flag_set("VISION_FORCE_ANCHOR") {
        return NodeRole::Anchor;
    }
    if crate::vision_constants::is_env_flag_set("VISION_FORCE_EDGE") {
        return NodeRole::Edge;
    }

    // Auto-detection based on health
    let lag = snapshot.height_lag();
    let in_sync_window = (0..=MAX_MINING_LAG_BLOCKS).contains(&lag);

    if snapshot.public_reachable.unwrap_or(false)
        && snapshot.peer_count() >= MIN_ANCHOR_PEERS
        && in_sync_window
    {
        NodeRole::Anchor
    } else {
        NodeRole::Edge
    }
}

/// Update global node role based on current health
///
/// Uses hysteresis to prevent role flapping - only changes role
/// if MIN_ROLE_STABILITY_SECS have passed since last change.
///
/// Call this regularly (e.g., in sync loop or miner tick) with fresh snapshot.
pub fn update_node_role(snapshot: &SyncHealthSnapshot) {
    let new_role = detect_node_role(snapshot);

    let mut guard = NODE_ROLE.write().unwrap();
    let now = Instant::now();

    // Only change role if stable period has passed
    if new_role != guard.current
        && now.duration_since(guard.last_change) >= Duration::from_secs(MIN_ROLE_STABILITY_SECS)
    {
        tracing::info!(
            "[ROLE] 🔄 Node role changed: {:?} -> {:?} (reachable={}, peers={}, lag={})",
            guard.current,
            new_role,
            snapshot.public_reachable.unwrap_or(false),
            snapshot.peer_count(),
            snapshot.height_lag()
        );
        guard.current = new_role;
        guard.last_change = now;
    }
}

/// Get current node role
pub fn current_node_role() -> NodeRole {
    NODE_ROLE.read().unwrap().current
}

/// Check if node is currently operating as an Anchor
pub fn is_anchor() -> bool {
    matches!(current_node_role(), NodeRole::Anchor)
}

/// Check if node is currently operating as an Edge node
pub fn is_edge() -> bool {
    matches!(current_node_role(), NodeRole::Edge)
}
