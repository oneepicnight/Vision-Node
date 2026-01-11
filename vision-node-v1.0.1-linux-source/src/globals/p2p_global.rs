//! Global P2P Manager with safe late-initialization
//! Supports initialization after node_id is known, without panics if called before ready

use std::sync::Arc;
use std::ops::Deref;
use once_cell::sync::OnceCell;
use crate::p2p::connection::{P2PConnectionManager, P2PMessage};

/// Safe wrapper around P2PConnectionManager that supports late initialization
pub struct GlobalP2PManager {
    inner: OnceCell<Arc<P2PConnectionManager>>,
}

impl GlobalP2PManager {
    /// Create an uninitialized P2P manager
    pub const fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }

    /// Check if manager is ready
    pub fn is_ready(&self) -> bool {
        self.inner.get().is_some()
    }

    /// Initialize the P2P manager with a node ID
    pub fn init(&self, node_id: String) -> Result<(), &'static str> {
        let mgr = Arc::new(P2PConnectionManager::new(node_id));
        self.inner.set(mgr).map_err(|_| "P2P manager already initialized")
    }

    /// Get reference to inner manager if ready
    fn get(&self) -> Option<&Arc<P2PConnectionManager>> {
        self.inner.get()
    }

    // ============================================================================
    // Public API Methods (matches what codebase actually calls)
    // ============================================================================

    /// Get node ID, or "UNSET" if not initialized
    pub fn get_node_id(&self) -> String {
        self.get()
            .map(|m| m.get_node_id().to_owned())
            .unwrap_or_else(|| "UNSET".to_string())
    }

    /// Broadcast message to all connected peers
    pub async fn broadcast_message(
        &self,
        msg: P2PMessage,
    ) -> (usize, usize) {
        match self.get() {
            Some(mgr) => mgr.broadcast_message(msg).await,
            None => (0, 0), // No peers when uninitialized
        }
    }

    /// Get list of connected peer addresses
    pub async fn get_peer_addresses(&self) -> Vec<String> {
        match self.get() {
            Some(mgr) => mgr.get_peer_addresses().await,
            None => vec![],
        }
    }

    /// Get count of connected peers
    pub async fn connected_peer_count(&self) -> usize {
        match self.get() {
            Some(mgr) => mgr.connected_peer_count().await,
            None => 0,
        }
    }

    /// Get list of connected peer IDs
    pub async fn connected_peer_ids(&self) -> Vec<String> {
        match self.get() {
            Some(mgr) => mgr.connected_peer_ids().await,
            None => vec![],
        }
    }

    /// PATCH 1: Check if a peer is currently connected
    pub async fn is_peer_connected(&self, peer_addr: &str) -> bool {
        match self.get() {
            Some(mgr) => mgr.is_peer_connected(peer_addr).await,
            None => false,
        }
    }

    /// Clone the inner Arc for direct access (e.g., Arc::clone(&*crate::P2P_MANAGER))
    /// Panics if not initialized (matching existing usage pattern)
    pub fn clone_inner(&self) -> Arc<P2PConnectionManager> {
        Arc::clone(
            self.get()
                .expect("P2P manager not initialized"),
        )
    }
}

/// Implement Deref to return the Arc<P2PConnectionManager>
/// This allows Arc::clone(&*P2P_MANAGER) pattern to work
impl Deref for GlobalP2PManager {
    type Target = Arc<P2PConnectionManager>;

    fn deref(&self) -> &Self::Target {
        self.get()
            .expect("P2P manager not initialized; call init() at startup")
    }
}

impl Default for GlobalP2PManager {
    fn default() -> Self {
        Self::new()
    }
}
