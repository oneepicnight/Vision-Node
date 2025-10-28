#![allow(unused_mut)]
// --- imports & globals (merged from restored variant) ---
use std::{env, time::{SystemTime, UNIX_EPOCH, Duration}, net::SocketAddr};
use std::error::Error;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use sled::{Db, IVec};
// we replaced many legacy AtomicU64 metrics with Prometheus-native metrics

use axum::{
    Router,
    Json,
    extract::{Query, Path, ConnectInfo},
    response::IntoResponse,
    routing::{get, post},
};
use axum::http::{StatusCode, HeaderMap, HeaderValue};
// We'll use a small custom CORS middleware to enforce VISION_CORS_ORIGINS precisely.
use tower_http::cors::{CorsLayer, Any};
// additional imports for serving static files & version router
use tower_http::services::{ServeDir, ServeFile};
use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use thiserror::Error;

use reqwest::Client;

// shared HTTP client used around the node
static HTTP: Lazy<Client> = Lazy::new(|| {
    // Global client with a sensible default timeout to avoid long-hanging requests.
    Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .expect("build http client")
});

use blake3::Hasher;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use tracing::{info, debug};
use tracing_subscriber::EnvFilter;
use dashmap::DashMap;
use prometheus::{Registry, Histogram, HistogramOpts, Encoder, TextEncoder, IntCounter, IntGauge, IntCounterVec, Gauge};
use dashmap::DashMap as DashMapLocal;
mod mempool;
mod version;

// Replace earlier DashMap usage with a simple Mutex-wrapped BTreeMap to avoid adding deps
static PEERS: Lazy<Mutex<BTreeMap<String, __PeerMeta>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));

// Broadcaster senders set during startup
static TX_BCAST_SENDER: once_cell::sync::OnceCell<tokio::sync::mpsc::Sender<Tx>> = once_cell::sync::OnceCell::new();
static BLOCK_BCAST_SENDER: once_cell::sync::OnceCell<tokio::sync::mpsc::Sender<Block>> = once_cell::sync::OnceCell::new();

// Node metrics / counters used across the binary
// keep a small recent history of sweep timestamps (unix secs)
// history entries: (unix_secs, removed_count, duration_ms, mempool_size)
static VISION_MEMPOOL_SWEEP_HISTORY: Lazy<Mutex<std::collections::VecDeque<(u64, u64, u64, u64)>>> = Lazy::new(|| Mutex::new(std::collections::VecDeque::new()));

// Prometheus registry + histogram for sweep durations
static PROM_REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());
static VISION_MEMPOOL_SWEEP_DURATION_HISTOGRAM: Lazy<Histogram> = Lazy::new(|| {
    let opts = HistogramOpts::new("vision_mempool_sweep_duration_seconds", "Mempool sweep duration seconds");
    let h = Histogram::with_opts(opts).expect("create histogram");
    // register, ignore error if already registered
    let g = CHAIN.lock();

    // Compute values once and set PROM_* collectors so registry contains everything
    let height: u64 = g.blocks.last().map(|b| b.header.number).unwrap_or(0);
    PROM_VISION_HEIGHT.set(height as i64);

    let peers: u64 = g.peers.len() as u64;
    PROM_VISION_PEERS.set(peers as i64);

    let mempool_len: u64 = (g.mempool_critical.len() + g.mempool_bulk.len()) as u64;
    PROM_VISION_MEMPOOL_LEN.set(mempool_len as i64);
    PROM_VISION_MEMPOOL_CRIT_LEN.set(g.mempool_critical.len() as i64);
    PROM_VISION_MEMPOOL_BULK_LEN.set(g.mempool_bulk.len() as i64);

    let block_weight_limit = g.limits.block_weight_limit;
    PROM_VISION_BLOCK_WEIGHT_LIMIT.set(block_weight_limit as i64);
    let last_weight: u64 = PROM_VISION_BLOCK_WEIGHT_LAST.get() as u64;
    let weight_util = if block_weight_limit > 0 { (last_weight as f64) / (block_weight_limit as f64) } else { 0.0 };
    PROM_VISION_BLOCK_WEIGHT_UTIL.set(weight_util);

    PROM_VISION_DIFFICULTY_BITS.set(g.blocks.last().map(|b| b.header.difficulty).unwrap_or(1) as i64);
    PROM_VISION_TARGET_BLOCK_TIME.set(g.limits.target_block_time as i64);
    PROM_VISION_RETARGET_WINDOW.set(g.limits.retarget_window as i64);

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let last_ts = g.blocks.last().map(|b| b.header.timestamp).unwrap_or(now);
    PROM_VISION_LAST_BLOCK_TIME_SECS.set(now.saturating_sub(last_ts) as i64);

    // tip percentiles
    let mut tips: Vec<u64> = Vec::new();
    for t in g.mempool_critical.iter() { tips.push(t.tip); }
    for t in g.mempool_bulk.iter() { tips.push(t.tip); }
    tips.sort_unstable();
    let tip_p50: f64 = if tips.is_empty() { 0.0 } else { tips[tips.len()/2] as f64 };
    let tip_p95: f64 = if tips.is_empty() { 0.0 } else { tips[(tips.len() * 95 / 100).min(tips.len()-1)] as f64 };
    PROM_VISION_FEE_TIP_P50.set(tip_p50);
    PROM_VISION_FEE_TIP_P95.set(tip_p95);

    // fee-per-byte percentiles
    let mut fee_per_byte: Vec<f64> = Vec::new();
    for t in g.mempool_critical.iter().chain(g.mempool_bulk.iter()) {
        let w = est_tx_weight(t) as f64;
        if w > 0.0 { fee_per_byte.push((t.tip as f64) / w); }
    }
    fee_per_byte.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let fpb_p50 = if fee_per_byte.is_empty() { 0.0 } else { fee_per_byte[fee_per_byte.len()/2] };
    let fpb_p95 = if fee_per_byte.is_empty() { 0.0 } else { fee_per_byte[(fee_per_byte.len()*95/100).min(fee_per_byte.len()-1)] };
    PROM_VISION_FEE_PER_BYTE_P50.set(fpb_p50);
    PROM_VISION_FEE_PER_BYTE_P95.set(fpb_p95);

    PROM_VISION_SNAPSHOT_EVERY_BLOCKS.set(g.limits.snapshot_every_blocks as i64);

    // mempool sweep metrics already updated elsewhere; just ensure values available
    PROM_VISION_MEMPOOL_SWEEPS.set(PROM_VISION_MEMPOOL_SWEEPS.get() as i64);
    PROM_VISION_MEMPOOL_REMOVED_TOTAL.set(PROM_VISION_MEMPOOL_REMOVED_TOTAL.get() as i64);
    PROM_VISION_MEMPOOL_REMOVED_LAST.set(PROM_VISION_MEMPOOL_REMOVED_LAST.get());
    PROM_VISION_MEMPOOL_SWEEP_LAST_MS.set(PROM_VISION_MEMPOOL_SWEEP_LAST_MS.get());

    // assemble registry output only
    let encoder = TextEncoder::new();
    let metric_families = PROM_REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).ok();
    let prom_text = String::from_utf8_lossy(&buffer).into_owned();
    let headers = [(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("text/plain; version=0.0.4"))];
    (headers, prom_text)
    let recent_fail = m.values().filter(|p| p.fail_count > 0).count();
    Json(serde_json::json!({ "peers_len": peers_len, "recent_ok": recent_ok, "recent_fail": recent_fail }))
}

async fn peers_ping(Query(q): Query<std::collections::HashMap<String,String>>) -> Json<serde_json::Value> {
    if let Some(raw) = q.get("url") {
        let url = normalize_url(raw);
        let start = std::time::Instant::now();
        let resp = HTTP.get(format!("{}/status", url)).timeout(Duration::from_millis(500)).send().await;
        let ms = start.elapsed().as_millis() as u64;
        if let Ok(r) = resp {
            if r.status().is_success() { return Json(serde_json::json!({ "ok": true, "ms": ms })); }
        }
        return Json(serde_json::json!({ "ok": false, "ms": ms }));
    }
    Json(serde_json::json!({ "error": "missing url" }))
}

// --- small helper stubs ---
fn normalize_url(raw: &str) -> String { raw.trim_end_matches('/').to_string() }

async fn panel_config() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok":true}))
}

async fn peers_add_handler(Json(req): Json<AddPeerReq>) -> (StatusCode, Json<serde_json::Value>) {
    peers_add(&req.url);
    (StatusCode::OK, Json(serde_json::json!({"ok": true, "url": req.url})))
}

fn peers_add(u: &str) {
    let mut g = CHAIN.lock();
    if hygiene_allow_add(u) { g.peers.insert(u.to_string()); let _ = g.db.insert(format!("{}{}", PEER_PREFIX, u).as_bytes(), IVec::from(&b"1"[..])); }
}

async fn admin_ping_handler(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    // debug log incoming admin auth attempts
    // keep admin auth logging minimal in prod; avoid printing tokens
        if !check_admin(headers.clone(), &q) {
            return api_error_struct(StatusCode::UNAUTHORIZED, "unauthorized", "invalid or missing admin token"); }
    // prom
    PROM_ADMIN_PING_TOTAL.inc();
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}

async fn admin_info() -> (StatusCode, Json<serde_json::Value>) {
    // admin_info should be protected by check_admin at call sites; provide basic info
    let version = env::var("VISION_VERSION").unwrap_or_else(|_| "dev".into());
    (StatusCode::OK, Json(serde_json::json!({"version": version})))
}

async fn admin_mempool_sweeper(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !check_admin(headers, &q) {
        return api_error_struct(StatusCode::UNAUTHORIZED, "unauthorized", "invalid or missing admin token");
    }
    let ttl = mempool_ttl_secs();
    let sweep_secs = std::env::var("VISION_MEMPOOL_SWEEP_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(60);
    let sweeps = PROM_VISION_MEMPOOL_SWEEPS.get() as u64;
    let removed_total = PROM_VISION_MEMPOOL_REMOVED_TOTAL.get() as u64;
    let removed_last = PROM_VISION_MEMPOOL_REMOVED_LAST.get() as i64 as u64;
    let last_ms = PROM_VISION_MEMPOOL_SWEEP_LAST_MS.get() as i64 as u64;
    let mut recent: Vec<serde_json::Value> = Vec::new();
    {
        let h = VISION_MEMPOOL_SWEEP_HISTORY.lock();
        for (ts, removed, dur, msize) in h.iter() { recent.push(serde_json::json!({ "ts": *ts, "removed": *removed, "duration_ms": *dur, "mempool_size": *msize })); }
    }
    (StatusCode::OK, Json(serde_json::json!({
        "ok": true,
        "mempool_ttl_secs": ttl,
        "mempool_sweep_secs": sweep_secs,
        "sweeps_total": sweeps,
        "removed_total": removed_total,
        "removed_last": removed_last,
        "last_sweep_ms": last_ms,
        "recent_sweeps_unix_secs": recent
    })))
}


// =================== Config / Constants ===================

// Centralized runtime limits loaded from environment (FIX21)
#[derive(Debug, Clone)]
pub struct Limits {
    pub block_weight_limit: u64,
    pub block_target_txs: usize,
    pub max_reorg: u64,
    pub mempool_max: usize,
    pub rate_submit_rps: u64,
    pub rate_gossip_rps: u64,
    pub snapshot_every_blocks: u64,
    pub target_block_time: u64,
    pub retarget_window: u64,
}

fn load_limits() -> Limits {
    Limits {
        block_weight_limit: std::env::var("VISION_BLOCK_WEIGHT_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(400_000),
        block_target_txs: std::env::var("VISION_BLOCK_TARGET_TXS").ok().and_then(|s| s.parse().ok()).unwrap_or(200),
        max_reorg: std::env::var("VISION_MAX_REORG").ok().and_then(|s| s.parse().ok()).unwrap_or(36),
        mempool_max: std::env::var("VISION_MEMPOOL_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(10_000),
        rate_submit_rps: std::env::var("VISION_RATE_SUBMIT_TX_RPS").ok().and_then(|s| s.parse().ok()).unwrap_or(8),
        rate_gossip_rps: std::env::var("VISION_RATE_GOSSIP_RPS").ok().and_then(|s| s.parse().ok()).unwrap_or(20),
        snapshot_every_blocks: std::env::var("VISION_SNAPSHOT_EVERY_BLOCKS").ok().and_then(|s| s.parse().ok()).unwrap_or(1000),
        target_block_time: std::env::var("VISION_TARGET_BLOCK_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(5),
        retarget_window: std::env::var("VISION_RETARGET_WINDOW").ok().and_then(|s| s.parse().ok()).unwrap_or(20),
    }
}

#[cfg(test)]
mod proof_tests {
    use super::*;

    #[test]
    fn balance_proof_reconstructs_root() {
        let mut g = crate::fresh_chain();
        // populate some balances
        g.balances.insert(acct_key("alice"), 100u128);
        g.balances.insert(acct_key("bob"), 200u128);
        g.balances.insert(acct_key("carol"), 50u128);
        // get proof for bob
        let proof = get_balance_proof(&g, "bob").expect("proof");
        // reconstruct root from leaf and path
        let mut cur = hex::decode(&proof.leaf).expect("leaf hex");
        for (sibling_hex, sibling_on_left) in proof.path.iter() {
            let sib = hex::decode(sibling_hex).expect("sib hex");
            let mut hasher = blake3::Hasher::new();
            if *sibling_on_left { hasher.update(&sib); hasher.update(&cur); } else { hasher.update(&cur); hasher.update(&sib); }
            let out = hasher.finalize();
            cur = out.as_bytes().to_vec();
        }
        let reconstructed = hex::encode(&cur);
        assert_eq!(reconstructed, proof.root);
    }
}

#[cfg(test)]
mod classify_tests {
    use super::*;

    #[test]
    fn classify_std_io_errors() {
        // Connection refused should classify accordingly
        let cre = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let label = classify_error_any(&cre);
        assert_eq!(label, "connection_refused");

        // Timed out should classify as timeout
        let toe = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let label2 = classify_error_any(&toe);
        assert_eq!(label2, "timeout");

        // Generic io error falls back to request_error
        let other = std::io::Error::new(std::io::ErrorKind::Other, "other");
        let label3 = classify_error_any(&other);
        assert_eq!(label3, "request_error");
    }
}

// Simple token-bucket for per-IP rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_per_sec: f64,
    last_ts: u64,
}
impl TokenBucket {
    fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self { tokens: capacity, capacity, refill_per_sec, last_ts: now_ts() }
    }
    fn allow(&mut self, cost: f64) -> bool {
        let now = now_ts();
        let elapsed = (now.saturating_sub(self.last_ts)) as f64;
        self.last_ts = now;
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        if self.tokens + 1e-9 >= cost {
            self.tokens -= cost;
            true
        } else { false }
    }
}

// Per-IP token buckets
static IP_TOKEN_BUCKETS: once_cell::sync::Lazy<DashMap<String, TokenBucket>> = once_cell::sync::Lazy::new(|| DashMap::new());

static FEE_BASE: Lazy<Mutex<u128>> = Lazy::new(|| {
    Mutex::new(env::var("VISION_FEE_BASE").ok().and_then(|s| s.parse().ok()).unwrap_or(1))
});
static CHAIN: Lazy<Mutex<Chain>> = Lazy::new(|| {
    // Per-port data dir so multi-nodes on one machine don't step on each other
    let port: u16 = env::var("VISION_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(7070);
    let dir = format!("./vision_data_{}", port);
    Mutex::new(Chain::init(&dir))
});

fn fee_base() -> u128 { *FEE_BASE.lock() }
fn fee_per_recipient() -> u128 {
    env::var("VISION_FEE_PER_RECIPIENT").ok().and_then(|s| s.parse().ok()).unwrap_or(0)
}
fn miner_require_sync() -> bool {
    env::var("VISION_MINER_REQUIRE_SYNC").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(0) != 0
}
fn miner_max_lag() -> u64 {
    env::var("VISION_MINER_MAX_LAG").ok().and_then(|s| s.parse().ok()).unwrap_or(0)
}
fn discovery_secs() -> u64 {
    std::env::var("VISION_DISCOVERY_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(15)
}
fn block_target_txs() -> usize {
    env::var("VISION_BLOCK_TARGET_TXS").ok().and_then(|s| s.parse().ok()).unwrap_or(200)
}
fn block_util_high() -> f64 {
    env::var("VISION_BLOCK_UTIL_HIGH").ok().and_then(|s| s.parse().ok()).unwrap_or(0.8)
}
fn block_util_low() -> f64 {
    env::var("VISION_BLOCK_UTIL_LOW").ok().and_then(|s| s.parse().ok()).unwrap_or(0.3)
}
// mempool max is available on Chain.limits; keep this helper marked dead
#[allow(dead_code)]
fn mempool_max() -> usize {
    env::var("VISION_MEMPOOL_MAX").ok().and_then(|s| s.parse().ok()).unwrap_or(10_000)
}
fn mempool_ttl_secs() -> u64 {
    // Default to 15 minutes TTL for mempool entries unless explicitly disabled (0)
    std::env::var("VISION_MEMPOOL_TTL_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(900) // seconds; 0 = disabled
}

// sled keys/prefixes
const BAL_PREFIX: &str = "bal:";           // bal:acct:<addr> -> u128 BE
const NONCE_PREFIX: &str = "nonce:";       // nonce:acct:<addr> -> u64 BE
const BLK_PREFIX: &str = "blk:";           // blk:<height_be8> -> json(Block)
const META_HEIGHT: &str = "meta:height";   // -> u64 BE
const META_GM: &str = "meta:gamemaster";   // -> bytes (hex string)
const RCPT_PREFIX: &str = "rcpt:";         // rcpt:<tx_hash_hex> -> json(Receipt)
const PEER_PREFIX: &str = "peer:";         // peer:<url> -> b"1"

const META_FEE_BASE: &str = "meta:fee_base"; // -> u128 BE

// =================== Primitives ===================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tx {
    pub nonce: u64,
    pub sender_pubkey: String,
    pub access_list: Vec<String>,
    pub module: String,
    pub method: String,
    pub args: Vec<u8>,
    pub tip: u64,
    pub fee_limit: u64,
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_hash: String,
    pub number: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub pow_hash: String,
    pub state_root: String,
    pub tx_root: String,
    pub receipts_root: String,
    pub da_commitment: Option<String>,
}

// --- Cumulative work helpers (heaviest-chain support) ---
#[inline]
#[allow(dead_code)]
fn block_work(bits: u64) -> u128 {
    let b = if bits > 120 { 120 } else { bits as u32 };
    1u128 << b
}
#[allow(dead_code)]
fn chain_total_work(blocks: &[Block]) -> u128 {
    blocks.iter().map(|b| block_work(b.header.difficulty)).sum()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Tx>,
    #[serde(default)]
    pub weight: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub ok: bool,
    pub error: Option<String>,
    pub height: u64,
    pub block_hash: String,
}

#[derive(Debug, Clone)]
pub struct Chain {
    pub blocks: Vec<Block>,
    pub difficulty: u64,
    pub ema_block_time: f64,
    pub mempool_critical: VecDeque<Tx>,
    pub mempool_bulk: VecDeque<Tx>,
    pub mempool_ts: BTreeMap<String, u64>, // tx_hash -> arrival unix secs
    pub balances: BTreeMap<String, u128>,
    pub nonces: BTreeMap<String, u64>,
    pub gamemaster: Option<String>,
    pub db: Db,
    pub peers: BTreeSet<String>,
    pub seen_txs: BTreeSet<String>,
    pub seen_blocks: BTreeSet<String>,
    // side blocks indexed by pow_hash (blocks not on the current main chain)
    pub side_blocks: BTreeMap<String, Block>,
    // cumulative work per block hash (work from genesis up to and including this block)
    pub cumulative_work: BTreeMap<String, u128>,
    // runtime limits loaded from env
    pub limits: Limits,
}

impl Chain {
    pub fn init(path: &str) -> Self {
        let db = match sled::open(path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(path = %path, err = ?e, "failed to open sled database");
                // Cannot continue without storage; exit with non-zero code so supervisors can restart or surface the issue.
                std::process::exit(1);
            }
        };
        // Load dynamic fee base if persisted
        if let Ok(Some(v)) = db.get(META_FEE_BASE.as_bytes()) {
            if v.len() == 16 {
                let mut arr = [0u8;16];
                arr.copy_from_slice(&v);
                let loaded = u128::from_be_bytes(arr);
                *FEE_BASE.lock() = loaded;
            } else if let Ok(s) = String::from_utf8(v.to_vec()) {
                if let Ok(parsed) = s.parse::<u128>() {
                    *FEE_BASE.lock() = parsed;
                }
            }
        }


        // Load balances / nonces
        let mut balances = BTreeMap::new();
        let mut nonces = BTreeMap::new();
        for kv in db.scan_prefix(BAL_PREFIX.as_bytes()) {
            let (k, v) = kv.expect("bal kv");
            let key = String::from_utf8(k.to_vec()).unwrap();
            let addr = key[BAL_PREFIX.len()..].to_string();
            let amt = u128_from_be(&v);
            balances.insert(addr, amt);
        }
        for kv in db.scan_prefix(NONCE_PREFIX.as_bytes()) {
            let (k, v) = kv.expect("nonce kv");
            let key = String::from_utf8(k.to_vec()).unwrap();
            let addr = key[NONCE_PREFIX.len()..].to_string();
            let n = u64_from_be(&v);
            nonces.insert(addr, n);
        }

        // Load blocks (or build genesis)
        let mut blocks: Vec<Block> = Vec::new();
        if let Some(hv) = db.get(META_HEIGHT).unwrap() {
            let tip = u64_from_be(&hv);
            for h in 0..=tip {
                let key = blk_key(h);
                if let Some(bytes) = db.get(&key).unwrap() {
                    let b: Block = serde_json::from_slice(&bytes).expect("decode block");
                    blocks.push(b);
                } else {
                    break;
                }
            }
            if blocks.is_empty() {
                let g = genesis_block();
                persist_block_only(&db, 0, &g);
                blocks.push(g);
            }
        } else {
            let g = genesis_block();
            persist_block_only(&db, 0, &g);
            blocks.push(g);
        }

        // Load GameMaster
        let gm = db.get(META_GM).unwrap().map(|v| String::from_utf8(v.to_vec()).unwrap());

        // Load persisted peers
        let mut peers = BTreeSet::new();
        for kv in db.scan_prefix(PEER_PREFIX.as_bytes()) {
            let (k, _v) = kv.expect("peer kv");
            let key = String::from_utf8(k.to_vec()).unwrap();
            let url = key[PEER_PREFIX.len()..].to_string();
            peers.insert(url);
        }

        // Bootstrap peers from env
        if let Ok(bs) = env::var("VISION_BOOTSTRAP") {
            for raw in bs.split(',') {
                let url = raw.trim();
                if url.is_empty() { continue; }
                peers.insert(url.to_string());
            }
        }

        // compute cumulative work for loaded main chain
        let mut cumulative_work: BTreeMap<String, u128> = BTreeMap::new();
        let mut prev_cum: u128 = 0;
        for b in &blocks {
            prev_cum = prev_cum.saturating_add(block_work(b.header.difficulty));
            cumulative_work.insert(b.header.pow_hash.clone(), prev_cum);
        }

        let limits = load_limits();

        // load persisted difficulty & ema if present
        let mut difficulty: u64 = 1;
        if let Ok(Some(v)) = db.get("meta:difficulty".as_bytes()) {
            if v.len() == 8 { let mut arr = [0u8;8]; arr.copy_from_slice(&v); difficulty = u64::from_be_bytes(arr); }
            else if let Ok(s) = String::from_utf8(v.to_vec()) { if let Ok(n) = s.parse::<u64>() { difficulty = n; } }
        }
        let mut ema_block_time: f64 = limits.target_block_time as f64;
        if let Ok(Some(v)) = db.get("meta:ema_block_time".as_bytes()) {
            if let Ok(s) = String::from_utf8(v.to_vec()) { if let Ok(f) = s.parse::<f64>() { ema_block_time = f; } }
        }
        Chain {
            blocks,
            difficulty,
            ema_block_time,
            mempool_critical: VecDeque::new(),
            mempool_bulk: VecDeque::new(),
            mempool_ts: BTreeMap::new(),
            balances,
            nonces,
            gamemaster: gm,
            db,
            peers,
            seen_txs: BTreeSet::new(),
            seen_blocks: BTreeSet::new(),
            side_blocks: BTreeMap::new(),
            cumulative_work,
            limits,
        }
    }
}

// =================== Helpers ===================
fn now_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

// helper: parse boolean env flags (0/1, true/false)
fn env_flag(k: &str) -> bool {
    std::env::var(k).ok().and_then(|s| {
        let sl = s.to_ascii_lowercase();
        if sl == "1" || sl == "true" || sl == "yes" { Some(true) } else if sl == "0" || sl == "false" || sl == "no" { Some(false) } else { None }
    }).unwrap_or(false)
}

// strict reorg validation toggle (env VISION_REORG_STRICT = 1/true to enable)
fn reorg_strict() -> bool { env_flag("VISION_REORG_STRICT") }

// prune undo entries older than `keep` snapshots/heights
#[allow(dead_code)]
fn prune_old_undos(db: &Db, keep_heights: &[u64]) {
    // keep_heights is the list of heights to keep; remove undo keys not in this list
    let mut keep_set = std::collections::BTreeSet::new();
    for h in keep_heights { keep_set.insert(*h); }
    for kv in db.scan_prefix("meta:undo:".as_bytes()) {
        if let Ok((k, _v)) = kv {
            if let Ok(s) = String::from_utf8(k.to_vec()) {
                if let Some(hs) = s.strip_prefix("meta:undo:") {
                    if let Ok(hv) = hs.parse::<u64>() {
                        if !keep_set.contains(&hv) {
                            let key = format!("meta:undo:{}", hv);
                    let _ = db.remove(key.as_bytes());
                    debug!(removed_undo = hv, "pruned undo");
                        }
                    }
                }
            }
        }
    }
}
// Return the number of undos pruned and update metrics
#[allow(dead_code)]
fn prune_old_undos_count(db: &Db, keep_heights: &[u64]) -> u64 {
    let mut removed = 0u64;
    let mut keep_set = std::collections::BTreeSet::new();
    for h in keep_heights { keep_set.insert(*h); }
    for kv in db.scan_prefix("meta:undo:".as_bytes()) {
        if let Ok((k, _v)) = kv {
            if let Ok(s) = String::from_utf8(k.to_vec()) {
                if let Some(hs) = s.strip_prefix("meta:undo:") {
                    if let Ok(hv) = hs.parse::<u64>() {
                        if !keep_set.contains(&hv) {
                            let key = format!("meta:undo:{}", hv);
                            let _ = db.remove(key.as_bytes());
                            removed = removed.saturating_add(1);
                            debug!(removed_undo = hv, "pruned undo");
                        }
                    }
                }
            }
        }
    }
    PROM_VISION_UNDOS_PRUNED.inc_by(removed);
    removed
}
fn hash_bytes(b: &[u8]) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(b);
    *h.finalize().as_bytes()
}
fn hex32(b: [u8; 32]) -> String { hex::encode(b) }

// --- PoW leading-zero helpers ---
#[inline]
fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut n = 0u32;
    for &b in bytes {
        if b == 0 { n += 8; continue; }
        n += b.leading_zeros();
        break;
    }
    n
}
#[inline]
fn meets_difficulty_bits(hash32: [u8;32], bits: u64) -> bool {
    leading_zero_bits(&hash32) as u64 >= bits
}



// --- Difficulty retarget: nudge ±1 toward target over window ---
#[allow(dead_code)]
fn current_difficulty_bits(g: &Chain) -> u64 {
    let win = g.limits.retarget_window as usize;
    let tgt = g.limits.target_block_time as f64;
    let len = g.blocks.len();
    if len < 3 { return 1; }
    let start = len.saturating_sub(win + 1);
    let slice = &g.blocks[start..];
    if slice.len() < 2 { return 1; }
    let mut gaps: Vec<f64> = Vec::with_capacity(slice.len() - 1);
    for w in slice.windows(2) {
        let a = w[0].header.timestamp as i64;
        let b = w[1].header.timestamp as i64;
        let dt = (b - a).max(1) as f64;
        gaps.push(dt);
    }
    let avg = gaps.iter().sum::<f64>() / gaps.len().max(1) as f64;
    let cur = g.blocks.last().map(|b| b.header.difficulty).unwrap_or(1);
    let mut next = cur;
    if avg > tgt * 1.10 {
        next = cur.saturating_sub(1).max(1);
    } else if avg < tgt * 0.90 {
        next = cur.saturating_add(1).min(248);
    }
    next
}
fn decode_hex32(s: &str) -> Result<[u8; 32], String> {
    let v = hex::decode(s).map_err(|e| e.to_string())?;
    if v.len() != 32 { return Err("expected 32 bytes".into()); }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&v);
    Ok(arr)
}
fn decode_hex64(s: &str) -> Result<[u8; 64], String> {
    let v = hex::decode(s).map_err(|e| e.to_string())?;
    if v.len() != 64 { return Err("expected 64 bytes".into()); }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&v);
    Ok(arr)
}
fn signable_tx_bytes(tx: &Tx) -> Vec<u8> {
    let mut tmp = tx.clone();
    tmp.sig = String::new();
    serde_json::to_vec(&tmp).unwrap()
}
fn tx_hash(tx: &Tx) -> [u8; 32] { hash_bytes(&signable_tx_bytes(tx)) }
fn tx_root_placeholder(txs: &[Tx]) -> String {
    // Build a binary Merkle tree over tx hashes using blake3 as the node hash.
    // Leaves are `tx_hash(tx)` (32 bytes). For odd number of leaves we duplicate
    // the last leaf to form a pair (common simple approach).
    if txs.is_empty() {
        return "0".repeat(64);
    }
    let mut level: Vec<[u8;32]> = txs.iter().map(|t| tx_hash(t)).collect();
    while level.len() > 1 {
        let mut next: Vec<[u8;32]> = Vec::with_capacity((level.len()+1)/2);
        for i in (0..level.len()).step_by(2) {
            let left = level[i];
            let right = if i + 1 < level.len() { level[i+1] } else { level[i] };
            let mut h = Hasher::new();
            h.update(&left);
            h.update(&right);
            let out = h.finalize();
            let mut arr = [0u8;32];
            arr.copy_from_slice(out.as_bytes());
            next.push(arr);
        }
        level = next;
    }
    hex32(level[0])
}
fn header_pow_bytes(h: &BlockHeader) -> Vec<u8> { serde_json::to_vec(h).unwrap() }
fn compute_state_root(balances: &BTreeMap<String, u128>, gm: &Option<String>) -> String {
    let mut h = Hasher::new();
    for (k, v) in balances { h.update(format!("{k}={v}\n").as_bytes()); }
    h.update(b"gm=");
    if let Some(g) = gm { h.update(g.as_bytes()); }
    hex32(*h.finalize().as_bytes())
}
fn genesis_block() -> Block {
    let hdr = BlockHeader {
        parent_hash: "0".repeat(64),
        number: 0,
        timestamp: 0,
        difficulty: 1,
        nonce: 0,
        pow_hash: "0".repeat(64),
        state_root: "0".repeat(64),
        tx_root: "0".repeat(64),
        receipts_root: "0".repeat(64),
        da_commitment: None,
    };
    Block { header: hdr, txs: vec![], weight: 0 }
}
fn blk_key(height: u64) -> Vec<u8> {
    let mut be = [0u8; 8];
    be.copy_from_slice(&height.to_be_bytes());
    let mut v = Vec::with_capacity(BLK_PREFIX.len() + 8);
    v.extend_from_slice(BLK_PREFIX.as_bytes());
    v.extend_from_slice(&be);
    v
}

