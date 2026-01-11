#![allow(dead_code)]
// Pool worker client - handles JoinPool mode mining
//
// When a node is in JoinPool mode, this module:
// 1. Registers with the pool host
// 2. Fetches mining jobs from the pool
// 3. Mines locally with the provided job parameters
// 4. Submits shares (and blocks) back to the pool

use crate::pool::protocol::{PoolJob, RegistrationRequest, RegistrationResponse, ShareSubmission};
use crate::pow::visionx::{VisionXMiner, VisionXParams};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BATCH_SIZE: u32 = 10_000; // Nonces to try per batch
const JOB_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

/// Pool worker that mines for a remote pool
pub struct PoolWorker {
    inner: Arc<PoolWorkerInner>,
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
}

struct PoolWorkerInner {
    pool_url: String,
    worker_id: String,
    worker_name: Option<String>,
    wallet_address: String,
    enabled: AtomicBool,
    target_threads: AtomicU64,
    current_job: Mutex<Option<ActivePoolJob>>,
    params: VisionXParams,
    engine: Mutex<Option<Arc<VisionXMiner>>>,
    total_shares_submitted: AtomicU64,
    last_share_time: Mutex<Option<Instant>>,
}

#[derive(Clone)]
struct ActivePoolJob {
    job: PoolJob,
    fetched_at: Instant,
    nonce_counter: Arc<AtomicU64>,
}

impl PoolWorker {
    /// Create a new pool worker
    pub fn new(
        pool_url: String,
        wallet_address: String,
        worker_id: String,
        worker_name: Option<String>,
    ) -> Self {
        // ⚠️ FORK-CRITICAL: Pool workers MUST use consensus params
        // Using VisionXParams::default() would cause fork if params differ from network!
        let params = crate::consensus_params_to_visionx(&crate::VISIONX_CONSENSUS_PARAMS);

        let inner = Arc::new(PoolWorkerInner {
            pool_url,
            worker_id,
            worker_name,
            wallet_address,
            enabled: AtomicBool::new(false),
            target_threads: AtomicU64::new(0),
            current_job: Mutex::new(None),
            params,
            engine: Mutex::new(None),
            total_shares_submitted: AtomicU64::new(0),
            last_share_time: Mutex::new(None),
        });

        Self {
            inner,
            threads: Mutex::new(Vec::new()),
        }
    }

    /// Start mining as a pool worker
    pub fn start(&self, threads: usize) -> Result<(), String> {
        // Register with pool
        self.register_with_pool()?;

        // Start mining threads
        self.inner.enabled.store(true, Ordering::Relaxed);
        self.set_threads(threads);

        // Start job fetcher thread
        let inner = self.inner.clone();
        let fetcher_handle = thread::spawn(move || {
            Self::job_fetcher_loop(inner);
        });

        self.threads.lock().unwrap().push(fetcher_handle);

        Ok(())
    }

    /// Stop all mining threads
    pub fn stop(&self) {
        self.inner.enabled.store(false, Ordering::Relaxed);
        self.inner.target_threads.store(0, Ordering::Relaxed);

        // Wait for all threads to finish
        let mut threads = self.threads.lock().unwrap();
        while let Some(handle) = threads.pop() {
            let _ = handle.join();
        }
    }

    /// Set number of mining threads
    pub fn set_threads(&self, threads: usize) {
        let max_threads = num_cpus::get() * 2;
        let threads = threads.min(max_threads);

        self.inner
            .target_threads
            .store(threads as u64, Ordering::Relaxed);

        let mut thread_handles = self.threads.lock().unwrap();

        // Clean up finished threads
        thread_handles.retain(|h| !h.is_finished());

        // Count current worker threads (excluding job fetcher)
        let current_workers = thread_handles
            .iter()
            .filter(|_| true)
            .count()
            .saturating_sub(1);

        if threads > current_workers {
            // Spawn more workers
            for worker_id in current_workers..threads {
                let inner = self.inner.clone();
                let handle = thread::spawn(move || {
                    Self::worker_loop(inner, worker_id);
                });
                thread_handles.push(handle);
            }
        }
    }

    /// Get total shares submitted
    pub fn shares_submitted(&self) -> u64 {
        self.inner.total_shares_submitted.load(Ordering::Relaxed)
    }

