//! Enhanced miner manager with active mining loop
//!
//! Manages mining workers, distributes work, and submits found blocks.

use crate::config::miner::MinerConfig;
use crate::consensus_pow::{BlockBuilder, BlockSubmitter, DifficultyConfig, DifficultyTracker};
#[cfg(feature = "miner-tuning")]
use crate::miner::auto_tuner;
#[cfg(feature = "miner-tuning")]
use crate::miner::perf_store::{MinerPerfStore, PerfKey};
use crate::miner_manager::{MinerCfg, MinerSpeed};
use crate::pow::visionx::{PowJob, VisionXMiner, VisionXParams};
use crate::util::cpu_info::{detect_cpu_summary, CpuSummary};
use crate::BlockHeader;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, Instant};
use tracing::info;

const BATCH_SIZE: u32 = 1000; // Nonces to try per batch (will be replaced by configurable batch_size)

/// Resolve the number of mining threads based on config and CPU info
fn resolve_mining_threads(cfg: &MinerConfig, cpu: &CpuSummary) -> usize {
    // Explicit override wins if valid
    if let Some(explicit) = cfg.mining_threads {
        if explicit > 0 {
            return explicit.min(cpu.logical_cores.max(1));
        }
    }

    let profile = cfg
        .mining_profile
        .as_deref()
        .unwrap_or("balanced")
        .to_lowercase();

    match profile.as_str() {
        "laptop" => cpu.logical_cores.min(4).max(1),
        "beast" => cpu.logical_cores.max(1),
        "balanced" | _ => ((cpu.logical_cores as f32) * 0.5).ceil() as usize,
    }
}

/// Resolve SIMD batch size from config
fn resolve_simd_batch_size(cfg: &MinerConfig) -> u64 {
    let raw = cfg.simd_batch_size.unwrap_or(4);
    raw.max(1).min(1024) // keep it sane
}

/// Hashrate sampler for windowed performance tracking
pub struct HashrateSampler {
    window_secs: u64,
    samples: Vec<f64>,
    last_emit: Instant,
}

impl HashrateSampler {
    pub fn new(window_secs: u64) -> Self {
        Self {
            window_secs,
            samples: Vec::new(),
            last_emit: Instant::now(),
        }
    }

    /// Add a hashrate sample and return windowed average if window complete
    pub fn add_sample(&mut self, hps: f64) -> Option<f64> {
        self.samples.push(hps);
        let now = Instant::now();

        if now.duration_since(self.last_emit).as_secs() >= self.window_secs {
            let avg = if self.samples.is_empty() {
                0.0
            } else {
                self.samples.iter().copied().sum::<f64>() / self.samples.len() as f64
            };

            self.samples.clear();
            self.last_emit = now;
            Some(avg)
        } else {
            None
        }
    }
}

/// Active miner with worker threads
pub struct ActiveMiner {
    inner: Arc<MinerInner>,
    workers: Mutex<Vec<thread::JoinHandle<()>>>,
}

struct MinerInner {
    // Configuration
    params: VisionXParams,
    target_threads: AtomicU64,
    enabled: AtomicBool,
    batch_size: AtomicU64, // SIMD-friendly batch size

    // Mining state
    current_job: Mutex<Option<MiningJob>>,
    nonce_counter: AtomicU64,

    // VisionX engine (rebuilt when epoch changes)
    engine: Mutex<Arc<VisionXMiner>>,

    // Block building
    block_builder: BlockBuilder,
    submitter: Arc<BlockSubmitter>,
    difficulty_tracker: Mutex<DifficultyTracker>,

    // Statistics
    hash_samples: Mutex<VecDeque<(Instant, u64)>>,
    sample_window: Duration,
    global_hash_counter: AtomicU64, // Total hashes for periodic logging
}

#[derive(Clone)]
struct MiningJob {
    header: BlockHeader,    // Full BlockHeader with all fields
    message_bytes: Vec<u8>, // Pre-computed pow_message_bytes
    pow_job: PowJob,
    started_at: Instant,
    epoch_seed: [u8; 32],         // Seed hash for this epoch's dataset
    epoch: u64,                   // Which epoch this block belongs to
    winner_flag: Arc<AtomicBool>, // Shared flag to stop workers when solution found
    preview_only: bool,           // UI preview mode: build job template but NO hashing
    last_updated_ms: u64,         // Heartbeat timestamp for UI motion
}