// =================== Errors ===================
#[derive(Error, Debug)]
enum NodeError {
    #[error("invalid signature")] BadSig,
    #[error("tx too big")] TxTooBig,
    #[error("json error")] Json,
}

// =================== RPC DTOs ===================
#[derive(Serialize)]
struct HeadInfo {
    height: u64,
    hash: String,
    time: u64,
    state_root: String,
}
#[derive(Deserialize)]
struct BalancesQuery { addrs: String }
#[derive(Deserialize)]
struct ReceiptsQuery { hashes: String }
#[derive(Deserialize)]
struct SubmitTx { tx: Tx }
#[derive(Serialize, Deserialize, Clone)]
struct GameMasterView { gamemaster: Option<String> }

#[allow(dead_code)]
#[derive(Deserialize)]
struct AirdropReq {
    from: Option<String>,         // ignored for multi_mint (sender = GM)
    tip: Option<u64>,             // ignored for multi_mint
    miner_addr: Option<String>,   // used
    payments_csv: Option<String>,
    payments: Option<Vec<Payment>>,
}

// peers/gossip DTOs
#[derive(Deserialize)]
struct AddPeerReq { url: String }
#[derive(Serialize, Deserialize, Clone)]
struct PeersView { peers: Vec<String> }
#[derive(Deserialize)]
struct GossipTxEnvelope { tx: Tx }
#[derive(Deserialize)]
struct GossipBlockEnvelope { block: Block }

// sync DTOs
#[derive(Deserialize)]
struct SyncPullReq { src: String, from: Option<u64>, to: Option<u64> }
#[derive(Deserialize)]
struct SyncPushReq { blocks: Vec<Block> }

// status DTO
#[derive(Serialize)]
struct StatusView {
    height: u64,
    best_peer_height: u64,
    lag: i64,
    mempool: usize,
    mining_allowed: bool,
    gating: bool,
    max_lag: u64,
    peers: Vec<String>,
}

// =================== Main ===================
#[tokio::main]
async fn main() {

    // Startup masked admin-token info for debugging (does not print the secret)
    let _admin_token_mask = match std::env::var("VISION_ADMIN_TOKEN") {
        Ok(t) if !t.is_empty() => format!("set (len={})", t.len()),
        _ => "unset".to_string(),
    };
    // init tracing from env RUST_LOG or VISION_LOG
    let filter = std::env::var("VISION_LOG").unwrap_or_else(|_| std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()));
    let env_filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    info!(admin_token = %_admin_token_mask, "Vision node starting up");

    // background: lightweight peer discovery if configured
    if discovery_secs() > 0 {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(discovery_secs())).await;
                // copy peers
                let peers: Vec<String> = {
                    let g = CHAIN.lock();
                    g.peers.iter().cloned().collect()
                };
                for p in peers {
                    let url = format!("{}/peers", p.trim_end_matches('/'));
                    if let Ok(resp) = HTTP.get(url).send().await {
                        if let Ok(text) = resp.text().await {
                            if let Ok(view) = serde_json::from_str::<PeersView>(&text) {
                                let mut g = CHAIN.lock();
                                for peer in view.peers {
                                    if g.peers.insert(peer.clone()) {
                                        let key = format!("{}{}", PEER_PREFIX, peer);
                                        let _ = g.db.insert(key.as_bytes(), IVec::from(&b"1"[..]));
                                    }
                                }
                                let _ = g.db.flush();
                            }
                        }
                    }
                }
            }
        });
    }

    // background: undo pruning job
    let prune_interval = std::env::var("VISION_PRUNE_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(30);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(prune_interval)).await;
            // compute keep heights from snapshots (last N)
            let retain = std::env::var("VISION_SNAPSHOT_RETENTION").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
            let mut snaps: Vec<u64> = Vec::new();
            let g = CHAIN.lock();
            for kv in g.db.scan_prefix("meta:snapshot:".as_bytes()) {
                if let Ok((k, _v)) = kv {
                    if let Ok(s) = String::from_utf8(k.to_vec()) {
                        if let Some(hs) = s.strip_prefix("meta:snapshot:") {
                            if let Ok(hv) = hs.parse::<u64>() { snaps.push(hv); }
                        }
                    }
                }
            }
            snaps.sort_unstable();
            let keep: Vec<u64> = snaps.into_iter().rev().take(retain).collect();
            drop(g); // release lock before pruning
            let g2 = CHAIN.lock();
            let removed = prune_old_undos_count(&g2.db, &keep);
                PROM_VISION_PRUNE_RUNS.inc();
                PROM_VISION_UNDOS_PRUNED.inc_by(removed);
        }
    });

    // --- gossip fanout channels (per-topic) ---
    // We'll create mpsc channels and store the Sender in the module-level OnceCell so handlers can enqueue without blocking.

    {
        // create channels and background workers
        let (tx_s, mut tx_rx) = tokio::sync::mpsc::channel::<Tx>(1024);
        let (blk_s, mut blk_rx) = tokio::sync::mpsc::channel::<Block>(256);
        let _ = TX_BCAST_SENDER.set(tx_s.clone());
        let _ = BLOCK_BCAST_SENDER.set(blk_s.clone());

        // peer-level gap between successive sends to avoid spamming peers (ms)
        let peer_gap_ms = std::env::var("VISION_GOSSIP_PEER_MS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(50);

        // tx fanout worker
        tokio::spawn(async move {
            while let Some(tx) = tx_rx.recv().await {
                let peers: Vec<String> = { let g = CHAIN.lock(); g.peers.iter().cloned().collect() };
                for p in peers {
                    let url = format!("{}/gossip/tx", p.trim_end_matches('/'));
                    let _ = HTTP.post(url).json(&serde_json::json!({ "tx": tx.clone() })).send().await;
                    tokio::time::sleep(Duration::from_millis(peer_gap_ms)).await;
                }
            }
        });

        // block fanout worker
        tokio::spawn(async move {
            while let Some(block) = blk_rx.recv().await {
                let peers: Vec<String> = { let g = CHAIN.lock(); g.peers.iter().cloned().collect() };
                for p in peers {
                    let url = format!("{}/gossip/block", p.trim_end_matches('/'));
                    let _ = HTTP.post(url).json(&serde_json::json!({ "block": block.clone() })).send().await;
                    tokio::time::sleep(Duration::from_millis(peer_gap_ms)).await;
                }
            }
        });
    }

    // Background: cleanup idle IP token buckets to bound memory
    tokio::spawn(async move {
        let ttl_secs = std::env::var("VISION_IP_BUCKET_TTL_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(300);
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let cutoff = now_ts().saturating_sub(ttl_secs);
            let mut to_remove: Vec<String> = Vec::new();
            for entry in IP_TOKEN_BUCKETS.iter() {
                if entry.value().last_ts < cutoff {
                    to_remove.push(entry.key().clone());
                }
            }
            for k in to_remove { IP_TOKEN_BUCKETS.remove(&k); }
        }
    });

    // Auto-bootstrap from VISION_BOOTNODES (comma-separated)
    if let Ok(boot) = std::env::var("VISION_BOOTNODES") {
        for raw in boot.split(',') {
            let u = raw.trim();
            if u.is_empty() { continue; }
            peers_add(u);
        }
    }

    // Spawn background peer hygiene loop (pings peers' /status and updates metadata)
    tokio::spawn(async move {
        peer_hygiene_loop().await;
    });

    // Spawn a background mempool sweeper controlled by VISION_MEMPOOL_SWEEP_SECS (default 60s)
    mempool::spawn_mempool_sweeper();
    // Log mempool TTL and sweeper interval for operator visibility
    {
        let ttl = mempool_ttl_secs();
        let sweep = std::env::var("VISION_MEMPOOL_SWEEP_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(60);
        info!(mempool_ttl_secs = ttl, mempool_sweep_secs = sweep, "mempool TTL sweeping configured");
    }

    
    // handlers are defined at module scope (moved)
    // Build the app/router
    let app = build_app();
    // Configure CORS: in dev allow Any, else use VISION_CORS_ORIGINS if provided
    let dev_mode = std::env::var("VISION_DEV").ok().as_deref() == Some("1");
    let cors = if dev_mode {
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    } else if let Ok(raw) = std::env::var("VISION_CORS_ORIGINS") {
        // parse comma-separated origins into HeaderValue list; invalid entries are skipped
        use tower_http::cors::AllowOrigin;
        let mut list: Vec<HeaderValue> = Vec::new();
        for part in raw.split(',').map(|s| s.trim()) {
            if part.is_empty() { continue; }
            if let Ok(hv) = HeaderValue::from_str(part) {
                list.push(hv);
            }
        }
        if list.is_empty() {
            // no valid origins -> deny cross-origin
            CorsLayer::new().allow_methods(Any)
        } else {
            CorsLayer::new().allow_origin(AllowOrigin::list(list)).allow_methods(Any).allow_headers(Any)
        }
    } else {
        // no public CORS allowed by default in prod
        CorsLayer::new().allow_methods(Any)
    };
    let app = app.layer(cors);

    let port: u16 = env::var("VISION_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(7070);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!(listen = %addr.to_string(), "Vision node listening");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(listen = %addr.to_string(), err = ?e, "failed to bind to address");
            std::process::exit(1);
        }
    };
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            let g = CHAIN.lock();
            let _ = g.db.flush();
        })
        .await
        .unwrap();
}

