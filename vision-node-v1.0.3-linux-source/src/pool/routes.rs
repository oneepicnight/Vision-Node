//! HTTP endpoint handlers for mining pool functionality

use axum::{
    extract::{Json, Query},
    http::StatusCode,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::HashMap;

use crate::pool::{protocol::*, MiningMode, PoolConfig, PoolState};

/// Generate pool URL for joining miners
/// Attempts to detect public IP, falls back to localhost
fn generate_pool_url(port: u16) -> String {
    // Try to get local network IP
    if let Ok(hostname) = hostname::get() {
        if let Some(hostname_str) = hostname.to_str() {
            return format!("http://{}:{}", hostname_str, port);
        }
    }

    // Fallback to localhost
    format!("http://localhost:{}", port)
}

/// Global pool state (only active when in HostPool mode)
pub static POOL_STATE: Lazy<Mutex<Option<PoolState>>> = Lazy::new(|| Mutex::new(None));

/// Global mining mode configuration
pub static MINING_MODE: Lazy<Mutex<MiningMode>> = Lazy::new(|| Mutex::new(MiningMode::Solo));

/// Global pool config
pub static POOL_CONFIG: Lazy<Mutex<PoolConfig>> = Lazy::new(|| Mutex::new(PoolConfig::default()));

/// POST /pool/register - Register a worker to the pool
pub async fn pool_register(
    Json(req): Json<RegistrationRequest>,
) -> Result<Json<RegistrationResponse>, (StatusCode, String)> {
    // Check if we're hosting a pool
    let mode = *MINING_MODE.lock();
    if mode != MiningMode::HostPool {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "This node is not hosting a pool".to_string(),
        ));
    }

    // Get pool state
    let pool_guard = POOL_STATE.lock();
    let pool = pool_guard.as_ref().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Pool not initialized".to_string(),
    ))?;

    // Register worker
    let worker_name_display = req
        .worker_name
        .as_ref()
        .map(|n| format!(" ({})", n))
        .unwrap_or_default();

    match pool.register_worker(
        req.worker_id.clone(),
        req.wallet_address,
        req.worker_name.clone(),
    ) {
        Ok(()) => {
            tracing::info!(
                "✅ Worker {}{} registered to pool",
                req.worker_id,
                worker_name_display
            );
            Ok(Json(RegistrationResponse {
                ok: true,
                message: Some("Worker registered successfully".to_string()),
                pool_fee_bps: pool.config.pool_fee_bps,
                foundation_fee_bps: pool.config.foundation_fee_bps,
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e)),
    }
}

/// GET /pool/job - Get current mining job
pub async fn pool_get_job(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PoolJob>, (StatusCode, String)> {
    let job_start = std::time::Instant::now();

    // Check if we're hosting a pool
    let mode = *MINING_MODE.lock();
    if mode != MiningMode::HostPool {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "This node is not hosting a pool".to_string(),
        ));
    }

    let worker_id = params.get("worker_id").ok_or((
        StatusCode::BAD_REQUEST,
        "Missing worker_id parameter".to_string(),
    ))?;

    // Get current chain state to build job
    let g = crate::CHAIN.lock();
    let last_block = g.blocks.last().unwrap();
    let height = last_block.header.number + 1;
    let prev_hash = last_block.header.pow_hash.clone();
    let difficulty = g.difficulty;
    drop(g);

    // Check cache first
    if let Some(cached_job) = crate::pool::JOB_CACHE.get(height) {
        // Update metrics
        {
            let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
            let response_ms = job_start.elapsed().as_millis() as f64;
            if metrics.avg_job_response_time_ms == 0.0 {
                metrics.avg_job_response_time_ms = response_ms;
            } else {
                metrics.avg_job_response_time_ms =
                    (metrics.avg_job_response_time_ms * 0.9) + (response_ms * 0.1);
            }
        }

        return Ok(Json(cached_job));
    }

    // Build new job (cache miss)
    let g = crate::CHAIN.lock();

    // Calculate target from difficulty
    let target = if difficulty > 0 {
        let mut t = [0xFFu8; 32];
        let max = u128::MAX;
        let target_val = max / difficulty as u128;
        let bytes = target_val.to_be_bytes();
        t[16..].copy_from_slice(&bytes);
        t
    } else {
        [0xFFu8; 32]
    };
    drop(g);

    // Get pool config for share difficulty
    let pool_state = POOL_STATE.lock();
    let pool = pool_state.as_ref().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Pool not initialized".to_string(),
    ))?;

    let share_divisor = pool.config.share_difficulty_divisor;
    let share_difficulty = difficulty / share_divisor.max(1);

    // Calculate share target (easier than full block)
    let share_target = if share_difficulty > 0 {
        let mut target_bytes = [0xFFu8; 32];
        let max = u128::MAX;
        let target_val = max / share_difficulty as u128;
        let bytes = target_val.to_be_bytes();
        target_bytes[16..].copy_from_slice(&bytes);
        hex::encode(target_bytes)
    } else {
        hex::encode([0xFFu8; 32])
    };

    // Generate job ID
    let job_id = format!("{}-{}", height, chrono::Utc::now().timestamp());

    // Assign extra nonce range based on worker (simple scheme: use hash of worker_id)
    let worker_hash = blake3::hash(worker_id.as_bytes());
    let extra_nonce_start = u32::from_be_bytes([
        worker_hash.as_bytes()[0],
        worker_hash.as_bytes()[1],
        worker_hash.as_bytes()[2],
        worker_hash.as_bytes()[3],
    ]);
    let extra_nonce_end = extra_nonce_start.wrapping_add(100_000_000); // 100M range per worker

    // Build merkle root placeholder
    let merkle_root =
        "0x0000000000000000000000000000000000000000000000000000000000000000".to_string();

    let job = PoolJob {
        job_id: job_id.clone(),
        height,
        prev_hash,
        merkle_root,
        target: hex::encode(target),
        share_target,
        extra_nonce_start,
        extra_nonce_end,
        difficulty,
    };

    // Cache the job for future requests
    crate::pool::JOB_CACHE.set(height, job.clone());

    // Update active job ID
    drop(pool_state);
    let mut pool_state = POOL_STATE.lock();
    if let Some(ref mut pool) = *pool_state {
        pool.active_job_id = Some(job_id);
    }

    // Update metrics
    {
        let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
        let response_ms = job_start.elapsed().as_millis() as f64;
        if metrics.avg_job_response_time_ms == 0.0 {
            metrics.avg_job_response_time_ms = response_ms;
        } else {
            metrics.avg_job_response_time_ms =
                (metrics.avg_job_response_time_ms * 0.9) + (response_ms * 0.1);
        }
    }

    Ok(Json(job))
}

