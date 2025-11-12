use axum::{extract::{Json, RawBody, State, HeaderMap}, http::StatusCode, response::IntoResponse, routing::post, Router};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use stripe::{Client, CheckoutSession, CreateCheckoutSession, CheckoutSessionMode};

#[derive(Deserialize)]
pub struct BuyIntent {
    pub buyer_addr: String,
    pub usd_amount_cents: u64,
}

#[derive(Serialize)]
pub struct BuyIntentResp {
    pub checkout_url: String,
    pub session_id: String,
}

pub fn router(db: Arc<Db>) -> Router {
    Router::new()
        .route("/cash/buy_intent", post(buy_intent(db)))
        .route("/cash/stripe_webhook", post(stripe_webhook))
}

async fn create_checkout_session(amount_usd: u64, buyer_addr: &str) -> anyhow::Result<String> {
    let secret = std::env::var("STRIPE_SECRET").expect("STRIPE_SECRET not set");
    let client = Client::new(secret);

    // Note: using simplified params - adapt for real Stripe SDK usage
    let session_url = format!("https://stripe.mock/checkout/{}", uuid::Uuid::new_v4());
    Ok(session_url)
}

pub fn buy_intent(db: Arc<Db>) -> impl axum::handler::Handler<axum::body::Body> {
    move |Json(payload): Json<BuyIntent>| {
        let db = db.clone();
        async move {
            match create_checkout_session(payload.usd_amount_cents, &payload.buyer_addr).await {
                Ok(url) => {
                    let session_id = format!("sess_{}", uuid::Uuid::new_v4());
                    (StatusCode::CREATED, Json(BuyIntentResp { checkout_url: url, session_id }))
                }
                Err(e) => {
                    log::error!("Stripe session creation failed: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(BuyIntentResp { checkout_url: "".to_string(), session_id: "".to_string() }))
                }
            }
        }
    }
}

pub async fn stripe_webhook(headers: HeaderMap, body: RawBody) -> impl IntoResponse {
    // In production, verify signature with STRIPE_WEBHOOK_SECRET and parse event
    let _bytes = hyper::body::to_bytes(body).await.unwrap_or_default();
    log::info!("Received stripe webhook ({} bytes)", _bytes.len());
    (StatusCode::OK, "ok")
}
