//! Miner Configuration
//!
//! Manages mining reward addresses and auto-population from wallet.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Reward eligibility requirements to prevent fake-rich scenarios on testnet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEligibilityConfig {
    /// Minimum number of connected peers required before we pay block rewards
    #[serde(default = "default_min_peers_for_rewards")]
    pub min_peers_for_rewards: u16,

    /// Maximum allowed desync between local tip and network estimated height
    /// before we pay block rewards (blocks)
    #[serde(default = "default_max_reward_desync_blocks")]
    pub max_reward_desync_blocks: u64,

    /// Height below which we do not pay any block subsidy (warm-up era)
    #[serde(default)]
    pub reward_warmup_height: u64,

    /// Node startup timestamp (for timeout-based quorum escape)
    #[serde(skip)]
    pub node_started_at: Option<Instant>,
}

fn default_min_peers_for_rewards() -> u16 {
    3
}
fn default_max_reward_desync_blocks() -> u64 {
    5
}

impl Default for RewardEligibilityConfig {
    fn default() -> Self {
        Self {
            min_peers_for_rewards: default_min_peers_for_rewards(),
            max_reward_desync_blocks: default_max_reward_desync_blocks(),
            reward_warmup_height: 0,
            node_started_at: None,
        }
    }
}

/// Snapshot of node sync health for reward eligibility checks
#[derive(Debug, Clone)]
pub struct SyncHealthSnapshot {
    pub connected_peers: u16,
    pub p2p_health: String, // "ok", "stable", "isolated", etc.
    pub sync_height: u64,
    pub network_estimated_height: u64,
    pub height_quorum_ok: bool, // Are we synced with network height consensus?
    pub height_quorum_peers: usize, // How many peers agree on the quorum height
    pub height_quorum_height: Option<u64>, // What height the network has converged on
}