fn build_app() -> Router {
    // Apply global middleware: request body size limit and per-request timeout.
    // RequestBodyLimitLayer caps the incoming request body to N bytes (env VISION_MAX_BODY_BYTES).
    // TimeoutLayer enforces an overall request read timeout (env VISION_READ_TIMEOUT_SECS).
    let _body_limit: usize = std::env::var("VISION_MAX_BODY_BYTES").ok().and_then(|s| s.parse().ok()).unwrap_or(256*1024); // 256KB default
    let _timeout_secs: u64 = std::env::var("VISION_READ_TIMEOUT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);

    let base = Router::new();
    let svc = base
        .route("/panel_status", get(panel_status))
        .route("/panel_config", get(panel_config))
        .route("/metrics.prom", axum::routing::get(metrics_prom))
    .route("/peers/stats", axum::routing::get(peers_summary))
    .route("/peers/add", axum::routing::post(peers_add_handler))
    .route("/peers/list", axum::routing::get(peers_list))
    .route("/peers/ping", axum::routing::get(peers_ping))
        .route("/peers/evict_slow", axum::routing::post(peers_evict_slow))
        .route("/snapshot/save", post(snapshot_save))
        .route("/snapshot/latest", get(snapshot_latest))
        .route("/snapshot/download", get(snapshot_download))
    // Basic info
    .route("/health", get(|| async { "ok" }))
        .route("/config", get(get_config))
        .route("/height", get(get_height))
        .route("/block/latest", get(get_block_latest))
        .route("/head", get(head))
        .route("/mempool_size", get(mempool_size))
        .route("/status", get(status))
        .route("/block/last", get(get_block_latest))                // alias
        .route("/receipts/latest", get(get_receipts_latest))        // latest N receipts
        .route("/mempool", get(get_mempool))                        // mempool listing
        .route("/supply", get(supply))                              // total supply
        .route("/events/longpoll", get(events_longpoll))            // SSE-lite

        // State queries
        .route("/balance/:addr", get(get_balance))
    .route("/proof/balance/:addr", get(proof_balance))
        .route("/balances", get(get_balances_batch))
        .route("/nonce/:addr", get(get_nonce))
        // Admin / GM (consensus-safe)
        .route("/gamemaster", get(get_gamemaster))
        .route("/set_gamemaster", post(set_gamemaster_protected))
        .route("/airdrop", post(airdrop_protected))
        .route("/submit_admin_tx", post(submit_admin_tx))
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
        .route("/admin/ping", get(admin_ping_handler).post(admin_ping_handler))
        .route("/admin/info", get(admin_info).post(admin_info))
    .route("/admin/mempool/sweeper", get(admin_mempool_sweeper).post(admin_mempool_sweeper))
        // Explorer
        .route("/block/:height/tx_hashes", get(get_block_tx_hashes))
        .route("/block/:height", get(get_block))
        .route("/tx/:hash", get(get_tx))
        .route("/receipt/:hash", get(get_receipt))
        .route("/receipts", get(get_receipts_batch))
    .route("/openapi.yaml", get(openapi_spec))
        // Tx + mining
        .route("/submit_tx", post(submit_tx))
        .route("/mine_block", post(mine_block))
        // P2P
        .route("/peers", get(get_peers))
        .route("/peer/add", post(add_peer_protected))
        .route("/gossip/tx", post(gossip_tx))
        .route("/gossip/block", post(gossip_block))
        // Sync helpers
        .route("/sync/pull", post(sync_pull))
        .route("/sync/push", post(sync_push))
    // Dev-only tools (enabled by VISION_DEV=1 and X-Dev-Token or ?dev_token=)
    .route("/dev/faucet_mint", post(dev_faucet_mint))
    .route("/dev/spam_txs", post(dev_spam_txs));

    // Add a simple axum middleware that enforces a Content-Length based body limit
    // and a per-request timeout. This avoids pulling feature-gated tower-http layers
    // while providing the operational protections we want.
    use axum::middleware::Next;
    use axum::http::Request;
    use axum::response::IntoResponse;
    use axum::body::Body;

    async fn request_limits_middleware(req: Request<Body>, next: Next) -> impl IntoResponse {
            // read env per-request (cheap). Defaults: 256KB body, 10s timeout
            let body_limit: usize = std::env::var("VISION_MAX_BODY_BYTES").ok().and_then(|s| s.parse().ok()).unwrap_or(256*1024);
            let timeout_secs: u64 = std::env::var("VISION_READ_TIMEOUT_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);

        // If Content-Length is present and larger than allowed, reject early
        if let Some(clv) = req.headers().get(axum::http::header::CONTENT_LENGTH) {
            if let Ok(s) = clv.to_str() {
                if let Ok(n) = s.parse::<usize>() {
                    if n > body_limit {
                        return (axum::http::StatusCode::PAYLOAD_TOO_LARGE, "request body too large").into_response();
                    }
                }
            }
        }

        // Run the inner service with a timeout
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), next.run(req)).await {
            Ok(resp) => resp,
            Err(_) => (axum::http::StatusCode::GATEWAY_TIMEOUT, "request timed out").into_response(),
        }
    }

    // apply middleware to API routes
    let api = svc.layer(axum::middleware::from_fn(request_limits_middleware));

    // Serve static files from an absolute path. Prefer VISION_PUBLIC_DIR if set,
    // otherwise resolve relative to the running executable's directory and
    // fall back to the current working directory. This avoids depending on the
    // process working directory at startup.
    // Prefer an explicit override. If not set, resolve `public/` relative to
    // the running executable's location so releases that place `public/`
    // next to the binary will work without relying on a specific cwd.
    let public_dir: PathBuf = std::env::var("VISION_PUBLIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap())
                .join("public")
        });
    let index_file = public_dir.join("index.html");
    info!(public_dir = %public_dir.display(), "serving static files from");
    let static_files = Router::new().nest_service(
        "/",
        ServeDir::new(public_dir).not_found_service(ServeFile::new(index_file)),
    );

    // Mount API first, then version route, then static UI fallback.
    // This ensures API routes (e.g., /status) have priority and everything else
    // falls back to the single-page app entry.
    let app = Router::new()
        .merge(api)
        .merge(version::router())
        .merge(static_files);

    app
}


#[derive(Serialize)]
struct UiConfig {
    fee_base: u128,
    fee_per_recipient: u128,
    block_target_txs: usize,
    miner_require_sync: bool,
    miner_max_lag: u64,
}

async fn get_config() -> Json<UiConfig> {
    Json(UiConfig {
        fee_base: fee_base(),
        fee_per_recipient: fee_per_recipient(),
        block_target_txs: block_target_txs(),
        miner_require_sync: miner_require_sync(),
        miner_max_lag: miner_max_lag(),
    })
}

// ---- New: mempool listing ----
async fn get_mempool() -> Json<Vec<String>> {
    let g = CHAIN.lock();
    let mut v: Vec<String> = Vec::new();
    for t in g.mempool_critical.iter() { v.push(hex::encode(tx_hash(t))); }
    for t in g.mempool_bulk.iter() { v.push(hex::encode(tx_hash(t))); }
    Json(v)
}

// ---- New: receipts latest ----
async fn get_receipts_latest(Query(q): Query<std::collections::HashMap<String,String>>) -> Json<serde_json::Value> {
    // Cursor pagination: cursor is optional and encoded as "<height>:<txhash>" representing last-seen
    let limit = q.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(10).min(100);
    let cursor_opt = q.get("cursor").cloned();
    let g = CHAIN.lock();
    let mut v: Vec<(String, Receipt)> = Vec::new();
    for kv in g.db.scan_prefix(RCPT_PREFIX.as_bytes()) {
        let (k, bytes) = kv.expect("rcpt kv");
        if let Ok(r) = serde_json::from_slice::<Receipt>(&bytes) {
            let key = String::from_utf8(k.to_vec()).unwrap_or_default(); // rcpt:<txhash>
            let txh = key.trim_start_matches(RCPT_PREFIX).to_string();
            v.push((txh, r));
        }
    }
    // sort by height desc then txhash
    v.sort_by(|a,b| b.1.height.cmp(&a.1.height).then(b.0.cmp(&a.0)));

    // if cursor provided, skip until we find it
    let start_index = if let Some(cur) = cursor_opt {
        // cursor format: "<height>:<txhash>"
        let mut idx = 0usize;
        for (i, (txh, r)) in v.iter().enumerate() {
            let token = format!("{}:{}", r.height, txh);
            if token == cur { idx = i + 1; break; }
        }
        idx
    } else { 0 };

    let mut out: Vec<Receipt> = Vec::new();
    let mut next_cursor: Option<String> = None;
    for (_i, (_txh, r)) in v.into_iter().enumerate().skip(start_index) {
        if out.len() >= limit { next_cursor = Some(format!("{}:{}", r.height, _txh)); break; }
        out.push(r);
    }
    Json(serde_json::json!({ "receipts": out, "next_cursor": next_cursor }).into())
}

// ---- New: supply endpoint ----
async fn supply() -> String {
    let g = CHAIN.lock();
    let mut sum: u128 = 0;
    for (k, v) in &g.balances {
        if k.starts_with("acct:") { sum = sum.saturating_add(*v); }
    }
    sum.to_string()
}

// ---- New: long-poll events (SSE-lite) ----
async fn events_longpoll(Query(q): Query<std::collections::HashMap<String,String>>) -> Json<serde_json::Value> {
    let since = q.get("since").and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let deadline = std::time::Instant::now() + Duration::from_secs(25);
    loop {
        let (h, hash_opt) = {
            let g = CHAIN.lock();
            let h = g.blocks.last().map(|b| b.header.number).unwrap_or(0);
            let hash = g.blocks.last().map(|b| b.header.pow_hash.clone());
            (h, hash)
        };
        if h > since {
            return Json(serde_json::json!({"event":"new_block","height":h,"hash": hash_opt}));
        }
        if std::time::Instant::now() >= deadline {
            return Json(serde_json::json!({"event":"timeout","height":h}));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

// =================== Simple views ===================
async fn get_height() -> String {
    let g = CHAIN.lock();
    g.blocks.last().map(|b| b.header.number).unwrap_or(0).to_string()
}
async fn get_block_latest() -> Json<serde_json::Value> {
    let g = CHAIN.lock();
    if let Some(b) = g.blocks.last() { return Json(serde_json::json!(b)); }
    Json(serde_json::json!({"error":"empty chain"}))
}
async fn head() -> Json<HeadInfo> {
    let g = CHAIN.lock();
    let tip = g.blocks.last().unwrap();
    Json(HeadInfo {
        height: tip.header.number,
        hash: tip.header.pow_hash.clone(),
        time: tip.header.timestamp,
        state_root: tip.header.state_root.clone(),
    })
}
async fn mempool_size() -> String {
    let g = CHAIN.lock();
    format!("{}", g.mempool_critical.len() + g.mempool_bulk.len())
}

async fn status() -> Json<StatusView> {
    // Copy local snapshots without holding the mutex during HTTP
    let (height, peers, gating, max_lag, mempool_len) = {
        let g = CHAIN.lock();
        (
            g.blocks.last().map(|b| b.header.number).unwrap_or(0),
            g.peers.iter().cloned().collect::<Vec<_>>(),
            miner_require_sync(),
            miner_max_lag(),
            g.mempool_critical.len() + g.mempool_bulk.len(),
        )
    };

    // Query peers' heights
    let mut best_peer_h = height;
    for p in &peers {
        let url = format!("{}/height", p.trim_end_matches('/'));
        if let Ok(resp) = HTTP.get(url).send().await {
            if let Ok(text) = resp.text().await {
                if let Ok(h) = text.trim().parse::<u64>() {
                    if h > best_peer_h { best_peer_h = h; }
                }
            }
        }
    }
    let lag = best_peer_h as i64 - height as i64;
    let mining_allowed = if !gating { true } else { lag <= max_lag as i64 };

    Json(StatusView {
        height,
        best_peer_height: best_peer_h,
        lag,
        mempool: mempool_len,
        mining_allowed,
        gating,
        max_lag,
        peers,
    })
}

async fn get_balance(Path(addr): Path<String>) -> String {
    let g = CHAIN.lock();
    let key = acct_key(&addr);
    g.balances.get(&key).cloned().unwrap_or(0).to_string()
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct BalanceProof {
    pub addr: String,
    pub value: u128,
    pub leaf: String,
    pub root: String,
    pub path: Vec<(String, bool)>,
}

fn get_balance_proof(g: &Chain, addr: &str) -> Option<BalanceProof> {
    let key = acct_key(addr);
    let mut items: Vec<(String, u128)> = g.balances.iter().filter(|(k, _)| k.starts_with("acct:")).map(|(k,v)| (k.clone(), *v)).collect();
    items.sort_by(|a,b| a.0.cmp(&b.0));
    let pos = items.iter().position(|(k, _)| k == &key)?;
    let mut level: Vec<[u8;32]> = items.iter().map(|(k,v)| {
        let leaf = blake3::hash(format!("{}:{}", k, v).as_bytes());
        let mut arr = [0u8;32]; arr.copy_from_slice(leaf.as_bytes()); arr
    }).collect();
    let leaf_hex = hex::encode(level[pos]);
    let mut index = pos;
    let mut path: Vec<(String,bool)> = Vec::new();
    while level.len() > 1 {
        let mut next: Vec<[u8;32]> = Vec::new();
        for i in (0..level.len()).step_by(2) {
            let left = level[i];
            let right = if i+1 < level.len() { level[i+1] } else { level[i] };
            if index == i || index == i+1 {
                let sibling_idx = if index == i { i+1 } else { i };
                let sibling = if sibling_idx < level.len() { level[sibling_idx] } else { level[i] };
                let sibling_on_left = sibling_idx < index;
                path.push((hex::encode(sibling), sibling_on_left));
                index = next.len();
            }
            let mut hasher = blake3::Hasher::new();
            hasher.update(&left); hasher.update(&right);
            let out = hasher.finalize();
            let mut arr = [0u8;32]; arr.copy_from_slice(out.as_bytes());
            next.push(arr);
        }
        level = next;
    }
    let root_hex = if level.is_empty() { hex::encode([0u8;32]) } else { hex::encode(level[0]) };
    let value = items[pos].1;
    Some(BalanceProof { addr: addr.to_string(), value, leaf: leaf_hex, root: root_hex, path })
}

async fn proof_balance(Path(addr): Path<String>) -> (StatusCode, Json<serde_json::Value>) {
    let g = CHAIN.lock();
    if let Some(p) = get_balance_proof(&g, &addr) {
        return (StatusCode::OK, Json(serde_json::json!({"proof": p}))); 
    }
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": { "code": "not_found", "message": "account not found" } })))
}
async fn get_balances_batch(Query(q): Query<BalancesQuery>) -> Json<BTreeMap<String, String>> {
    let g = CHAIN.lock();
    let mut out = BTreeMap::new();
    for raw in q.addrs.split(',') {
        let addr = raw.trim();
        if addr.is_empty() { continue; }
        let key = acct_key(addr);
        let v = g.balances.get(&key).cloned().unwrap_or(0).to_string();
        out.insert(addr.to_string(), v);
    }
    Json(out)
}
async fn get_nonce(Path(addr): Path<String>) -> String {
    let g = CHAIN.lock();
    let key = acct_key(&addr);
    g.nonces.get(&key).cloned().unwrap_or(0).to_string()
}

// ----- GameMaster endpoints -----
async fn get_gamemaster() -> Json<GameMasterView> {
    let g = CHAIN.lock();
    Json(GameMasterView { gamemaster: g.gamemaster.clone() })
}

// Build an on-chain system/set_gamemaster tx (consensus-safe)
#[derive(Deserialize)]
struct SetGameMasterReq { addr: Option<String> }
async fn set_gamemaster_protected(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<SetGameMasterReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !check_admin(headers, &q) {
    return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid or missing admin token"})))
}

    let mut g = CHAIN.lock();
    let sender = g.gamemaster.clone().unwrap_or_default(); // "" means bootstrap
    let nonce = if let Some(ref gm) = g.gamemaster {
        let k = acct_key(gm);
        g.balances.entry(k.clone()).or_insert(0);
        g.nonces.entry(k.clone()).or_insert(0);
        *g.nonces.get(&k).unwrap_or(&0)
    } else { 0 };

    #[derive(Serialize, Deserialize)]
    struct SetArgs { addr: Option<String> }
    let args = serde_json::to_vec(&SetArgs { addr: req.addr.clone() }).unwrap();
    let tx = Tx {
        nonce,
        sender_pubkey: sender,
        access_list: vec![],
        module: "system".into(),
        method: "set_gamemaster".into(),
        args,
        tip: 0,
        fee_limit: 0,
        sig: String::new(),
    };

    let parent = g.blocks.last().cloned();
    let (block, _results) = execute_and_mine(&mut g, vec![tx], "miner", parent.as_ref());

    let peers: Vec<String> = g.peers.iter().cloned().collect();
    let block_clone = block.clone();
    tokio::spawn(async move {
        let _ = broadcast_block_to_peers(peers, block_clone).await;
    });

    (StatusCode::OK, Json(serde_json::json!({
        "gamemaster": g.gamemaster,
        "height": block.header.number,
        "hash": block.header.pow_hash
    })))
}

// ----- Tx submission (signature required) -----
async fn submit_tx(ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(SubmitTx { tx }): Json<SubmitTx>) -> impl axum::response::IntoResponse {
    let ip = addr.ip().to_string();
    let base_headers = mempool::build_rate_limit_headers(&ip);

    // quick preflight checks
    {
        let g = CHAIN.lock();
        if let Some(msg) = preflight_violation(&tx, &g) {
            return (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "status":"rejected", "error": { "code": "preflight", "message": msg } }))));
        }
    }

    // per-IP token bucket (submit endpoint)
    {
        let limits = { let g = CHAIN.lock(); g.limits.clone() };
        let mut entry = IP_TOKEN_BUCKETS.entry(ip.clone()).or_insert_with(|| {
            TokenBucket::new(limits.rate_submit_rps as f64, limits.rate_submit_rps as f64)
        });
        if !entry.value_mut().allow(1.0) {
            return rate_limited_response_with_headers(&base_headers, "ip_rate_limit");
        }
    }

    // basic rate limiting by peer (use sender_pubkey as proxy for peer id)
    if !peer_allow(&tx.sender_pubkey) {
        return rate_limited_response_with_headers(&base_headers, "peer_rate_limited");
    }

    match verify_tx(&tx) {
        Ok(_) => {
            PROM_VISION_GOSSIP_IN.inc();
            let mut g = CHAIN.lock();

            // Fast fail: ensure the tx's fee_limit covers base_fee * estimated_weight
            let weight = est_tx_weight(&tx) as u128;
            let base = fee_base();
            let need = base.saturating_mul(weight);
            let have = tx.fee_limit as u128;
            if have < need {
                let body = Json(serde_json::json!({
                    "status": "rejected",
                    "error": {
                        "code": "insufficient_fee_limit",
                        "message": "fee_limit too low",
                        "need_at_least": need.to_string(),
                        "have": have.to_string(),
                        "base_fee": base.to_string(),
                        "weight": weight
                    }
                }));
                return (base_headers.clone(), (StatusCode::BAD_REQUEST, body));
            }

            // Try RBF: allow replacing an existing (sender,nonce) tx only if incoming has strictly higher tip
            match mempool::try_replace_sender_nonce(&mut g, &tx) {
                Ok(true) => { /* replaced older tx; continue to insert incoming */ }
                Ok(false) => { /* no existing tx with same sender+nonce */ }
                Err(e) => {
                    if e == "rbf_tip_too_low" {
                        return (base_headers.clone(), (StatusCode::CONFLICT, Json(serde_json::json!({ "status":"rejected", "error": { "code": "rbf_tip_too_low", "message": "incoming tip not strictly higher than existing" } }))));
                    } else {
                        return (base_headers.clone(), (StatusCode::CONFLICT, Json(serde_json::json!({ "status":"rejected", "error": { "code": "rbf_replace_error", "message": e } }))));
                    }
                }
            }

            if let Err(e) = mempool::validate_for_mempool(&tx, &g) {
                return (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({"status":"rejected","error": { "code": "mempool_reject", "message": e } })))); 
            }

            // Admission check under load: if mempool near capacity, reject low-priority
            if let Err(reason) = mempool::admission_check_under_load(&g, &tx) {
                // log for operator visibility
                debug!(mempool="admit_reject", reason=%reason, tx_hash=%hex::encode(tx_hash(&tx)), tip=%tx.tip);
                return (base_headers.clone(), (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"rejected","error": { "code": "admission_reject", "message": reason } })))); 
            }

            // mempool cap with low-tip eviction — prefer evicting from bulk lane first
            let total_len = g.mempool_critical.len() + g.mempool_bulk.len();
            if total_len >= g.limits.mempool_max {
                if let Some(idx) = mempool::bulk_eviction_index(&g, &tx) {
                    g.mempool_bulk.remove(idx);
                } else if let Some((idx, min_tip)) = g.mempool_critical.iter().enumerate()
                    .min_by_key(|(_, t)| t.tip).map(|(i, t)| (i, t.tip)) {
                    if tx.tip > min_tip {
                        g.mempool_critical.remove(idx);
                    } else {
                        return (base_headers.clone(), (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"rejected","error": { "code": "mempool_full", "message": "mempool full; tip too low" } }))));
                    }
                } else {
                        return (base_headers.clone(), (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"rejected","error": { "code": "mempool_full", "message": "mempool full" } }))));
                }
            }

            // insert into mempool
            let h = hex::encode(tx_hash(&tx));
            if !g.seen_txs.insert(h.clone()) {
                return (base_headers.clone(), (StatusCode::OK, Json(serde_json::json!({"status":"ignored","reason":"duplicate"}))));
            }
            let critical_threshold: u64 = std::env::var("VISION_CRITICAL_TIP_THRESHOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(1000);
            if tx.tip >= critical_threshold {
                g.mempool_critical.push_back(tx.clone());
            } else {
                g.mempool_bulk.push_back(tx.clone());
            }
            let th = hex::encode(tx_hash(&tx));
            g.mempool_ts.insert(th, now_ts());

            // best-effort fanout via local channel or spawn
            if let Some(sender) = TX_BCAST_SENDER.get() {
                let _ = sender.try_send(tx.clone());
            } else {
                let peers: Vec<String> = g.peers.iter().cloned().collect();
                let tx_clone = tx.clone();
                tokio::spawn(async move { let _ = broadcast_tx_to_peers(peers, tx_clone).await; });
            }

            (base_headers.clone(), (StatusCode::OK, Json(serde_json::json!({"status":"accepted","tx_hash": hex::encode(tx_hash(&tx))}))))
        }
        Err(e) => (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "status":"rejected","error": { "code": "bad_sig", "message": e.to_string() } })))),
    }
}
fn verify_tx(tx: &Tx) -> Result<(), NodeError> {
    if serde_json::to_vec(tx).map_err(|_| NodeError::Json)?.len() > 64 * 1024 {
        return Err(NodeError::TxTooBig);
    }
    let pubkey_bytes = decode_hex32(&tx.sender_pubkey).map_err(|_| NodeError::BadSig)?;
    let vk = PublicKey::from_bytes(&pubkey_bytes).map_err(|_| NodeError::BadSig)?;
    let sig_bytes = decode_hex64(&tx.sig).map_err(|_| NodeError::BadSig)?;
    let sig = Signature::from_bytes(&sig_bytes).map_err(|_| NodeError::BadSig)?;
    vk.verify(&signable_tx_bytes(tx), &sig).map_err(|_| NodeError::BadSig)?;
    Ok(())
}

