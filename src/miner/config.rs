//! Mining configuration with auto-detection and performance tuning
//!
//! **CRITICAL RULE**: Do NOT change the PoW result, only how fast we compute it.
//! All AVX / NUMA / threading must be pure optimization; the hash output must stay identical.

use std::sync::Arc;
use serde::{Serialize, Deserialize};

/// Mining configuration with auto-detection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Number of mining threads
    pub num_threads: usize,
    
    /// How many nonces per inner loop (batch size)
    pub hash_batch_size: usize,
    
    /// High-core profile enabled (24+ cores)
    pub high_core_profile: bool,
    
    /// Enable core affinity pinning
    pub enable_affinity: bool,
    
    /// CPU model detected
    pub cpu_model: String,
    
    /// Total system RAM in MB
    pub total_ram_mb: usize,
    
    /// Logical CPU cores detected (includes hyper-threading)
    /// Example: 16-core/32-thread CPU will show 32 logical cores
    pub logical_cores: usize,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self::auto_detect()
    }
}

impl MiningConfig {
    /// Auto-detect optimal mining configuration
    pub fn auto_detect() -> Self {
        let logical_cores = num_cpus::get();
        
        // Auto thread scaling based on core count
        let num_threads = if logical_cores <= 8 {
            logical_cores
        } else if logical_cores <= 16 {
            logical_cores - 2
        } else {
            logical_cores - 4
        };
        
        // High-core profile for monster rigs
        let high_core_profile = logical_cores > 16;
        
        // Batch size scaling
        let hash_batch_size = if logical_cores <= 8 {
            512
        } else if logical_cores <= 16 {
            1024
        } else {
            2048
        };
        
        // Detect CPU model (best-effort)
        let cpu_model = Self::detect_cpu_model();
        
        // Detect total RAM
        let total_ram_mb = Self::detect_total_ram_mb();
        
        Self {
            num_threads,
            hash_batch_size,
            high_core_profile,
            enable_affinity: true, // Enable by default
            cpu_model,
            total_ram_mb,
            logical_cores,
        }
    }
    
    /// Detect CPU model (platform-specific)
    fn detect_cpu_model() -> String {
        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/cpuinfo") {
                for line in contents.lines() {
                    if line.starts_with("model name") {
                        if let Some(model) = line.split(':').nth(1) {
                            return model.trim().to_string();
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            // Try to read from registry (best-effort)
            // For now, just return generic
        }
        
        format!("{}-core CPU", num_cpus::get())
    }
    
    /// Detect total system RAM in MB
    fn detect_total_ram_mb() -> usize {
        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
                for line in contents.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                return kb / 1024; // Convert KB to MB
                            }
                        }
                    }
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            // For Windows, we'd use GlobalMemoryStatusEx via winapi
            // For now, estimate based on typical system
        }
        
        // Fallback: assume 8GB
        8192
    }
    
    /// Print detection banner
    pub fn print_banner(&self) {
        tracing::info!(
            target: "vision_node::miner",
            "╔════════════════════════════════════════════════════════════════╗"
        );
        tracing::info!(
            target: "vision_node::miner",
            "║        VISION NODE MINER - CONFIGURATION DETECTED              ║"
        );
        tracing::info!(
            target: "vision_node::miner",
            "╠════════════════════════════════════════════════════════════════╣"
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ CPU Model:       {:<45} ║",
            truncate(&self.cpu_model, 45)
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ Logical Cores:   {:<45} ║",
            self.logical_cores
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ Mining Threads:  {:<45} ║",
            self.num_threads
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ Batch Size:      {:<45} ║",
            self.hash_batch_size
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ Total RAM:       {:<45} ║",
            format!("{} MB", self.total_ram_mb)
        );
        tracing::info!(
            target: "vision_node::miner",
            "║ Core Affinity:   {:<45} ║",
            if self.enable_affinity { "Enabled" } else { "Disabled" }
        );
        tracing::info!(
            target: "vision_node::miner",
            "╠════════════════════════════════════════════════════════════════╣"
        );
        
        if self.high_core_profile {
            tracing::info!(
                target: "vision_node::miner",
                "║ ⚡ HIGH-CORE PROFILE ENABLED                                   ║"
            );
            tracing::info!(
                target: "vision_node::miner",
                "║ 🚀 Detected monster rig! Engaging Threadripper optimizations. ║"
            );
            tracing::info!(
                target: "vision_node::miner",
                "║ 💫 Thank you for powering the Vision constellation.           ║"
            );
            tracing::info!(
                target: "vision_node::miner",
                "╠════════════════════════════════════════════════════════════════╣"
            );
        }
        
        tracing::info!(
            target: "vision_node::miner",
            "╚════════════════════════════════════════════════════════════════╝"
        );
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len-3])
    }
}

/// Set thread affinity (best-effort)
pub fn set_thread_affinity(worker_id: usize) -> bool {
    if let Some(core_ids) = core_affinity::get_core_ids() {
        if !core_ids.is_empty() {
            let core_id = core_ids[worker_id % core_ids.len()];
            if core_affinity::set_for_current(core_id) {
                tracing::debug!(
                    target: "vision_node::miner",
                    "[AFFINITY] Thread {} pinned to core {:?}",
                    worker_id,
                    core_id
                );
                return true;
            }
        }
    }
    
    tracing::warn!(
        target: "vision_node::miner",
        "[AFFINITY] Failed to pin thread {} to core (not critical, continuing)",
        worker_id
    );
    false
}