/// POST /pool/share - Submit a share or block solution
pub async fn pool_submit_share(
    Json(submission): Json<ShareSubmission>,
) -> Result<Json<ShareResponse>, (StatusCode, String)> {
    let submit_start = std::time::Instant::now();

    // Check if we're hosting a pool
    let mode = *MINING_MODE.lock();
    if mode != MiningMode::HostPool {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "This node is not hosting a pool".to_string(),
        ));
    }

    // Check if worker is banned
    if crate::pool::BAN_MANAGER.is_banned(&submission.worker_id) {
        return Err((
            StatusCode::FORBIDDEN,
            format!(
                "Worker {} is banned for excessive invalid shares",
                submission.worker_id
            ),
        ));
    }

    // Rate limiting
    if !crate::pool::SHARE_RATE_LIMITER.allow_share(&submission.worker_id) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded. Please slow down share submissions.".to_string(),
        ));
    }

    let pool_guard = POOL_STATE.lock();
    let pool = pool_guard.as_ref().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Pool not initialized".to_string(),
    ))?;

    // Verify job ID matches current job
    if pool.active_job_id.as_ref() != Some(&submission.job_id) {
        return Err((StatusCode::BAD_REQUEST, "Stale job ID".to_string()));
    }

    // Parse submitted hash
    let hash_bytes = hex::decode(submission.hash.trim_start_matches("0x"))
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid hash format".to_string()))?;

    if hash_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Hash must be 32 bytes".to_string()));
    }

    // Get current difficulty and target
    let g = crate::CHAIN.lock();
    let difficulty = g.difficulty;

    // Calculate network target from difficulty
    let network_target = if difficulty > 0 {
        let mut t = [0xFFu8; 32];
        let max = u128::MAX;
        let target_val = max / difficulty as u128;
        let bytes = target_val.to_be_bytes();
        t[16..].copy_from_slice(&bytes);
        t
    } else {
        [0xFFu8; 32]
    };
    drop(g);

    let share_difficulty = difficulty / pool.config.share_difficulty_divisor.max(1);

    // Calculate share target bytes
    let share_target_bytes = if share_difficulty > 0 {
        let mut t = [0xFFu8; 32];
        let max = u128::MAX;
        let target_val = max / share_difficulty as u128;
        let bytes = target_val.to_be_bytes();
        t[16..].copy_from_slice(&bytes);
        t
    } else {
        [0xFFu8; 32]
    };

    // Compare hashes (byte-wise comparison, big-endian)
    let is_valid_share = hash_bytes.as_slice() <= &share_target_bytes[..];
    let is_block = hash_bytes.as_slice() <= &network_target[..];

    if !is_valid_share {
        // Invalid share - record it
        let _ = pool.record_invalid_share(&submission.worker_id);
        let banned = crate::pool::BAN_MANAGER.record_invalid(&submission.worker_id);

        // Update metrics
        {
            let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
            metrics.invalid_shares_received += 1;
        }

        if banned {
            return Err((
                StatusCode::FORBIDDEN,
                format!(
                    "Worker {} banned for excessive invalid shares",
                    submission.worker_id
                ),
            ));
        }

        return Err((
            StatusCode::BAD_REQUEST,
            "Share does not meet difficulty target".to_string(),
        ));
    }

    // Record valid share
    pool.record_share(&submission.worker_id, share_difficulty)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    // Record in ban manager
    crate::pool::BAN_MANAGER.record_valid(&submission.worker_id);

    // Update metrics
    {
        let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
        metrics.total_shares_received += 1;

        // Update response time (simple moving average)
        let response_ms = submit_start.elapsed().as_millis() as f64;
        if metrics.avg_share_response_time_ms == 0.0 {
            metrics.avg_share_response_time_ms = response_ms;
        } else {
            metrics.avg_share_response_time_ms =
                (metrics.avg_share_response_time_ms * 0.9) + (response_ms * 0.1);
        }
    }

    // Update worker hashrate if provided
    if let Some(hashrate) = submission.hashrate {
        let _ = pool.update_worker_hashrate(&submission.worker_id, hashrate);
    }

    let total_shares = pool.get_total_shares();

    tracing::info!(
        "✅ Valid share from worker {} (is_block: {})",
        submission.worker_id,
        is_block
    );

    // If this is a block, handle block found
    if is_block {
        tracing::info!("🎉 BLOCK FOUND by worker {}!", submission.worker_id);

        // Invalidate job cache
        crate::pool::JOB_CACHE.invalidate();

        // Update metrics
        {
            let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
            metrics.blocks_found += 1;
        }

        // Get block reward and calculate payouts
        let g = crate::CHAIN.lock();
        let block_height = g.blocks.last().unwrap().header.number + 1;
        drop(g);

        let block_reward = crate::vision_constants::land_block_reward(block_height);
        let protocol_fee =
            crate::vision_constants::land_amount(crate::vision_constants::PROTOCOL_FEE_LAND);
        let miner_reward = block_reward.saturating_sub(protocol_fee);

        // Calculate payouts for all workers
        match crate::pool::compute_pool_payouts(pool, miner_reward) {
            Ok(payouts) => {
                eprintln!(
                    "💰 Distributing {} LAND to {} workers",
                    miner_reward as f64 / 100_000_000.0,
                    payouts.len()
                );

                // Calculate total payout for metrics
                let total_payout: u128 = payouts.iter().map(|(_, amt)| amt).sum();

                // Log payout details
                for (address, amount) in &payouts {
                    let land_amount = *amount as f64 / 100_000_000.0;
                    eprintln!("   {} -> {} LAND", address, land_amount);
                }

                // Submit payouts (direct balance update for now - TODO: use proper transactions)
                if let Err(e) = crate::pool::payouts::distribute_pool_payouts_direct(payouts) {
                    eprintln!("❌ Failed to distribute payouts: {}", e);
                } else {
                    eprintln!("✅ Pool payouts distributed successfully");

                    // Update metrics
                    {
                        let mut metrics = crate::pool::POOL_METRICS.lock().unwrap();
                        metrics.total_payouts += total_payout;
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to calculate payouts: {}", e);
            }
        }

        // Reset shares after block found
        drop(pool_guard); // Drop read guard before getting write guard
        let mut pool_mut = POOL_STATE.lock();
        if let Some(ref mut state) = *pool_mut {
            state.reset_shares_after_block(block_height);
        }
    }

    Ok(Json(ShareResponse {
        ok: true,
        message: Some(
            if is_block {
                "Block found!"
            } else {
                "Share accepted"
            }
            .to_string(),
        ),
        is_block,
        total_shares,
        estimated_payout: None, // TODO: Calculate estimated payout
    }))
}

/// GET /pool/stats - Get pool statistics
pub async fn pool_get_stats() -> Result<Json<PoolStatsResponse>, (StatusCode, String)> {
    let mode = *MINING_MODE.lock();
    if mode != MiningMode::HostPool {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "This node is not hosting a pool".to_string(),
        ));
    }

    let pool_guard = POOL_STATE.lock();
    let pool = pool_guard.as_ref().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Pool not initialized".to_string(),
    ))?;

    let stats = pool.get_stats();
    let workers = pool.get_workers();

    // Get current block reward for estimation
    let g = crate::CHAIN.lock();
    let height = g.blocks.last().unwrap().header.number + 1;
    drop(g);

    let block_reward = crate::vision_constants::land_block_reward(height);
    let protocol_fee =
        crate::vision_constants::land_amount(crate::vision_constants::PROTOCOL_FEE_LAND);
    let miner_reward = block_reward.saturating_sub(protocol_fee);

    let worker_stats: Vec<WorkerStats> = workers
        .iter()
        .map(|w| {
            let estimated = w.estimated_payout(stats.total_shares, miner_reward);
            WorkerStats {
                worker_id: w.id.clone(),
                worker_name: w.worker_name.clone(),
                wallet_address: w.wallet_address.clone(),
                total_shares: w.total_shares,
                reported_hashrate: w.reported_hashrate,
                estimated_payout: crate::format_land(estimated),
            }
        })
        .collect();

    // Get pool config for metadata
    let config = POOL_CONFIG.lock();
    let pool_name = config.pool_name.clone();
    let pool_port = config.pool_port;
    let pool_url = generate_pool_url(pool_port);
    drop(config);

    Ok(Json(PoolStatsResponse {
        worker_count: stats.worker_count,
        total_shares: stats.total_shares,
        total_hashrate: stats.total_hashrate,
        blocks_found: stats.blocks_found,
        last_block_height: stats.last_block_height,
        workers: worker_stats,
        pool_name,
        pool_url,
        pool_port,
    }))
}