impl ActiveMiner {
    pub fn new(
        params: VisionXParams,
        difficulty_config: DifficultyConfig,
        initial_difficulty: u64,
        found_block_callback: Option<
            tokio::sync::mpsc::UnboundedSender<crate::consensus_pow::FoundPowBlock>,
        >,
    ) -> Self {
        let prev_hash = [0u8; 32]; // Genesis
        let epoch = 0;

        let engine = Mutex::new(Arc::new(VisionXMiner::new(params, &prev_hash, epoch)));
        let submitter = Arc::new(BlockSubmitter::new(params, found_block_callback));
        let difficulty_tracker = DifficultyTracker::new(difficulty_config, initial_difficulty);

        let available_cpus = num_cpus::get() as u64;
        let initial_threads = available_cpus;
        
        eprintln!(
            "⚙️  Miner initialized: available_cpus={}, starting_threads={}",
            available_cpus, initial_threads
        );

        let inner = Arc::new(MinerInner {
            params,
            target_threads: AtomicU64::new(initial_threads),
            enabled: AtomicBool::new(false),
            batch_size: AtomicU64::new(4), // Default SIMD batch size
            current_job: Mutex::new(None),
            nonce_counter: AtomicU64::new(0),
            engine,
            block_builder: BlockBuilder::new(),
            submitter,
            difficulty_tracker: Mutex::new(difficulty_tracker),
            hash_samples: Mutex::new(VecDeque::with_capacity(120)),
            sample_window: Duration::from_secs(120),
            global_hash_counter: AtomicU64::new(0),
        });

        Self {
            inner,
            workers: Mutex::new(Vec::new()),
        }
    }

    /// Create an ActiveMiner that uses a minimal dataset so it's cheap to construct
    /// This is intended for test environments where mining is not required.
    pub fn new_disabled(
        params: VisionXParams,
        difficulty_config: DifficultyConfig,
        initial_difficulty: u64,
        found_block_callback: Option<
            tokio::sync::mpsc::UnboundedSender<crate::consensus_pow::FoundPowBlock>,
        >,
    ) -> Self {
        let _prev_hash = [0u8; 32]; // Genesis
        let engine = Mutex::new(Arc::new(VisionXMiner::new_disabled(params)));
        let submitter = Arc::new(BlockSubmitter::new(params, found_block_callback));
        let difficulty_tracker = DifficultyTracker::new(difficulty_config, initial_difficulty);

        let inner = Arc::new(MinerInner {
            params,
            target_threads: AtomicU64::new(0),
            enabled: AtomicBool::new(false),
            batch_size: AtomicU64::new(4), // Default SIMD batch size
            current_job: Mutex::new(None),
            nonce_counter: AtomicU64::new(0),
            engine,
            block_builder: BlockBuilder::new(),
            submitter,
            difficulty_tracker: Mutex::new(difficulty_tracker),
            hash_samples: Mutex::new(VecDeque::with_capacity(120)),
            sample_window: Duration::from_secs(120),
            global_hash_counter: AtomicU64::new(0),
        });

        Self {
            inner,
            workers: Mutex::new(Vec::new()),
        }
    }

    /// Start mining with MinerConfig (CPU-aware, with auto-tuning)
    pub fn start_with_config(&self, cfg: &MinerConfig) {
        // NEW v2.7.0: No blocking gate - miner loop handles eligibility checks
        tracing::info!(
            target: "vision_node::miner",
            "[MINER] ⛏️  Mining enabled with config - miner will start when network conditions allow"
        );

        // Detect CPU info
        let cpu = detect_cpu_summary();
        #[allow(unused_mut)]
        let mut effective_threads = resolve_mining_threads(cfg, &cpu);
        #[allow(unused_mut)]
        let mut effective_batch = resolve_simd_batch_size(cfg);

        // Load performance store and apply auto-tuning if enabled
        #[cfg(feature = "miner-tuning")]
        if cfg.auto_tune_enabled {
            let perf_path = PathBuf::from("vision_data/miner_perf.json");

            match MinerPerfStore::load(perf_path) {
                Ok(perf_store) => {
                    info!(
                        target: "miner::autotune",
                        "Loaded {} historical performance samples",
                        perf_store.len()
                    );

                    // Apply auto-tuning decision
                    let profile = cfg.mining_profile.as_deref().unwrap_or("balanced");

                    if let Some(decision) = auto_tuner::decide_new_tuning(
                        &cpu.model,
                        profile,
                        effective_threads,
                        effective_batch as u32,
                        cfg,
                        &perf_store,
                    ) {
                        info!(
                            target: "miner::autotune",
                            "Auto-tune: applying new settings: threads={} -> {}, batch={} -> {} ({})",
                            effective_threads,
                            decision.new_threads,
                            effective_batch,
                            decision.new_batch,
                            decision.reason
                        );

                        effective_threads = decision.new_threads;
                        effective_batch = decision.new_batch as u64;
                    } else {
                        info!(
                            target: "miner::autotune",
                            "Auto-tune: current settings optimal or exploring"
                        );
                    }
                }
                Err(e) => {
                    info!(
                        target: "miner::autotune",
                        "No performance history found ({}), using defaults",
                        e
                    );
                }
            }
        }

        // Store batch size for workers
        self.inner
            .batch_size
            .store(effective_batch, Ordering::Relaxed);

        // Print beautiful CPU configuration banner
        print_miner_banner(&cpu, effective_threads, effective_batch, cfg);

        self.inner.enabled.store(true, Ordering::Relaxed);
        self.set_threads(effective_threads);

        // Start hashrate logging with performance tracking
        #[cfg(feature = "miner-tuning")]
        self.start_hashrate_logging_with_perf_tracking(cfg, cpu);
    }

