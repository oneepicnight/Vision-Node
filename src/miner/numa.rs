//! NUMA Topology Detection and Thread Affinity
//!
//! For high-end servers with multiple NUMA nodes (Threadripper, EPYC, Xeon):
//! - Detects NUMA topology
//! - Pins worker threads to specific NUMA nodes
//! - Prevents cross-NUMA memory thrashing
//! - Enables per-NUMA performance tracking

use serde::{Deserialize, Serialize};

/// NUMA topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumaTopology {
    /// Number of NUMA nodes detected
    pub num_nodes: usize,
    /// Logical CPU IDs per NUMA node
    pub cpus_per_node: Vec<Vec<usize>>,
}

impl NumaTopology {
    /// Detect NUMA topology (with optional hwloc2 support)
    pub fn detect() -> Self {
        #[cfg(feature = "hwloc")]
        {
            // Try hwloc2 detection first
            if let Ok(topo) = Self::detect_with_hwloc() {
                return topo;
            }
        }

        // Fallback: basic detection
        Self::detect_basic()
    }

    /// Basic detection: assume single NUMA node
    fn detect_basic() -> Self {
        let num_cpus = num_cpus::get();
        let cpus: Vec<usize> = (0..num_cpus).collect();

        Self {
            num_nodes: 1,
            cpus_per_node: vec![cpus],
        }
    }

    /// Advanced detection using hwloc2
    #[cfg(feature = "hwloc")]
    fn detect_with_hwloc() -> Result<Self, Box<dyn std::error::Error>> {
        use hwloc2::{ObjectType, Topology};

        let topo = Topology::new()?;

        // Get NUMA nodes
        let numa_nodes = topo.objects_with_type(&ObjectType::NUMANode)?;

        if numa_nodes.is_empty() {
            // No NUMA nodes detected, fallback to basic
            return Ok(Self::detect_basic());
        }

        let mut cpus_per_node = Vec::new();

        for numa_node in numa_nodes {
            let cpuset = numa_node.cpuset()?;
            let mut node_cpus = Vec::new();

            // Extract CPU IDs from cpuset
            for cpu_id in 0..num_cpus::get() {
                if cpuset.is_set(cpu_id) {
                    node_cpus.push(cpu_id);
                }
            }

            if !node_cpus.is_empty() {
                cpus_per_node.push(node_cpus);
            }
        }

        if cpus_per_node.is_empty() {
            return Ok(Self::detect_basic());
        }

        Ok(Self {
            num_nodes: cpus_per_node.len(),
            cpus_per_node,
        })
    }

    #[cfg(not(feature = "hwloc"))]
    fn detect_with_hwloc() -> Result<Self, Box<dyn std::error::Error>> {
        Err("hwloc feature not enabled".into())
    }

    /// Check if system has multiple NUMA nodes
    pub fn is_multi_numa(&self) -> bool {
        self.num_nodes > 1
    }

    /// Get total CPU count across all nodes
    pub fn total_cpus(&self) -> usize {
        self.cpus_per_node.iter().map(|cpus| cpus.len()).sum()
    }

    /// Distribute thread count evenly across NUMA nodes
    pub fn distribute_threads(&self, total_threads: usize) -> Vec<usize> {
        if !self.is_multi_numa() {
            return vec![total_threads];
        }

        let per_node = total_threads / self.num_nodes;
        let remainder = total_threads % self.num_nodes;

        let mut distribution = vec![per_node; self.num_nodes];

        // Distribute remainder to first N nodes
        for i in 0..remainder {
            distribution[i] += 1;
        }

        distribution
    }

    /// Get recommended thread count per node based on CPUs available
    pub fn optimal_threads_per_node(&self) -> Vec<usize> {
        self.cpus_per_node.iter().map(|cpus| cpus.len()).collect()
    }
}

/// NUMA-aware configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NumaConfig {
    /// Enable NUMA-aware thread placement
    pub enabled: bool,
}

/// NUMA-aware mining coordinator
pub struct NumaCoordinator {
    config: NumaConfig,
    topology: NumaTopology,
}

impl NumaCoordinator {
    pub fn new(config: NumaConfig) -> Self {
        let topology = NumaTopology::detect();

        Self { config, topology }
    }

    /// Get topology information
    pub fn topology(&self) -> &NumaTopology {
        &self.topology
    }

    /// Check if NUMA optimization is enabled and beneficial
    pub fn should_use_numa(&self) -> bool {
        self.config.enabled && self.topology.is_multi_numa()
    }