// ----- Mining -----
#[derive(Deserialize)]
struct MineReq { max_txs: Option<usize>, miner_addr: Option<String> }
async fn mine_block(Json(req): Json<MineReq>) -> (StatusCode, Json<serde_json::Value>) {
    // mining gate: require sync if configured
    let gating = miner_require_sync();
    let max_lag = miner_max_lag();

    // compute local height and best peer height
    let (height, peers) = {
        let g = CHAIN.lock();
        (g.blocks.last().unwrap().header.number, g.peers.iter().cloned().collect::<Vec<_>>())
    };
    let mut best_peer_h = height;
    for p in &peers {
        if let Ok(resp) = HTTP.get(format!("{}/height", p.trim_end_matches('/'))).send().await {
            if let Ok(text) = resp.text().await {
                if let Ok(h) = text.trim().parse::<u64>() {
                    if h > best_peer_h { best_peer_h = h; }
                }
            }
        }
    }
    let lag = best_peer_h as i64 - height as i64;
    if gating && lag > max_lag as i64 {
        return (StatusCode::CONFLICT, Json(serde_json::json!({
            "error": format!("mining gated: lag {} > max_lag {}", lag, max_lag),
            "height": height, "best_peer_height": best_peer_h
        })));
    }

    let mut g = CHAIN.lock();
    let parent = g.blocks.last().unwrap().clone();

    // Select transactions for block using block-builder
    let max_txs = req.max_txs.unwrap_or(500);
    let weight_limit = g.limits.block_weight_limit;
    let txs = mempool::build_block_from_mempool(&mut g, max_txs, weight_limit);

    let miner_addr = req.miner_addr.unwrap_or_else(|| "miner".to_string());
    let (block, _exec_results) = execute_and_mine(&mut g, txs, &miner_addr, Some(&parent));

    // prune mempool after mining (drop stale nonces)
    mempool::prune_mempool(&mut g);

    // ---- Auto-nudge base fee based on block fullness ----
    {
        let included = block.txs.len();
        let target = block_target_txs().max(1);
        let util = (included as f64) / (target as f64);
        let high = block_util_high();
        let low = block_util_low();
        let mut base = fee_base();
        if util > high {
            base = base.saturating_add(1);
        } else if util < low && base > 1 {
            base = base.saturating_sub(1);
        }
        // update global and persist
        *FEE_BASE.lock() = base;
        persist_fee_base(&g.db, base);
    }

    // Broadcast: enqueue via fanout channels if available, otherwise fallback to immediate spawn
    if let Some(tx_sender) = TX_BCAST_SENDER.get() {
        for tx in block.txs.iter() {
            let _ = tx_sender.try_send(tx.clone());
        }
    } else {
        let peers = g.peers.iter().cloned().collect::<Vec<_>>();
        let txs_clone = block.txs.clone();
        tokio::spawn(async move {
            for tx in txs_clone {
                let _ = broadcast_tx_to_peers(peers.clone(), tx).await;
            }
        });
    }
    if let Some(blk_sender) = BLOCK_BCAST_SENDER.get() {
        let _ = blk_sender.try_send(block.clone());
    } else {
        let peers = g.peers.iter().cloned().collect::<Vec<_>>();
        let blk_clone = block.clone();
        tokio::spawn(async move { let _ = broadcast_block_to_peers(peers, blk_clone).await; });
    }

    (StatusCode::OK, Json(serde_json::json!({
        "height": block.header.number,
        "hash": block.header.pow_hash,
        "state_root": block.header.state_root,
        "miner_addr": miner_addr,
        "txs": block.txs.len()
    })))
}

// Mempool-related helpers (build/prune/admission) moved to `src/mempool.rs`.
// Calls in this file use the `mempool::` namespace.

// See `src/mempool.rs::bulk_eviction_index`.

// ----- Admin utilities (no signature; token required) -----
#[derive(Deserialize)]
struct AdminTxReq { tx: Tx, miner_addr: Option<String> }

async fn submit_admin_tx(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<AdminTxReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !check_admin(headers, &q) {
    return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid or missing admin token"})))
}

    let mut g = CHAIN.lock();
    let miner_addr = req.miner_addr.unwrap_or_else(|| "miner".to_string());
    let parent = g.blocks.last().cloned();
    let (block, _exec_results) = execute_and_mine(&mut g, vec![req.tx.clone()], &miner_addr, parent.as_ref());

    prune_mempool(&mut g);

    if let Some(sender_blk) = once_cell::sync::OnceCell::get(&BLOCK_BCAST_SENDER) {
        let _ = sender_blk.try_send(block.clone());
    }

    (StatusCode::OK, Json(serde_json::json!({
        "height": block.header.number,
        "hash": block.header.pow_hash,
        "txs": block.txs.len()
    })))
}

#[derive(Deserialize)]
struct CashMintArgs { to: String, amount: u128 }
#[derive(Deserialize)]
struct CashTransferArgs { to: String, amount: u128 }
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Payment { to: String, amount: u128 }
// New for multi_mint (GM-only)
#[derive(Deserialize)]
struct CashMultiMintArgs { mints: Vec<Payment> }

// Protected CSV/JSON airdrop → builds a single cash/multi_mint (GM-only) and mines it.
async fn airdrop_protected(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<AirdropReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !check_admin(headers, &q) {
    return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid or missing admin token"})))
}
    let mut mints: Vec<Payment> = vec![];
    if let Some(list) = req.payments {
        mints = list;
    } else if let Some(csv) = req.payments_csv {
        for (lineno, line) in csv.lines().enumerate() {
            let t = line.trim();
            if t.is_empty() { continue; }
            let parts: Vec<&str> = t.split(',').map(|s| s.trim()).collect();
            if parts.len() != 2 { return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("bad csv at line {}", lineno+1)}))); }
            let addr = parts[0].to_string();
            let amount: u128 = parts[1].parse().unwrap_or_default();
            if amount == 0 { return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("zero/invalid amount at line {}", lineno+1)}))); }
            mints.push(Payment { to: addr, amount });
        }
    } else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"no payments provided"})))
    }
    if mints.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"empty payments"})))
    }

    let mut g = CHAIN.lock();
    let gm = if let Some(s) = g.gamemaster.clone() {
        s
    } else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"no gamemaster set"})));
    };
    let gm_key = acct_key(&gm);
    g.balances.entry(gm_key.clone()).or_insert(0);
    g.nonces.entry(gm_key.clone()).or_insert(0);
    let nonce = *g.nonces.get(&gm_key).unwrap_or(&0);

    // Build multi_mint
    let access_list: Vec<String> = mints.iter().map(|p| format!("acct:{}", p.to)).collect();
    let args = serde_json::to_vec(&serde_json::json!({ "mints": mints })).unwrap();
    let mut tx = Tx {
        nonce,
        sender_pubkey: gm.clone(),
        access_list,
        module: "cash".into(),
        method: "multi_mint".into(),
        args,
        tip: 0,
        fee_limit: 0,
        sig: String::new(),
    };
    tx = apply_optional_tip(tx, req.tip);

    let miner_addr = req.miner_addr.unwrap_or_else(|| "miner".to_string());
    let parent = g.blocks.last().cloned();
    let (block, _results) = execute_and_mine(&mut g, vec![tx.clone()], &miner_addr, parent.as_ref());

    prune_mempool(&mut g);

    let peers: Vec<String> = g.peers.iter().cloned().collect();
    let block_clone = block.clone();
    tokio::spawn(async move {
        let _ = broadcast_block_to_peers(peers, block_clone).await;
    });

    (StatusCode::OK, Json(serde_json::json!({
        "status":"ok",
        "height": block.header.number,
        "hash": block.header.pow_hash,
        "tx_hash": hex::encode(tx_hash(&tx)),
        "receipt": format!("/receipt/{}", hex::encode(tx_hash(&tx)))
    })))
}

// =================== Explorer ===================
async fn get_block(Path(height): Path<u64>) -> Json<serde_json::Value> {
    let g = CHAIN.lock();
    if let Some(b) = g.blocks.get(height as usize) { return Json(serde_json::json!(b)); }
    Json(serde_json::json!({"error":"block not found"}))
}
async fn get_block_tx_hashes(Path(height): Path<u64>) -> Json<serde_json::Value> {
    let g = CHAIN.lock();
    if let Some(b) = g.blocks.get(height as usize) {
        let v: Vec<String> = b.txs.iter().map(|t| hex::encode(tx_hash(t))).collect();
        return Json(serde_json::json!({ "height": height, "tx_hashes": v }));
    }
    Json(serde_json::json!({"error":"block not found"}))
}
async fn get_tx(Path(hash_hex): Path<String>) -> Json<serde_json::Value> {
    let g = CHAIN.lock();
    for b in g.blocks.iter().rev() {
      for tx in &b.txs {
        if hex::encode(tx_hash(tx)) == hash_hex {
                    return Json(serde_json::json!({
                        "height": b.header.number, "block_hash": b.header.pow_hash, "tx": tx
                    }));
                }
      }
    }
    Json(serde_json::json!({"error": { "code": "not_found", "message": "tx not found" }}))
}
async fn get_receipt(Path(hash_hex): Path<String>) -> Json<serde_json::Value> {
    let g = CHAIN.lock();
    let key = format!("{}{}", RCPT_PREFIX, hash_hex);
    if let Some(v) = g.db.get(key.as_bytes()).unwrap() {
        if let Ok(r) = serde_json::from_slice::<Receipt>(&v) {
            return Json(serde_json::json!(r));
        }
    }
    Json(serde_json::json!({"error": { "code": "not_found", "message": "receipt not found" }}))
}
async fn get_receipts_batch(Query(q): Query<ReceiptsQuery>) -> Json<BTreeMap<String, serde_json::Value>> {
    let g = CHAIN.lock();
    let mut out = BTreeMap::new();
    for raw in q.hashes.split(',') {
        let h = raw.trim();
        if h.is_empty() { continue; }
        let key = format!("{}{}", RCPT_PREFIX, h);
        if let Some(v) = g.db.get(key.as_bytes()).unwrap() {
            if let Ok(r) = serde_json::from_slice::<Receipt>(&v) {
                out.insert(h.to_string(), serde_json::json!(r));
                continue;
            }
        }
        out.insert(h.to_string(), serde_json::json!({"error":"receipt not found"}));
    }
    Json(out)
}

// =================== Execution & Rules ===================
fn require_access(list: &[String], needed: impl IntoIterator<Item = impl AsRef<str>>) -> Result<(), String> {
    for k in needed {
        let key = k.as_ref();
        if !list.iter().any(|s| s == key) {
            return Err(format!("missing access key: {key}"));
        }
    }
    Ok(())
}
fn acct_key(addr: &str) -> String { format!("acct:{addr}") }

fn fee_for_transfer(_count: usize, tip: u64) -> (u128, u128) {
    let base = fee_base();
    let per = fee_per_recipient();
    let fee = base + per; // count=1
    (fee + tip as u128, fee + tip as u128)
}
#[allow(dead_code)]
fn fee_for_multi(count: usize, tip: u64) -> (u128, u128) {
    let base = fee_base();
    let per = fee_per_recipient();
    let fee = base + per.saturating_mul(count as u128);
    (fee + tip as u128, fee + tip as u128)
}

/// Execute tx; mutates balances/nonces; may update gm via system/set_gamemaster.
fn execute_tx_with_nonce_and_fees(
    tx: &Tx,
    balances: &mut BTreeMap<String, u128>,
    nonces: &mut BTreeMap<String, u64>,
    miner_key: &str,
    gm: &mut Option<String>,
) -> Result<(), String> {
    let sender_addr = &tx.sender_pubkey;
    let from_key = acct_key(sender_addr);
    balances.entry(from_key.clone()).or_insert(0);
    nonces.entry(from_key.clone()).or_insert(0);
    let expected = *nonces.get(&from_key).unwrap_or(&0);
    if tx.nonce != expected {
        return Err(format!("bad nonce: got {}, want {}", tx.nonce, expected));
    }

    match tx.module.as_str() {
        "system" => {
            match tx.method.as_str() {
                "set_gamemaster" => {
                    #[derive(Deserialize)]
                    struct SetArgs { addr: Option<String> }
                    let args: SetArgs = serde_json::from_slice(&tx.args).map_err(|_| "bad args".to_string())?;
                    // Authorization: if a GM exists, only that GM can change it. If no GM yet, allow bootstrap.
                    if let Some(current) = gm.clone() {
                        if current != *sender_addr {
                            return Err("not authorized: only current gamemaster may change GM".into());
                        }
                    }
                    *gm = args.addr.clone();
                    *nonces.get_mut(&from_key).unwrap() = expected + 1;
                    Ok(())
                }
                _ => Err("unsupported system method".into())
            }
        }
        "cash" => {
            match tx.method.as_str() {
                "mint" => {
                    if let Some(gmaddr) = gm.as_ref() {
                        if gmaddr != sender_addr {
                            return Err("mint not authorized".into());
                        }
                    } else {
                        return Err("mint disabled: no gamemaster set".into());
                    }
                    let args: CashMintArgs = serde_json::from_slice(&tx.args).map_err(|_| "bad args".to_string())?;
                    let to_key = acct_key(&args.to);
                    // access control: must include acct:<to>
                    require_access(&tx.access_list, [&to_key])?;
                    let to_bal = balances.entry(to_key).or_insert(0);
                    *to_bal = (*to_bal).saturating_add(args.amount);
                    *nonces.get_mut(&from_key).unwrap() = expected + 1;
                    Ok(())
                }
                "transfer" => {
                    let args: CashTransferArgs = serde_json::from_slice(&tx.args).map_err(|_| "bad args".to_string())?;
                    let to_key = acct_key(&args.to);
                    require_access(&tx.access_list, [&from_key, &to_key])?;

                    let (fee_and_tip, miner_reward) = fee_for_transfer(1, tx.tip);
                    let total_cost = (args.amount as u128).saturating_add(fee_and_tip);

                    let from_bal = balances.entry(from_key.clone()).or_insert(0);
                    if *from_bal < total_cost { return Err("insufficient funds (amount+fee+tip)".into()); }

                    *from_bal -= total_cost;
                    let to_bal = balances.entry(to_key).or_insert(0);
                    *to_bal = (*to_bal).saturating_add(args.amount);

                    let miner_bal = balances.entry(miner_key.to_string()).or_insert(0);
                    *miner_bal = (*miner_bal).saturating_add(miner_reward);

                    *nonces.get_mut(&from_key).unwrap() = expected + 1;
                    Ok(())
                }
                "multi_mint" => {
                    // GM-only mint to many accounts; no miner reward / fee
                    match gm.clone() {
                        None => return Err("multi_mint disabled: no gamemaster set".into()),
                        Some(gmaddr) if gmaddr != *sender_addr => return Err("multi_mint not authorized".into()),
                        _ => {}
                    }
                    let args: CashMultiMintArgs = serde_json::from_slice(&tx.args).map_err(|_| "bad args".to_string())?;
                    if args.mints.is_empty() { return Err("no mints".into()); }

                    // require access for each destination
                    let to_keys: Vec<String> = args.mints.iter().map(|p| acct_key(&p.to)).collect();
                    require_access(&tx.access_list, to_keys.iter().map(|s| s.as_str()))?;

                    for p in &args.mints {
                        let k = acct_key(&p.to);
                        let to_bal = balances.entry(k).or_insert(0);
                        *to_bal = (*to_bal).saturating_add(p.amount);
                    }
                    *nonces.get_mut(&from_key).unwrap() = expected + 1;
                    Ok(())
                }
                _ => Err("unsupported cash method".into()),
            }
        }
        _ => Err("unsupported module".into()),
    }
}

// Common executor+miner used by mine_block/admin endpoints
fn execute_and_mine(
    g: &mut Chain,
    txs: Vec<Tx>,
    miner_addr: &str,
    parent_opt: Option<&Block>,
) -> (Block, BTreeMap<String, Result<(), String>>) {
    let parent = parent_opt.cloned().unwrap_or_else(|| g.blocks.last().unwrap().clone());

    let mut balances = g.balances.clone();
    let mut nonces = g.nonces.clone();
    let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
    let mut gm = g.gamemaster.clone(); // LOCAL gm that can be changed by txs this block

    let miner_key = acct_key(miner_addr);
    balances.entry(miner_key.clone()).or_insert(0);
    nonces.entry(miner_key.clone()).or_insert(0);

    for tx in &txs {
        let h = hex::encode(tx_hash(tx));
        let res = execute_tx_with_nonce_and_fees(
            tx, &mut balances, &mut nonces, &miner_key, &mut gm
        );
        exec_results.insert(h, res);
    }

    let new_state_root = compute_state_root(&balances, &gm);
    let tx_root = if txs.is_empty() { parent.header.tx_root.clone() } else { tx_root_placeholder(&txs) };

    let mut hdr = BlockHeader {
        parent_hash: parent.header.pow_hash.clone(),
        number: parent.header.number + 1,
        timestamp: now_ts(),
        // dynamic difficulty based on recent blocks
        difficulty: current_difficulty_bits(g),
        nonce: 0,
        pow_hash: "0".repeat(64),
        state_root: new_state_root.clone(),
        tx_root,
        // fill receipts_root after exec results and pow are available
        receipts_root: parent.header.receipts_root.clone(),
        da_commitment: None,
    };

    let mut nonce_ctr = 0u64;
    loop {
        hdr.nonce = nonce_ctr;
        let h = hash_bytes(&header_pow_bytes(&hdr));
        // simple PoW: require first byte zero
        if meets_difficulty_bits(h, hdr.difficulty) { hdr.pow_hash = hex32(h); break; }
        nonce_ctr = nonce_ctr.wrapping_add(1);
    }

    // Build receipts (in the same tx order) and compute a Merkle root over their hashes.
    // Each receipt commits to (ok, error, height, block_hash).
    let receipt_height = parent.header.number + 1;
    let mut receipts_vec: Vec<Receipt> = Vec::new();
    for tx in &txs {
        let th = hex::encode(tx_hash(tx));
        if let Some(res) = exec_results.get(&th) {
            let r = Receipt { ok: res.is_ok(), error: res.clone().err(), height: receipt_height, block_hash: hdr.pow_hash.clone() };
            receipts_vec.push(r);
        } else {
            // should not happen; create a negative receipt
            let r = Receipt { ok: false, error: Some("missing exec result".to_string()), height: receipt_height, block_hash: hdr.pow_hash.clone() };
            receipts_vec.push(r);
        }
    }
    // compute merkle root of receipts: leaf = blake3(serialized_receipt)
    let receipts_root = if receipts_vec.is_empty() {
        "0".repeat(64)
    } else {
        let mut level: Vec<[u8;32]> = Vec::with_capacity(receipts_vec.len());
        for r in &receipts_vec {
            let bytes = serde_json::to_vec(r).unwrap_or_default();
            let hash = blake3::hash(&bytes);
            let mut arr = [0u8;32]; arr.copy_from_slice(hash.as_bytes());
            level.push(arr);
        }
        while level.len() > 1 {
            let mut next: Vec<[u8;32]> = Vec::with_capacity((level.len()+1)/2);
            for i in (0..level.len()).step_by(2) {
                let left = level[i];
                let right = if i+1 < level.len() { level[i+1] } else { level[i] };
                let mut h = Hasher::new();
                h.update(&left);
                h.update(&right);
                let out = h.finalize();
                let mut arr = [0u8;32]; arr.copy_from_slice(out.as_bytes());
                next.push(arr);
            }
            level = next;
        }
        hex32(level[0])
    };
    hdr.receipts_root = receipts_root;

    // Accept new state
    // compute undo deltas
    let undo = compute_undo(&g.balances, &g.nonces, &g.gamemaster, &balances, &nonces, &gm);
    persist_undo(&g.db, parent.header.number + 1, &undo);
    g.balances = balances.clone();
    g.nonces = nonces.clone();
    g.gamemaster = gm.clone();

    let mut block = Block { header: hdr, txs, weight: 0 };
    // compute serialized weight and record
        if let Ok(bts) = serde_json::to_vec(&block) {
        let w = bts.len() as u64;
        block.weight = w;
        PROM_VISION_BLOCK_WEIGHT_LAST.set(w as i64);
    }

    persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
    persist_block_only(&g.db, block.header.number, &block);

    for (txh, res) in exec_results.iter() {
        let r = Receipt {
            ok: res.is_ok(),
            error: res.clone().err(),
            height: block.header.number,
            block_hash: block.header.pow_hash.clone(),
        };
        let key = format!("{}{}", RCPT_PREFIX, txh);
        let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
    }
    let _ = g.db.flush();

    g.blocks.push(block.clone());
    // update EMA and retarget difficulty
    let observed_interval = if g.blocks.len() >= 2 {
        let len = g.blocks.len();
        let prev_ts = g.blocks[len-2].header.timestamp as f64;
        let cur_ts = g.blocks[len-1].header.timestamp as f64;
        (cur_ts - prev_ts).max(1.0)
    } else { g.limits.target_block_time as f64 };
    let alpha = 0.3_f64;
    g.ema_block_time = alpha * observed_interval + (1.0 - alpha) * g.ema_block_time;
    let win = g.limits.retarget_window as usize;
    if g.blocks.len() >= win {
        let target = g.limits.target_block_time as f64;
        let cur = g.difficulty as f64;
        let scale = (target / g.ema_block_time).max(0.25).min(4.0);
        let max_change = 0.25_f64;
        let mut factor = scale;
        if factor > 1.0 + max_change { factor = 1.0 + max_change; }
        if factor < 1.0 - max_change { factor = 1.0 - max_change; }
        let mut next = (cur * factor).round() as u64;
        if next < 1 { next = 1; }
        if next > 248 { next = 248; }
        g.difficulty = next;
    }
    // persist EMA & difficulty
    persist_ema(&g.db, g.ema_block_time);
    persist_difficulty(&g.db, g.difficulty);
    // metrics
    PROM_VISION_BLOCKS_MINED.inc();
    let txs_last = g.blocks.last().unwrap().txs.len() as u64;
    PROM_VISION_TXS_APPLIED.inc_by(txs_last);
    (block, exec_results)
}

