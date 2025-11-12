use axum::http::StatusCode;
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LandListing {
    pub listing_id: String,
    pub seller_addr: String,
    pub qty_land: u128,
    pub price_amount: u128,
    pub price_chain: String,
    pub pay_to: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub buyer_expected_txid: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateListingReq {
    pub seller_addr: String,
    pub qty_land: u128,
    pub price_amount: u128,
    pub price_chain: String,
}

fn tree_name_listings() -> &'static str {
    "market_land_listings"
}
fn tree_name_settlements() -> &'static str {
    "market_land_settlements"
}

pub fn router(_db: Arc<Db>) -> axum::Router {
    Router::new()
        .route("/market/land/list", post(create_listing))
        .route("/market/land/listings", get(list_open_listings))
        .route("/market/land/listings/{id}", get(get_listing_by_id))
        .route("/market/land/signal_payment", post(signal_payment))
        .route("/_market/land/confirm", post(confirm_and_transfer))
}

async fn get_listing_by_id(
    Extension(_db): Extension<Arc<Db>>,
    Path(id): Path<String>,
) -> (StatusCode, Json<serde_json::Value>) {
    let tree = _db.open_tree(tree_name_listings()).unwrap();
    if let Ok(Some(v)) = tree.get(id.as_bytes()) {
        if let Ok(listing) = serde_json::from_slice::<LandListing>(&v) {
            return (StatusCode::OK, Json(serde_json::json!(listing)));
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error":"not found"})),
    )
}

async fn create_listing(
    Extension(db): Extension<Arc<Db>>,
    Json(payload): Json<CreateListingReq>,
) -> (StatusCode, Json<LandListing>) {
    let listing_id = Uuid::new_v4().to_string();
    let pay_to =
        crate::market::crypto_watch::generate_invoice_address(&payload.price_chain, &listing_id);

    let listing = LandListing {
        listing_id: listing_id.clone(),
        seller_addr: payload.seller_addr.clone(),
        qty_land: payload.qty_land,
        price_amount: payload.price_amount,
        price_chain: payload.price_chain.clone(),
        pay_to: pay_to.clone(),
        status: "open".to_string(),
        created_at: Utc::now().timestamp_millis() as u64,
        expires_at: None,
        buyer_expected_txid: None,
    };

    let tree = db.open_tree(tree_name_listings()).unwrap();
    tree.insert(listing_id.as_bytes(), serde_json::to_vec(&listing).unwrap())
        .unwrap();
    tree.flush().unwrap();

    log::info!("created listing id={} pay_to={}", listing_id, pay_to);
    (StatusCode::CREATED, Json(listing))
}

async fn list_open_listings(
    Extension(db): Extension<Arc<Db>>,
) -> (StatusCode, Json<Vec<LandListing>>) {
    let tree = db.open_tree(tree_name_listings()).unwrap();
    let mut out: Vec<LandListing> = vec![];
    for (_, v) in tree.iter().flatten() {
        if let Ok(listing) = serde_json::from_slice::<LandListing>(&v) {
            if listing.status == "open" {
                out.push(listing);
            }
        }
    }
    log::info!("list_open_listings returning {} open listings", out.len());
    (StatusCode::OK, Json(out))
}

#[derive(Deserialize)]
pub struct SignalPaymentReq {
    pub listing_id: String,
    pub txid: Option<String>,
}

async fn signal_payment(
    Extension(db): Extension<Arc<Db>>,
    Json(payload): Json<SignalPaymentReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    let tree = db.open_tree(tree_name_listings()).unwrap();
    if let Ok(Some(v)) = tree.get(payload.listing_id.as_bytes()) {
        if let Ok(mut listing) = serde_json::from_slice::<LandListing>(&v) {
            listing.status = "in_mempool".to_string();
            listing.buyer_expected_txid = payload.txid.clone();
            tree.insert(
                payload.listing_id.as_bytes(),
                serde_json::to_vec(&listing).unwrap(),
            )
            .unwrap();
            tree.flush().unwrap();
            return (StatusCode::OK, Json(serde_json::json!(listing)));
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error":"listing not found"})),
    )
}

#[derive(Deserialize)]
pub struct ConfirmReq {
    pub listing_id: String,
    pub observed_txid: String,
    pub chain: String,
}

async fn confirm_and_transfer(
    Extension(db): Extension<Arc<Db>>,
    Json(payload): Json<ConfirmReq>,
) -> (StatusCode, Json<serde_json::Value>) {
    let tree = db.open_tree(tree_name_listings()).unwrap();
    if let Ok(Some(v)) = tree.get(payload.listing_id.as_bytes()) {
        if let Ok(mut listing) = serde_json::from_slice::<LandListing>(&v) {
            listing.status = "settled".to_string();
            tree.insert(
                payload.listing_id.as_bytes(),
                serde_json::to_vec(&listing).unwrap(),
            )
            .unwrap();
            tree.flush().unwrap();

            let settlement_tree = db.open_tree(tree_name_settlements()).unwrap();
            let rec = serde_json::json!({
                "observed_txid": payload.observed_txid,
                "chain": payload.chain,
                "seen_at": Utc::now().timestamp_millis(),
                "conf_req": 0,
                "conf_count": 0,
                "confirmed_at": Utc::now().timestamp_millis()
            });
            settlement_tree
                .insert(
                    payload.listing_id.as_bytes(),
                    serde_json::to_vec(&rec).unwrap(),
                )
                .unwrap();
            settlement_tree.flush().unwrap();

            return (
                StatusCode::OK,
                Json(serde_json::json!({"status":"settled"})),
            );
        }
    }
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error":"listing not found"})),
    )
}

// Ledger helper stubs (to be implemented against Vision node ledger)
#[allow(dead_code)]
pub fn move_land_tokens(_from: &str, _to: &str, qty: u64) -> anyhow::Result<()> {
    log::info!("Transferring {} LAND from {} -> {}", qty, _from, _to);
    Ok(())
}

#[allow(dead_code)]
pub fn reserve_land_balance(_seller: &str, _qty: u64) -> anyhow::Result<()> {
    log::info!("Reserving {} LAND for seller {}", _qty, _seller);
    Ok(())
}

#[allow(dead_code)]
pub fn unreserve_land_balance(_seller: &str, _qty: u64) -> anyhow::Result<()> {
    log::info!("Unreserving {} LAND for seller {}", _qty, _seller);
    Ok(())
}
