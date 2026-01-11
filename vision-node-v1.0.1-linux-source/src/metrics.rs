#![allow(dead_code)]

use std::sync::Arc;
use once_cell::sync::Lazy;

use axum::http::{HeaderValue, StatusCode};
use axum::response::IntoResponse;
use prometheus::{opts, Encoder, IntCounter, IntGauge, Registry, TextEncoder};

/// Global atomic tx counters (shared across modules)
pub static PROM_VISION_ATOMIC_TXS: Lazy<IntCounter> = Lazy::new(|| {
    IntCounter::new("vision_atomic_txs", "Atomic tx attempts").expect("atomic tx counter")
});

pub static PROM_VISION_ATOMIC_FAILURES: Lazy<IntCounter> = Lazy::new(|| {
    IntCounter::new("vision_atomic_failures", "Atomic tx failures")
        .expect("atomic tx failure counter")
});

/// Wrapper around the sled database for shared access
#[derive(Clone)]
pub struct DbCtx {
    pub db: sled::Db,
}

/// Public handle you'll keep in AppState
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,

    // Tokenomics
    // (Tokenomics gauges removed — now handled via Vision constants modules)

    // Ops/health
    pub blocks_height: IntGauge,
    pub mempool_len: IntGauge,
    pub peers_connected: IntGauge,

    // Difficulty/Mining metrics
    pub diff_current: IntGauge,
    pub diff_block_time_ms: IntGauge,
    pub diff_avg_block_time_ms: IntGauge,
    pub diff_window_size: IntGauge,

    // Wallet operations
    pub wallet_transfers_total: IntCounter,
    pub wallet_transfer_volume: IntCounter,
    pub wallet_fees_collected: IntCounter,
    pub wallet_receipts_written: IntCounter,

    // Performance metrics (Phase 4)
    pub block_validations_total: IntCounter,
    pub tx_verifications_total: IntCounter,
    pub cache_hits_total: IntCounter,
    pub cache_misses_total: IntCounter,
    pub atomic_tx_total: IntCounter,
    pub atomic_tx_failures: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let registry =
            Registry::new_custom(Some("vision".to_string()), None).expect("metrics registry");

        // Tokenomics gauges removed. Use vision_constants and direct state queries where needed.

        // --- Ops gauges ---
        let blocks_height =
            IntGauge::with_opts(opts!("blocks_height", "Best chain height")).unwrap();
        let mempool_len =
            IntGauge::with_opts(opts!("mempool_len", "Current mempool length")).unwrap();
        let peers_connected =
            IntGauge::with_opts(opts!("peers_connected", "Connected peers")).unwrap();

        // --- Difficulty/Mining gauges ---
        let diff_current =
            IntGauge::with_opts(opts!("diff_current", "Current mining difficulty")).unwrap();
        let diff_block_time_ms = IntGauge::with_opts(opts!(
            "diff_block_time_ms",
            "Last block time in milliseconds"
        ))
        .unwrap();
        let diff_avg_block_time_ms = IntGauge::with_opts(opts!(
            "diff_avg_block_time_ms",
            "Average block time over window (ms)"
        ))
        .unwrap();
        let diff_window_size =
            IntGauge::with_opts(opts!("diff_window_size", "LWMA window size in blocks")).unwrap();

        // --- Wallet counters ---
        let wallet_transfers_total = IntCounter::with_opts(opts!(
            "wallet_transfers_total",
            "Total number of wallet transfers"
        ))
        .unwrap();
        let wallet_transfer_volume = IntCounter::with_opts(opts!(
            "wallet_transfer_volume",
            "Total volume of tokens transferred"
        ))
        .unwrap();
        let wallet_fees_collected = IntCounter::with_opts(opts!(
            "wallet_fees_collected",
            "Total fees collected from transfers"
        ))
        .unwrap();
        let wallet_receipts_written =
            IntCounter::with_opts(opts!("wallet_receipts_written", "Total receipts written"))
                .unwrap();

        // --- Performance metrics (Phase 4) ---
        let block_validations_total = IntCounter::with_opts(opts!(
            "block_validations_total",
            "Total block validations performed"
        ))
        .unwrap();
        let tx_verifications_total = IntCounter::with_opts(opts!(
            "tx_verifications_total",
            "Total transaction verifications performed"
        ))
        .unwrap();
        let cache_hits_total =
            IntCounter::with_opts(opts!("cache_hits_total", "Mining template cache hits")).unwrap();
        let cache_misses_total =
            IntCounter::with_opts(opts!("cache_misses_total", "Mining template cache misses"))
                .unwrap();
        let atomic_tx_total = IntCounter::with_opts(opts!(
            "atomic_tx_total",
            "Total atomic transactions attempted"
        ))
        .unwrap();
        let atomic_tx_failures =
            IntCounter::with_opts(opts!("atomic_tx_failures", "Atomic transaction failures"))
                .unwrap();

        for m in [
            &blocks_height,
            &mempool_len,
            &peers_connected,
            &diff_current,
            &diff_block_time_ms,
            &diff_avg_block_time_ms,
            &diff_window_size,
        ] {
            registry.register(Box::new(m.clone())).unwrap();
        }

        // Register wallet counters
        registry
            .register(Box::new(wallet_transfers_total.clone()))
            .unwrap();
        registry
            .register(Box::new(wallet_transfer_volume.clone()))
            .unwrap();
        registry
            .register(Box::new(wallet_fees_collected.clone()))
            .unwrap();
        registry
            .register(Box::new(wallet_receipts_written.clone()))
            .unwrap();

        // Register performance counters
        registry
            .register(Box::new(block_validations_total.clone()))
            .unwrap();
        registry
            .register(Box::new(tx_verifications_total.clone()))
            .unwrap();
        registry
            .register(Box::new(cache_hits_total.clone()))
            .unwrap();
        registry
            .register(Box::new(cache_misses_total.clone()))
            .unwrap();
        registry
            .register(Box::new(atomic_tx_total.clone()))
            .unwrap();
        registry
            .register(Box::new(atomic_tx_failures.clone()))
            .unwrap();

        Self {
            registry,
            blocks_height,
            mempool_len,
            peers_connected,
            diff_current,
            diff_block_time_ms,
            diff_avg_block_time_ms,
            diff_window_size,
            wallet_transfers_total,
            wallet_transfer_volume,
            wallet_fees_collected,
            wallet_receipts_written,
            block_validations_total,
            tx_verifications_total,
            cache_hits_total,
            cache_misses_total,
            atomic_tx_total,
            atomic_tx_failures,
        }
    }

    // Lightweight setters you can call from engine/mempool/peer code
    pub fn set_height(&self, h: u64) {
        self.blocks_height.set(h as i64);
    }
    pub fn set_mempool_len(&self, n: usize) {
        self.mempool_len.set(n as i64);
    }
    pub fn set_peers(&self, n: usize) {
        self.peers_connected.set(n as i64);
    }
}

/// GET /metrics — Prometheus text exposition
pub async fn metrics_handler(_db: sled::Db, metrics: Arc<Metrics>) -> impl IntoResponse {
    // Note: Tokenomics gauges were removed; metrics reflect live state from other handlers.

    let metric_families = metrics.registry.gather();
    let mut buf = Vec::with_capacity(8 * 1024);
    let encoder = TextEncoder::new();
    if let Err(_e) = encoder.encode(&metric_families, &mut buf) {
        // On encoder failure, return 500 with a tiny text body (still text/plain)
        let mut resp = (StatusCode::INTERNAL_SERVER_ERROR, "metrics encode error").into_response();
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
        return resp;
    }

    let body = match String::from_utf8(buf) {
        Ok(s) => s,
        Err(_) => String::from("# encoding error\n"),
    };

    let mut resp = (StatusCode::OK, body).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    resp
}

// Tokenomics database refresh removed - tokenomics handled via Vision constants and dedicated modules.
