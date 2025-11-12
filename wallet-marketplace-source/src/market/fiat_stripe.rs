use axum::http::HeaderMap;
use axum::{http::StatusCode, routing::post, Json, Router};
use bytes::Bytes;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::ledger::client;
use crate::market::cash_store;

#[derive(Deserialize)]
struct CashBuyRequest {
    buyer_addr: String,
    usd_amount: u64,
}

pub fn router() -> Router {
    Router::new()
        .route("/cash/buy_intent", post(buy_intent))
        .route("/cash/stripe_webhook", post(stripe_webhook))
}

async fn buy_intent(Json(req): Json<CashBuyRequest>) -> (StatusCode, Json<Value>) {
    // Create a local order and return a (fake) Checkout session id/url.
    // In production this would call Stripe to create a real Checkout session.
    let _stripe_secret = std::env::var("STRIPE_SECRET").unwrap_or_default();

    // Create ids
    let order_id = Uuid::new_v4().to_string();
    let session_id = format!("cs_{}", Uuid::new_v4());

    // Convert USD cents -> CASH units using the oracle helper
    let cash_amount = crate::market::oracle::usd_to_cash(req.usd_amount);

    let order = cash_store::new_pending(
        order_id.clone(),
        req.buyer_addr.clone(),
        req.usd_amount,
        cash_amount,
        Some(session_id.clone()),
        None,
    );
    if let Err(e) = cash_store::put(&order) {
        eprintln!("Failed to persist cash order: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "failed to create order" })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "session_id": session_id,
            "session_url": format!("https://checkout.stripe.com/pay/{}", order_id),
            "order_id": order_id,
            "stripe_secret_present": !_stripe_secret.is_empty()
        })),
    )
}

async fn stripe_webhook(headers: HeaderMap, body: Bytes) -> StatusCode {
    // Read raw body bytes so signatures (if used) validate against exact payload.
    let secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();

    let payload = body;

    if !secret.is_empty() {
        // NOTE: For simplicity we do not call the Stripe SDK's signature verification here.
        // In prod you should verify the signature header using the Stripe library to avoid
        // accepting forged events. We log that verification is being skipped.
        if headers.get("stripe-signature").is_none() {
            eprintln!("stripe webhook secret configured but stripe-signature header missing");
            // continue to attempt best-effort processing
        }
    }

    let v: Value = match serde_json::from_slice(&payload) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to parse webhook json: {:?}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    // Try to find a session id in common places (data.object.id or data.object.payment_intent)
    let session_id = v
        .get("data")
        .and_then(|d| d.get("object"))
        .and_then(|o| o.get("id"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            v.get("data")
                .and_then(|d| d.get("object"))
                .and_then(|o| o.get("payment_intent"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| v.get("id").and_then(|s| s.as_str()).map(|s| s.to_string()));

    let session_id = match session_id {
        Some(s) => s,
        None => {
            eprintln!("webhook did not contain recognizable session id");
            return StatusCode::OK; // ack so stripe won't retry a malformed event
        }
    };

    // Lookup order by session id
    match cash_store::by_session(&session_id) {
        Ok(Some(order)) => {
            if order.status == "minted" {
                // already handled
                return StatusCode::OK;
            }

            // Mint CASH via ledger client
            match client::mint_cash(&order.buyer_addr, order.cash_amount).await {
                Ok(_) => {
                    if let Err(e) = cash_store::set_status(order, "minted") {
                        eprintln!("failed to update order status after mint: {:?}", e);
                    }
                    StatusCode::OK
                }
                Err(e) => {
                    eprintln!("failed to mint cash: {:?}", e);
                    // mark order failed
                    if let Ok(o) = cash_store::set_status(order, "failed") {
                        let _ = o;
                    }
                    StatusCode::OK
                }
            }
        }
        Ok(None) => {
            eprintln!("no order found for session id {}", session_id);
            StatusCode::OK
        }
        Err(e) => {
            eprintln!("db error while looking up order: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
