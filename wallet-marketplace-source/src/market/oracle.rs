use axum::{
    routing::{get, post},
    Json, Router,
};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

lazy_static! {
    static ref ORACLE: Mutex<Rates> = Mutex::new(Rates::default());
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Rates {
    pub btc_usd: f64,
    pub bch_usd: f64,
    pub doge_usd: f64,
    pub usd_cash_rate: f64,
}

pub fn router() -> Router {
    Router::new()
        .route("/oracle/rates", get(get_rates))
        .route("/admin/oracle/override", post(override_rate))
}

async fn get_rates() -> Json<Rates> {
    Json(ORACLE.lock().unwrap().clone())
}

async fn override_rate(Json(new): Json<Rates>) -> Json<Rates> {
    *ORACLE.lock().unwrap() = new.clone();
    Json(new)
}

/// Convert usd cents -> CASH units using the current oracle rate.
pub fn usd_to_cash(usd_cents: u64) -> u64 {
    let rates = ORACLE.lock().unwrap().clone();
    let usd = (usd_cents as f64) / 100.0;
    let cash = usd * rates.usd_cash_rate;
    // Round to nearest integer CASH unit
    cash.round() as u64
}