/// POST /pool/configure - Configure pool settings (host only)
#[derive(Deserialize)]
pub struct PoolConfigureRequest {
    /// Pool fee as percentage (e.g., 1.5 for 1.5%)
    pub pool_fee: Option<f64>,
    pub host_address: Option<String>,
    /// Pool name (visible to world)
    pub pool_name: Option<String>,
    /// Pool port (7072 or 8082)
    pub pool_port: Option<u16>,
    /// Worker name (for pool joiners)
    pub worker_name: Option<String>,
    /// If true, save config and restart node
    pub save_and_restart: Option<bool>,
}

pub async fn pool_configure(
    Json(req): Json<PoolConfigureRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut config = POOL_CONFIG.lock();

    if let Some(fee_percent) = req.pool_fee {
        // Convert percentage (1.5) to basis points (150)
        let fee_bps = (fee_percent * 100.0) as u16;
        if fee_bps > 1000 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Pool fee cannot exceed 10%".to_string(),
            ));
        }
        config.pool_fee_bps = fee_bps;
    }

    if let Some(addr) = req.host_address {
        config.host_address = addr.clone();
    }

    if let Some(name) = req.pool_name {
        config.pool_name = name;
    }

    if let Some(port) = req.pool_port {
        if port != 7072 && port != 8082 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Pool port must be 7072 or 8082".to_string(),
            ));
        }
        config.pool_port = port;
    }

    if let Some(name) = req.worker_name {
        config.worker_name = Some(name);
    }

    // Save mining mode in config
    config.mining_mode = Some(*MINING_MODE.lock());

    // If save_and_restart is true, persist config and exit for restart
    if req.save_and_restart.unwrap_or(false) {
        // Get data directory
        let port: u16 = std::env::var("VISION_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(7070);
        let data_dir = std::path::PathBuf::from(format!("./vision_data_{}", port));

        // Save config
        if let Err(e) = crate::save_pool_config(&data_dir, &config) {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to save config: {}", e),
            ));
        }

        tracing::info!("Pool configuration saved to disk");
        tracing::info!("Node will exit so launcher can restart it...");

        drop(config); // Release lock before exiting

        // Schedule exit to allow HTTP response to flush
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            tracing::info!("Exiting for restart...");
            std::process::exit(0);
        });

        return Ok(Json(serde_json::json!({
            "ok": true,
            "status": "node_exiting_for_restart"
        })));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "pool_fee_bps": config.pool_fee_bps,
        "foundation_fee_bps": config.foundation_fee_bps,
        "host_address": config.host_address,
        "pool_name": config.pool_name,
        "pool_port": config.pool_port,
    })))
}