// Apply incoming block from peer (with GM in consensus)
fn apply_block_from_peer(g: &mut Chain, blk: &Block) -> Result<(), String> {
    // dedupe
    if g.seen_blocks.contains(&blk.header.pow_hash) {
        return Err("duplicate block".into());
    }

    // Verify PoW first
    let mut hdr = blk.header.clone();
    let original_pow = hdr.pow_hash.clone();
    hdr.pow_hash = "0".repeat(64);
    let h = hash_bytes(&header_pow_bytes(&hdr));
    if !meets_difficulty_bits(h, hdr.difficulty) || hex32(h) != original_pow {
        return Err("invalid PoW".into());
    }

    // Insert into side_blocks (we'll decide whether to reorg)
    g.side_blocks.insert(blk.header.pow_hash.clone(), blk.clone());
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    // compute cumulative work for this block
    let parent_cum = g.cumulative_work.get(&blk.header.parent_hash).cloned().unwrap_or(0);
    let my_cum = parent_cum.saturating_add(block_work(blk.header.difficulty));
    g.cumulative_work.insert(blk.header.pow_hash.clone(), my_cum);

    // find heaviest tip among current main tip and side blocks
    let mut heaviest_hash = g.blocks.last().unwrap().header.pow_hash.clone();
    let mut heaviest_work = *g.cumulative_work.get(&heaviest_hash).unwrap_or(&0);
    for (hsh, w) in g.cumulative_work.iter() {
        if *w > heaviest_work {
            heaviest_work = *w;
            heaviest_hash = hsh.clone();
        }
    }

    // if heaviest is current tip, we're done (no reorg)
    let current_tip_hash = g.blocks.last().unwrap().header.pow_hash.clone();
    if heaviest_hash == current_tip_hash {
        // if block extends current tip, append it
        if blk.header.parent_hash == current_tip_hash {
            // execute and append
            let mut balances = g.balances.clone();
            let mut nonces = g.nonces.clone();
            let mut gm = g.gamemaster.clone();
            let miner_key = acct_key("miner");
            balances.entry(miner_key.clone()).or_insert(0);
            nonces.entry(miner_key.clone()).or_insert(0);
            let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
            for tx in &blk.txs {
                let h = hex::encode(tx_hash(tx));
                let res = execute_tx_with_nonce_and_fees(tx, &mut balances, &mut nonces, &miner_key, &mut gm);
                exec_results.insert(h, res);
            }
            let new_state_root = compute_state_root(&balances, &gm);
            if new_state_root != blk.header.state_root { return Err("state_root mismatch".into()); }
            let tip = g.blocks.last().unwrap();
            let tx_root = if blk.txs.is_empty() { tip.header.tx_root.clone() } else { tx_root_placeholder(&blk.txs) };
            if tx_root != blk.header.tx_root { return Err("tx_root mismatch".into()); }
            // Accept
            g.balances = balances; g.nonces = nonces; g.gamemaster = gm;
            for (txh, res) in exec_results.iter() {
                let r = Receipt { ok: res.is_ok(), error: res.clone().err(), height: blk.header.number, block_hash: blk.header.pow_hash.clone() };
                let key = format!("{}{}", RCPT_PREFIX, txh);
                let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
            }
            persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
            persist_block_only(&g.db, blk.header.number, blk);
            // update last-seen block weight metric
            PROM_VISION_BLOCK_WEIGHT_LAST.set(blk.weight as i64);
            let _ = g.db.flush();
            g.blocks.push(blk.clone());
            g.seen_blocks.insert(blk.header.pow_hash.clone());
            info!(block = %blk.header.pow_hash, height = blk.header.number, "accepted block");
            // snapshot periodically (env-driven cadence)
            if g.limits.snapshot_every_blocks > 0 && (g.blocks.len() as u64) % g.limits.snapshot_every_blocks == 0 {
                persist_snapshot(&g.db, blk.header.number, &g.balances, &g.nonces, &g.gamemaster);
            }
            // update EMA and possibly retarget difficulty (same logic as local mining)
            let observed_interval = if g.blocks.len() >= 2 {
                let len = g.blocks.len();
                let prev_ts = g.blocks[len-2].header.timestamp as f64;
                let cur_ts = g.blocks[len-1].header.timestamp as f64;
                (cur_ts - prev_ts).max(1.0)
            } else { g.limits.target_block_time as f64 };
            let alpha = 0.3_f64;
            g.ema_block_time = alpha * observed_interval + (1.0 - alpha) * g.ema_block_time;
            let win = g.limits.retarget_window as usize;
            if g.blocks.len() >= win {
                let target = g.limits.target_block_time as f64;
                let cur = g.difficulty as f64;
                let scale = (target / g.ema_block_time).max(0.25).min(4.0);
                let max_change = 0.25_f64;
                let mut factor = scale;
                if factor > 1.0 + max_change { factor = 1.0 + max_change; }
                if factor < 1.0 - max_change { factor = 1.0 - max_change; }
                let mut next = (cur * factor).round() as u64;
                if next < 1 { next = 1; }
                if next > 248 { next = 248; }
                g.difficulty = next;
            }
            persist_ema(&g.db, g.ema_block_time);
            persist_difficulty(&g.db, g.difficulty);
            PROM_VISION_BLOCKS_MINED.inc();
            let _txs = blk.txs.len() as u64;
            PROM_VISION_TXS_APPLIED.inc_by(_txs);
        }
        return Ok(());
    }

    // Heavier chain found -> perform reorg to heaviest_hash
    info!(heaviest = %heaviest_hash, "reorg: adopting heavier tip");
    PROM_VISION_REORGS.inc();
    // MAX_REORG guard: don't accept reorganizations that are too large
    let max_reorg = g.limits.max_reorg;
    let old_tip_index = g.blocks.len().saturating_sub(1);
    // compute ancestor index (we compute it below, but we can preliminarily check by walking back from heaviest_hash)
    // For safety, find the ancestor as we already compute path; we'll compute path first then check length.
    let reorg_start = std::time::Instant::now();
    // Build path from heaviest_hash back to a block in current main chain
    let mut path: Vec<String> = Vec::new();
    let mut cursor = heaviest_hash.clone();
    loop {
        if g.blocks.iter().any(|b| b.header.pow_hash == cursor) { break; }
        path.push(cursor.clone());
        if let Some(b) = g.side_blocks.get(&cursor) {
            cursor = b.header.parent_hash.clone();
        } else {
            // missing parent, cannot adopt
            return Err("missing parent for candidate tip".into());
        }
    }
    // cursor is now ancestor hash that exists in main chain (could be genesis)
    let ancestor_hash = cursor.clone();
    // find ancestor index in main chain
    let ancestor_index = g.blocks.iter().position(|b| b.header.pow_hash == ancestor_hash).unwrap();

    // Now check the reorg size: old_tip_index - ancestor_index
    if old_tip_index.saturating_sub(ancestor_index) as u64 > max_reorg {
    PROM_VISION_REORG_REJECTED.inc();
        return Err(format!("reorg too large: {} > max {}", old_tip_index.saturating_sub(ancestor_index), max_reorg));
    }

    // compute orphaned blocks (old main blocks after ancestor)
    let orphaned: Vec<Block> = if ancestor_index + 1 <= g.blocks.len() - 1 {
        g.blocks.iter().skip(ancestor_index + 1).cloned().collect()
    } else { Vec::new() };

    // First try fast rollback using per-block undos
    let mut undo_ok = true;
    let old_tip_index = g.blocks.len().saturating_sub(1);
    for h in (ancestor_index + 1..=old_tip_index).rev() {
        let height = g.blocks[h].header.number;
        if let Some(undo) = load_undo(&g.db, height) {
            // apply undo: revert balances
            for (k, vopt) in undo.balances.iter() {
                match vopt {
                    Some(v) => { g.balances.insert(k.clone(), *v); }
                    _ => { g.balances.remove(k); }
                }
            }
            // revert nonces
            for (k, vopt) in undo.nonces.iter() {
                match vopt {
                    Some(v) => { g.nonces.insert(k.clone(), *v); }
                    _ => { g.nonces.remove(k); }
                }
            }
            // revert gamemaster if present
            if let Some(prev_gm_opt) = &undo.gamemaster {
                g.gamemaster = prev_gm_opt.clone();
            }
            // drop the block from in-memory chain
            g.blocks.pop();
        } else {
            // missing undo for this height -> need snapshot fallback
            undo_ok = false;
            break;
        }
    }

    // If undos were not available for a full fast rollback, fallback to snapshot+replay
    if !undo_ok {
        // look for the best snapshot <= ancestor_index
        let mut best_snap: Option<u64> = None;
        for kv in g.db.scan_prefix("meta:snapshot:".as_bytes()) {
            if let Ok((k, _v)) = kv {
                if let Ok(s) = String::from_utf8(k.to_vec()) {
                    if let Some(hs) = s.strip_prefix("meta:snapshot:") {
                        if let Ok(hv) = hs.parse::<u64>() {
                            if hv <= g.blocks[ancestor_index].header.number {
                                best_snap = Some(best_snap.map_or(hv, |b| b.max(hv)));
                            }
                        }
                    }
                }
            }
        }
        if best_snap.is_none() {
            return Err("missing undos and no usable snapshot for rollback".into());
        }
        let snap_h = best_snap.unwrap();
        // load snapshot contents
        let snap_key = format!("meta:snapshot:{}", snap_h);
        let snap_bytes = g.db.get(snap_key.as_bytes()).unwrap().ok_or_else(|| "failed to read snapshot".to_string())?;
        let snap_val: serde_json::Value = serde_json::from_slice(&snap_bytes).map_err(|e| e.to_string())?;
        let balances: BTreeMap<String,u128> = serde_json::from_value(snap_val["balances"].clone()).unwrap_or_default();
        let nonces: BTreeMap<String,u64> = serde_json::from_value(snap_val["nonces"].clone()).unwrap_or_default();
        let gm: Option<String> = serde_json::from_value(snap_val["gm"].clone()).ok();

        // rebuild in-memory blocks 0..=ancestor_index from persisted DB
        let mut rebuilt: Vec<Block> = Vec::new();
        for h in 0..=g.blocks[ancestor_index].header.number {
            let key = blk_key(h);
            if let Some(bytes) = g.db.get(&key).unwrap() {
                let b: Block = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                rebuilt.push(b);
            } else {
                return Err(format!("missing block {} in DB during snapshot replay", h));
            }
        }
        // apply snapshot state
        g.blocks = rebuilt;
        g.balances = balances;
        g.nonces = nonces;
        g.gamemaster = gm;
        persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
    }

    // At this point, memory state is at ancestor. Now apply the new branch blocks in order.
    path.reverse();
    let miner_key = acct_key("miner");
    let mut applied = 0usize;
    for hsh in &path {
        let b = if let Some(bb) = g.side_blocks.get(hsh) {
            bb.clone()
        } else {
            return Err("missing side block during reorg".into());
        };

        // execute txs against current state
    let mut balances2 = g.balances.clone();
    let mut nonces2 = g.nonces.clone();
    let mut gm2 = g.gamemaster.clone();
    balances2.entry(miner_key.clone()).or_insert(0);
    nonces2.entry(miner_key.clone()).or_insert(0);
        let mut exec_results: BTreeMap<String, Result<(), String>> = BTreeMap::new();
        for tx in &b.txs {
            let h = hex::encode(tx_hash(tx));
            let res = execute_tx_with_nonce_and_fees(tx, &mut balances2, &mut nonces2, &miner_key, &mut gm2);
            if res.is_err() { return Err(format!("replay/apply failed for block {}: {}", b.header.number, res.err().unwrap_or_default())); }
            exec_results.insert(h, res);
        }
        // Optionally enforce strict validation
        if reorg_strict() {
            let new_state_root = compute_state_root(&balances2, &gm2);
            if new_state_root != b.header.state_root {
                return Err("state_root mismatch during strict reorg apply".into());
            }
            let tip = g.blocks.last().unwrap();
            let tx_root = if b.txs.is_empty() { tip.header.tx_root.clone() } else { tx_root_placeholder(&b.txs) };
            if tx_root != b.header.tx_root { return Err("tx_root mismatch during strict reorg apply".into()); }
        }

        // compute and persist undo for this applied block
        let undo = compute_undo(&g.balances, &g.nonces, &g.gamemaster, &balances2, &nonces2, &gm2);
        persist_undo(&g.db, b.header.number, &undo);

        // accept block
        g.balances = balances2;
        g.nonces = nonces2;
        g.gamemaster = gm2;
        for (txh, res) in exec_results.iter() {
            let r = Receipt { ok: res.is_ok(), error: res.clone().err(), height: b.header.number, block_hash: b.header.pow_hash.clone() };
            let key = format!("{}{}", RCPT_PREFIX, txh);
            let _ = g.db.insert(key.as_bytes(), serde_json::to_vec(&r).unwrap());
        }
        persist_state(&g.db, &g.balances, &g.nonces, &g.gamemaster);
    persist_block_only(&g.db, b.header.number, &b);
    // record last block weight in Prometheus (remove legacy atomic)
    PROM_VISION_BLOCK_WEIGHT_LAST.set(b.weight as i64);
        let _ = g.db.flush();

        g.blocks.push(b.clone());
        g.seen_blocks.insert(b.header.pow_hash.clone());
        for tx in &b.txs { g.seen_txs.insert(hex::encode(tx_hash(tx))); }
        applied += 1;
    }

    // record reorg length (number of blocks switched)
    let reorg_len = applied as u64;
    PROM_VISION_REORG_LENGTH_TOTAL.inc_by(reorg_len);

    // recompute cumulative_work and ensure side block metric
    g.cumulative_work.clear();
    let mut prev_cum: u128 = 0;
    for b in &g.blocks {
        prev_cum = prev_cum.saturating_add(block_work(b.header.difficulty));
        g.cumulative_work.insert(b.header.pow_hash.clone(), prev_cum);
    }

    // snapshot after reorg
    persist_snapshot(&g.db, g.blocks.last().unwrap().header.number, &g.balances, &g.nonces, &g.gamemaster);

    let dur_ms = reorg_start.elapsed().as_millis() as u64;
    PROM_VISION_REORG_DURATION_MS.set(dur_ms as i64);

    // Re-add orphaned txs to mempool if not present in new chain
    let now = now_ts();
    for b in orphaned {
        for tx in b.txs {
            let th = hex::encode(tx_hash(&tx));
            if !g.seen_txs.contains(&th) {
                // push orphaned txs into bulk lane
                g.mempool_bulk.push_back(tx.clone());
                g.mempool_ts.insert(th, now);
            }
        }
    }

    // update side-block metric
    PROM_VISION_SIDE_BLOCKS.set(g.side_blocks.len() as i64);

    Ok(())
}

// Drop duplicate / stale mempool entries (nonce < current expected)
fn prune_mempool(g: &mut Chain) {
    let ttl = mempool_ttl_secs();
    let now = now_ts();
    let mut keep_hashes: BTreeSet<String> = BTreeSet::new();

    // process critical lane
    let mut filtered_crit: VecDeque<Tx> = VecDeque::new();
    for tx in g.mempool_critical.drain(..) {
        if ttl > 0 {
            let th_tmp = hex::encode(tx_hash(&tx));
            if let Some(ts) = g.mempool_ts.get(&th_tmp).cloned() {
                if now.saturating_sub(ts) > ttl { g.mempool_ts.remove(&th_tmp); continue; }
            }
        }
        if verify_tx(&tx).is_err() { continue; }
        let from_key = acct_key(&tx.sender_pubkey);
        let expected = *g.nonces.get(&from_key).unwrap_or(&0);
        if tx.nonce < expected { continue; }
        let h = hex::encode(tx_hash(&tx));
        if keep_hashes.insert(h) { filtered_crit.push_back(tx); }
    }
    g.mempool_critical = filtered_crit;

    // process bulk lane
    let mut filtered_bulk: VecDeque<Tx> = VecDeque::new();
    for tx in g.mempool_bulk.drain(..) {
        if ttl > 0 {
            let th_tmp = hex::encode(tx_hash(&tx));
            if let Some(ts) = g.mempool_ts.get(&th_tmp).cloned() {
                if now.saturating_sub(ts) > ttl { g.mempool_ts.remove(&th_tmp); continue; }
            }
        }
        if verify_tx(&tx).is_err() { continue; }
        let from_key = acct_key(&tx.sender_pubkey);
        let expected = *g.nonces.get(&from_key).unwrap_or(&0);
        if tx.nonce < expected { continue; }
        let h = hex::encode(tx_hash(&tx));
        if keep_hashes.insert(h) { filtered_bulk.push_back(tx); }
    }
    g.mempool_bulk = filtered_bulk;

    // Retain ts for kept
    if ttl > 0 {
        let mut live = std::collections::BTreeSet::new();
        for t in g.mempool_critical.iter() { live.insert(hex::encode(tx_hash(t))); }
        for t in g.mempool_bulk.iter() { live.insert(hex::encode(tx_hash(t))); }
        g.mempool_ts.retain(|k,_| live.contains(k));
    }
}

// shared test helpers
#[cfg(test)]
pub(crate) fn fresh_chain() -> Chain {
    let td = tempfile::tempdir().expect("tmp");
    Chain::init(td.path().to_str().unwrap())
}

// ----------------- tests for reorg/fork-choice -----------------
#[cfg(test)]
mod reorg_tests {
    use super::*;

    // note: use shared `fresh_chain()` from parent scope

    // Build a small main chain of `n` additional blocks on top of genesis
    fn build_chain(mut g: Chain, n: usize, miner: &str) -> Chain {
        for _ in 0..n {
            let txs: Vec<Tx> = vec![];
            let (_b, _res) = execute_and_mine(&mut g, txs, miner, None);
        }
        g
    }

    #[test]
    fn reorg_when_side_becomes_heavier() {
        // fresh node with 3-block main chain
        let mut g = fresh_chain();
        g = build_chain(g, 3, "miner");

        let genesis = g.blocks[0].clone();

        // Manually craft two high-difficulty side blocks that chain genesis -> b1 -> b2
        let mut parent = genesis.clone();
        let mut side_blocks: Vec<Block> = Vec::new();
        for i in 1..=2 {
            let mut hdr = BlockHeader {
                parent_hash: parent.header.pow_hash.clone(),
                number: i as u64,
                timestamp: now_ts(),
                difficulty: 10, // high bits to make chain heavy
                nonce: 0,
                pow_hash: "0".repeat(64),
                state_root: parent.header.state_root.clone(),
                tx_root: parent.header.tx_root.clone(),
                receipts_root: parent.header.receipts_root.clone(),
                da_commitment: None,
            };
            loop {
                let h = hash_bytes(&header_pow_bytes(&hdr));
                if meets_difficulty_bits(h, hdr.difficulty) { hdr.pow_hash = hex32(h); break; }
                hdr.nonce = hdr.nonce.wrapping_add(1);
            }
            let b = Block { header: hdr, txs: vec![], weight: 0 };
            parent = b.clone();
            side_blocks.push(b);
        }

        // submit side blocks to main node
        for b in &side_blocks {
            let res = apply_block_from_peer(&mut g, b);
            assert!(res.is_ok(), "apply side block failed: {:?}", res);
        }

        // After submitting, the main chain should have reorganized to adopt the heavier side
        let tip = g.blocks.last().unwrap();
        assert_eq!(tip.header.number, 2u64, "expected tip to be side chain height 2");
    }

    #[test]
    fn fail_adopt_when_parent_missing() {
        let mut g = fresh_chain();
        // craft a block with a parent hash that doesn't exist
        let mut b = genesis_block();
        b.header.parent_hash = "deadbeef".repeat(4);
        b.header.number = g.blocks.last().unwrap().header.number + 1;
        // tamper pow to be valid for its difficulty
        let mut hdr = b.header.clone();
        hdr.pow_hash = "0".repeat(64);
        hdr.nonce = 0;
        loop {
            let h = hash_bytes(&header_pow_bytes(&hdr));
            if meets_difficulty_bits(h, hdr.difficulty) { hdr.pow_hash = hex32(h); break; }
            hdr.nonce = hdr.nonce.wrapping_add(1);
        }
        b.header = hdr;

        let res = apply_block_from_peer(&mut g, &b);
        // Behavior: we accept orphan into side_blocks (do not error); ensure it was stored
        assert!(res.is_ok(), "expected orphan to be stored as side block, got: {:?}", res);
        assert!(g.side_blocks.contains_key(&b.header.pow_hash));
    }

    #[test]
    fn snapshot_fallback_when_undos_missing() {
        let mut g = fresh_chain();
        g = build_chain(g, 3, "miner");

        // create a side block chain that will be heavier
        let genesis = g.blocks[0].clone();
        let mut parent = genesis.clone();
        let mut side_blocks: Vec<Block> = Vec::new();
        for i in 1..=2 {
            let mut hdr = BlockHeader {
                parent_hash: parent.header.pow_hash.clone(),
                number: i as u64,
                timestamp: now_ts(),
                difficulty: 10,
                nonce: 0,
                pow_hash: "0".repeat(64),
                state_root: parent.header.state_root.clone(),
                tx_root: parent.header.tx_root.clone(),
                receipts_root: parent.header.receipts_root.clone(),
                da_commitment: None,
            };
            loop {
                let h = hash_bytes(&header_pow_bytes(&hdr));
                if meets_difficulty_bits(h, hdr.difficulty) { hdr.pow_hash = hex32(h); break; }
                hdr.nonce = hdr.nonce.wrapping_add(1);
            }
            let b = Block { header: hdr, txs: vec![], weight: 0 };
            parent = b.clone();
            side_blocks.push(b);
        }

        // submit side blocks (creates snapshots as implemented)
        for b in &side_blocks { let res = apply_block_from_peer(&mut g, b); assert!(res.is_ok()); }

    // ensure a snapshot exists at genesis so fallback can use it (ancestor may be genesis)
    persist_snapshot(&g.db, g.blocks[0].header.number, &g.balances, &g.nonces, &g.gamemaster);
    // delete undo entries to force fallback
    for h in 1..=g.blocks.last().unwrap().header.number { let _ = g.db.remove(format!("meta:undo:{}", h).as_bytes()); }

        // craft another heavier branch extending genesis
        let mut parent = genesis.clone();
        let mut extra: Vec<Block> = Vec::new();
        for i in 1..=3 {
            let mut hdr = BlockHeader { parent_hash: parent.header.pow_hash.clone(), number: i as u64, timestamp: now_ts(), difficulty: 12, nonce: 0, pow_hash: "0".repeat(64), state_root: parent.header.state_root.clone(), tx_root: parent.header.tx_root.clone(), receipts_root: parent.header.receipts_root.clone(), da_commitment: None };
            loop { let h = hash_bytes(&header_pow_bytes(&hdr)); if meets_difficulty_bits(h, hdr.difficulty) { hdr.pow_hash = hex32(h); break; } hdr.nonce = hdr.nonce.wrapping_add(1); }
            let b = Block { header: hdr, txs: vec![], weight: 0 };
            parent = b.clone(); extra.push(b);
        }

        // apply extra blocks; should trigger snapshot fallback and succeed
        for b in &extra { let res = apply_block_from_peer(&mut g, b); assert!(res.is_ok(), "apply failed: {:?}", res); }
    }

