//! Receipts route handlers
//!
//! Provides HTTP handlers for:
//! - GET /receipts/latest?limit=N - Query latest transaction receipts

use axum::{
    extract::{Query, State},
    response::IntoResponse,
};

use crate::{receipts, DB_CTX, PROM_METRICS};

/// Handler for GET /receipts/latest?limit=N
///
/// Returns the latest N transaction receipts from the 'receipts' tree.
/// Receipts are sorted by monotonic ID (timestamp-based).
/// Default limit is 20, max is 100.
pub async fn receipts_latest_handler(Query(q): Query<receipts::LatestQuery>) -> impl IntoResponse {
    let state = receipts::AppState {
        dbctx: DB_CTX.clone(),
        metrics: PROM_METRICS.clone(),
    };
    receipts::get_latest(State(state), Query(q)).await
}