/// POST /pool/start - Start hosting a pool
pub async fn pool_start(
    Json(req): Json<PoolConfigureRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mode = MINING_MODE.lock();
    *mode = MiningMode::HostPool;

    // Update pool config with provided parameters
    let mut config = POOL_CONFIG.lock();
    if let Some(fee_percent) = req.pool_fee {
        // Convert percentage (1.5) to basis points (150)
        let fee_bps = (fee_percent * 100.0) as u16;
        if fee_bps > 1000 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Pool fee cannot exceed 10%".to_string(),
            ));
        }
        config.pool_fee_bps = fee_bps;
    }

    if let Some(name) = req.pool_name {
        config.pool_name = name;
    }

    if let Some(port) = req.pool_port {
        if port != 7072 && port != 8082 {
            return Err((
                StatusCode::BAD_REQUEST,
                "Pool port must be 7072 or 8082".to_string(),
            ));
        }
        config.pool_port = port;
    }

    // Generate automatic pool URL for joining miners
    let pool_url = generate_pool_url(config.pool_port);
    let pool_name = config.pool_name.clone();
    let pool_port = config.pool_port;

    // Initialize pool state
    let pool_config = config.clone();
    drop(config); // Release lock before creating pool
    drop(mode); // Release mode lock

    let pool = PoolState::new(pool_config);
    *POOL_STATE.lock() = Some(pool);

    tracing::info!(
        "🏊 Pool '{}' hosting started on port {}",
        pool_name,
        pool_port
    );

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Pool hosting started",
        "mode": "host_pool",
        "pool_name": pool_name,
        "pool_url": pool_url,
        "pool_port": pool_port,
    })))
}