    #[test]
    fn bulk_eviction_prefers_lower_fee_per_byte() {
        // create a fresh chain and populate bulk lane with two txs: low fee/byte and mid fee/byte
        let mut g = fresh_chain();
        // Construct txs with same estimated weight via est_tx_weight default
        let low = Tx { nonce: 0, sender_pubkey: "aa".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 1, fee_limit:0, sig: String::new() };
        let mid = Tx { nonce: 0, sender_pubkey: "bb".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 10, fee_limit:0, sig: String::new() };
        g.mempool_bulk.push_back(low.clone());
        g.mempool_bulk.push_back(mid.clone());
        // incoming tx with high tip should evict the low fee/byte tx (index 0)
        let incoming = Tx { nonce: 0, sender_pubkey: "cc".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 100, fee_limit:0, sig: String::new() };
    let idx = mempool::bulk_eviction_index(&g, &incoming);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap(), 0usize);
    }

    #[test]
    fn replacement_allows_higher_tip_same_sender_nonce() {
        let mut g = fresh_chain();
        // existing tx from alice nonce 0 with tip 1
        let t1 = Tx { nonce: 0, sender_pubkey: "alice".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 1, fee_limit:0, sig: String::new() };
        g.mempool_bulk.push_back(t1.clone());
        let th1 = hex::encode(tx_hash(&t1));
        g.mempool_ts.insert(th1.clone(), now_ts());
        g.seen_txs.insert(th1.clone());

        // incoming bumped tx with higher tip
        let t2 = Tx { nonce: 0, sender_pubkey: "alice".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 10, fee_limit:0, sig: String::new() };
    assert_eq!(mempool::try_replace_sender_nonce(&mut g, &t2).unwrap(), true);
        // ensure old removed
        assert!(!g.seen_txs.contains(&th1));
    }

    #[test]
    fn replacement_rejects_lower_or_equal_tip() {
        let mut g = fresh_chain();
        // existing tx from alice nonce 0 with tip 10
        let t1 = Tx { nonce: 0, sender_pubkey: "alice".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 10, fee_limit:0, sig: String::new() };
        g.mempool_bulk.push_back(t1.clone());
        let th1 = hex::encode(tx_hash(&t1));
        g.mempool_ts.insert(th1.clone(), now_ts());
        g.seen_txs.insert(th1.clone());

        // incoming with equal tip should be rejected
        let t_eq = Tx { nonce: 0, sender_pubkey: "alice".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 10, fee_limit:0, sig: String::new() };
        let res_eq = mempool::try_replace_sender_nonce(&mut g, &t_eq);
        assert!(res_eq.is_err());
        assert_eq!(res_eq.unwrap_err(), "rbf_tip_too_low");

        // incoming with lower tip should be rejected
        let t_low = Tx { nonce: 0, sender_pubkey: "alice".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 5, fee_limit:0, sig: String::new() };
        let res_low = mempool::try_replace_sender_nonce(&mut g, &t_low);
        assert!(res_low.is_err());
        assert_eq!(res_low.unwrap_err(), "rbf_tip_too_low");
    }

    #[test]
    fn admission_rejects_low_priority_when_full() {
        let mut g = fresh_chain();
        // set small mempool cap
        g.limits.mempool_max = 2;
        // add two high-priority txs
        let a = Tx { nonce: 0, sender_pubkey: "a".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 100, fee_limit:0, sig: String::new() };
        let b = Tx { nonce: 0, sender_pubkey: "b".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 90, fee_limit:0, sig: String::new() };
        g.mempool_bulk.push_back(a.clone()); g.mempool_ts.insert(hex::encode(tx_hash(&a)), now_ts()); g.seen_txs.insert(hex::encode(tx_hash(&a)));
        g.mempool_bulk.push_back(b.clone()); g.mempool_ts.insert(hex::encode(tx_hash(&b)), now_ts()); g.seen_txs.insert(hex::encode(tx_hash(&b)));

        // incoming low tip should be rejected
        let low = Tx { nonce: 0, sender_pubkey: "x".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 1, fee_limit:0, sig: String::new() };
    let res = mempool::admission_check_under_load(&g, &low);
        assert!(res.is_err());

        // incoming higher tip should be accepted
        let high = Tx { nonce: 0, sender_pubkey: "y".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 200, fee_limit:0, sig: String::new() };
    let res2 = mempool::admission_check_under_load(&g, &high);
        assert!(res2.is_ok());
    }
}

#[cfg(test)]
mod extra_tests {
    use super::*;

    #[test]
    fn signable_bytes_deterministic_and_excludes_sig() {
        let mut tx = Tx { nonce: 1, sender_pubkey: "aa".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![1,2,3], tip: 10, fee_limit: 100, sig: "deadbeef".into() };
        let a = signable_tx_bytes(&tx);
        tx.sig = "cafebabe".into();
        let b = signable_tx_bytes(&tx);
        assert_eq!(a, b, "signable bytes must not include sig and be deterministic");
    }

    #[test]
    fn fee_limit_rejected_if_below_intrinsic() {
        let mut g = fresh_chain();
        // create tx with tiny fee_limit
        let tx = Tx { nonce: 0, sender_pubkey: "pk0".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 0, fee_limit: 0, sig: String::new() };
    let res = mempool::validate_for_mempool(&tx, &g);
        assert!(res.is_err(), "expected fee_limit 0 to be rejected");
    }

    #[test]
    fn reject_duplicate_sender_nonce_in_mempool() {
        let mut g = fresh_chain();
        let tx1 = Tx { nonce: 0, sender_pubkey: "pkdup".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 1000, fee_limit: 1000, sig: String::new() };
        g.mempool_bulk.push_back(tx1.clone());
        let tx2 = Tx { nonce: 0, sender_pubkey: "pkdup".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 1000, fee_limit: 1000, sig: String::new() };
    let res = mempool::validate_for_mempool(&tx2, &g);
        assert!(res.is_err(), "duplicate sender+nonce should be rejected");
    }

    #[test]
    fn mempool_pruned_after_nonce_advanced_by_mining() {
        let mut g = fresh_chain();
        // create simple txs from same sender with consecutive nonces
        let tx0 = Tx { nonce: 0, sender_pubkey: "pka".into(), access_list: vec![], module: "noop".into(), method: "ping".into(), args: vec![], tip: 1000, fee_limit: 1000, sig: String::new() };
        let tx1 = Tx { nonce: 1, sender_pubkey: "pka".into(), access_list: vec![], module: "noop".into(), method: "ping".into(), args: vec![], tip: 1000, fee_limit: 1000, sig: String::new() };
        g.mempool_bulk.push_back(tx0.clone());
        g.mempool_bulk.push_back(tx1.clone());
        // mine a block that consumes nonce 0 by executing tx0
        let (block, _res) = execute_and_mine(&mut g, vec![tx0.clone()], "miner", None);
        // prune mempool should remove tx0 (stale) and keep tx1 if now valid
    mempool::prune_mempool(&mut g);
        // verify no tx in mempool has nonce < expected
        let expected = *g.nonces.get(&acct_key("pka")).unwrap_or(&0);
        for t in g.mempool_critical.iter().chain(g.mempool_bulk.iter()) {
            assert!(t.nonce >= expected, "found stale tx in mempool");
        }
    }
}

#[cfg(test)]
mod builder_tests {
    use super::*;

    #[test]
    fn block_builder_respects_weight_limit() {
        let mut g = crate::fresh_chain();
        // populate mempool with several txs of varying tip; est_tx_weight returns 200 by default
        for i in 0..10 {
            let tx = Tx { nonce: i, sender_pubkey: format!("pk{}", i%3), access_list: vec![], module: "cash".into(), method: "transfer".into(), args: vec![], tip: (1000 - i as u64 * 10), fee_limit: 0, sig: String::new() };
            g.mempool_bulk.push_back(tx);
        }
        let weight_limit = 3 * est_tx_weight(&g.mempool_bulk[0]); // allow only 3 txs
    let chosen = mempool::build_block_from_mempool(&mut g, 10, weight_limit);
        assert!(chosen.len() <= 3, "builder exceeded weight limit: {}", chosen.len());
    }
}

#[cfg(test)]
mod receipts_tests {
    use super::*;

    #[test]
    fn receipts_merkle_root_and_persistence() {
        let mut g = fresh_chain();
        // create a simple tx that will be executed (execute_tx_with_nonce_and_fees does not verify sig)
        let tx = Tx { nonce: 0, sender_pubkey: "pk0".into(), access_list: vec![], module: "noop".into(), method: "ping".into(), args: vec![], tip: 0, fee_limit: 1000, sig: String::new() };
    let parent = g.blocks.last().cloned().unwrap();
    let parent_receipts_root = parent.header.receipts_root.clone();
    let (block, _res) = execute_and_mine(&mut g, vec![tx.clone()], "miner", Some(&parent));
        // receipts_root should be set (non-zero when txs included)
        assert!(block.header.receipts_root.len() == 64, "receipts_root must be 64-hex chars");
        // persisted receipt exists in DB under RCPT_PREFIX + tx_hash
        let th = hex::encode(tx_hash(&tx));
        let key = format!("{}{}", RCPT_PREFIX, th);
        let found = g.db.get(key.as_bytes()).unwrap();
        assert!(found.is_some(), "receipt must be persisted to DB");
        let rbytes = found.unwrap();
        let r: Receipt = serde_json::from_slice(&rbytes).expect("deserialize receipt");
        assert_eq!(r.height, block.header.number);
        assert_eq!(r.block_hash, block.header.pow_hash);
        // if parent had no receipts and we included one, root should not equal parent
        if parent_receipts_root == "0".repeat(64) {
            assert_ne!(block.header.receipts_root, parent_receipts_root);
        }
    }
}

#[cfg(test)]
mod mempool_sweeper_tests {
    use super::*;

    #[test]
    fn prune_mempool_increments_sweep_metrics_when_ttl_and_expired() {
        // configure TTL small and create an expired tx
        std::env::set_var("VISION_MEMPOOL_TTL_SECS", "1");
        let mut g = fresh_chain();
        let tx = Tx { nonce: 0, sender_pubkey: "pka".into(), access_list: vec![], module: "noop".into(), method: "ping".into(), args: vec![], tip: 1, fee_limit: 1000, sig: String::new() };
        g.mempool_bulk.push_back(tx.clone());
        let th = hex::encode(tx_hash(&tx));
        // set timestamp far in the past to mark expired
        g.mempool_ts.insert(th.clone(), now_ts().saturating_sub(3600));
        g.seen_txs.insert(th.clone());

    // reset Prometheus metrics to known values for the test
    PROM_VISION_MEMPOOL_SWEEPS.reset();
    PROM_VISION_MEMPOOL_REMOVED_TOTAL.reset();
    PROM_VISION_MEMPOOL_REMOVED_LAST.set(0);

        mempool::prune_mempool(&mut g);

    let sweeps = PROM_VISION_MEMPOOL_SWEEPS.get() as u64;
    let removed_last = PROM_VISION_MEMPOOL_REMOVED_LAST.get() as i64 as u64;
    let removed_total = PROM_VISION_MEMPOOL_REMOVED_TOTAL.get() as u64;
        assert!(sweeps >= 1, "expected at least one sweep run after prune_mempool");
        assert!(removed_last > 0 || removed_total > 0, "expected some removed entries when TTL expired");
    }
}

// Generic error classifier (downcast-based) used by sync helpers and tests.
fn classify_error_any(err: &(dyn std::error::Error + 'static)) -> &'static str {
    // Walk the source chain looking for concrete std::io::Error kinds
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur {
        if let Some(ioe) = e.downcast_ref::<std::io::Error>() {
            match ioe.kind() {
                std::io::ErrorKind::ConnectionRefused => return "connection_refused",
                std::io::ErrorKind::TimedOut => return "timeout",
                _ => {}
            }
            if let Some(code) = ioe.raw_os_error() {
                if code == 10061 { return "connection_refused"; }
            }
        }
        cur = e.source();
    }
    // If we reached here, try a conservative string-based DNS detection as a fallback
    let mut cur2: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur2 {
        let s = e.to_string().to_lowercase();
        if s.contains("name or service not known") || s.contains("no such host") || s.contains("getaddrinfo") || s.contains("could not resolve") || s.contains("dns") || s.contains("nodename") {
            return "dns_error";
        }
        cur2 = e.source();
    }
    "request_error"
}


// =================== Sync endpoints ===================

// Pull blocks from a remote peer: body { "src":"http://127.0.0.1:7070", "from": <opt>, "to": <opt> }
async fn sync_pull(Json(req): Json<SyncPullReq>) -> (StatusCode, Json<serde_json::Value>) {
    let src = req.src.trim().trim_end_matches('/').to_string();
    // helper to produce enriched BAD_GATEWAY responses that include the originating src
    let make_bad = |msg: String| -> (StatusCode, Json<serde_json::Value>) {
        (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"src": src.clone(), "error": msg})))
    };

    // helper to classify reqwest errors into prometheus label values (delegates to module helper)
    fn classify_reqwest_error(e: &reqwest::Error) -> &'static str {
        if e.is_timeout() { return "timeout"; }
        if e.is_connect() {
            if let Some(src) = e.source() {
                if let Some(ioe) = src.downcast_ref::<std::io::Error>() {
                    if ioe.kind() == std::io::ErrorKind::ConnectionRefused { return "connection_refused"; }
                }
            }
        }
        classify_error_any(e)
    }

    // per-peer backoff check
    let now_unix = now_secs();
    if let Some(next_allowed) = PEER_BACKOFF.get(&src) {
        if *next_allowed.value() > now_unix {
            PROM_SYNC_PULL_FAILURES.with_label_values(&["backoff"]).inc();
            return make_bad("peer temporarily backoffed".to_string());
        }
    }

    // remote height (with timeout + retry/backoff + jitter)
    debug!(src = %src, "sync_pull: fetching remote height");
    let mut src_h_txt = String::new();
    let max_attempts = std::env::var("VISION_SYNC_PULL_MAX_ATTEMPTS").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(4);
    let base_backoff_ms = std::env::var("VISION_SYNC_PULL_BACKOFF_BASE_MS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
    let peer_backoff_secs = std::env::var("VISION_PEER_BACKOFF_SECS").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(60);
    for attempt in 1..=max_attempts {
        let res = tokio::time::timeout(std::time::Duration::from_secs(3), HTTP.get(format!("{}/height", src)).send()).await;
        match res {
            Ok(Ok(r)) => match r.text().await {
                Ok(s) => { src_h_txt = s; break; }
                Err(e) => {
                    debug!(src = %src, err = ?e, attempt = attempt, "sync_pull: failed reading src height body");
                    if attempt < max_attempts {
                        PROM_SYNC_PULL_RETRIES.inc();
                        // exponential backoff with jitter
                        let backoff = base_backoff_ms.saturating_mul(1 << (attempt - 1));
                        let jitter = (now_unix % (base_backoff_ms as u64)) as u64;
                        tokio::time::sleep(std::time::Duration::from_millis(backoff + jitter)).await;
                        continue;
                    }
                    PROM_SYNC_PULL_FAILURES.with_label_values(&["read_error"]).inc();
                    // set per-peer backoff
                    PEER_BACKOFF.insert(src.clone(), now_unix.saturating_add(peer_backoff_secs));
                    return make_bad(format!("src height read: {} | debug: {:?}", e, e));
                }
            },
            Ok(Err(e)) => {
                let reason = classify_reqwest_error(&e);
                debug!(src = %src, err = ?e, attempt = attempt, "sync_pull: reqwest error fetching src height");
                if attempt < max_attempts {
                    PROM_SYNC_PULL_RETRIES.inc();
                    let backoff = base_backoff_ms.saturating_mul(1 << (attempt - 1));
                    let jitter = (now_unix % (base_backoff_ms as u64)) as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(backoff + jitter)).await;
                    continue;
                }
                PROM_SYNC_PULL_FAILURES.with_label_values(&[reason]).inc();
                PEER_BACKOFF.insert(src.clone(), now_unix.saturating_add(peer_backoff_secs));
                return make_bad(format!("src height req: {} | debug: {:?}", e, e));
            }
            Err(_) => {
                debug!(src = %src, attempt = attempt, "sync_pull: src height request timed out");
                if attempt < max_attempts {
                    PROM_SYNC_PULL_RETRIES.inc();
                    let backoff = base_backoff_ms.saturating_mul(1 << (attempt - 1));
                    let jitter = (now_unix % (base_backoff_ms as u64)) as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(backoff + jitter)).await;
                    continue;
                }
                PROM_SYNC_PULL_FAILURES.with_label_values(&["timeout"]).inc();
                PEER_BACKOFF.insert(src.clone(), now_unix.saturating_add(peer_backoff_secs));
                return make_bad("src height req: timeout".to_string());
            }
        }
    }
    let src_h: u64 = match src_h_txt.trim().parse() {
        Ok(v) => v,
        Err(_) => {
            PROM_SYNC_PULL_FAILURES.with_label_values(&["bad_src_height"]).inc();
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error":"bad src height"})))
        }
    };
    // local height
    let dst_h = {
        let g = CHAIN.lock();
        g.blocks.last().unwrap().header.number
    };
    let start = req.from.unwrap_or(dst_h.saturating_add(1));
    let end = req.to.unwrap_or(src_h);
    if start > end {
        return (StatusCode::OK, Json(serde_json::json!({"pulled":0, "from": start, "to": end})))
    }

    let mut pulled = 0u64;
    for h in start..=end {
        debug!(src = %src, height = h, "sync_pull: fetching block");
        let mut blk_opt: Option<Block> = None;
        let max_attempts = 2u32;
        for attempt in 1..=max_attempts {
            let resp_res = tokio::time::timeout(std::time::Duration::from_secs(5), HTTP.get(format!("{}/block/{}", src, h)).send()).await;
            match resp_res {
                Ok(Ok(r)) => {
                    match r.json().await {
                        Ok(b) => { blk_opt = Some(b); break; }
                        Err(e) => {
                            debug!(src = %src, height = h, err = ?e, "sync_pull: failed decoding block JSON");
                            PROM_SYNC_PULL_FAILURES.with_label_values(&["decode_error"]).inc();
                            return make_bad(format!("decode block {}: {} | debug: {:?}", h, e, e));
                        }
                    }
                }
                Ok(Err(e)) => {
                    let reason = classify_reqwest_error(&e);
                    debug!(src = %src, height = h, err = ?e, attempt = attempt, "sync_pull: reqwest error fetching block");
                    if attempt < max_attempts {
                        PROM_SYNC_PULL_RETRIES.inc();
                        let backoff = base_backoff_ms.saturating_mul(1 << (attempt - 1));
                        let jitter = (now_unix % (base_backoff_ms as u64)) as u64;
                        tokio::time::sleep(std::time::Duration::from_millis(backoff + jitter)).await;
                        continue;
                    }
                    PROM_SYNC_PULL_FAILURES.with_label_values(&[reason]).inc();
                    return make_bad(format!("fetch block {}: {} | debug: {:?}", h, e, e));
                }
                Err(_) => {
                    debug!(src = %src, height = h, attempt = attempt, "sync_pull: fetch block timed out");
                    if attempt < max_attempts {
                        PROM_SYNC_PULL_RETRIES.inc();
                        let backoff = base_backoff_ms.saturating_mul(1 << (attempt - 1));
                        let jitter = (now_unix % (base_backoff_ms as u64)) as u64;
                        tokio::time::sleep(std::time::Duration::from_millis(backoff + jitter)).await;
                        continue;
                    }
                    PROM_SYNC_PULL_FAILURES.with_label_values(&["timeout"]).inc();
                    return make_bad(format!("fetch block {}: timeout", h));
                }
            }
        }
        let blk: Block = match blk_opt {
            Some(b) => b,
            None => { PROM_SYNC_PULL_FAILURES.with_label_values(&["request_error"]).inc(); return make_bad(format!("fetch block {}: unknown error", h)); }
        };
        // apply block via centralized handler (handles side-blocks and reorg)
        let mut g = CHAIN.lock();
        match apply_block_from_peer(&mut g, &blk) {
            Ok(()) => { pulled += 1; }
            Err(e) => return (StatusCode::CONFLICT, Json(serde_json::json!({"error": e}))),
        }
    }

    (StatusCode::OK, Json(serde_json::json!({"pulled": pulled, "from": start, "to": end})))
}

// Push a list of blocks to this node: body { "blocks": [ Block, ... ] }
async fn sync_push(Json(req): Json<SyncPushReq>) -> (StatusCode, Json<serde_json::Value>) {
    let mut g = CHAIN.lock();
    let mut applied = 0usize;
    for blk in req.blocks {
        match apply_block_from_peer(&mut g, &blk) {
            Ok(()) => { g.seen_blocks.insert(blk.header.pow_hash.clone()); applied += 1; }
            Err(e) => return (StatusCode::CONFLICT, Json(serde_json::json!({"error": e, "applied": applied}))),
        }
    }
    (StatusCode::OK, Json(serde_json::json!({"applied": applied})))
}


