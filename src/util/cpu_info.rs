#![allow(dead_code)]
//! CPU information detection for mining optimization
//!
//! Provides utilities to detect CPU model, physical cores, and logical cores
//! for intelligent mining thread allocation.

use sysinfo::System;

#[derive(Debug, Clone)]
pub struct CpuSummary {
    pub model: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
}

/// Detect CPU information including model name and core counts
pub fn detect_cpu_summary() -> CpuSummary {
    let sys = System::new_all();

    // Get CPU brand/model name
    let model = if let Some(cpu) = sys.cpus().first() {
        cpu.brand().to_string()
    } else {
        "Unknown CPU".to_string()
    };

    let logical_cores = num_cpus::get();
    let physical_cores = num_cpus::get_physical();

    CpuSummary {
        model,
        physical_cores,
        logical_cores,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cpu_summary() {
        let cpu = detect_cpu_summary();

        // Should have at least 1 core
        assert!(cpu.physical_cores >= 1);
        assert!(cpu.logical_cores >= 1);

        // Logical cores should be >= physical cores (due to hyperthreading)
        assert!(cpu.logical_cores >= cpu.physical_cores);

        // Model should not be empty
        assert!(!cpu.model.is_empty());

        println!("Detected CPU: {}", cpu.model);
        println!("Physical cores: {}", cpu.physical_cores);
        println!("Logical cores: {}", cpu.logical_cores);
    }
}
