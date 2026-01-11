use crate::land_deeds;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct DeedMarketState {
    pub db: sled::Db,
}

#[derive(Deserialize)]
pub struct ListDeedRequest {
    pub deed_id: u64,
    pub seller_addr: String,
    pub price_land: u128, // Price in atomic units (1e8 per LAND)
}

#[derive(Serialize)]
pub struct ListDeedResponse {
    pub ok: bool,
    pub deed_id: u64,
    pub seller: String,
    pub price_land: u128,
    pub message: String,
}

#[derive(Deserialize)]
pub struct PurchaseDeedRequest {
    pub deed_id: u64,
    pub buyer_addr: String,
    pub price_paid: u128, // Must match or exceed listing price
}

#[derive(Serialize)]
pub struct PurchaseDeedResponse {
    pub ok: bool,
    pub deed_id: u64,
    pub new_owner: String,
    pub price_paid: u128,
    pub auto_stocked: bool, // Whether auto-stock triggered
    pub message: String,
}

/// Get the owner of a deed from the database
fn get_deed_owner(db: &sled::Db, deed_id: u64) -> Result<String, String> {
    let key = format!("land:deed:{}", deed_id);
    match db.get(key.as_bytes()) {
        Ok(Some(val)) => {
            let owner = String::from_utf8(val.to_vec())
                .map_err(|e| format!("invalid owner bytes: {}", e))?;
            Ok(owner)
        }
        Ok(None) => Err(format!("deed {} not found", deed_id)),
        Err(e) => Err(format!("db error: {}", e)),
    }
}

/// Set the owner of a deed in the database
fn set_deed_owner(db: &sled::Db, deed_id: u64, owner: &str) -> Result<(), String> {
    let key = format!("land:deed:{}", deed_id);
    db.insert(key.as_bytes(), owner.as_bytes())
        .map_err(|e| format!("failed to update deed owner: {}", e))?;
    Ok(())
}

/// List a deed for sale
/// Validates that the seller actually owns the deed
async fn list_deed_handler(
    State(state): State<DeedMarketState>,
    Json(req): Json<ListDeedRequest>,
) -> Json<ListDeedResponse> {
    let deed_id = req.deed_id;
    let seller = req.seller_addr.clone();
    let price = req.price_land;

    // Get current owner
    match get_deed_owner(&state.db, deed_id) {
        Ok(current_owner) => {
            // Verify seller is the owner
            if current_owner.to_lowercase() != seller.to_lowercase() {
                let msg = format!(
                    "Seller {} does not own deed {}. Current owner: {}",
                    &seller, deed_id, current_owner
                );
                return Json(ListDeedResponse {
                    ok: false,
                    deed_id,
                    seller,
                    price_land: price,
                    message: msg,
                });
            }

            tracing::info!(
                deed_id = deed_id,
                seller = %seller,
                price_land = price,
                "Deed listed for sale"
            );

            let msg = format!("Deed {} listed at {} LAND", deed_id, price / 100_000_000);
            Json(ListDeedResponse {
                ok: true,
                deed_id,
                seller,
                price_land: price,
                message: msg,
            })
        }
        Err(e) => Json(ListDeedResponse {
            ok: false,
            deed_id,
            seller,
            price_land: price,
            message: e,
        }),
    }
}

/// Purchase a deed (atomic transfer + auto-stock next tier if exhausted)
/// Enforces one-deed-per-wallet rule
async fn purchase_deed_handler(
    State(state): State<DeedMarketState>,
    Json(req): Json<PurchaseDeedRequest>,
) -> Json<PurchaseDeedResponse> {
    let deed_id = req.deed_id;
    let buyer = req.buyer_addr.clone();
    let price = req.price_paid;
    let mut auto_stocked = false;

    // Check if buyer already owns a deed (one-deed-per-wallet rule)
    if land_deeds::wallet_has_deed(&state.db, &buyer) {
        let existing_deed = land_deeds::get_owned_deed_id(&state.db, &buyer)
            .map(|id| format!("{:?}", id))
            .unwrap_or_else(|_| "unknown".to_string());
        let msg = format!(
            "Buyer {} already owns deed {}. One deed per wallet enforced.",
            &buyer, existing_deed
        );
        return Json(PurchaseDeedResponse {
            ok: false,
            deed_id,
            new_owner: buyer,
            price_paid: price,
            auto_stocked: false,
            message: msg,
        });
    }

    // Get current owner
    match get_deed_owner(&state.db, deed_id) {
        Ok(current_owner) => {
            // Atomic transaction: update deed owner + indices
            let transfer_result: Result<(), sled::transaction::TransactionError> =
                state.db.transaction(|tx_db| {
                    // 1. Update deed owner record
                    let deed_key = format!("land:deed:{}", deed_id);
                    tx_db.insert(deed_key.as_bytes(), buyer.as_bytes())?;

                    // 2. Remove old owner's reverse index
                    let old_owner_key = format!("land:deed:by-owner:{}", current_owner);
                    tx_db.remove(old_owner_key.as_bytes())?;

                    // 3. Insert new owner's reverse index
                    let new_owner_key = format!("land:deed:by-owner:{}", buyer);
                    let deed_id_bytes = deed_id.to_be_bytes();
                    tx_db.insert(new_owner_key.as_bytes(), &deed_id_bytes[..])?;

                    Ok(())
                });

            match transfer_result {
                Ok(_) => {
                    tracing::info!(
                        deed_id = deed_id,
                        from = %current_owner,
                        to = %buyer,
                        price_land = price,
                        "Deed purchased and transferred atomically"
                    );

                    // Check if current tier is exhausted and auto-stock next tier
                    if let Err(e) = land_deeds::check_and_auto_stock_next_tier(&state.db) {
                        tracing::warn!("Failed to check auto-stock: {}", e);
                    } else {
                        auto_stocked = true;
                    }

                    let msg = format!("Deed {} purchased and transferred to {}", deed_id, &buyer);
                    Json(PurchaseDeedResponse {
                        ok: true,
                        deed_id,
                        new_owner: buyer,
                        price_paid: price,
                        auto_stocked,
                        message: msg,
                    })
                }
                Err(e) => {
                    let msg = format!("Atomic transfer failed: {}", e);
                    tracing::error!("Deed {} transfer failed: {}", deed_id, e);
                    Json(PurchaseDeedResponse {
                        ok: false,
                        deed_id,
                        new_owner: buyer,
                        price_paid: price,
                        auto_stocked: false,
                        message: msg,
                    })
                }
            }
        }
        Err(e) => Json(PurchaseDeedResponse {
            ok: false,
            deed_id,
            new_owner: buyer,
            price_paid: price,
            auto_stocked: false,
            message: e,
        }),
    }
}

pub fn router(db: sled::Db) -> Router {
    let state = DeedMarketState { db };

    Router::new()
        .route("/deeds/list", post(list_deed_handler))
        .route("/deeds/purchase", post(purchase_deed_handler))
        .with_state(state)
}