fn persist_fee_base(db: &Db, v: u128) {
    let _ = db.insert(META_FEE_BASE.as_bytes(), u128_to_be(v));
}
// =================== Persistence ===================
fn persist_state(db: &Db, balances: &BTreeMap<String, u128>, nonces: &BTreeMap<String, u64>, gm: &Option<String>) {
    for (k, v) in balances {
        if k.starts_with("acct:") {
            let key = format!("{}{}", BAL_PREFIX, k);
            let _ = db.insert(key.as_bytes(), u128_to_be(*v));
        }
    }
    for (k, v) in nonces {
        if k.starts_with("acct:") {
            let key = format!("{}{}", NONCE_PREFIX, k);
            let _ = db.insert(key.as_bytes(), u64_to_be(*v));
        }
    }
    // persist GM as part of state
    if let Some(s) = gm {
        let _ = db.insert(META_GM.as_bytes(), IVec::from(s.as_bytes()));
    } else {
        let _ = db.remove(META_GM.as_bytes());
    }
    let _ = db.flush();
}
fn persist_block_only(db: &Db, height: u64, block: &Block) {
    let key = blk_key(height);
    let _ = db.insert(key, serde_json::to_vec(block).unwrap());
    let _ = db.insert(META_HEIGHT.as_bytes(), u64_to_be(height));
    let _ = db.flush();
}

fn persist_snapshot(db: &Db, height: u64, balances: &BTreeMap<String,u128>, nonces: &BTreeMap<String,u64>, gm: &Option<String>) {
    let snap_key = format!("meta:snapshot:{}", height);
    let snap = serde_json::json!({ "height": height, "balances": balances, "nonces": nonces, "gm": gm });
    let _ = db.insert(snap_key.as_bytes(), serde_json::to_vec(&snap).unwrap());
    let _ = db.flush();
    PROM_VISION_SNAPSHOTS.inc();
            info!(snapshot_height = height, "snapshot persisted");
            // prune old snapshots/undos based on retention (env seconds as number of snapshots to keep)
            let retain = std::env::var("VISION_SNAPSHOT_RETENTION").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
            // List snapshot keys and remove older ones beyond `retain`
            let mut snaps: Vec<u64> = Vec::new();
            for kv in db.scan_prefix("meta:snapshot:".as_bytes()) {
                if let Ok((k, _v)) = kv {
                    if let Ok(s) = String::from_utf8(k.to_vec()) {
                        if let Some(hs) = s.strip_prefix("meta:snapshot:") {
                            if let Ok(hv) = hs.parse::<u64>() { snaps.push(hv); }
                        }
                    }
                }
            }
            snaps.sort_unstable();
            if snaps.len() > retain {
                let remove_count = snaps.len() - retain;
                for old in snaps.into_iter().take(remove_count) {
                    let k = format!("meta:snapshot:{}", old);
                    let _ = db.remove(k.as_bytes());
                    // also remove corresponding undos
                    let _ = db.remove(format!("meta:undo:{}", old).as_bytes());
                    debug!(removed_snapshot = old, "pruned old snapshot and undo");
                }
            }
}

fn persist_difficulty(db: &Db, diff: u64) {
    let _ = db.insert("meta:difficulty".as_bytes(), diff.to_be_bytes().to_vec());
}
fn persist_ema(db: &Db, ema: f64) {
    let _ = db.insert("meta:ema_block_time".as_bytes(), ema.to_string().as_bytes());
}

#[allow(dead_code)]
fn load_latest_snapshot(db: &Db) -> Option<(u64, BTreeMap<String,u128>, BTreeMap<String,u64>, Option<String>)> {
    // naive: scan for keys starting with meta:snapshot: and pick the highest
    let mut best_h: Option<u64> = None;
    for kv in db.scan_prefix("meta:snapshot:".as_bytes()) {
    if let Ok((k,_v)) = kv {
            if let Ok(s) = String::from_utf8(k.to_vec()) {
                if let Some(hs) = s.strip_prefix("meta:snapshot:") {
                    if let Ok(hv) = hs.parse::<u64>() {
                        best_h = Some(best_h.map_or(hv, |b| b.max(hv)));
                    }
                }
            }
        }
    }
    if let Some(h) = best_h {
        let key = format!("meta:snapshot:{}", h);
        if let Ok(Some(bytes)) = db.get(key.as_bytes()) {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                let balances = serde_json::from_value(v["balances"].clone()).unwrap_or_default();
                let nonces = serde_json::from_value(v["nonces"].clone()).unwrap_or_default();
                let gm = serde_json::from_value(v["gm"].clone()).ok();
                return Some((h, balances, nonces, gm));
            }
        }
    }
    None
}

// Per-block undo deltas so we can roll back blocks without full replay
#[derive(Serialize, Deserialize)]
struct Undo {
    balances: BTreeMap<String, Option<u128>>,
    nonces: BTreeMap<String, Option<u64>>,
    gamemaster: Option<Option<String>>,
}

fn persist_undo(db: &Db, height: u64, u: &Undo) {
    let key = format!("meta:undo:{}", height);
    let _ = db.insert(key.as_bytes(), serde_json::to_vec(u).unwrap());
}

fn load_undo(db: &Db, height: u64) -> Option<Undo> {
    let key = format!("meta:undo:{}", height);
    if let Ok(Some(v)) = db.get(key.as_bytes()) {
        if let Ok(u) = serde_json::from_slice::<Undo>(&v) { return Some(u); }
    }
    None
}

fn compute_undo(prev_bal: &BTreeMap<String,u128>, prev_nonce: &BTreeMap<String,u64>, prev_gm: &Option<String>, new_bal: &BTreeMap<String,u128>, new_nonce: &BTreeMap<String,u64>, new_gm: &Option<String>) -> Undo {
    let mut ub = BTreeMap::new();
    for (k, v) in new_bal.iter() {
        let pv = prev_bal.get(k).cloned();
        if pv != Some(*v) {
            ub.insert(k.clone(), pv);
        }
    }
    // also find keys removed
    for (k, v) in prev_bal.iter() {
        if !new_bal.contains_key(k) {
            ub.insert(k.clone(), Some(*v));
        }
    }
    let mut un = BTreeMap::new();
    for (k, v) in new_nonce.iter() {
        let pv = prev_nonce.get(k).cloned();
        if pv != Some(*v) {
            un.insert(k.clone(), pv);
        }
    }
    for (k, v) in prev_nonce.iter() {
        if !new_nonce.contains_key(k) {
            un.insert(k.clone(), Some(*v));
        }
    }
    let ugm = if prev_gm != new_gm { Some(prev_gm.clone()) } else { None };
    Undo { balances: ub, nonces: un, gamemaster: ugm }
}
fn u128_to_be(x: u128) -> IVec { IVec::from(x.to_be_bytes().to_vec()) }
fn u64_to_be(x: u64) -> IVec { IVec::from(x.to_be_bytes().to_vec()) }
fn u128_from_be(v: &IVec) -> u128 { let mut a=[0u8;16]; a.copy_from_slice(v.as_ref()); u128::from_be_bytes(a) }
fn u64_from_be(v: &IVec) -> u64 { let mut a=[0u8;8]; a.copy_from_slice(v.as_ref()); u64::from_be_bytes(a) }

// =================== Admin token helper ===================
fn check_admin(headers: HeaderMap, q: &std::collections::HashMap<String, String>) -> bool {
    let expected = match env::var("VISION_ADMIN_TOKEN") {
        Ok(v) if !v.is_empty() => v,
        _ => return false,
    };
    if let Some(tok) = q.get("token") {
        if tok == &expected { return true; }
    }
    // Accept x-admin-token header (simple) or Authorization: Bearer <token>
    if let Some(hv) = headers.get("x-admin-token") {
        if let Ok(s) = hv.to_str() { if s.trim() == expected { return true; } }
    }
    if let Some(hv) = headers.get("authorization") {
        if let Ok(s) = hv.to_str() {
            if let Some(rest) = s.strip_prefix("Bearer ") {
                return rest.trim() == expected;
            }
        }
    }
    false
}

// Canonical rate-limited response helper. Returns a tuple (headers, (StatusCode, Json body))
fn rate_limited_response_with_headers(base_headers: &axum::http::HeaderMap, reason: &str) -> (axum::http::HeaderMap, (StatusCode, Json<serde_json::Value>)) {
    (base_headers.clone(), (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({
        "ok": false,
        "code": "rate_limited",
        "reason": reason
    }))))
}

// =================== Peer APIs ===================
async fn get_peers() -> Json<PeersView> {
    let g = CHAIN.lock();
    Json(PeersView { peers: g.peers.iter().cloned().collect() })
}

#[derive(Deserialize)]
struct DevFaucetReq { to: String, amount: u128 }

async fn dev_faucet_mint(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<DevFaucetReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if std::env::var("VISION_DEV").ok().and_then(|s| if s == "1" { Some(1) } else { None }).is_none() {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"dev disabled"}))); }
    // token check
    let expected = std::env::var("VISION_DEV_TOKEN").unwrap_or_default();
    let ok = q.get("dev_token").map(|t| t == &expected).unwrap_or(false) || headers.get("x-dev-token").and_then(|h| h.to_str().ok()).map(|s| s == expected).unwrap_or(false);
    if !ok { return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"bad dev token"}))); }

    // Build a GM mint tx and mine it immediately
    let mut g = CHAIN.lock();
    let gm = match g.gamemaster.clone() { Some(s) => s, _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"no gamemaster"}))) };
    let gm_key = acct_key(&gm);
    g.balances.entry(gm_key.clone()).or_insert(0);
    g.nonces.entry(gm_key.clone()).or_insert(0);
    let nonce = *g.nonces.get(&gm_key).unwrap_or(&0);
    let args = serde_json::to_vec(&serde_json::json!({ "to": req.to, "amount": req.amount })).unwrap();
    let tx = Tx { nonce, sender_pubkey: gm.clone(), access_list: vec!["acct:to".into()], module: "cash".into(), method: "mint".into(), args, tip: 0, fee_limit: 0, sig: String::new() };
    let parent = g.blocks.last().cloned();
    let (block, _res) = execute_and_mine(&mut g, vec![tx], "miner", parent.as_ref());
    (StatusCode::OK, Json(serde_json::json!({ "height": block.header.number, "hash": block.header.pow_hash })))
}

#[derive(Deserialize)]
struct DevSpamReq { count: Option<usize> }
async fn dev_spam_txs(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<DevSpamReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if std::env::var("VISION_DEV").ok().and_then(|s| if s == "1" { Some(1) } else { None }).is_none() {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"dev disabled"}))); }
    let expected = std::env::var("VISION_DEV_TOKEN").unwrap_or_default();
    let ok = q.get("dev_token").map(|t| t == &expected).unwrap_or(false) || headers.get("x-dev-token").and_then(|h| h.to_str().ok()).map(|s| s == expected).unwrap_or(false);
    if !ok { return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"bad dev token"}))); }
    let count = req.count.unwrap_or(10).min(1000);
    let mut g = CHAIN.lock();
    for _i in 0..count {
        let tx = Tx { nonce: 0, sender_pubkey: "dev".into(), access_list: vec![], module: "noop".into(), method: "ping".into(), args: vec![], tip: 0, fee_limit: 0, sig: String::new() };
        g.mempool_bulk.push_back(tx);
    }
    (StatusCode::OK, Json(serde_json::json!({"spammed": count})))
}

async fn add_peer_protected(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<AddPeerReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !check_admin(headers, &q) {
    return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"invalid or missing admin token"})))
}
    if req.url.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"empty url"})))
    }
    let mut g = CHAIN.lock();
    if g.peers.insert(req.url.clone()) {
        let key = format!("{}{}", PEER_PREFIX, req.url);
        let _ = g.db.insert(key.as_bytes(), IVec::from(&b"1"[..]));
        let _ = g.db.flush();
    }
    (StatusCode::OK, Json(serde_json::json!({"ok":true,"peers": g.peers.iter().cloned().collect::<Vec<_>>() })))
}

async fn gossip_tx(ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(GossipTxEnvelope { tx }): Json<GossipTxEnvelope>) -> impl axum::response::IntoResponse {
    let ip = addr.ip().to_string();
    let base_headers = mempool::build_rate_limit_headers(&ip);
    // per-IP token bucket (gossip endpoint)
    {
        let ip = ip.clone();
        let limits = { let g = CHAIN.lock(); g.limits.clone() };
        let mut entry = IP_TOKEN_BUCKETS.entry(ip.clone()).or_insert_with(|| {
            TokenBucket::new(limits.rate_gossip_rps as f64, limits.rate_gossip_rps as f64)
        });
        if !entry.value_mut().allow(1.0) {
            return rate_limited_response_with_headers(&base_headers, "ip_rate_limit");
        }
    }
    {
        let g = CHAIN.lock();
        if let Some(msg) = preflight_violation(&tx, &g) {
            return (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "status":"ignored", "error": { "code": "preflight", "message": msg } }))));
        }
    }

    // rate limit gossip by sender id
    if !peer_allow(&tx.sender_pubkey) {
        return rate_limited_response_with_headers(&base_headers, "peer_rate_limited");
    }
    match verify_tx(&tx) {
        Ok(_) => {
            let mut g = CHAIN.lock();
            let h = hex::encode(tx_hash(&tx));
            if !g.seen_txs.insert(h.clone()) {
                return (base_headers.clone(), (StatusCode::OK, Json(serde_json::json!({"status":"ignored","reason":"duplicate"}))));
            }
            // reject stale nonce quickly
            let from_key = acct_key(&tx.sender_pubkey);
            let expected = *g.nonces.get(&from_key).unwrap_or(&0);
                if tx.nonce < expected {
                return (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({"status":"rejected","error": { "code": "stale_nonce", "message": format!("stale nonce: got {}, want >= {}", tx.nonce, expected) } }))));
            }

            // enforce mempool cap with fee-per-byte eviction preference for bulk lane
                    let total_len = g.mempool_critical.len() + g.mempool_bulk.len();
                    if total_len >= g.limits.mempool_max {
                if let Some(idx) = mempool::bulk_eviction_index(&g, &tx) {
                    g.mempool_bulk.remove(idx);
                } else if let Some((idx, min_tip)) = g.mempool_critical.iter().enumerate()
                    .min_by_key(|(_, t)| t.tip).map(|(i, t)| (i, t.tip)) {
                        if tx.tip > min_tip {
                            g.mempool_critical.remove(idx);
                        } else {
                            return (base_headers.clone(), (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"ignored","error": { "code": "mempool_full", "message": "mempool full; tip too low" } }))));
                        }
                } else {
                    return (base_headers.clone(), (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"ignored","error": { "code": "mempool_full", "message": "mempool full" } }))));
                }
            }

            // route into critical or bulk lane
            let critical_threshold: u64 = std::env::var("VISION_CRITICAL_TIP_THRESHOLD").ok().and_then(|s| s.parse().ok()).unwrap_or(1000);
            if tx.tip >= critical_threshold {
                g.mempool_critical.push_back(tx.clone());
            } else {
                g.mempool_bulk.push_back(tx.clone());
            }
            let th = hex::encode(tx_hash(&tx));
            g.mempool_ts.insert(th, now_ts());

            // best-effort fanout via local channel, else immediate peer broadcast
            if let Some(sender) = TX_BCAST_SENDER.get() {
                let _ = sender.try_send(tx.clone());
            } else {
                let peers: Vec<String> = g.peers.iter().cloned().collect();
                let tx_clone = tx.clone();
                tokio::spawn(async move { let _ = broadcast_tx_to_peers(peers, tx_clone).await; });
            }

            (base_headers.clone(), (StatusCode::OK, Json(serde_json::json!({"status":"accepted","tx_hash":h}))))
        }
        Err(e) => (base_headers.clone(), (StatusCode::BAD_REQUEST, Json(serde_json::json!({"status":"rejected","error": { "code": "bad_sig", "message": e.to_string() } })))),
    }
}

async fn gossip_block(ConnectInfo(addr): ConnectInfo<SocketAddr>, Json(GossipBlockEnvelope { block }): Json<GossipBlockEnvelope>) -> impl axum::response::IntoResponse {
    let ip = addr.ip().to_string();
    let base_headers = mempool::build_rate_limit_headers(&ip);
    let mut g = CHAIN.lock();
    if g.seen_blocks.contains(&block.header.pow_hash) {
        return (base_headers.clone(), Json(serde_json::json!({"status":"ignored","reason":"duplicate"})));
    }
    match apply_block_from_peer(&mut g, &block) {
        Ok(()) => {
            g.seen_blocks.insert(block.header.pow_hash.clone());
            let peers: Vec<String> = g.peers.iter().cloned().collect();
            let bclone = block.clone();
            tokio::spawn(async move {
                let _ = broadcast_block_to_peers(peers, bclone).await;
            });
            (base_headers.clone(), Json(serde_json::json!({"status":"accepted","height":block.header.number})))
        }
        Err(e) => (base_headers.clone(), Json(serde_json::json!({"status":"rejected","error": { "code": "apply_block_error", "message": e } }))),
    }
}

// =================== Broadcast helpers ===================
async fn broadcast_tx_to_peers(peers: Vec<String>, tx: Tx) -> Result<(), ()> {
    let env = serde_json::json!({ "tx": tx });
    for p in peers {
        let url = format!("{}/gossip/tx", p.trim_end_matches('/'));
        let _ = HTTP.post(url).json(&env).send().await;
        PROM_VISION_GOSSIP_OUT.inc();
    }
    Ok(())
}
async fn broadcast_block_to_peers(peers: Vec<String>, block: Block) -> Result<(), ()> {
    let env = serde_json::json!({ "block": block });
    for p in peers {
        let url = format!("{}/gossip/block", p.trim_end_matches('/'));
    let _ = HTTP.post(url).json(&env).send().await;
    PROM_VISION_GOSSIP_OUT.inc();
    }
    Ok(())
}

// ===== Snapshot types & endpoints (Phase A) =====
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SnapshotMeta {
    height: u64,
    state_root: String,
    ts: u64,
}

const SNAP_BLOB:   &str = "snap:blob";   // -> raw blob for snapshot
const SNAP_LATEST: &str = "snap:latest"; // -> json(SnapshotMeta)

async fn snapshot_save() -> (StatusCode, Json<serde_json::Value>) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let g = CHAIN.lock();
    let (height, state_root) = match g.blocks.last() {
        Some(b) => (b.header.number, b.header.state_root.clone()),
        None => (0, String::new()),
    };
    let blob = serde_json::json!({
        "height": height,
        "state_root": state_root,
        "balances": g.balances,
        "nonces": g.nonces,
    });
    let blob_bytes = serde_json::to_vec(&blob).unwrap_or_default();
    let meta = SnapshotMeta {
        height,
        state_root,
        ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    };
    let _ = g.db.insert(SNAP_LATEST.as_bytes(), serde_json::to_vec(&meta).unwrap());
    let _ = g.db.insert(SNAP_BLOB.as_bytes(), blob_bytes);
    let _ = g.db.flush();
    (StatusCode::OK, Json(serde_json::json!({"status":"ok","saved_height":height})))
}

async fn snapshot_latest() -> (StatusCode, Json<serde_json::Value>) {
    let g = CHAIN.lock();
    if let Some(m) = g.db.get(SNAP_LATEST.as_bytes()).unwrap() {
        if let Ok(meta) = serde_json::from_slice::<SnapshotMeta>(&m) {
            return (StatusCode::OK, Json(serde_json::json!(meta)));
        }
    }
    (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"no snapshot"})))
}

async fn snapshot_download() -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/octet-stream"));
    headers.insert(axum::http::header::CONTENT_DISPOSITION, axum::http::HeaderValue::from_static("attachment; filename=\"vision-snapshot.json\""));
    let g = CHAIN.lock();
    if let Some(b) = g.db.get(SNAP_BLOB.as_bytes()).unwrap() {
        return (StatusCode::OK, headers, b.to_vec());
    }
    (StatusCode::NOT_FOUND, headers, Vec::new())
}

#[inline]
fn apply_optional_tip(mut tx: Tx, opt_tip: Option<u64>) -> Tx {
    if let Some(t) = opt_tip {
        if t > tx.tip { tx.tip = t; }
    }
    tx
}

#[inline]
#[allow(dead_code)]
fn metrics_block_weight_limit() -> u64 {
    std::env::var("VISION_BLOCK_WEIGHT_LIMIT")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000_000)
}

// ===================== D O S  &  P E E R  H Y G I E N E  &  PROM METRICS =====================
// This block is self-contained and uses fully-qualified paths to avoid duplicate imports.

#[inline] fn __env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(default)
}
#[inline] fn __env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(default)
}

