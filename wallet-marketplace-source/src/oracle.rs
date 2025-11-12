use axum::{extract::Json, http::StatusCode, response::IntoResponse, routing::get, routing::post, Router};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use chrono::Utc;

#[derive(Serialize, Deserialize, Clone)]
pub struct Rates {
    pub btc_usd: f64,
    pub bch_usd: f64,
    pub doge_usd: f64,
    pub usd_cash_rate: f64,
    pub ts: i64,
}

pub fn router(_db: Arc<Db>) -> Router {
    Router::new()
        .route("/oracle/rates", get(get_rates))
        .route("/admin/oracle/override", post(override_rate))
}

pub async fn get_rates() -> impl IntoResponse {
    let r = Rates {
        btc_usd: 60000.0,
        bch_usd: 450.0,
        doge_usd: 0.16,
        usd_cash_rate: 100.0,
        ts: Utc::now().timestamp(),
    };
    (StatusCode::OK, Json(r))
}

#[derive(Deserialize)]
pub struct OverrideReq {
    pub field: String,
    pub value: f64,
    pub ttl_secs: Option<u64>,
}

pub async fn override_rate(Json(_payload): Json<OverrideReq>) -> impl IntoResponse {
    // For now just acknowledge
    (StatusCode::OK, Json(serde_json::json!({"ok": true})))
}