    /// Plan thread distribution across NUMA nodes
    pub fn plan_thread_distribution(&self, total_threads: usize) -> ThreadDistributionPlan {
        if !self.should_use_numa() {
            return ThreadDistributionPlan {
                numa_aware: false,
                node_assignments: vec![NodeAssignment {
                    node_id: 0,
                    thread_count: total_threads,
                    cpu_ids: (0..num_cpus::get()).collect(),
                }],
            };
        }

        let thread_counts = self.topology.distribute_threads(total_threads);
        let mut assignments = Vec::new();

        for (node_id, &thread_count) in thread_counts.iter().enumerate() {
            if thread_count > 0 {
                let cpu_ids = self
                    .topology
                    .cpus_per_node
                    .get(node_id)
                    .cloned()
                    .unwrap_or_default();

                assignments.push(NodeAssignment {
                    node_id,
                    thread_count,
                    cpu_ids,
                });
            }
        }

        ThreadDistributionPlan {
            numa_aware: true,
            node_assignments: assignments,
        }
    }

    /// Generate NUMA layout string for performance tracking
    pub fn layout_string(&self, distribution: &ThreadDistributionPlan) -> String {
        if !distribution.numa_aware {
            return "single".to_string();
        }

        distribution
            .node_assignments
            .iter()
            .map(|assignment| format!("node{}:{}", assignment.node_id, assignment.thread_count))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Thread distribution plan across NUMA nodes
#[derive(Debug, Clone)]
pub struct ThreadDistributionPlan {
    pub numa_aware: bool,
    pub node_assignments: Vec<NodeAssignment>,
}

/// Thread assignment for a specific NUMA node
#[derive(Debug, Clone)]
pub struct NodeAssignment {
    pub node_id: usize,
    pub thread_count: usize,
    pub cpu_ids: Vec<usize>,
}

/// Set thread affinity (platform-specific)
#[cfg(target_os = "linux")]
pub fn set_thread_affinity(cpu_ids: &[usize]) -> Result<(), String> {
    #[cfg(feature = "hwloc")]
    {
        use hwloc2::{Bitmap, CpuBindFlags, Topology};

        if let Ok(topo) = Topology::new() {
            let mut cpuset = Bitmap::new();
            for &cpu_id in cpu_ids {
                cpuset.set(cpu_id);
            }

            if topo.set_cpubind(&cpuset, CpuBindFlags::THREAD).is_ok() {
                return Ok(());
            }
        }
    }

    // Fallback: use libc directly
    #[cfg(not(feature = "hwloc"))]
    {
        use std::mem;

        unsafe {
            let mut cpu_set: libc::cpu_set_t = mem::zeroed();
            libc::CPU_ZERO(&mut cpu_set);

            for &cpu_id in cpu_ids {
                if cpu_id < 1024 {
                    libc::CPU_SET(cpu_id, &mut cpu_set);
                }
            }

            let result = libc::sched_setaffinity(0, mem::size_of::<libc::cpu_set_t>(), &cpu_set);

            if result == 0 {
                return Ok(());
            }
        }
    }

    Err("Failed to set thread affinity".to_string())
}

#[cfg(target_os = "windows")]
pub fn set_thread_affinity(cpu_ids: &[usize]) -> Result<(), String> {
    #[cfg(feature = "hwloc")]
    {
        use hwloc2::{Bitmap, CpuBindFlags, Topology};

        if let Ok(topo) = Topology::new() {
            let mut cpuset = Bitmap::new();
            for &cpu_id in cpu_ids {
                cpuset.set(cpu_id);
            }

            if topo.set_cpubind(&cpuset, CpuBindFlags::THREAD).is_ok() {
                return Ok(());
            }
        }
    }

    // Fallback: use Windows API
    #[cfg(not(feature = "hwloc"))]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentThread() -> *mut std::ffi::c_void;
            fn SetThreadAffinityMask(thread: *mut std::ffi::c_void, mask: usize) -> usize;
        }

        unsafe {
            let mut mask: usize = 0;
            for &cpu_id in cpu_ids {
                if cpu_id < 64 {
                    mask |= 1 << cpu_id;
                }
            }

            let handle = GetCurrentThread();
            let result = SetThreadAffinityMask(handle, mask);

            if result != 0 {
                return Ok(());
            }
        }
    }

    Err("Failed to set thread affinity".to_string())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn set_thread_affinity(_cpu_ids: &[usize]) -> Result<(), String> {
    Err("Thread affinity not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_detection() {
        let topology = NumaTopology::detect();
        assert!(topology.num_nodes >= 1);
        assert_eq!(topology.cpus_per_node.len(), topology.num_nodes);
    }

    #[test]
    fn test_thread_distribution() {
        let topology = NumaTopology {
            num_nodes: 2,
            cpus_per_node: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
        };

        let distribution = topology.distribute_threads(16);
        assert_eq!(distribution, vec![8, 8]);

        let distribution = topology.distribute_threads(17);
        assert_eq!(distribution, vec![9, 8]);
    }

    #[test]
    fn test_layout_string() {
        let config = NumaConfig { enabled: true };
        let mut coordinator = NumaCoordinator::new(config);

        // Override topology for testing
        coordinator.topology = NumaTopology {
            num_nodes: 2,
            cpus_per_node: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
        };

        let plan = coordinator.plan_thread_distribution(16);
        let layout = coordinator.layout_string(&plan);

        assert!(layout.contains("node0:8") || layout.contains("single"));
    }
}