// ---- Prometheus exporter ----
async fn metrics_prom() -> impl axum::response::IntoResponse {
    let g = CHAIN.lock();

    let height: u64 = g.blocks.last().map(|b| b.header.number).unwrap_or(0);
    let peers: u64 = g.peers.len() as u64;
    let mempool_len: u64 = (g.mempool_critical.len() + g.mempool_bulk.len()) as u64;

    let block_weight_limit = g.limits.block_weight_limit;
    let last_weight: u64 = PROM_VISION_BLOCK_WEIGHT_LAST.get() as u64;
    let weight_util = if block_weight_limit > 0 {
        (last_weight as f64) / (block_weight_limit as f64)
    } else { 0.0 };

    let diff_bits: u64 = g.blocks.last().map(|b| b.header.difficulty).unwrap_or(1);
    let target_block_time = g.limits.target_block_time;
    let retarget_win = g.limits.retarget_window;

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let last_ts = g.blocks.last().map(|b| b.header.timestamp).unwrap_or(now);
    let last_block_time_secs = now.saturating_sub(last_ts);

    let mtp: u64 = 0;
    // compute tip percentiles across both lanes
    let mut tips: Vec<u64> = Vec::new();
    for t in g.mempool_critical.iter() { tips.push(t.tip); }
    for t in g.mempool_bulk.iter() { tips.push(t.tip); }
    tips.sort_unstable();
    let tip_p50: u64 = if tips.is_empty() { 0 } else { tips[tips.len()/2] };
    let tip_p95: u64 = if tips.is_empty() { 0 } else { tips[(tips.len() * 95 / 100).min(tips.len()-1)] };
    let mempool_crit_len = g.mempool_critical.len() as u64;
    let mempool_bulk_len = g.mempool_bulk.len() as u64;
    let reorgs = PROM_VISION_REORGS.get() as u64;
    // compute fee-per-byte percentiles for mempool (simple est_tx_weight)
    let mut fee_per_byte: Vec<f64> = Vec::new();
    for t in g.mempool_critical.iter().chain(g.mempool_bulk.iter()) {
        let w = est_tx_weight(t) as f64;
        if w > 0.0 { fee_per_byte.push((t.tip as f64) / w); }
    }
    fee_per_byte.sort_by(|a,b| a.partial_cmp(b).unwrap());
    let fpb_p50 = if fee_per_byte.is_empty() { 0.0 } else { fee_per_byte[fee_per_byte.len()/2] };
    let fpb_p95 = if fee_per_byte.is_empty() { 0.0 } else { fee_per_byte[(fee_per_byte.len()*95/100).min(fee_per_byte.len()-1)] };
    let body = format!(
        "# HELP vision_height Current chain height\n# TYPE vision_height gauge\nvision_height {}\n\
         # HELP vision_peers Connected peers\n# TYPE vision_peers gauge\nvision_peers {}\n\
         # HELP vision_mempool_len Mempool size\n# TYPE vision_mempool_len gauge\nvision_mempool_len {}\n\
         # HELP vision_mempool_critical_len Critical lane mempool size\n# TYPE vision_mempool_critical_len gauge\nvision_mempool_critical_len {}\n\
         # HELP vision_mempool_bulk_len Bulk lane mempool size\n# TYPE vision_mempool_bulk_len gauge\nvision_mempool_bulk_len {}\n\
         # HELP vision_block_weight_util Block weight utilization (0..1)\n# TYPE vision_block_weight_util gauge\nvision_block_weight_util {}\n\
         # HELP vision_difficulty_bits Current target difficulty (leading-zero bits)\n# TYPE vision_difficulty_bits gauge\nvision_difficulty_bits {}\n\
         # HELP vision_target_block_time Target block time (seconds)\n# TYPE vision_target_block_time gauge\nvision_target_block_time {}\n\
         # HELP vision_retarget_window Retarget window (blocks)\n# TYPE vision_retarget_window gauge\nvision_retarget_window {}\n\
         # HELP vision_last_block_time_secs Seconds since tip timestamp\n# TYPE vision_last_block_time_secs gauge\nvision_last_block_time_secs {}\n\
         # HELP vision_mtp Median time past\n# TYPE vision_mtp gauge\nvision_mtp {}\n\
    # HELP vision_reorgs Reorg events observed\n# TYPE vision_reorgs counter\nvision_reorgs {}\n\
    # HELP vision_side_blocks Number of stored side/orphan blocks\n# TYPE vision_side_blocks gauge\nvision_side_blocks {}\n\
    # HELP vision_snapshot_count Number of snapshots persisted\n# TYPE vision_snapshot_count counter\nvision_snapshot_count {}\n\
    # HELP vision_reorg_length_total Total number of blocks moved by reorgs\n# TYPE vision_reorg_length_total counter\nvision_reorg_length_total {}\n\
         # HELP vision_fee_tip_p50 Median fee tip\n# TYPE vision_fee_tip_p50 gauge\nvision_fee_tip_p50 {}\n\
         # HELP vision_fee_tip_p95 95th percentile fee tip\n# TYPE vision_fee_tip_p95 gauge\nvision_fee_tip_p95 {}\n",
        height, peers, mempool_len, mempool_crit_len, mempool_bulk_len, weight_util, diff_bits, target_block_time, retarget_win, last_block_time_secs, mtp, reorgs, PROM_VISION_SIDE_BLOCKS.get() as u64, PROM_VISION_SNAPSHOTS.get() as u64, PROM_VISION_REORG_LENGTH_TOTAL.get() as u64, tip_p50, tip_p95
    );

    // include admin ping counter (from Prometheus registry)
    let admin_pings = PROM_ADMIN_PING_TOTAL.get() as u64;
    let sweeps_count = PROM_VISION_MEMPOOL_SWEEPS.get() as u64;
    let removed_total = PROM_VISION_MEMPOOL_REMOVED_TOTAL.get() as u64;
    let removed_last = PROM_VISION_MEMPOOL_REMOVED_LAST.get() as i64 as u64;
    let last_ms = PROM_VISION_MEMPOOL_SWEEP_LAST_MS.get() as i64 as u64;
    let body = format!("{}\n# HELP vision_admin_ping_total Total admin ping requests\n# TYPE vision_admin_ping_total counter\nvision_admin_ping_total {}\n", body, admin_pings);
    let body = format!("{}\n# HELP vision_block_weight_limit Configured block weight limit (bytes)\n# TYPE vision_block_weight_limit gauge\nvision_block_weight_limit {}\n# HELP vision_snapshot_every_blocks Snapshot cadence (blocks)\n# TYPE vision_snapshot_every_blocks gauge\nvision_snapshot_every_blocks {}\n# HELP vision_fee_per_byte_p50 Median fee-per-byte in mempool\n# TYPE vision_fee_per_byte_p50 gauge\nvision_fee_per_byte_p50 {}\n# HELP vision_fee_per_byte_p95 95th percentile fee-per-byte in mempool\n# TYPE vision_fee_per_byte_p95 gauge\nvision_fee_per_byte_p95 {}\n# HELP vision_mempool_sweep_runs Total mempool sweeper runs\n# TYPE vision_mempool_sweep_runs counter\nvision_mempool_sweep_runs {}\n# HELP vision_mempool_removed_total Total mempool entries removed by TTL\n# TYPE vision_mempool_removed_total counter\nvision_mempool_removed_total {}\n# HELP vision_mempool_removed_last Number of entries removed in the last sweep\n# TYPE vision_mempool_removed_last gauge\nvision_mempool_removed_last {}\n# HELP vision_mempool_sweep_last_ms Last mempool sweep duration (ms)\n# TYPE vision_mempool_sweep_last_ms gauge\nvision_mempool_sweep_last_ms {}\n",
        body, block_weight_limit, g.limits.snapshot_every_blocks, fpb_p50, fpb_p95, sweeps_count, removed_total, removed_last, last_ms);

    // Render prometheus registry metrics (including the native histogram) and append to body
    let encoder = TextEncoder::new();
    let metric_families = PROM_REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).ok();
    let prom_text = String::from_utf8_lossy(&buffer);
    let body = format!("{}{}", body, prom_text);
    let headers = [(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("text/plain; version=0.0.4"))];
    (headers, body)
}

// ---- Peer hygiene core ----
#[derive(Serialize, Clone, Debug, Default)]
struct __PeerMeta {
    next_retry_at: u64,
    last_rtt_ms: u32,
    fail_count: u32,
    cap_window_start: u64,
    cap_count: u32,
    last_ok: Option<u64>,
}

// public alias used by handlers
type PeerMeta = __PeerMeta;
#[allow(dead_code)]
#[derive(Default)]
struct __PeerHygiene {
    meta: std::collections::BTreeMap<String, __PeerMeta>,
    recent: std::collections::VecDeque<(u64, String)>,
    recent_set: std::collections::BTreeSet<String>,
}
static __PEER_HYGIENE: once_cell::sync::Lazy<parking_lot::Mutex<__PeerHygiene>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(__PeerHygiene::default()));

#[inline] fn __now_secs() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() }

const __PEER_RECENT_LRU: usize = 256;
const __PEER_RECENT_TTL_SECS: u64 = 30;
const __PEER_BACKOFF_MAX_MS: u64 = 30_000;
const __PEER_BACKOFF_BASE_MS: u64 = 500;
const __PEER_SLOW_RTT_MS: u64 = 1500;
const __PEER_CAP_WINDOW_SECS: u64 = 60;
const __PER_PEER_TX_CAP: usize = 500;

#[allow(dead_code)]
fn hygiene_allow_add(url: &str) -> bool {
    let now = __now_secs();
    let mut h = __PEER_HYGIENE.lock();
    while let Some((t,u)) = h.recent.front().cloned() {
        if now.saturating_sub(t) > __PEER_RECENT_TTL_SECS || h.recent.len() > __PEER_RECENT_LRU {
            h.recent.pop_front(); h.recent_set.remove(&u);
        } else { break; }
    }
    if h.recent_set.contains(url) { return false; }
    h.recent.push_back((now, url.to_string()));
    h.recent_set.insert(url.to_string());
    h.meta.entry(url.to_string()).or_default();
    true
}
#[allow(dead_code)]
fn hygiene_on_fail(url: &str) {
    let now = __now_secs();
    let mut h = __PEER_HYGIENE.lock();
    let m = h.meta.entry(url.to_string()).or_default();
    m.fail_count = m.fail_count.saturating_add(1);
    let exp = (1u64 << m.fail_count.min(10)) * __PEER_BACKOFF_BASE_MS;
    let jitter = (now % 200) * 7;
    let next = (exp + jitter).min(__PEER_BACKOFF_MAX_MS);
    m.next_retry_at = now + (next / 1000);
}
#[allow(dead_code)]
fn hygiene_on_ok(url: &str, rtt_ms: u32) {
    let now = __now_secs();
    let mut h = __PEER_HYGIENE.lock();
    let m = h.meta.entry(url.to_string()).or_default();
    m.last_rtt_ms = rtt_ms;
    m.fail_count = 0;
    m.next_retry_at = now;
}
#[allow(dead_code)]
fn hygiene_should_dial(url: &str) -> bool {
    let now = __now_secs();
    let h = __PEER_HYGIENE.lock();
    match h.meta.get(url) { Some(m) => now >= m.next_retry_at, _ => true }
}
fn hygiene_should_evict(url: &str) -> bool {
    let h = __PEER_HYGIENE.lock();
    if let Some(m) = h.meta.get(url) { return (m.last_rtt_ms as u64) > __PEER_SLOW_RTT_MS && m.fail_count >= 2; }
    false
}
#[allow(dead_code)]
fn hygiene_ingest_allow(url: &str) -> bool {
    let now = __now_secs();
    let mut h = __PEER_HYGIENE.lock();
    let m = h.meta.entry(url.to_string()).or_default();
    if now.saturating_sub(m.cap_window_start) >= __PEER_CAP_WINDOW_SECS {
        m.cap_window_start = now; m.cap_count = 0;
    }
    if (m.cap_count as usize) >= __PER_PEER_TX_CAP { return false; }
    m.cap_count += 1;
    true
}

// Simple per-peer leaky bucket rate limiter
#[derive(Clone)]
struct LeakyBucket {
    capacity: u64,
    refill_per_sec: u64,
    tokens: f64,
    last_ts: u64,
}
impl LeakyBucket {
    fn allow(&mut self) -> bool {
        let now = __now_secs();
        let elapsed = now.saturating_sub(self.last_ts) as f64;
        self.tokens = (self.tokens + (elapsed * self.refill_per_sec as f64)).min(self.capacity as f64);
        self.last_ts = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else { false }
    }
}

static PER_PEER_BUCKETS: once_cell::sync::Lazy<parking_lot::Mutex<std::collections::BTreeMap<String, LeakyBucket>>> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(std::collections::BTreeMap::new()));

fn peer_allow(peer: &str) -> bool {
    let cap = __env_u64("VISION_RATE_CAP", 10);
    let refill = __env_u64("VISION_RATE_REFILL_PER_SEC", 5);
    let mut m = PER_PEER_BUCKETS.lock();
    let b = m.entry(peer.to_string()).or_insert(LeakyBucket { capacity: cap, refill_per_sec: refill, tokens: cap as f64, last_ts: __now_secs() });
    b.allow()
}

// ---- Peer hygiene admin ----
#[derive(serde::Serialize)]
struct __PeerStat {
    url: String,
    next_retry_at: u64,
    last_rtt_ms: u32,
    fail_count: u32,
    cap_window_start: u64,
    cap_count: u32,
}
#[derive(serde::Serialize)]
struct __PeerStatsView { stats: Vec<__PeerStat> }

async fn peers_stats() -> axum::Json<__PeerStatsView> {
    let h = __PEER_HYGIENE.lock();
    let mut out = Vec::new();
    for (url, m) in h.meta.iter() {
        out.push(__PeerStat {
            url: url.clone(),
            next_retry_at: m.next_retry_at,
            last_rtt_ms: m.last_rtt_ms,
            fail_count: m.fail_count,
            cap_window_start: m.cap_window_start,
            cap_count: m.cap_count,
        });
    }
    axum::Json(__PeerStatsView { stats: out })
}

#[derive(serde::Deserialize)]
struct __EvictReq { url: String }

async fn peers_evict_slow(axum::Json(req): axum::Json<__EvictReq>) -> impl axum::response::IntoResponse {
    let url = req.url;
    if hygiene_should_evict(&url) {
        let mut g = CHAIN.lock();
        let existed = g.peers.remove(&url);
        return (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "evicted": existed, "url": url })));
    }
    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({ "evicted": false, "url": url })))
}

// Background loop: periodically ping peers' /status and update PEERS metadata
async fn peer_hygiene_loop() {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        // snapshot peer list
        let peers: Vec<String> = { let m = PEERS.lock(); m.keys().cloned().collect() };
        for p in peers {
            let url = format!("{}/status", p.trim_end_matches('/'));
            let start = std::time::Instant::now();
            let resp = HTTP.get(&url).timeout(Duration::from_millis(500)).send().await;
            let _ms = start.elapsed().as_millis() as u64;
            match resp {
                Ok(r) => {
                    if r.status().is_success() {
                        hygiene_on_ok(&p, 0);
                        let mut m = PEERS.lock(); if let Some(entry) = m.get_mut(&p) { entry.last_ok = Some(now_ts()); entry.fail_count = 0; }
                    } else {
                        hygiene_on_fail(&p);
                        let mut m = PEERS.lock(); if let Some(entry) = m.get_mut(&p) { entry.fail_count = entry.fail_count.saturating_add(1); }
                    }
                }
                Err(_) => {
                    hygiene_on_fail(&p);
                    let mut m = PEERS.lock(); if let Some(entry) = m.get_mut(&p) { entry.fail_count = entry.fail_count.saturating_add(1); }
                }
            }
            // evict peers with too many failures
            let mut m = PEERS.lock();
            if let Some(entry) = m.get(&p) {
                if entry.fail_count >= 5 {
                    m.remove(&p);
                }
            }
        }
    }
}

// ---- DoS preflight ----
#[inline] fn dos_mempool_max() -> usize { __env_usize("VISION_MEMPOOL_MAX", 10_000) }
#[inline] fn dos_mempool_per_sender_max() -> usize { __env_usize("VISION_MEMPOOL_PER_SENDER_MAX", 2000) }
#[inline] fn dos_tx_weight_max() -> u64 { __env_u64("VISION_TX_WEIGHT_MAX", 50_000) }
#[allow(dead_code)]
#[inline] fn dos_tx_args_max() -> usize { __env_usize("VISION_TX_ARGS_MAX", 64) }

#[inline] fn tx_sender_id(_tx: &Tx) -> String { "anon".to_string() }

#[inline] fn est_tx_weight(_tx: &Tx) -> u64 { 200 }

fn preflight_violation(tx: &Tx, chain: &Chain) -> Option<String> {
    let w = est_tx_weight(tx);
    if w > dos_tx_weight_max() {
        return Some(format!("tx too heavy: {} > {}", w, dos_tx_weight_max()));
    }
    if chain.mempool_critical.len() + chain.mempool_bulk.len() >= dos_mempool_max() {
        return Some("mempool full".to_string());
    }
    let sid = tx_sender_id(tx);
    let mut count = 0usize;
    for t in chain.mempool_critical.iter().chain(chain.mempool_bulk.iter()) {
        if tx_sender_id(t) == sid { count += 1; if count >= dos_mempool_per_sender_max() { break; } }
    }
    if count >= dos_mempool_per_sender_max() {
        return Some("sender mempool cap reached".to_string());
    }
    None
}

// Intrinsic cost model: currently a simple weight-based cost
#[inline]
fn intrinsic_cost(tx: &Tx) -> u64 {
    // For now intrinsic cost is the estimated weight in units. This can be extended
    // to include per-arg costs, per-recipient costs, or a basefee multiplier.
    est_tx_weight(tx) as u64
}

// Mempool helpers moved to src/mempool.rs; use `mempool::` functions.
// ===================== / PACK END =====================

// Serve OpenAPI YAML from repo root if present
async fn openapi_spec() -> (StatusCode, HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::CONTENT_TYPE, HeaderValue::from_static("application/yaml"));
    match std::fs::read_to_string("openapi.yaml") {
        Ok(s) => (StatusCode::OK, headers, s),
        Err(_) => (StatusCode::NOT_FOUND, headers, String::new()),
    }
}

// Canonical API error helper returning JSON error body with proper status
fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": message })))
}

// Structured API error with optional machine code
#[derive(Serialize)]
struct ApiErrorBody {
    code: String,
    message: String,
}
fn api_error_struct(status: StatusCode, code: &str, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    let body = serde_json::json!({ "error": { "code": code, "message": message } });
    (status, Json(body))
}
    

#[cfg(test)]
mod extra_api_tests {
    use super::*;
    #[test]
    fn build_rate_limit_headers_returns_headers_when_bucket_present() {
        // ensure there's an entry for test IP
        IP_TOKEN_BUCKETS.insert("127.0.0.1".to_string(), TokenBucket { tokens: 2.0, capacity: 5.0, refill_per_sec: 1.0, last_ts: now_ts() });
    let h = mempool::build_rate_limit_headers("127.0.0.1");
        assert!(h.contains_key("x-ratelimit-limit"));
        assert!(h.contains_key("x-ratelimit-remaining"));
        IP_TOKEN_BUCKETS.remove("127.0.0.1");
    }

    #[tokio::test]
    async fn submit_tx_rbf_tip_too_low_returns_409() {
        use super::*;
    // Build a fresh chain and insert a tx into mempool with tip 100
    let mut g = fresh_chain();
    // read alice pubkey so existing and incoming share the same sender id
    let keys_json = std::fs::read_to_string("keys/alice.json").expect("read alice key");
    let kv: serde_json::Value = serde_json::from_str(&keys_json).unwrap();
    let pk_hex = kv.get("public_key").and_then(|v| v.as_str()).expect("public_key");
    let existing = Tx { nonce: 0, sender_pubkey: pk_hex.to_string(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 100, fee_limit: 1000, sig: String::new() };
    g.mempool_bulk.push_back(existing.clone());
        let mut global = CHAIN.lock();
        *global = g;
    // release the lock so the handler can acquire it (avoid deadlock in-test)
    drop(global);

        // Now attempt to submit a tx with same sender+nonce but lower tip
    // sign the incoming tx with alice's secret so verify_tx passes
    let mut incoming = Tx { nonce: 0, sender_pubkey: "498081449d1c3b867223905f36e2cff8dab7621c13a46696f6ad2581c01ad1bf".into(), access_list: vec![], module: "m".into(), method: "mm".into(), args: vec![], tip: 50, fee_limit: 1000, sig: String::new() };
    // load secret key from test fixtures
    let keys_json = std::fs::read_to_string("keys/alice.json").expect("read alice key");
    let kv: serde_json::Value = serde_json::from_str(&keys_json).unwrap();
    let sk_hex = kv.get("secret_key").and_then(|v| v.as_str()).expect("secret_key");
    let pk_hex = kv.get("public_key").and_then(|v| v.as_str()).expect("public_key");
    let mut keypair_bytes: Vec<u8> = hex::decode(sk_hex).unwrap();
    keypair_bytes.extend_from_slice(&hex::decode(pk_hex).unwrap());
    let keypair = ed25519_dalek::Keypair::from_bytes(&keypair_bytes).expect("keypair");
    use ed25519_dalek::Signer;
    let msg = signable_tx_bytes(&incoming);
    let sig = keypair.sign(&msg).to_bytes();
    incoming.sig = hex::encode(sig);
        let submit = SubmitTx { tx: incoming };
        let addr = std::net::SocketAddr::from(([127,0,0,1], 0));
    let resp = submit_tx(ConnectInfo(addr), Json(submit)).await;
    // Convert into a concrete Response and inspect
    let response = resp.into_response();
    // inspect body to determine which early-return fired
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024).await.expect("body read");
    let body_txt = std::str::from_utf8(&bytes).unwrap_or("<non-utf8>");
    eprintln!("submit_tx test response status={} body={}", status, body_txt);
    // try to parse error code if present
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if let Some(code) = v.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_str()) {
            assert_eq!(code, "rbf_tip_too_low", "expected rbf_tip_too_low code");
        } else if let Some(obj) = v.get("error").and_then(|e| e.get("error")).and_then(|e| e.get("code")).and_then(|c| c.as_str()) {
            // some handlers nest error under error.error.code
            assert_eq!(obj, "rbf_tip_too_low", "expected nested rbf_tip_too_low code");
        } else {
            panic!("unexpected error body: {}", body_txt);
        }
    } else {
        panic!("non-json body: {}", body_txt);
    }
    }

    #[tokio::test]
    async fn openapi_spec_serves_file_if_present() {
        let (status, headers, _body) = openapi_spec().await;
        // either OK or NOT_FOUND depending on workspace file presence; ensure Content-Type is set when returned
        assert!(headers.get(axum::http::header::CONTENT_TYPE).is_some());
        // status should be either OK or NOT_FOUND
        assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cors_preflight_returns_allow_origin() {
        use tower::util::ServiceExt;
        // build a simple router with one route and apply permissive CORS
        let app = build_app().layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));
        let req = axum::http::Request::builder()
            .method(axum::http::Method::OPTIONS)
            .uri("/submit_tx")
            .header("origin", "http://example.com")
            .header("access-control-request-method", "POST")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // preflight should include Access-Control-Allow-Origin when allowed
        assert!(resp.headers().get("access-control-allow-origin").is_some());
    }
}
