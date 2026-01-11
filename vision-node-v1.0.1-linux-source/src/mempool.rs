use crate::{
    acct_key, est_tx_weight, fee_base, intrinsic_cost, now_ts, tx_hash, Chain, Tx, IP_TOKEN_BUCKETS,
};
use axum::http::{HeaderMap, HeaderValue};
use std::collections::BTreeMap;

// Two-lane block builder: exposed helper used by main
pub fn build_block_from_mempool(g: &mut Chain, max_txs: usize, weight_limit: u64) -> Vec<Tx> {
    // Two-lane builder: pick from critical lane first (by tip ordering), then fill from bulk.
    let _critical_threshold: u64 = std::env::var("VISION_CRITICAL_TIP_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let mut per_sender_expected: BTreeMap<String, u64> = g.nonces.clone();
    let mut chosen: Vec<Tx> = Vec::new();
    let mut used_weight: u64 = 0;

    // Helper to try select a tx
    let mut try_select = |tx: &Tx| -> bool {
        let w = est_tx_weight(tx);
        if weight_limit > 0 && used_weight.saturating_add(w) > weight_limit {
            return false;
        }
        let from = acct_key(&tx.sender_pubkey);
        let expected = *per_sender_expected.get(&from).unwrap_or(&0);
        if tx.nonce != expected {
            return false;
        }
        per_sender_expected.insert(from, expected.saturating_add(1));
        used_weight = used_weight.saturating_add(w);
        true
    };

    // Select from critical lane first (by tip desc)
    let mut crit: Vec<Tx> = g.mempool_critical.iter().cloned().collect();
    crit.sort_by(|a, b| b.tip.cmp(&a.tip));
    for tx in crit.iter() {
        if chosen.len() >= max_txs {
            break;
        }
        if try_select(tx) {
            chosen.push(tx.clone());
        }
    }

    // Fill from bulk lane
    if chosen.len() < max_txs {
        // Sort bulk by fee-per-weight (tip / est_weight) desc
        let mut bulk: Vec<Tx> = g.mempool_bulk.iter().cloned().collect();
        bulk.sort_by(|a, b| {
            let wa = est_tx_weight(a) as f64;
            let wb = est_tx_weight(b) as f64;
            let fa = if wa > 0.0 { (a.tip as f64) / wa } else { 0.0 };
            let fb = if wb > 0.0 { (b.tip as f64) / wb } else { 0.0 };
            fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
        });
        for tx in bulk.iter() {
            if chosen.len() >= max_txs {
                break;
            }
            if try_select(tx) {
                chosen.push(tx.clone());
            }
        }
    }

    // Remove chosen txs from mempool lanes by matching tx hash
    for tx in &chosen {
        let th = hex::encode(tx_hash(tx));
        if let Some(pos) = g
            .mempool_critical
            .iter()
            .position(|t| hex::encode(tx_hash(t)) == th)
        {
            g.mempool_critical.remove(pos);
            continue;
        }
        if let Some(pos) = g
            .mempool_bulk
            .iter()
            .position(|t| hex::encode(tx_hash(t)) == th)
        {
            g.mempool_bulk.remove(pos);
            continue;
        }
    }

    chosen
}

pub fn bulk_eviction_index(g: &Chain, incoming: &Tx) -> Option<usize> {
    const SCALE: u128 = 1_000_000;
    if g.mempool_bulk.is_empty() {
        return None;
    }
    let in_w = est_tx_weight(incoming) as u128;
    if in_w == 0 {
        return None;
    }
    let in_score = (incoming.tip as u128).saturating_mul(SCALE) / in_w;
    let mut min_idx: Option<usize> = None;
    let mut min_score: u128 = u128::MAX;
    for (i, t) in g.mempool_bulk.iter().enumerate() {
        let w = est_tx_weight(t) as u128;
        if w == 0 {
            continue;
        }
        let score = (t.tip as u128).saturating_mul(SCALE) / w;
        if score < min_score {
            min_score = score;
            min_idx = Some(i);
        }
    }
    if let Some(idx) = min_idx {
        if in_score > min_score {
            return Some(idx);
        }
    }
    None
}

pub fn prune_mempool(g: &mut Chain) {
    // measure duration for any caller
    let start = std::time::Instant::now();
    let ttl = std::env::var("VISION_MEMPOOL_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    if ttl == 0 {
        return;
    }
    let now = now_ts();
    let mut removed: Vec<String> = Vec::new();
    g.mempool_critical.retain(|tx| {
        let h = hex::encode(tx_hash(tx));
        if let Some(ts) = g.mempool_ts.get(&h) {
            if now.saturating_sub(*ts) > ttl {
                removed.push(h.clone());
                return false;
            }
        }
        true
    });
    g.mempool_bulk.retain(|tx| {
        let h = hex::encode(tx_hash(tx));
        if let Some(ts) = g.mempool_ts.get(&h) {
            if now.saturating_sub(*ts) > ttl {
                removed.push(h.clone());
                return false;
            }
        }
        true
    });
    for h in &removed {
        g.seen_txs.remove(h);
        g.mempool_ts.remove(h);
    }
    // update global sweep metrics (use Prometheus metrics; remove legacy atomics)
    let removed_count = removed.len() as u64;
    crate::PROM_VISION_MEMPOOL_SWEEPS.inc();
    crate::PROM_VISION_MEMPOOL_REMOVED_TOTAL.inc_by(removed_count);
    crate::PROM_VISION_MEMPOOL_REMOVED_LAST.set(removed_count as i64);
    // finish timing & update duration metric (Prometheus gauge + histogram)
    let dur_ms = start.elapsed().as_millis() as u64;
    crate::PROM_VISION_MEMPOOL_SWEEP_LAST_MS.set(dur_ms as i64);
    // determine mempool size at end of sweep
    let mempool_len = (g.mempool_critical.len() + g.mempool_bulk.len()) as u64;
    // record history entry (timestamp, removed_count, duration_ms, mempool_size)
    {
        let mut hist = crate::VISION_MEMPOOL_SWEEP_HISTORY.lock();
        hist.push_back((now, removed_count, dur_ms, mempool_len));
        if hist.len() > 10 {
            hist.pop_front();
        }
    }
    // observe prometheus histogram (seconds)
    let dur_secs = (dur_ms as f64) / 1000.0;
    crate::VISION_MEMPOOL_SWEEP_DURATION_HISTOGRAM.observe(dur_secs);
}

pub fn spawn_mempool_sweeper() {
    let interval = std::env::var("VISION_MEMPOOL_SWEEP_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            let mut g = crate::CHAIN.lock();
            prune_mempool(&mut g);
        }
    });
}

