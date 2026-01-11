use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MarketState {
    pub db: sled::Db,
}

#[derive(Deserialize)]
pub struct TestSaleRequest {
    pub amount: u128,
}

#[derive(Serialize)]
pub struct TestSaleResponse {
    pub ok: bool,
    pub total: u128,
    pub vault_amount: u128,
    pub fund_amount: u128,
    pub founder1_amount: u128,
    pub founder2_amount: u128,
}

async fn test_sale_handler(
    State(_state): State<MarketState>,
    Json(req): Json<TestSaleRequest>,
) -> Json<TestSaleResponse> {
    // Hard-coded Vision chain splits (from token_accounts.toml)
    const VAULT_PCT: u128 = 50;
    const FUND_PCT: u128 = 30;
    const TREASURY_PCT: u128 = 20;
    const FOUNDER1_PCT: u128 = 50;
    const FOUNDER2_PCT: u128 = 50;

    // Calculate splits
    let total = req.amount;

    let vault_amt = total * VAULT_PCT / 100u128;
    let fund_amt = total * FUND_PCT / 100u128;
    let treasury_amt = total * TREASURY_PCT / 100u128;

    let f1_amt = treasury_amt * FOUNDER1_PCT / 100u128;
    let f2_amt = treasury_amt.saturating_sub(f1_amt);

    // Route the proceeds (will need to update settlement.rs to use hard-coded addresses)
    // For now, comment out until settlement.rs is updated
    // let _ = crate::market::settlement::route_proceeds(&state.tok_accounts, &state.db, total);

    Json(TestSaleResponse {
        ok: true,
        total,
        vault_amount: vault_amt,
        fund_amount: fund_amt,
        founder1_amount: f1_amt,
        founder2_amount: f2_amt,
    })
}

pub fn router(db: sled::Db) -> Router {
    let state = MarketState { db };

    Router::new()
        .route("/market/test-sale", post(test_sale_handler))
        .with_state(state)
}
