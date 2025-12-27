//! Guardian Role Management
//!
//! Handles guardian election, rotation, and autonomous emergence when
//! the current guardian becomes unreachable.

use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use tracing::{info, warn};

const GUARDIAN_ROLE_KEY: &[u8] = b"current_guardian_role";

/// Configuration for guardian rotation behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianRoleConfig {
    /// Minimum uptime score required to be guardian
    pub min_guardian_uptime: f64,

    /// Seconds of unreachability before rotating guardian
    pub rotation_timeout_secs: u64,
}

impl Default for GuardianRoleConfig {
    fn default() -> Self {
        Self {
            min_guardian_uptime: 0.7,
            rotation_timeout_secs: 300, // 5 minutes
        }
    }
}

/// Current guardian role state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardianRoleState {
    /// EBID of current guardian
    pub current_guardian_ebid: String,

    /// Unix timestamp of last guardian change
    pub last_guardian_change: u64,

    /// Last time guardian was confirmed reachable
    pub last_guardian_ping: u64,
}

/// Guardian role manager
pub struct GuardianRole {
    /// Persistent storage
    db: Arc<sled::Tree>,

    /// Current state (cached)
    state: Option<GuardianRoleState>,

    /// Configuration
    config: GuardianRoleConfig,
}

impl GuardianRole {
    /// Create a new guardian role manager
    pub fn new(db: &Db, config: GuardianRoleConfig) -> Result<Self, String> {
        let tree = db
            .open_tree("guardian_role")
            .map_err(|e| format!("Failed to open guardian_role tree: {}", e))?;

        let mut role = Self {
            db: Arc::new(tree),
            state: None,
            config,
        };

        role.load_state()?;

        Ok(role)
    }

    /// Load state from persistent storage
    fn load_state(&mut self) -> Result<(), String> {
        if let Some(data) = self
            .db
            .get(GUARDIAN_ROLE_KEY)
            .map_err(|e| format!("Failed to read guardian role: {}", e))?
        {
            let state = bincode::deserialize::<GuardianRoleState>(&data)
                .map_err(|e| format!("Failed to deserialize guardian role: {}", e))?;

            info!(
                target: "vision_node::guardian::role",
                "[GUARDIAN_ROLE] Loaded current guardian: {}",
                state.current_guardian_ebid
            );

            self.state = Some(state);
        }

        Ok(())
    }

    /// Save state to persistent storage
    fn save_state(&self) -> Result<(), String> {
        if let Some(ref state) = self.state {
            let serialized = bincode::serialize(state)
                .map_err(|e| format!("Failed to serialize guardian role: {}", e))?;

            self.db
                .insert(GUARDIAN_ROLE_KEY, serialized)
                .map_err(|e| format!("Failed to save guardian role: {}", e))?;

            self.db
                .flush()
                .map_err(|e| format!("Failed to flush guardian role: {}", e))?;
        }

        Ok(())
    }

    /// Get current guardian EBID (if any)
    pub fn get_current_guardian(&self) -> Option<String> {
        self.state.as_ref().map(|s| s.current_guardian_ebid.clone())
    }

    /// Set a new guardian
    pub fn set_current_guardian(&mut self, ebid: String) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        info!(
            target: "vision_node::guardian::role",
            "[GUARDIAN_ROLE] 👑 Guardian rotated to EBID: {}",
            ebid
        );

        self.state = Some(GuardianRoleState {
            current_guardian_ebid: ebid,
            last_guardian_change: now,
            last_guardian_ping: now,
        });

        self.save_state()
    }

    /// Update guardian ping time (confirms reachability)
    pub fn ping_guardian(&mut self) -> Result<(), String> {
        if let Some(ref mut state) = self.state {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            state.last_guardian_ping = now;
            self.save_state()?;
        }

        Ok(())
    }

    /// Check if guardian should be rotated due to unreachability
    pub fn should_rotate(&self, now: u64) -> bool {
        if let Some(ref state) = self.state {
            let time_since_ping = now.saturating_sub(state.last_guardian_ping);

            if time_since_ping > self.config.rotation_timeout_secs {
                warn!(
                    target: "vision_node::guardian::role",
                    "[GUARDIAN_ROLE] Guardian {} unreachable for {}s (threshold: {}s)",
                    state.current_guardian_ebid,
                    time_since_ping,
                    self.config.rotation_timeout_secs
                );
                return true;
            }
        }

        false
    }

    /// Get time since last guardian change
    pub fn time_since_change(&self, now: u64) -> Option<u64> {
        self.state
            .as_ref()
            .map(|s| now.saturating_sub(s.last_guardian_change))
    }

    /// Get time since last guardian ping
    pub fn time_since_ping(&self, now: u64) -> Option<u64> {
        self.state
            .as_ref()
            .map(|s| now.saturating_sub(s.last_guardian_ping))
    }

    /// Check if guardian is considered reachable
    pub fn is_guardian_reachable(&self, now: u64) -> bool {
        if let Some(time_since_ping) = self.time_since_ping(now) {
            time_since_ping < self.config.rotation_timeout_secs
        } else {
            false
        }
    }

    /// Get current state (for API responses)
    pub fn get_state(&self) -> Option<&GuardianRoleState> {
        self.state.as_ref()
    }
}
