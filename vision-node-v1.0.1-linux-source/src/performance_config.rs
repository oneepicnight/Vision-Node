//! Performance Optimization Configuration
//!
//! Tuned parameters for production performance based on benchmarks

use std::time::Duration;

/// P2P Connection Pool Configuration
pub struct P2PPoolConfig {
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Keep-alive interval
    pub keepalive_interval: Duration,
    /// Read buffer size (optimized for network MTU)
    pub read_buffer_size: usize,
    /// Write buffer size
    pub write_buffer_size: usize,
}

impl Default for P2PPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 128,                   // Up from default 100
            connect_timeout: Duration::from_secs(10),
            keepalive_interval: Duration::from_secs(30),
            read_buffer_size: 65536,                // 64KB buffer
            write_buffer_size: 65536,
        }
    }
}

/// Peer Manager Cache Configuration
pub struct PeerCacheConfig {
    /// Cache size for hot peers
    pub hot_cache_size: usize,
    /// Warm peer cache size
    pub warm_cache_size: usize,
    /// Cold peer cache size (smaller, less frequently accessed)
    pub cold_cache_size: usize,
    /// Cache eviction interval
    pub eviction_interval: Duration,
}

impl Default for PeerCacheConfig {
    fn default() -> Self {
        Self {
            hot_cache_size: 100,
            warm_cache_size: 500,
            cold_cache_size: 1000,
            eviction_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Database Write Batching
pub struct DbBatchConfig {
    /// Batch size before flush
    pub batch_size: usize,
    /// Maximum time before flush
    pub batch_timeout: Duration,
    /// Enable write-ahead logging
    pub enable_wal: bool,
}

impl Default for DbBatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,                        // Batch 100 writes
            batch_timeout: Duration::from_secs(5),  // Flush every 5s
            enable_wal: true,
        }
    }
}

/// Lock-Free Data Structures Configuration
pub struct LockFreeConfig {
    /// Use lock-free queues for message passing
    pub use_lock_free_queues: bool,
    /// Queue capacity
    pub queue_capacity: usize,
    /// Use atomic operations for counters
    pub atomic_counters: bool,
}

impl Default for LockFreeConfig {
    fn default() -> Self {
        Self {
            use_lock_free_queues: true,
            queue_capacity: 10000,
            atomic_counters: true,
        }
    }
}

/// Memory Pool Configuration
pub struct MemoryPoolConfig {
    /// Pre-allocate peer objects
    pub peer_pool_size: usize,
    /// Pre-allocate message buffers
    pub message_buffer_pool_size: usize,
    /// Buffer size
    pub buffer_size: usize,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            peer_pool_size: 1000,
            message_buffer_pool_size: 5000,
            buffer_size: 4096,
        }
    }
}

/// Thread Pool Configuration
pub struct ThreadPoolConfig {
    /// Worker threads for async runtime
    pub worker_threads: usize,
    /// Blocking threads pool size
    pub blocking_threads: usize,
    /// Thread stack size
    pub stack_size: usize,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            worker_threads: cpu_count * 2,          // 2x CPU cores
            blocking_threads: cpu_count * 4,        // 4x for blocking ops
            stack_size: 2 * 1024 * 1024,           // 2MB stack
        }
    }
}

/// Performance Monitoring Configuration
pub struct PerfMonitorConfig {
    /// Enable performance metrics
    pub enabled: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Slow operation threshold
    pub slow_op_threshold: Duration,
}

impl Default for PerfMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(60),
            slow_op_threshold: Duration::from_millis(100),
        }
    }
}

/// Master Performance Configuration
pub struct PerformanceConfig {
    pub p2p_pool: P2PPoolConfig,
    pub peer_cache: PeerCacheConfig,
    pub db_batch: DbBatchConfig,
    pub lock_free: LockFreeConfig,
    pub memory_pool: MemoryPoolConfig,
    pub thread_pool: ThreadPoolConfig,
    pub perf_monitor: PerfMonitorConfig,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            p2p_pool: P2PPoolConfig::default(),
            peer_cache: PeerCacheConfig::default(),
            db_batch: DbBatchConfig::default(),
            lock_free: LockFreeConfig::default(),
            memory_pool: MemoryPoolConfig::default(),
            thread_pool: ThreadPoolConfig::default(),
            perf_monitor: PerfMonitorConfig::default(),
        }
    }
}

/// Load performance config from environment or use defaults
pub fn load_performance_config() -> PerformanceConfig {
    let mut config = PerformanceConfig::default();
    
    // Override from environment variables
    if let Ok(max_conn) = std::env::var("P2P_MAX_CONNECTIONS") {
        if let Ok(n) = max_conn.parse() {
            config.p2p_pool.max_connections = n;
        }
    }
    
    if let Ok(workers) = std::env::var("WORKER_THREADS") {
        if let Ok(n) = workers.parse() {
            config.thread_pool.worker_threads = n;
        }
    }
    
    if let Ok(batch) = std::env::var("DB_BATCH_SIZE") {
        if let Ok(n) = batch.parse() {
            config.db_batch.batch_size = n;
        }
    }
    
    config
}

/// Performance optimization hints
pub fn print_performance_info() {
    let config = load_performance_config();
    println!("🚀 Performance Configuration:");
    println!("  - Worker Threads: {}", config.thread_pool.worker_threads);
    println!("  - Max P2P Connections: {}", config.p2p_pool.max_connections);
    println!("  - DB Batch Size: {}", config.db_batch.batch_size);
    println!("  - Peer Cache: Hot={}, Warm={}, Cold={}",
        config.peer_cache.hot_cache_size,
        config.peer_cache.warm_cache_size,
        config.peer_cache.cold_cache_size
    );
    println!("  - Lock-Free Queues: {}", config.lock_free.use_lock_free_queues);
}