    /// Start mining with specified number of threads (legacy method)
    pub fn start(&self, threads: usize) {
        // NEW v2.7.0: No blocking gate - just set enabled_by_user and let miner loop decide eligibility
        // Miner loop will pause/resume based on sync state, not reject the start command
        tracing::info!(
            target: "vision_node::miner",
            "[MINER] ⛏️  Mining enabled by user - miner will start when network conditions allow"
        );

        let available_cpus = num_cpus::get();
        
        // Apply mining profile logic from config
        let actual_threads =
            if let Ok(config) = crate::config::miner::MinerConfig::load_or_create("miner.json") {
                // Check for explicit thread override first
                if let Some(override_threads) = config.mining_threads {
                    if override_threads > 0 {
                        override_threads
                    } else {
                        threads // 0 means use default/auto
                    }
                } else {
                    // Apply profile percentage
                    let cores = num_cpus::get();
                    let profile_threads = match config.mining_profile.as_deref() {
                        Some("laptop") => cores / 2,       // 50% cores
                        Some("balanced") => cores * 3 / 4, // 75% cores
                        Some("beast") => cores,            // 100% cores
                        _ => cores * 3 / 4,                // default to balanced
                    };

                    // Use profile threads if higher priority than request
                    if profile_threads > 0 {
                        profile_threads
                    } else {
                        threads
                    }
                }
            } else {
                threads // Fallback to requested threads if config fails
            };

        // Log thread clamping if it occurred
        if actual_threads != threads {
            eprintln!(
                "⚙️  Mining threads adjusted: requested={}, actual={}, available_cpus={} (reason: config profile/override)",
                threads, actual_threads, available_cpus
            );
        } else if actual_threads > available_cpus {
            eprintln!(
                "⚠️  Warning: Mining with {} threads but only {} CPUs available",
                actual_threads, available_cpus
            );
        } else {
            eprintln!(
                "⚙️  Starting mining: requested={}, actual={}, available_cpus={}",
                threads, actual_threads, available_cpus
            );
        }

        self.inner.enabled.store(true, Ordering::Relaxed);
        self.set_threads(actual_threads);
    }

    /// Stop all mining threads
    pub fn stop(&self) {
        self.inner.enabled.store(false, Ordering::Relaxed);
        self.inner.target_threads.store(0, Ordering::Relaxed);

        // Wait for workers to finish
        let mut workers = self.workers.lock().unwrap();
        while let Some(handle) = workers.pop() {
            let _ = handle.join();
        }
    }

    /// Clear the current mining job (used when sync starts to prevent stale work)
    pub fn clear_job(&self) {
        *self.inner.current_job.lock().unwrap() = None;
    }

    /// Set number of mining threads
    pub fn set_threads(&self, threads: usize) {
        let max_threads = num_cpus::get() * 2;
        let threads = threads.min(max_threads).max(0);

        // Set enabled state based on thread count
        if threads > 0 {
            self.inner.enabled.store(true, Ordering::Relaxed);
        } else {
            self.inner.enabled.store(false, Ordering::Relaxed);
        }

        self.inner
            .target_threads
            .store(threads as u64, Ordering::Relaxed);

        // Adjust worker count
        let mut workers = self.workers.lock().unwrap();

        // Clean up finished workers first
        workers.retain(|h| !h.is_finished());

        // If we need more threads, spawn them starting from 0
        // (old workers will exit when they see worker_id >= target_threads)
        if threads > 0 {
            // Clear out all old workers and start fresh
            workers.clear();

            // Spawn workers with IDs 0 to threads-1
            for worker_id in 0..threads {
                let inner = self.inner.clone();
                let handle = thread::spawn(move || {
                    Self::worker_loop(inner, worker_id);
                });
                workers.push(handle);
            }
            eprintln!("⛏️  Started {} mining threads", threads);
        } else {
            // threads == 0, just clear everything
            workers.clear();
            eprintln!("⏸️  Stopped all mining threads");
        }
    }

    /// Get current thread count
    pub fn get_threads(&self) -> usize {
        self.inner.target_threads.load(Ordering::Relaxed) as usize
    }