/// Check if node is eligible for block rewards based on sync health
pub fn is_reward_eligible(
    cfg: &RewardEligibilityConfig,
    snapshot: &SyncHealthSnapshot,
    current_height: u64,
) -> bool {
    use crate::vision_constants::MINING_QUORUM_TIMEOUT_SECS;

    // Check 1: Must be past warmup period
    if current_height < cfg.reward_warmup_height {
        return false;
    }

    // Check 2: Must have minimum peers
    if snapshot.connected_peers < cfg.min_peers_for_rewards {
        return false;
    }

    // Check 3: P2P health must be acceptable (not isolated/weak)
    let health_ok = matches!(snapshot.p2p_health.as_str(), "ok" | "stable" | "immortal");
    if !health_ok {
        return false;
    }

    // Check 4: Must not be desynced beyond threshold
    let desync = snapshot
        .network_estimated_height
        .saturating_sub(snapshot.sync_height);
    if desync > cfg.max_reward_desync_blocks {
        return false;
    }

    // Check 5: Height quorum gate - must be synced with network consensus
    // UNLESS timeout has elapsed (allows isolated mining after 5 min)
    if !snapshot.height_quorum_ok {
        // Check if we can use timeout escape
        let timeout_elapsed = if let Some(start_time) = cfg.node_started_at {
            start_time.elapsed().as_secs() >= MINING_QUORUM_TIMEOUT_SECS
        } else {
            // No start time recorded, be conservative
            false
        };

        if !timeout_elapsed {
            // Fix C: Clear explanation of why mining is blocked
            if snapshot.height_quorum_peers > 0 {
                eprintln!(
                    "[MINING GATE] BLOCKED: quorum_same_height not met (have={} peers at height={:?}, we're at local={}, need=2+)",
                    snapshot.height_quorum_peers,
                    snapshot.height_quorum_height,
                    snapshot.sync_height
                );
            } else {
                eprintln!(
                    "[MINING GATE] BLOCKED: no height quorum detected (have=0 peers, need=2+ for network convergence)"
                );
            }
            return false;
        } else {
            // Timeout elapsed, allow mining in isolated mode
            eprintln!(
                "[MINING GATE] ✅ ALLOWED: ISOLATED mode (quorum timeout elapsed after {}s)",
                MINING_QUORUM_TIMEOUT_SECS
            );
        }
    }

    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    /// Mining reward address (LAND only)
    pub reward_address: Option<String>,

    /// Enable automatic mining (default: false)
    #[serde(default)]
    pub auto_mine: bool,

    /// Maximum transactions per block
    #[serde(default = "default_max_txs")]
    pub max_txs: usize,

    /// Reward eligibility requirements (testnet anti-fake-rich)
    #[serde(default)]
    pub reward_eligibility: RewardEligibilityConfig,

    /// Optional mining profile: "laptop", "balanced", "beast".
    /// If None, default to "balanced".
    #[serde(default = "default_mining_profile")]
    pub mining_profile: Option<String>,

    /// Explicit mining thread count override.
    /// - If Some(n) and n > 0, use that.
    /// - If None or invalid, auto-detect from CPU and mining_profile.
    #[serde(default)]
    pub mining_threads: Option<usize>,

    /// SIMD-friendly batch size for nonce processing.
    /// 1 = old behavior, >1 = multiple nonces per inner loop.
    #[serde(default = "default_simd_batch_size")]
    pub simd_batch_size: Option<u64>,

    // ========== AUTO-TUNING CONFIGURATION ==========
    /// Enable automatic performance tuning
    #[serde(default = "default_auto_tune_enabled")]
    pub auto_tune_enabled: bool,

    /// Auto-tuning aggressiveness mode
    #[serde(default)]
    pub auto_tune_mode: AutoTuneMode,

    /// Minimum allowed threads for auto-tuning
    #[serde(default)]
    pub min_threads: Option<usize>,

    /// Maximum allowed threads for auto-tuning
    #[serde(default)]
    pub max_threads: Option<usize>,

    /// Minimum allowed batch size for auto-tuning
    #[serde(default)]
    pub min_batch_size: Option<u32>,

    /// Maximum allowed batch size for auto-tuning
    #[serde(default)]
    pub max_batch_size: Option<u32>,

    /// Evaluation window in seconds (collect samples before recording)
    #[serde(default = "default_evaluation_window_secs")]
    pub evaluation_window_secs: u64,

    /// Re-evaluation interval in seconds (how often to try new settings)
    #[serde(default = "default_reeval_interval_secs")]
    pub reeval_interval_secs: u64,

    // ========== NETWORK TELEMETRY ==========
    /// Enable anonymous telemetry reporting to network
    #[serde(default)]
    pub telemetry_enabled: bool,

    /// Telemetry server endpoint (optional)
    #[serde(default)]
    pub telemetry_endpoint: Option<String>,

    // ========== THERMAL PROTECTION ==========
    /// Enable thermal monitoring and protection
    #[serde(default = "default_thermal_enabled")]
    pub thermal_protection_enabled: bool,

    /// Soft thermal limit (start throttling, °C)
    #[serde(default)]
    pub thermal_soft_limit_c: Option<u32>,

    /// Hard thermal limit (aggressive throttling, °C)
    #[serde(default)]
    pub thermal_hard_limit_c: Option<u32>,

    /// Thermal cooldown period (seconds)
    #[serde(default = "default_thermal_cooldown_secs")]
    pub thermal_cooldown_secs: u64,

    // ========== POWER MODE SENSITIVITY ==========
    /// Enable power-aware throttling (battery vs AC)
    #[serde(default = "default_power_sensitivity")]
    pub power_mode_sensitivity: bool,

    /// Thread cap when on battery power
    #[serde(default)]
    pub battery_threads_cap: Option<usize>,

    /// Batch size cap when on battery power
    #[serde(default)]
    pub battery_batch_cap: Option<u32>,

    // ========== NUMA AWARENESS ==========
    /// Enable NUMA-aware thread placement (advanced)
    #[serde(default)]
    pub numa_aware_enabled: bool,

    // ========== P2P TUNING HINTS ==========
    /// Enable P2P tuning hints system (distributed learning)
    #[serde(default = "default_p2p_hints_enabled")]
    pub p2p_hints_enabled: bool,

    /// Minimum gain ratio to adopt a peer hint (0.03 = 3%)
    #[serde(default = "default_hint_trial_threshold")]
    pub hint_trial_threshold: f64,

    /// Maximum pending hints before dropping new ones
    #[serde(default = "default_hint_max_pending")]
    pub hint_max_pending: usize,

    /// Minimum peer reputation to accept hints from (0.0-100.0)
    #[serde(default = "default_hint_min_peer_reputation")]
    pub hint_min_peer_reputation: f32,

    /// Broadcast interval for sharing elite configs (minutes)
    #[serde(default = "default_hint_broadcast_interval_mins")]
    pub hint_broadcast_interval_mins: u64,
}

