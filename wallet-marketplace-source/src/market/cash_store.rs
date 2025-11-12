use anyhow::Result;
use chrono::Utc;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sled;
use std::sync::Mutex;

lazy_static! {
    static ref LAST_MIGRATION: Mutex<Option<u64>> = Mutex::new(None);
}

#[cfg(not(test))]
lazy_static! {
    // Initialize sled DB once using VISION_DB_PATH or default
    static ref SLED_DB: sled::Db = {
        let db_path = std::env::var("VISION_DB_PATH").unwrap_or_else(|_| "wallet_data/market".to_string());
        std::fs::create_dir_all(&db_path).ok();
        sled::open(&db_path).expect("open sled")
    };
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CashOrder {
    pub id: String,
    pub buyer_addr: String,
    pub usd_amount_cents: u64,
    pub cash_amount: u64,
    pub stripe_session_id: Option<String>,
    pub stripe_payment_intent: Option<String>,
    pub status: String, // "created"|"paid"|"minted"|"failed"
    pub created_at: i64,
    pub updated_at: i64,
}

/// Return an owned sled::Db. In non-test builds this clones the global singleton.
/// In tests we open the DB per-call to avoid cross-test locking issues.
pub fn db_owned() -> sled::Db {
    #[cfg(not(test))]
    {
        SLED_DB.clone()
    }
    #[cfg(test)]
    {
        let db_path =
            std::env::var("VISION_DB_PATH").unwrap_or_else(|_| "wallet_data/market".to_string());
        std::fs::create_dir_all(&db_path).ok();
        sled::open(&db_path).expect("open sled")
    }
}

fn tree() -> sled::Tree {
    db_owned()
        .open_tree("market_cash_orders")
        .expect("open tree")
}

pub fn put(order: &CashOrder) -> Result<()> {
    let t = tree();
    t.insert(order.id.as_bytes(), serde_json::to_vec(order)?)?;
    t.flush()?;
    Ok(())
}

pub fn get(id: &str) -> Result<Option<CashOrder>> {
    let t = tree();
    Ok(t.get(id.as_bytes())?
        .map(|ivec| serde_json::from_slice(&ivec).unwrap()))
}

pub fn by_session(session_id: &str) -> Result<Option<CashOrder>> {
    let t = tree();
    for item in t.iter() {
        let (_, val) = item?;
        let o: CashOrder = serde_json::from_slice(&val)?;
        if o.stripe_session_id.as_deref() == Some(session_id) {
            return Ok(Some(o));
        }
    }
    Ok(None)
}

#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub fn by_payment_intent(pi_id: &str) -> Result<Option<CashOrder>> {
    let t = tree();
    for item in t.iter() {
        let (_, val) = item?;
        let o: CashOrder = serde_json::from_slice(&val)?;
        if o.stripe_payment_intent.as_deref() == Some(pi_id) {
            return Ok(Some(o));
        }
    }
    Ok(None)
}

pub fn list_all() -> Result<Vec<CashOrder>> {
    let t = tree();
    let mut out = Vec::new();
    for item in t.iter() {
        let (_, val) = item?;
        let o: CashOrder = serde_json::from_slice(&val)?;
        out.push(o);
    }
    Ok(out)
}

pub fn new_pending(
    id: String,
    buyer_addr: String,
    usd_amount_cents: u64,
    cash_amount: u64,
    stripe_session_id: Option<String>,
    stripe_payment_intent: Option<String>,
) -> CashOrder {
    let now = Utc::now().timestamp();
    CashOrder {
        id,
        buyer_addr,
        usd_amount_cents,
        cash_amount,
        stripe_session_id,
        stripe_payment_intent,
        status: "created".into(),
        created_at: now,
        updated_at: now,
    }
}

pub fn set_status(mut order: CashOrder, status: &str) -> Result<CashOrder> {
    order.status = status.to_string();
    order.updated_at = Utc::now().timestamp();
    put(&order)?;
    Ok(order)
}

/// Migrate legacy keys that used the prefix "cash_order:<id>" in the DB root into the new tree.
/// Returns number migrated.
pub fn migrate_legacy_prefix() -> Result<u64> {
    let db = db_owned();
    let t_new = tree();
    let mut migrated: u64 = 0;
    let prefix = b"cash_order:";
    for item in db.scan_prefix(prefix) {
        let (k, v) = item?;
        let key_str = String::from_utf8_lossy(&k).to_string();
        if let Some(id) = key_str.strip_prefix("cash_order:") {
            // move value into new tree under key id
            t_new.insert(id.as_bytes(), &v)?;
            migrated += 1;
        }
    }
    if migrated > 0 {
        t_new.flush()?;
    }
    // store last migration
    *LAST_MIGRATION.lock().unwrap() = Some(migrated);
    Ok(migrated)
}

#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub fn last_migration_count() -> Option<u64> {
    *LAST_MIGRATION.lock().unwrap()
}

/// Delete legacy keys with prefix `cash_order:` from the DB root.
/// Returns number removed.
pub fn cleanup_legacy_prefix() -> anyhow::Result<u64> {
    let db = db_owned();
    let mut removed = 0u64;
    for kv in db.scan_prefix("cash_order:") {
        let (k, _) = kv?;
        db.remove(k)?;
        removed += 1;
    }
    db.flush()?;
    Ok(removed)
}

/// Test helper: insert a raw key/value into the root of the DB (legacy layout)
#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub fn insert_legacy_raw(key: &str, bytes: &[u8]) -> Result<()> {
    let db = db_owned();
    db.insert(key.as_bytes(), bytes)?;
    db.flush()?;
    Ok(())
}

/// Test / migration helper: read a raw key from the DB root (legacy layout)
#[cfg(any(test, feature = "dev"))]
#[allow(dead_code)]
pub fn get_legacy_raw(key: &str) -> Result<Option<Vec<u8>>> {
    let db = db_owned();
    Ok(db.get(key.as_bytes())?.map(|ivec| ivec.to_vec()))
}
