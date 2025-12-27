#![allow(dead_code)]

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::metrics;

#[derive(Clone)]
pub struct Receipts {
    pub tree_name: &'static str, // "receipts"
}

impl Default for Receipts {
    fn default() -> Self {
        Self {
            tree_name: "receipts",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    pub id: String,   // monotonically-ordered key (ts_ns-<rand or counter>)
    pub ts_ms: u64,   // wallclock for UI
    pub kind: String, // "transfer" | "mint" | "burn" | "market_settle" | etc.
    pub from: String,
    pub to: String,
    pub amount: String, // decimal string to avoid JS rounding
    pub fee: String,    // decimal string
    pub memo: Option<String>,
    pub txid: Option<String>, // if you later attach L1 txid
    pub ok: bool,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    pub limit: Option<usize>, // default 100
}

#[derive(Clone)]
pub struct AppState {
    pub dbctx: Arc<metrics::DbCtx>,
    pub metrics: Arc<metrics::Metrics>,
    // add anything else your node already has in AppState
}

pub async fn get_latest(
    State(state): State<AppState>,
    Query(q): Query<LatestQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(100).min(500);

    let db = &state.dbctx.db;
    let tree = match db.open_tree("receipts") {
        Ok(t) => t,
        Err(e) => return api_err(500, &format!("db_open: {e}")),
    };

    // Iterate from the end (highest keys) backwards
    let mut out = Vec::with_capacity(limit);
    let mut count = 0usize;

    for item in tree.iter().rev() {
        if count >= limit {
            break;
        }
        match item {
            Ok((_, val)) => {
                if let Ok(rec) = bincode::deserialize::<Receipt>(&val) {
                    out.push(rec);
                    count += 1;
                }
            }
            Err(e) => return api_err(500, &format!("db_iter: {e}")),
        }
    }

    Json(out).into_response()
}

pub fn write_receipt(
    db: &sled::Db,
    metrics: Option<&crate::metrics::Metrics>,
    mut rec: Receipt,
) -> anyhow::Result<()> {
    let tree = db.open_tree("receipts")?;
    if rec.ts_ms == 0 {
        rec.ts_ms = now_ms();
    }
    // Monotonic-ish key: ts_ns + small counter component
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let key = format!("{:020}-{:06}", now_ns(), (c % 1_000_000));
    rec.id = key.clone();

    let bytes = bincode::serialize(&rec)?;
    tree.insert(key.as_bytes(), bytes)?;

    // Update metrics if provided
    if let Some(m) = metrics {
        m.wallet_receipts_written.inc();
    }

    Ok(())
}

// --- helpers ---

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

// Uniform JSON error per api_error_schema.md
fn api_err(code: u16, err: &str) -> axum::response::Response {
    use axum::http::{HeaderValue, StatusCode};
    use serde_json::json;

    let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = json!({
        "status": "rejected",
        "code": code,
        "error": err
    })
    .to_string();

    let mut resp = (status, body).into_response();
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    resp
}