    /// Update mining job (call when new block arrives or difficulty changes)
    /// Receives pre-computed message bytes from pow_message_bytes() that include ALL header fields
    pub fn update_job(
        &self,
        message_bytes: Vec<u8>,
        header: BlockHeader,
        prev_hash: [u8; 32],
        _difficulty: u64,
        epoch_seed: [u8; 32],
    ) {
        tracing::info!("[MINER-PARAMS] {}", self.inner.params.fingerprint());
        let local_test_mode = std::env::var("VISION_LOCAL_TEST_MODE").unwrap_or_default() == "true";
        let height = header.number;

        let difficulty = if local_test_mode {
            // LOCAL_TEST_MODE: Force easiest difficulty, bypass tracker
            let test_diff: u64 = std::env::var("VISION_LOCAL_TEST_DIFFICULTY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            tracing::info!(
                height = height,
                test_diff = test_diff,
                "[LOCAL_TEST_MODE] Forcing difficulty, disabling adjustment"
            );
            test_diff
        } else {
            self.inner
                .difficulty_tracker
                .lock()
                .unwrap()
                .current_difficulty()
        };

        // Recalculate target with potentially overridden difficulty
        let target = crate::pow::u256_from_difficulty(difficulty);

        // Calculate epoch
        let epoch = height / self.inner.params.epoch_blocks as u64;

        // Check if we need to rebuild the dataset (epoch changed)
        let current_job = self.inner.current_job.lock().unwrap();
        let needs_rebuild = if let Some(ref job) = *current_job {
            job.epoch != epoch
        } else {
            true // First job
        };
        drop(current_job);

        if needs_rebuild {
            eprintln!(
                "🔄 Rebuilding VisionX dataset for epoch {} with seed {:02x}{:02x}...",
                epoch, epoch_seed[0], epoch_seed[1]
            );

            // Rebuild engine with new epoch seed
            // Note: This is expensive (64MB dataset rebuild) but only happens every ~10 blocks
            let new_engine = Arc::new(VisionXMiner::new(self.inner.params, &epoch_seed, epoch));
            *self.inner.engine.lock().unwrap() = new_engine;
            
            eprintln!(
                "✅ VisionX dataset ready (epoch={}, seed={:02x}{:02x})",
                epoch, epoch_seed[0], epoch_seed[1]
            );
        } else {
            // Reusing cached dataset for same epoch
            tracing::debug!(
                "♻️  Reusing cached VisionX dataset (epoch={})",
                epoch
            );
        }

        // Create PowJob from pre-computed message bytes
        let pow_job = self.inner.block_builder.create_pow_job(
            message_bytes.clone(),
            height,
            prev_hash,
            target,
        );

        // PART 1.1: Diagnostic log for job creation
        let seed0 = &epoch_seed[0..4.min(epoch_seed.len())];
        let target0 = &target[0..8.min(target.len())];
        if local_test_mode {
            tracing::info!(
                height = height,
                difficulty = difficulty,
                target_hex = format!("{:02x?}", target0),
                "[LOCAL_TEST_MODE] Mining job with max target (easy)"
            );
        }
        let chain_tip_height = height.saturating_sub(1);
        let chain_tip_hash = format!("0x{}", hex::encode(prev_hash));
        
        // 🔍 SPLIT BRAIN DIAGNOSTIC: Log DB path and tip for comparison with sync
        let db_path = std::env::var("VISION_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .map(|p| format!("./vision_data_{}", p))
            .unwrap_or_else(|| "./vision_data_7070".to_string());
        
        let blocks_in_memory = {
            let chain = crate::CHAIN.lock();
            chain.blocks.len()
        };
        
        tracing::info!(
            chain_db_path = %db_path,
            chain_tip_height = chain_tip_height,
            chain_tip_hash = %chain_tip_hash,
            job_parent_hash = %chain_tip_hash,
            job_height = height,
            chain_db_scope = crate::vision_constants::VISION_NETWORK_ID,
            blocks_in_memory = blocks_in_memory,
            height = height,
            difficulty = difficulty,
            epoch = epoch,
            seed0 = ?seed0,
            target0 = ?target0,
            message_bytes_len = message_bytes.len(),
            "[MINER-JOB] Created mining job from CHAIN.lock()"
        );

        // Avoid job spam/reset when nothing materially changed
        {
            let prev_hash_hex = format!("0x{}", hex::encode(prev_hash));
            let current_job = self.inner.current_job.lock().unwrap();
            if let Some(ref existing) = *current_job {
                let same_tip = existing.header.number == height
                    && existing.header.parent_hash == prev_hash_hex;
                let same_root = existing.header.tx_root == header.tx_root;
                let same_target = existing.pow_job.target == pow_job.target;

                if !needs_rebuild && same_tip && same_root && same_target {
                    return;
                }
            }
        }

        let should_log = needs_rebuild || {
            let prev_hash_hex = format!("0x{}", hex::encode(prev_hash));
            let current_job = self.inner.current_job.lock().unwrap();
            match *current_job {
                Some(ref j) => j.header.number != height || j.header.parent_hash != prev_hash_hex,
                None => true,
            }
        };

        // Check mining eligibility at job creation time
        let eligible = crate::mining_readiness::is_mining_eligible();
        let preview_only = !eligible;

        if should_log {
            if preview_only {
                eprintln!(
                    "🧊 Miner idle (preview job) height={} — waiting for peers+sync...",
                    height
                );
            } else {
                eprintln!(
                    "🎯 Mining block #{} with epoch_seed={:02x}{:02x}..., epoch={}",
                    height, epoch_seed[0], epoch_seed[1], epoch
                );
            }
        }

        // Update job with fresh winner flag and preview mode
        let job = MiningJob {
            header,
            message_bytes,
            pow_job,
            started_at: Instant::now(),
            epoch_seed,
            epoch,
            winner_flag: Arc::new(AtomicBool::new(false)),
            preview_only,
            last_updated_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        *self.inner.current_job.lock().unwrap() = Some(job);
        self.inner.nonce_counter.store(0, Ordering::Relaxed);

        if should_log {
            // Enhanced logging with full target for debugging
            eprintln!(
                "🎯 New mining job: height={}, difficulty={}, target={}",
                height,
                difficulty,
                hex::encode(target)
            );
            eprintln!(
                "   Target (first 8 bytes): {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                target[0],
                target[1],
                target[2],
                target[3],
                target[4],
                target[5],
                target[6],
                target[7]
            );
        }
    }

    /// Get mining statistics
    pub fn stats(&self) -> MinerSpeed {
        let now = Instant::now();
        let mut samples = self.inner.hash_samples.lock().unwrap();

        // Remove old samples
        let cutoff = now - self.inner.sample_window;
        while let Some((ts, _)) = samples.front() {
            if *ts < cutoff {
                samples.pop_front();
            } else {
                break;
            }
        }

        // Calculate statistics
        let total_hashes: u64 = samples.iter().map(|(_, count)| count).sum();
        let (current_hashrate, average_hashrate) =
            if let (Some((oldest_ts, _)), Some((newest_ts, _))) = (samples.front(), samples.back())
            {
                let duration_secs = newest_ts.duration_since(*oldest_ts).as_secs_f64().max(1.0);
                let avg = total_hashes as f64 / duration_secs;

                // Current hashrate (last 5 seconds)
                let recent_cutoff = now - Duration::from_secs(5);
                let recent_hashes: u64 = samples
                    .iter()
                    .filter(|(ts, _)| *ts >= recent_cutoff)
                    .map(|(_, count)| count)
                    .sum();
                let current = recent_hashes as f64 / 5.0;

                (current, avg)
            } else {
                (0.0, 0.0)
            };

        // Build history
        let mut history = Vec::with_capacity(120);
        for i in (0..120).rev() {
            let bucket_start = now - Duration::from_secs(i + 1);
            let bucket_end = now - Duration::from_secs(i);

            let bucket_hashes: u64 = samples
                .iter()
                .filter(|(ts, _)| *ts >= bucket_start && *ts < bucket_end)
                .map(|(_, count)| count)
                .sum();

            history.push(bucket_hashes as f64);
        }

        MinerSpeed {
            current_hashrate,
            average_hashrate,
            history,
            threads: self.get_threads(),
        }
    }

    /// Get mining configuration
    pub fn config(&self) -> MinerCfg {
        MinerCfg {
            threads: self.get_threads(),
            enabled: self.inner.enabled.load(Ordering::Relaxed),
        }
    }

    /// Check if mining is enabled
    pub fn is_enabled(&self) -> bool {
        self.inner.enabled.load(Ordering::Relaxed)
    }

    /// Get mining stats (blocks found, rewards, etc.)
    pub fn mining_stats(&self) -> crate::consensus_pow::MiningStats {
        self.inner.submitter.stats()
    }

    /// Get mining stats in API response format
    pub fn get_stats(&self) -> crate::routes::miner::MiningStatsResponse {
        let stats = self.mining_stats();

        // Calculate average block time from recent blocks
        let average_block_time = if stats.recent_blocks.len() >= 2 {
            let times: Vec<u64> = stats
                .recent_blocks
                .iter()
                .zip(stats.recent_blocks.iter().skip(1))
                .map(|(prev, curr)| curr.timestamp.saturating_sub(prev.timestamp))
                .collect();

            if !times.is_empty() {
                Some((times.iter().sum::<u64>() as f64) / (times.len() as f64))
            } else {
                None
            }
        } else {
            None
        };

        crate::routes::miner::MiningStatsResponse {
            blocks_found: stats.blocks_found,
            blocks_accepted: stats.blocks_accepted,
            blocks_rejected: stats.blocks_rejected,
            last_block_time: stats.last_block_time,
            last_block_height: stats.last_block_height,
            total_rewards: stats.total_rewards,
            average_block_time,
        }
    }

    /// Start periodic hashrate logging with performance tracking
    #[cfg(feature = "miner-tuning")]
    fn start_hashrate_logging_with_perf_tracking(&self, cfg: &MinerConfig, cpu: CpuSummary) {
        let inner = self.inner.clone();
        let config = cfg.clone();
        let cpu_model = cpu.model.clone();

        thread::spawn(move || {
            let mut last_count = 0u64;
            let mut last_instant = Instant::now();
            let mut sampler = HashrateSampler::new(config.evaluation_window_secs);

            // Load performance store
            let perf_path = PathBuf::from("vision_data/miner_perf.json");
            let mut perf_store = MinerPerfStore::load(perf_path.clone()).unwrap_or_else(|_| {
                MinerPerfStore::load(perf_path.clone()).expect("Failed to create perf store")
            });

            loop {
                thread::sleep(Duration::from_secs(1));

                // Check if mining is still enabled
                if !inner.enabled.load(Ordering::Relaxed) {
                    break;
                }

                let total = inner.global_hash_counter.load(Ordering::Relaxed);
                let delta = total.saturating_sub(last_count);
                last_count = total;

                let elapsed = last_instant.elapsed().as_secs_f64();
                last_instant = Instant::now();

                let hps = (delta as f64) / elapsed.max(0.000_001);

                // Log hashrate every second
                if hps > 0.0 {
                    info!(
                        target: "miner",
                        "Hashrate ≈ {:.2} H/s",
                        hps
                    );

                    // Add sample to windowed average
                    if let Some(avg_window_hashrate) = sampler.add_sample(hps) {
                        // Record performance sample
                        let key = PerfKey {
                            cpu_model: cpu_model.clone(),
                            profile: config
                                .mining_profile
                                .as_deref()
                                .unwrap_or("balanced")
                                .to_string(),
                            pow_algo: "vision-pow-v1".to_string(), // Current PoW algorithm
                            threads: inner.target_threads.load(Ordering::Relaxed) as usize,
                            batch_size: inner.batch_size.load(Ordering::Relaxed) as u32,
                        };

                        let now_ts = chrono::Utc::now().timestamp();
                        perf_store.record_sample(&key, avg_window_hashrate, now_ts);

                        // Save periodically
                        if let Err(e) = perf_store.save() {
                            tracing::warn!(
                                target: "miner::perf",
                                "Failed to save performance data: {}",
                                e
                            );
                        } else {
                            info!(
                                target: "miner::perf",
                                "Recorded avg {:.1} H/s for {} threads × batch {} (window: {}s)",
                                avg_window_hashrate,
                                key.threads,
                                key.batch_size,
                                config.evaluation_window_secs
                            );
                        }
                    }
                }
            }
        });
    }

    /// Worker thread main loop
    fn worker_loop(inner: Arc<MinerInner>, worker_id: usize) {
        eprintln!("⛏️  Worker #{} started", worker_id);

        // Fix C: Rate-limited logging for mining gate blocks
        use std::sync::Mutex;
        use std::time::Instant;
        lazy_static::lazy_static! {
            static ref LAST_BLOCK_LOG: Mutex<Option<Instant>> = Mutex::new(None);
        }

        loop {
            // Check if we should exit
            let target_threads = inner.target_threads.load(Ordering::Relaxed) as usize;
            if !inner.enabled.load(Ordering::Relaxed) || worker_id >= target_threads {
                eprintln!("⏸️  Worker #{} stopping", worker_id);
                break;
            }

            // NEW v2.7.0: Check mining eligibility before proceeding
            // If not eligible (unsynced, insufficient peers), pause instead of blocking
            let eligible = crate::mining_readiness::is_mining_eligible();
            if !eligible {
                // Fix C: Log descriptive blocking reason (rate-limited to every 10s)
                if worker_id == 0 {
                    let should_log = {
                        let mut last = LAST_BLOCK_LOG.lock().unwrap();
                        if let Some(last_instant) = *last {
                            if last_instant.elapsed().as_secs() >= 10 {
                                *last = Some(Instant::now());
                                true
                            } else {
                                false
                            }
                        } else {
                            *last = Some(Instant::now());
                            true
                        }
                    };

                    if should_log {
                        // NON-BLOCKING: Just show basic status without network queries
                        let local_height = {
                            let chain = crate::CHAIN.lock();
                            chain.blocks.len() as u64
                        };
                        eprintln!(
                            "[MINING GATE] ⏸️ Mining paused: waiting for eligibility (height={})",
                            local_height
                        );
                    }
                }

                // Paused state: sleep briefly and recheck eligibility
                // This allows /api/miner/start to return immediately while miner waits for sync
                thread::sleep(Duration::from_secs(2));
                continue;
            }

            // Get current job
            let job = {
                let job_lock = inner.current_job.lock().unwrap();
                job_lock.clone()
            };

            if let Some(job) = job {
                // Preview mode: UI gets job data but worker does NOT hash
                if job.preview_only {
                    if worker_id == 0 {
                        // Rate-limited preview status (every 5s)
                        static PREVIEW_LOG: std::sync::Mutex<Option<Instant>> =
                            std::sync::Mutex::new(None);
                        let should_log = {
                            let mut last = PREVIEW_LOG.lock().unwrap();
                            if let Some(last_instant) = *last {
                                if last_instant.elapsed().as_secs() >= 5 {
                                    *last = Some(Instant::now());
                                    true
                                } else {
                                    false
                                }
                            } else {
                                *last = Some(Instant::now());
                                true
                            }
                        };
                        if should_log {
                            eprintln!("🧊 Miner idle (preview): height={}, target loaded, waiting for eligibility...", job.header.number);
                        }
                    }
                    // Sleep and recheck (keep job refresh cadence)
                    thread::sleep(Duration::from_secs(2));
                    continue;
                }

                // PART 1.2: Diagnostic log when worker receives job (rate-limited)
                if worker_id == 0 {
                    static JOB_LOG_STATE: std::sync::Mutex<Option<(u64, Instant)>> =
                        std::sync::Mutex::new(None);
                    let should_log = {
                        let mut state = JOB_LOG_STATE.lock().unwrap();
                        let should = match *state {
                            Some((last_height, last_time)) => {
                                job.header.number != last_height
                                    || last_time.elapsed().as_secs() >= 5
                            }
                            None => true,
                        };
                        if should {
                            *state = Some((job.header.number, Instant::now()));
                        }
                        should
                    };
                    if should_log {
                        tracing::debug!(
                            worker = worker_id,
                            height = job.header.number,
                            difficulty = job.header.difficulty,
                            "[MINER] Job received by worker"
                        );
                    }
                }
                // Check if another worker already found a solution
                if job.winner_flag.load(Ordering::Relaxed) {
                    // Another worker won, skip this batch
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }

                // Get configurable batch size (SIMD-friendly)
                let batch_size = inner.batch_size.load(Ordering::Relaxed);

                // Get nonce range for this batch
                let start_nonce = inner.nonce_counter.fetch_add(batch_size, Ordering::Relaxed);

                // Get current engine (may change when epoch changes)
                let engine = inner.engine.lock().unwrap().clone();

                // Mine batch with configurable size
                let (solutions, hashes_done) =
                    engine.mine_batch(&job.pow_job, start_nonce, batch_size as u32);

                // Update global hash counter for periodic logging
                inner
                    .global_hash_counter
                    .fetch_add(hashes_done as u64, Ordering::Relaxed);

                // Record hashes for statistics
                {
                    let mut samples = inner.hash_samples.lock().unwrap();
                    samples.push_back((Instant::now(), hashes_done as u64));
                }

                // If solution found, submit block
                if let Some(solution) = solutions.first() {
                    // PART 1.3: Diagnostic log when solution found
                    let digest0 = &solution.digest[0..4.min(solution.digest.len())];
                    let target0 = &job.pow_job.target[0..4.min(job.pow_job.target.len())];
                    tracing::info!(
                        worker = worker_id,
                        height = job.header.number,
                        nonce = solution.nonce,
                        digest0 = ?digest0,
                        target0 = ?target0,
                        "[MINER-FOUND] Solution found, digest <= target"
                    );

                    // Use compare-and-swap to ensure only first winner submits
                    if job.winner_flag.swap(true, Ordering::SeqCst) {
                        // Another worker already won, skip
                        continue;
                    }

                    // Create FoundPowBlock with full header and solution
                    let mut finalized_header = job.header.clone();
                    eprintln!("[MINER-SOLUTION] Worker {} found solution! solution.nonce={}, solution.digest={}", worker_id, solution.nonce, hex::encode(solution.digest));
                    finalized_header.nonce = solution.nonce;
                    finalized_header.pow_hash = format!("0x{}", hex::encode(solution.digest));

                    // DIAGNOSTIC: Log the pow_hash we're setting AND compute pow_message_bytes
                    tracing::info!(
                        "[MINER-POW-HASH] Set pow_hash={} from digest for height={}",
                        finalized_header.pow_hash,
                        finalized_header.number
                    );
                    
                    // DIAGNOSTIC: Compute pow_message_bytes with nonce=0 to verify encoding
                    // The actual mining was done with nonce=0 in the message, so log it that way too
                    let mut header_for_diagnostic = finalized_header.clone();
                    header_for_diagnostic.nonce = 0;  // Use nonce=0 to match what was actually hashed
                    
                    if let Ok(miner_pow_msg) = crate::consensus_pow::pow_message_bytes(&header_for_diagnostic) {
                        eprintln!("[MINER-POW-MSG] Block #{} pow_message_bytes:", finalized_header.number);
                        eprintln!("  parent_hash: {}", finalized_header.parent_hash);
                        eprintln!("  number: {}", finalized_header.number);
                        eprintln!("  timestamp: {}", finalized_header.timestamp);
                        eprintln!("  difficulty: {}", finalized_header.difficulty);
                        eprintln!("  nonce: {} (will be passed separately to visionx_hash)", finalized_header.nonce);
                        eprintln!("  tx_root: {}", finalized_header.tx_root);
                        eprintln!("  pow_msg length: {} bytes (with nonce=0 in message)", miner_pow_msg.len());
                        eprintln!("  pow_msg (first 64 bytes): {}", hex::encode(&miner_pow_msg[..64.min(miner_pow_msg.len())]));
                    }

                    let found_block = crate::consensus_pow::FoundPowBlock {
                        header: finalized_header.clone(),
                        digest: solution.digest, // Already [u8; 32]
                        nonce: solution.nonce,
                    };

                    // PART 1.4: Diagnostic log when submitting block
                    let pow_hash = hex::encode(solution.digest);
                    let pow8 = &pow_hash[0..8.min(pow_hash.len())];
                    tracing::info!(
                        worker = worker_id,
                        height = finalized_header.number,
                        nonce = solution.nonce,
                        pow8 = pow8,
                        "[MINER-SUBMIT] Submitting finalized block"
                    );

                    // Use legacy submitter for stats tracking
                    let result = inner.submitter.submit_block_from_found(
                        &found_block,
                        job.pow_job.target,
                        job.epoch_seed,
                    );

                    match result {
                        crate::consensus_pow::SubmitResult::Accepted { height, hash } => {
                            let new_difficulty = inner
                                .difficulty_tracker
                                .lock()
                                .unwrap()
                                .current_difficulty();
                            eprintln!(
                                "🎉 Worker #{} found block #{}! Hash: {}",
                                worker_id,
                                height,
                                hex::encode(hash)
                            );
                            eprintln!("   Digest: {}", hex::encode(solution.digest));
                            eprintln!("   Target: {}", hex::encode(job.pow_job.target));
                            eprintln!(
                                "   Difficulty updated: {} -> {}",
                                finalized_header.difficulty, new_difficulty
                            );

                            // Record block time for difficulty adjustment (skip in LOCAL_TEST_MODE)
                            let local_test = std::env::var("VISION_LOCAL_TEST_MODE")
                                .unwrap_or_default()
                                == "true";
                            if !local_test {
                                let timestamp = finalized_header.timestamp;
                                inner
                                    .difficulty_tracker
                                    .lock()
                                    .unwrap()
                                    .record_block(timestamp);
                            }

                            // Clear current job (will get new one from coordinator)
                            *inner.current_job.lock().unwrap() = None;
                        }
                        crate::consensus_pow::SubmitResult::Rejected { reason } => {
                            eprintln!("❌ Worker #{} block rejected: {}", worker_id, reason);
                            eprintln!("   Digest: {}", hex::encode(solution.digest));
                            eprintln!("   Target: {}", hex::encode(job.pow_job.target));
                            eprintln!(
                                "   Height: {}, Difficulty: {}",
                                job.header.number, job.header.difficulty
                            );
                        }
                        crate::consensus_pow::SubmitResult::Duplicate => {
                            eprintln!("⚠️  Worker #{} found duplicate block", worker_id);
                        }
                    }
                }
            } else {
                // No job available, sleep briefly
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

impl Drop for ActiveMiner {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Print beautiful miner configuration banner
fn print_miner_banner(cpu: &CpuSummary, threads: usize, batch: u64, _cfg: &MinerConfig) {
    use sysinfo::System;
    use tracing::info;

    // Helper to truncate long strings
    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    // Detect total RAM using sysinfo
    let sys = System::new_all();
    let total_ram_mb = (sys.total_memory() / 1024 / 1024) as usize;

    info!(
        target: "vision_node::miner",
        "╔════════════════════════════════════════════════════════════════╗"
    );
    info!(
        target: "vision_node::miner",
        "║        VISION NODE MINER - CONFIGURATION DETECTED              ║"
    );
    info!(
        target: "vision_node::miner",
        "╠════════════════════════════════════════════════════════════════╣"
    );
    info!(
        target: "vision_node::miner",
        "║ CPU Model:       {:<45} ║",
        truncate(&cpu.model, 45)
    );
    info!(
        target: "vision_node::miner",
        "║ Logical Cores:   {:<45} ║",
        cpu.logical_cores
    );
    info!(
        target: "vision_node::miner",
        "║ Mining Threads:  {:<45} ║",
        threads
    );
    info!(
        target: "vision_node::miner",
        "║ Batch Size:      {:<45} ║",
        batch
    );
    info!(
        target: "vision_node::miner",
        "║ Total RAM:       {:<45} ║",
        format!("{} MB", total_ram_mb)
    );
    info!(
        target: "vision_node::miner",
        "║ Core Affinity:   {:<45} ║",
        "Enabled"  // Always enabled in current implementation
    );
    info!(
        target: "vision_node::miner",
        "╠════════════════════════════════════════════════════════════════╣"
    );

    // Check for high-core profile (16+ cores)
    if cpu.logical_cores >= 16 {
        info!(
            target: "vision_node::miner",
            "║ ⚡ HIGH-CORE PROFILE ENABLED                                   ║"
        );
        info!(
            target: "vision_node::miner",
            "║ 🚀 Detected monster rig! Engaging Threadripper optimizations. ║"
        );
        info!(
            target: "vision_node::miner",
            "║ 💫 Thank you for powering the Vision constellation.           ║"
        );
        info!(
            target: "vision_node::miner",
            "╠════════════════════════════════════════════════════════════════╣"
        );
    }

    info!(
        target: "vision_node::miner",
        "╚════════════════════════════════════════════════════════════════╝"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_miner_creation() {
        let params = VisionXParams::default();
        let difficulty_config = DifficultyConfig::default();
        let miner = ActiveMiner::new(params, difficulty_config, 10000, None);

        assert_eq!(miner.get_threads(), num_cpus::get());
        assert!(!miner.config().enabled);
    }

    #[test]
    fn test_thread_adjustment() {
        let params = VisionXParams::default();
        let difficulty_config = DifficultyConfig::default();
        let miner = ActiveMiner::new(params, difficulty_config, 10000, None);

        miner.set_threads(4);
        assert_eq!(miner.get_threads(), 4);

        miner.set_threads(0);
        assert_eq!(miner.get_threads(), 0);
    }
}
