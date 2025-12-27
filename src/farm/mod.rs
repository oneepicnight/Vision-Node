use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub mod farmhand_config;
pub mod profile;
pub mod scheduler;
pub mod websocket;

/// Represents a remote mining rig in the farm
#[derive(Clone, Debug, Serialize)]
pub struct FarmRig {
    /// Unique identifier for this rig
    pub rig_id: String,
    /// Human-readable name
    pub name: String,
    /// Operating system (e.g., "Windows 11", "Ubuntu 22.04")
    pub os: String,
    /// Number of CPU threads available
    pub cpu_threads: u32,
    /// Current status: "pending", "online", "offline", "mining", "idle", "error"
    pub status: String,
    /// Current hashrate in H/s
    pub hashrate: f64,
    /// Last heartbeat timestamp (Unix seconds)
    pub last_heartbeat: u64,
    /// Active profile name (for v2)
    pub profile: Option<String>,
    /// Endpoint mode: "local" or "public"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_mode: Option<String>,
}

/// Global state for farm management
pub struct FarmState {
    /// All registered rigs
    pub rigs: HashMap<String, FarmRig>,
    /// Command channels for each rig (rig_id -> sender)
    pub commands: HashMap<String, mpsc::Sender<FarmCommand>>,
}

impl FarmState {
    pub fn new() -> Self {
        Self {
            rigs: HashMap::new(),
            commands: HashMap::new(),
        }
    }

    /// Register a new rig
    pub fn register_rig(&mut self, rig: FarmRig, command_tx: mpsc::Sender<FarmCommand>) {
        let rig_id = rig.rig_id.clone();
        self.rigs.insert(rig_id.clone(), rig);
        self.commands.insert(rig_id, command_tx);
    }

    /// Remove a rig (when disconnected)
    pub fn remove_rig(&mut self, rig_id: &str) {
        self.rigs.remove(rig_id);
        self.commands.remove(rig_id);
    }

    /// Update rig status
    pub fn update_rig(&mut self, rig_id: &str, update: RigUpdate) {
        if let Some(rig) = self.rigs.get_mut(rig_id) {
            if let Some(status) = update.status {
                rig.status = status;
            }
            if let Some(hashrate) = update.hashrate {
                rig.hashrate = hashrate;
            }
            rig.last_heartbeat = update.timestamp;
        }
    }

    /// Get rig by ID
    pub fn get_rig(&self, rig_id: &str) -> Option<&FarmRig> {
        self.rigs.get(rig_id)
    }

    /// Get all rigs
    pub fn get_all_rigs(&self) -> Vec<FarmRig> {
        self.rigs.values().cloned().collect()
    }

    /// Send command to a rig
    pub async fn send_command(&self, rig_id: &str, command: FarmCommand) -> Result<(), String> {
        if let Some(tx) = self.commands.get(rig_id) {
            tx.send(command)
                .await
                .map_err(|e| format!("Failed to send command: {}", e))?;
            Ok(())
        } else {
            Err(format!("Rig {} not found", rig_id))
        }
    }

    /// Get total farm statistics
    pub fn get_stats(&self) -> FarmStats {
        let total_rigs = self.rigs.len();
        let online_rigs = self
            .rigs
            .values()
            .filter(|r| r.status == "online" || r.status == "mining")
            .count();
        let mining_rigs = self.rigs.values().filter(|r| r.status == "mining").count();
        let total_hashrate: f64 = self.rigs.values().map(|r| r.hashrate).sum();

        FarmStats {
            total_rigs,
            online_rigs,
            mining_rigs,
            total_hashrate,
        }
    }
}

impl Default for FarmState {
    fn default() -> Self {
        Self::new()
    }
}

/// Update data for a rig
#[derive(Debug)]
pub struct RigUpdate {
    pub status: Option<String>,
    pub hashrate: Option<f64>,
    pub timestamp: u64,
}

/// Statistics for the entire farm
#[derive(Clone, Debug, Serialize)]
pub struct FarmStats {
    pub total_rigs: usize,
    pub online_rigs: usize,
    pub mining_rigs: usize,
    pub total_hashrate: f64,
}

/// Commands that can be sent to farm rigs
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FarmCommand {
    StartMining,
    StopMining,
    ApplyProfile {
        #[serde(flatten)]
        config: profile::FarmProfileConfig,
    },
}

/// Registration message from a rig agent
#[derive(Debug, Deserialize)]
pub struct RigRegistration {
    #[serde(rename = "type")]
    pub msg_type: String, // Should be "register"
    pub rig_id: String,
    pub name: String,
    pub os: String,
    pub cpu_threads: u32,
}

/// Heartbeat message from a rig agent
#[derive(Debug, Deserialize)]
pub struct RigHeartbeat {
    #[serde(rename = "type")]
    pub msg_type: String, // Should be "heartbeat"
    pub rig_id: String,
    pub status: String,
    pub hashrate: f64,
}

/// Generic message envelope for WebSocket communication
#[derive(Debug, Deserialize)]
pub struct RigMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}