pub fn is_higher_priority(a: &Tx, b: &Tx, g: &Chain) -> bool {
    if a.tip != b.tip {
        return a.tip > b.tip;
    }
    let ha = hex::encode(tx_hash(a));
    let hb = hex::encode(tx_hash(b));
    let ta = g.mempool_ts.get(&ha).cloned().unwrap_or(0);
    let tb = g.mempool_ts.get(&hb).cloned().unwrap_or(0);
    if ta != tb {
        return ta < tb;
    }
    let wa = est_tx_weight(a);
    let wb = est_tx_weight(b);
    wa < wb
}

/// Try to replace an existing tx with the same (sender_pubkey, nonce).
/// Returns Ok(true) if replaced, Ok(false) if no existing tx found,
/// or Err("rbf_tip_too_low") if an existing tx was found but incoming.tip is not strictly higher.
pub fn try_replace_sender_nonce(g: &mut Chain, incoming: &Tx) -> Result<bool, &'static str> {
    for (i, t) in g.mempool_critical.iter().enumerate() {
        if t.sender_pubkey == incoming.sender_pubkey && t.nonce == incoming.nonce {
            if incoming.tip > t.tip {
                let old_hash = hex::encode(tx_hash(t));
                g.mempool_critical.remove(i);
                g.seen_txs.remove(&old_hash);
                g.mempool_ts.remove(&old_hash);
                return Ok(true);
            } else {
                return Err("rbf_tip_too_low");
            }
        }
    }
    for (i, t) in g.mempool_bulk.iter().enumerate() {
        if t.sender_pubkey == incoming.sender_pubkey && t.nonce == incoming.nonce {
            if incoming.tip > t.tip {
                let old_hash = hex::encode(tx_hash(t));
                g.mempool_bulk.remove(i);
                g.seen_txs.remove(&old_hash);
                g.mempool_ts.remove(&old_hash);
                return Ok(true);
            } else {
                return Err("rbf_tip_too_low");
            }
        }
    }
    Ok(false)
}