/// Auto-tuning aggressiveness mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AutoTuneMode {
    /// Conservative exploration, small steps
    Conservative,
    /// Normal exploration, balanced
    #[default]
    Normal,
    /// Aggressive exploration, larger steps
    Aggressive,
}

fn default_max_txs() -> usize {
    1000
}

fn default_mining_profile() -> Option<String> {
    Some("balanced".to_string())
}

fn default_simd_batch_size() -> Option<u64> {
    Some(4)
}

fn default_auto_tune_enabled() -> bool {
    true
}

fn default_evaluation_window_secs() -> u64 {
    60 // 1 minute
}

fn default_reeval_interval_secs() -> u64 {
    900 // 15 minutes
}

fn default_thermal_enabled() -> bool {
    true
}

fn default_thermal_cooldown_secs() -> u64 {
    120 // 2 minutes
}

fn default_power_sensitivity() -> bool {
    true
}

fn default_p2p_hints_enabled() -> bool {
    true
}

fn default_hint_trial_threshold() -> f64 {
    0.03 // 3% improvement required
}

fn default_hint_max_pending() -> usize {
    50
}

fn default_hint_min_peer_reputation() -> f32 {
    30.0
}

fn default_hint_broadcast_interval_mins() -> u64 {
    30
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            reward_address: None,
            auto_mine: false,
            max_txs: default_max_txs(),
            reward_eligibility: RewardEligibilityConfig::default(),
            mining_profile: default_mining_profile(),
            mining_threads: None,
            simd_batch_size: default_simd_batch_size(),
            auto_tune_enabled: default_auto_tune_enabled(),
            auto_tune_mode: AutoTuneMode::default(),
            min_threads: None,
            max_threads: None,
            min_batch_size: None,
            max_batch_size: None,
            evaluation_window_secs: default_evaluation_window_secs(),
            reeval_interval_secs: default_reeval_interval_secs(),
            telemetry_enabled: false,
            telemetry_endpoint: None,
            thermal_protection_enabled: default_thermal_enabled(),
            thermal_soft_limit_c: Some(80),
            thermal_hard_limit_c: Some(90),
            thermal_cooldown_secs: default_thermal_cooldown_secs(),
            power_mode_sensitivity: default_power_sensitivity(),
            battery_threads_cap: Some(2),
            battery_batch_cap: Some(4),
            numa_aware_enabled: false,
            p2p_hints_enabled: default_p2p_hints_enabled(),
            hint_trial_threshold: default_hint_trial_threshold(),
            hint_max_pending: default_hint_max_pending(),
            hint_min_peer_reputation: default_hint_min_peer_reputation(),
            hint_broadcast_interval_mins: default_hint_broadcast_interval_mins(),
        }
    }
}

impl MinerConfig {
    /// Load miner config from file, creating default if missing
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read miner config: {}", e))?;
            let config: MinerConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse miner config: {}", e))?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Save miner config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize miner config: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write miner config: {}", e))?;
        Ok(())
    }

    /// Validate that reward address is a valid LAND address
    pub fn validate_reward_address(&self) -> Result<(), String> {
        if let Some(addr) = &self.reward_address {
            if !super::wallet::is_valid_land_address(addr) {
                return Err(format!(
                    "Invalid reward address: {}. Must be a LAND address (starts with 'land1')",
                    addr
                ));
            }
        }
        Ok(())
    }

    /// Get reward address, returning error if not set or invalid
    pub fn get_validated_reward_address(&self) -> Result<String, String> {
        match &self.reward_address {
            Some(addr) if super::wallet::is_valid_land_address(addr) => Ok(addr.clone()),
            Some(addr) => Err(format!(
                "Invalid LAND reward address: {}. Mining requires a valid LAND address.", 
                addr
            )),
            None => Err(
                "No mining reward address configured. Set 'reward_address' in miner.json to a LAND address.".to_string()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_miner_config_validation() {
        let mut config = MinerConfig::default();

        // No address should pass validation (it's optional)
        assert!(config.validate_reward_address().is_ok());

        // Invalid address should fail
        config.reward_address = Some("btc1invalid".to_string());
        assert!(config.validate_reward_address().is_err());

        // Valid LAND address should pass
        config.reward_address = Some("land1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszq".to_string());
        assert!(config.validate_reward_address().is_ok());
    }
}
