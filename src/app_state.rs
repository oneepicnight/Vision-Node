use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::p2p::node_id::NodeId;
use anyhow::Result;

/// Global application state for delayed node startup
/// Node waits for wallet configuration before starting P2P and mining
pub struct AppState {
    /// Main database handle
    pub db: Arc<sled::Db>,
    
    /// Configured wallet address (None until user saves in miner panel)
    pub wallet_address: RwLock<Option<String>>,
    
    /// Derived node ID (None until wallet is configured)
    pub node_id: RwLock<Option<NodeId>>,
    
    /// Whether full node (P2P + mining) has started
    pub full_node_started: AtomicBool,
}

impl AppState {
    pub fn new(db: Arc<sled::Db>) -> Self {
        Self {
            db,
            wallet_address: RwLock::new(None),
            node_id: RwLock::new(None),
            full_node_started: AtomicBool::new(false),
        }
    }
    
    /// Load existing wallet/node_id from database (for restart persistence)
    pub fn load_existing_identity(&self) -> Result<()> {
        // Load wallet address
        if let Some(addr_bytes) = self.db.get(b"node_wallet_address")? {
            let addr_str = String::from_utf8(addr_bytes.to_vec())?;
            tracing::info!("📝 Loaded existing wallet address: {}", addr_str);
            
            let mut w = self.wallet_address.write().unwrap();
            *w = Some(addr_str.clone());
            
            // Load node ID
            if let Some(id_bytes) = self.db.get(b"node_id")? {
                let id_str = String::from_utf8(id_bytes.to_vec())?;
                let node_id = NodeId(id_str.clone());
                tracing::info!("🆔 Loaded existing node ID: {}", id_str);
                
                let mut n = self.node_id.write().unwrap();
                *n = Some(node_id);
            }
        }
        
        Ok(())
    }
    
    /// Check if wallet is configured
    pub fn is_wallet_configured(&self) -> bool {
        self.wallet_address.read().unwrap().is_some()
    }
    
    /// Check if node ID is derived
    pub fn is_node_id_ready(&self) -> bool {
        self.node_id.read().unwrap().is_some()
    }
    
    /// Check if full node is running
    pub fn is_full_node_started(&self) -> bool {
        self.full_node_started.load(Ordering::SeqCst)
    }
    
    /// Get wallet address (if configured)
    pub fn get_wallet_address(&self) -> Option<String> {
        self.wallet_address.read().unwrap().clone()
    }
    
    /// Get node ID (if derived)
    pub fn get_node_id(&self) -> Option<NodeId> {
        self.node_id.read().unwrap().clone()
    }
}