    /// Register with pool
    fn register_with_pool(&self) -> Result<(), String> {
        let url = format!(
            "{}/api/pool/register",
            self.inner.pool_url.trim_end_matches('/')
        );

        let req = RegistrationRequest {
            worker_id: self.inner.worker_id.clone(),
            wallet_address: self.inner.wallet_address.clone(),
            worker_name: self.inner.worker_name.clone(),
            version: Some(crate::vision_constants::VISION_VERSION.to_string()),
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&url)
            .json(&req)
            .timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| format!("Failed to register with pool: {}", e))?;

        let reg_response: RegistrationResponse = response
            .json()
            .map_err(|e| format!("Failed to parse registration response: {}", e))?;

        if !reg_response.ok {
            return Err(format!(
                "Pool registration failed: {:?}",
                reg_response.message
            ));
        }

        eprintln!("✅ Registered with pool: {}", self.inner.pool_url);
        eprintln!("   Worker ID: {}", self.inner.worker_id);
        eprintln!(
            "   Foundation Fee: {}%",
            reg_response.foundation_fee_bps as f64 / 100.0
        );
        eprintln!("   Pool Fee: {}%", reg_response.pool_fee_bps as f64 / 100.0);

        Ok(())
    }

    /// Job fetcher thread - periodically fetches new jobs from pool
    fn job_fetcher_loop(inner: Arc<PoolWorkerInner>) {
        eprintln!("🔄 Pool job fetcher started");

        while inner.enabled.load(Ordering::Relaxed) {
            // Fetch new job
            match Self::fetch_job(&inner) {
                Ok(job) => {
                    let height = job.height;
                    let job_id = job.job_id.clone();

                    // Calculate epoch
                    let epoch = height / inner.params.epoch_blocks as u64;

                    // Parse epoch seed from prev_hash
                    let mut epoch_seed = [0u8; 32];
                    if let Ok(bytes) = hex::decode(job.prev_hash.trim_start_matches("0x")) {
                        epoch_seed.copy_from_slice(&bytes[0..32.min(bytes.len())]);
                    }

                    // Rebuild engine if epoch changed
                    let mut engine_guard = inner.engine.lock().unwrap();
                    let needs_rebuild = engine_guard.is_none() || {
                        // Check if epoch changed by comparing with current job
                        let current_job = inner.current_job.lock().unwrap();
                        current_job.as_ref().is_none_or(|j| {
                            j.job.height / inner.params.epoch_blocks as u64 != epoch
                        })
                    };

                    if needs_rebuild {
                        eprintln!("🔄 Rebuilding VisionX dataset for epoch {}...", epoch);
                        *engine_guard = Some(Arc::new(VisionXMiner::new(
                            inner.params,
                            &epoch_seed,
                            epoch,
                        )));
                    }
                    drop(engine_guard);

                    // Update current job
                    let active_job = ActivePoolJob {
                        job,
                        fetched_at: Instant::now(),
                        nonce_counter: Arc::new(AtomicU64::new(0)),
                    };

                    *inner.current_job.lock().unwrap() = Some(active_job);

                    eprintln!("📋 New pool job: height={}, job_id={}", height, job_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch pool job: {}", e);
                }
            }

            // Wait before fetching next job
            thread::sleep(JOB_REFRESH_INTERVAL);
        }

        eprintln!("🛑 Pool job fetcher stopped");
    }

    /// Fetch job from pool
    fn fetch_job(inner: &PoolWorkerInner) -> Result<PoolJob, String> {
        let url = format!(
            "{}/pool/job?worker_id={}",
            inner.pool_url.trim_end_matches('/'),
            inner.worker_id
        );

        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| format!("Failed to fetch job: {}", e))?;

        let job: PoolJob = response
            .json()
            .map_err(|e| format!("Failed to parse job: {}", e))?;

        Ok(job)
    }

    /// Worker thread main loop
    fn worker_loop(inner: Arc<PoolWorkerInner>, worker_id: usize) {
        eprintln!("⛏️  Pool worker #{} started", worker_id);

        loop {
            // Check if we should exit
            let target_threads = inner.target_threads.load(Ordering::Relaxed) as usize;
            if !inner.enabled.load(Ordering::Relaxed) || worker_id >= target_threads {
                eprintln!("⏸️  Pool worker #{} stopping", worker_id);
                break;
            }

            // Get current job
            let job = {
                let job_lock = inner.current_job.lock().unwrap();
                job_lock.clone()
            };

            if let Some(active_job) = job {
                // Check if job is stale (>60 seconds old)
                if active_job.fetched_at.elapsed() > Duration::from_secs(60) {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // Get nonce range for this batch
                let start_nonce = active_job
                    .nonce_counter
                    .fetch_add(BATCH_SIZE as u64, Ordering::Relaxed);

                // Ensure nonce is within assigned range
                if start_nonce < active_job.job.extra_nonce_start as u64 {
                    active_job
                        .nonce_counter
                        .store(active_job.job.extra_nonce_start as u64, Ordering::Relaxed);
                    continue;
                }

                if start_nonce >= active_job.job.extra_nonce_end as u64 {
                    // Exhausted nonce range, wait for new job
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }

                // Get engine
                let engine = {
                    let engine_guard = inner.engine.lock().unwrap();
                    engine_guard.clone()
                };

                let Some(_engine) = engine else {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                };

                // Parse targets
                let share_target = Self::parse_target(&active_job.job.share_target);
                let network_target = Self::parse_target(&active_job.job.target);

                // Parse prev_hash for mining
                let prev_hash = Self::parse_hash(&active_job.job.prev_hash);

                // Mine batch
                for nonce in start_nonce..(start_nonce + BATCH_SIZE as u64) {
                    if nonce >= active_job.job.extra_nonce_end as u64 {
                        break;
                    }

                    // Build header
                    let mut header_bytes = Vec::new();
                    header_bytes.extend_from_slice(&active_job.job.height.to_le_bytes());
                    header_bytes.extend_from_slice(&prev_hash);
                    header_bytes.extend_from_slice(&nonce.to_le_bytes());

                    // Hash with VisionX
                    let hash = blake3::hash(&header_bytes);
                    let hash_bytes = hash.as_bytes();

                    // Check against share target
                    if Self::meets_target(hash_bytes, &share_target) {
                        let is_block = Self::meets_target(hash_bytes, &network_target);

                        // Submit share
                        if let Err(e) = Self::submit_share(
                            &inner,
                            &active_job.job,
                            nonce,
                            nonce as u32,
                            hash_bytes,
                            is_block,
                        ) {
                            eprintln!("❌ Failed to submit share: {}", e);
                        } else {
                            inner.total_shares_submitted.fetch_add(1, Ordering::Relaxed);
                            *inner.last_share_time.lock().unwrap() = Some(Instant::now());

                            if is_block {
                                eprintln!("🎉 Worker #{} found BLOCK!", worker_id);
                            } else {
                                eprintln!("✅ Worker #{} submitted share", worker_id);
                            }
                        }
                    }
                }
            } else {
                // No job available, sleep briefly
                thread::sleep(Duration::from_millis(500));
            }
        }
    }

    /// Submit share to pool
    fn submit_share(
        inner: &PoolWorkerInner,
        job: &PoolJob,
        nonce: u64,
        extra_nonce: u32,
        hash: &[u8],
        _is_block: bool,
    ) -> Result<(), String> {
        let url = format!("{}/pool/share", inner.pool_url.trim_end_matches('/'));

        let submission = ShareSubmission {
            worker_id: inner.worker_id.clone(),
            wallet_address: inner.wallet_address.clone(),
            job_id: job.job_id.clone(),
            nonce,
            extra_nonce,
            hash: format!("0x{}", hex::encode(hash)),
            hashrate: None, // TODO: Calculate and report
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&url)
            .json(&submission)
            .timeout(Duration::from_secs(10))
            .send()
            .map_err(|e| format!("Failed to submit share: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Share rejected: {}", response.status()));
        }

        Ok(())
    }

    /// Parse target from hex string
    fn parse_target(hex_str: &str) -> [u8; 32] {
        let mut target = [0u8; 32];
        if let Ok(bytes) = hex::decode(hex_str.trim_start_matches("0x")) {
            let len = bytes.len().min(32);
            target[..len].copy_from_slice(&bytes[..len]);
        }
        target
    }

    /// Parse hash from hex string
    fn parse_hash(hex_str: &str) -> [u8; 32] {
        let mut hash = [0u8; 32];
        if let Ok(bytes) = hex::decode(hex_str.trim_start_matches("0x")) {
            let len = bytes.len().min(32);
            hash[..len].copy_from_slice(&bytes[..len]);
        }
        hash
    }

    /// Check if hash meets target
    fn meets_target(hash: &[u8], target: &[u8; 32]) -> bool {
        for i in 0..32 {
            if hash[i] < target[i] {
                return true;
            } else if hash[i] > target[i] {
                return false;
            }
        }
        true // Equal
    }
}

impl Drop for PoolWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
