use crate::market::cash_store::{self};
use crate::market::cursor::{decode_cursor, encode_cursor};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sled::Db;
use std::sync::Arc;

#[derive(Deserialize)]
struct ListQ {
    limit: Option<usize>,
    buyer_addr: Option<String>,
    after: Option<String>,
}

pub fn router() -> Router {
    Router::new()
        .route("/admin/cash/orders", get(list_orders))
        .route("/admin/cash/orders/{id}", get(get_one))
        .route("/admin/cash/orders/{id}/replay_mint", post(replay_mint))
}
async fn list_orders(
    headers: HeaderMap,
    Query(q): Query<ListQ>,
    Extension(_db): Extension<Arc<Db>>,
) -> axum::response::Response {
    // admin check
    if let Ok(expected) = std::env::var("ADMIN_TOKEN") {
        let got = headers
            .get("X-Admin-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if got != expected {
            return (axum::http::StatusCode::UNAUTHORIZED, "admin token required").into_response();
        }
    }
    let limit = q.limit.unwrap_or(50);
    let mut all = cash_store::list_all().unwrap_or_default();
    all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));

    let start_after = q.after.as_ref().and_then(|c| decode_cursor(c));
    let mut out: Vec<_> = Vec::new();
    for o in all.into_iter() {
        if let Some((after_at, after_id)) = &start_after {
            if !(o.updated_at > *after_at || (o.updated_at == *after_at && o.id > *after_id)) {
                continue;
            }
        }
        if let Some(b) = &q.buyer_addr {
            if &o.buyer_addr != b {
                continue;
            }
        }
        out.push(o);
        if out.len() >= limit {
            break;
        }
    }

    let next_cursor = out
        .last()
        .map(|last| encode_cursor(last.updated_at, &last.id));
    (
        axum::http::StatusCode::OK,
        Json(json!({"items": out, "next_cursor": next_cursor})),
    )
        .into_response()
}

async fn get_one(
    headers: HeaderMap,
    Path(id): Path<String>,
    Extension(_db): Extension<Arc<Db>>,
) -> axum::response::Response {
    if let Ok(expected) = std::env::var("ADMIN_TOKEN") {
        let got = headers
            .get("X-Admin-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if got != expected {
            return (axum::http::StatusCode::UNAUTHORIZED, "admin token required").into_response();
        }
    }
    match cash_store::get(&id) {
        Ok(Some(o)) => Json(o).into_response(),
        Ok(_) => (axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
        Err(_) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response(),
    }
}

async fn replay_mint(
    headers: HeaderMap,
    Path(id): Path<String>,
    Extension(_db): Extension<Arc<Db>>,
) -> axum::response::Response {
    if let Ok(expected) = std::env::var("ADMIN_TOKEN") {
        let got = headers
            .get("X-Admin-Token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if got != expected {
            return (axum::http::StatusCode::UNAUTHORIZED, "admin token required").into_response();
        }
    }
    let o = match cash_store::get(&id).ok().flatten() {
        Some(o) => o,
        _ => return (axum::http::StatusCode::NOT_FOUND, "not found").into_response(),
    };
    if o.status == "minted" {
        return Json(serde_json::json!({"ok": true, "status":"already_minted"})).into_response();
    }
    if let Err(e) = crate::ledger::client::mint_cash(&o.buyer_addr, o.cash_amount).await {
        eprintln!("replay_mint ledger error: {:?}", e);
        let _ = cash_store::set_status(o, "failed");
        return Json(serde_json::json!({"ok": false, "status":"failed"})).into_response();
    }
    let o2 = cash_store::set_status(o, "minted").expect("persist");
    Json(serde_json::json!({"ok": true, "status": o2.status})).into_response()
}