pub fn validate_for_mempool(tx: &Tx, g: &Chain) -> Result<(), String> {
    if serde_json::to_vec(tx)
        .map_err(|_| "json".to_string())?
        .len()
        > 64 * 1024
    {
        return Err("tx too big".into());
    }
    let base = fee_base();
    let required = intrinsic_cost(tx).saturating_add(base as u64);
    if tx.fee_limit < required {
        return Err(format!(
            "fee_limit {} below required (intrinsic+base) {}",
            tx.fee_limit, required
        ));
    }
    let from_key = acct_key(&tx.sender_pubkey);
    let expected = *g.nonces.get(&from_key).unwrap_or(&0);
    if tx.nonce < expected {
        return Err(format!(
            "stale nonce: got {}, want >= {}",
            tx.nonce, expected
        ));
    }
    if tx.nonce > expected + 1 {
        return Err(format!(
            "nonce gap too large: got {}, expected <= {}",
            tx.nonce,
            expected + 1
        ));
    }
    for t in g.mempool_critical.iter().chain(g.mempool_bulk.iter()) {
        if t.sender_pubkey == tx.sender_pubkey && t.nonce == tx.nonce {
            return Err("duplicate sender+nonce in mempool".into());
        }
    }
    Ok(())
}

pub fn admission_check_under_load(g: &Chain, incoming: &Tx) -> Result<(), String> {
    let cap = g.limits.mempool_max;
    let total = g.mempool_critical.len() + g.mempool_bulk.len();
    if total < cap {
        return Ok(());
    }
    let mut worst: Option<(&Tx, usize, bool)> = None;
    for (i, t) in g.mempool_critical.iter().enumerate() {
        if worst.is_none() {
            worst = Some((t, i, true));
        } else if let Some((w, _wi, _wc)) = worst {
            if is_higher_priority(w, t, g) {
                worst = Some((t, i, true));
            }
        }
    }
    for (i, t) in g.mempool_bulk.iter().enumerate() {
        if worst.is_none() {
            worst = Some((t, i, false));
        } else if let Some((w, _wi, _wc)) = worst {
            if is_higher_priority(w, t, g) {
                worst = Some((t, i, false));
            }
        }
    }
    if let Some((wtx, _idx, _is_crit)) = worst {
        if is_higher_priority(incoming, wtx, g) {
            return Ok(());
        }
        return Err("mempool full; tip too low under load".into());
    }
    Err("mempool full".into())
}

pub fn build_rate_limit_headers(ip: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if let Some(ent) = IP_TOKEN_BUCKETS.get(ip) {
        let tb = ent.value();
        let cap = tb.capacity as u64;
        let rem = tb.tokens.floor() as u64;
        let reset = if tb.refill_per_sec > 0.0 {
            ((tb.capacity - tb.tokens) / tb.refill_per_sec).ceil() as u64
        } else {
            0
        };
        headers.insert(
            axum::http::header::HeaderName::from_static("x-ratelimit-limit"),
            HeaderValue::from_str(&cap.to_string()).unwrap(),
        );
        headers.insert(
            axum::http::header::HeaderName::from_static("x-ratelimit-remaining"),
            HeaderValue::from_str(&rem.to_string()).unwrap(),
        );
        headers.insert(
            axum::http::header::HeaderName::from_static("x-ratelimit-reset"),
            HeaderValue::from_str(&reset.to_string()).unwrap(),
        );
    }
    headers
}
// additional small helpers available from crate

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MempoolCfg {
    pub max_bytes: usize,
    pub max_count: usize,
    pub max_per_sender: usize,
    pub min_tip: u64,
    pub max_tx_size: usize,
}

impl Default for MempoolCfg {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024, // 8MB
            max_count: 10_000,
            max_per_sender: 128,
            min_tip: 0,
            max_tx_size: 64 * 1024,
        }
    }
}

/// Admit a tx if it meets basic caps & policy. Caller enqueues on success.
#[allow(dead_code)]
pub fn admit_tx_with_policy(
    tx: &Tx,
    mempool_len: usize,
    mempool_bytes_estimate: usize,
    per_sender_counts: &BTreeMap<String, usize>,
    cfg: &MempoolCfg,
) -> Result<(), String> {
    if mempool_len >= cfg.max_count {
        return Err("mempool full".into());
    }
    if mempool_bytes_estimate >= cfg.max_bytes {
        return Err("mempool bytes cap reached".into());
    }
    // size check
    if serde_json::to_vec(tx).map_err(|_| "json")?.len() > cfg.max_tx_size {
        return Err("tx too large".into());
    }
    // tip
    if tx.tip < cfg.min_tip {
        return Err("tip below minimum".into());
    }
    // per-sender cap
    let sk = acct_key(&tx.sender_pubkey);
    if per_sender_counts.get(&sk).cloned().unwrap_or(0) >= cfg.max_per_sender {
        return Err("too many pending from this sender".into());
    }
    Ok(())
}