/// POST /pool/stop - Stop hosting pool
pub async fn pool_stop() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut mode = MINING_MODE.lock();
    *mode = MiningMode::Solo;

    *POOL_STATE.lock() = None;

    tracing::info!("🛑 Pool hosting stopped");

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": "Pool hosting stopped",
        "mode": "solo"
    })))
}

/// GET /pool/mode - Get current mining mode
pub async fn pool_get_mode() -> Json<serde_json::Value> {
    let mode = *MINING_MODE.lock();
    Json(serde_json::json!({
        "mode": mode.as_str(),
        "is_hosting": mode == MiningMode::HostPool,
        "is_worker": mode == MiningMode::JoinPool,
    }))
}

/// POST /pool/mode - Set mining mode
pub async fn pool_set_mode(
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mode_str = req
        .get("mode")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "Missing 'mode' field".to_string()))?;

    let mode = match mode_str {
        "Solo" => MiningMode::Solo,
        "HostPool" => MiningMode::HostPool,
        "JoinPool" => MiningMode::JoinPool,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid mode: {}", mode_str),
            ))
        }
    };

    *MINING_MODE.lock() = mode;

    tracing::info!("Mining mode set to: {:?}", mode);

    Ok(Json(serde_json::json!({
        "ok": true,
        "mode": mode.as_str()
    })))
}

/// GET /pool/metrics - Get pool performance metrics
pub async fn pool_get_metrics() -> Json<serde_json::Value> {
    let metrics = crate::pool::POOL_METRICS.lock().unwrap();

    Json(serde_json::json!({
        "total_shares_received": metrics.total_shares_received,
        "invalid_shares_received": metrics.invalid_shares_received,
        "invalid_share_rate": format!("{:.2}%", metrics.invalid_share_rate() * 100.0),
        "blocks_found": metrics.blocks_found,
        "total_payouts": metrics.total_payouts,
        "total_payouts_land": metrics.total_payouts as f64 / 100_000_000.0,
        "avg_job_response_time_ms": format!("{:.2}", metrics.avg_job_response_time_ms),
        "avg_share_response_time_ms": format!("{:.2}", metrics.avg_share_response_time_ms),
        "current_hashrate": metrics.current_hashrate,
    }))
}
