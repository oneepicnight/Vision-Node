use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct CashStatus {
    usd_rate: f64,
    note: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/cash/rate", get(get_rate))
}

async fn get_rate() -> Json<CashStatus> {
    Json(CashStatus {
        usd_rate: 100.0,
        note: "Static 1 USD = 100 CASH (placeholder)",
    })
}
