use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn migrates_legacy_and_allows_cleanup() {
    // isolated DB root
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("wallet_data").join("market");
    fs::create_dir_all(&root).unwrap();

    // chdir so store helpers use this relative path
    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    // write one legacy record (compat with your legacy prefix)
    let db = vision_market::market::cash_store::db_owned();
    let key = b"cash_order:abc123";
    let val = serde_json::json!({
        "id":"abc123","buyer_addr":"V1X","usd_amount_cents":2500u64,
        "cash_amount":250000u64,"status":"created","created_at":111,"updated_at":111
    });
    db.insert(key, serde_json::to_vec(&val).unwrap()).unwrap();
    db.flush().unwrap();

    // migrate (copy)
    let moved = vision_market::market::cash_store::migrate_legacy_prefix().unwrap();
    assert_eq!(moved, 1);

    // new tree contains the order under key "abc123"
    let tree = vision_market::market::cash_store::db_owned()
        .open_tree("market_cash_orders")
        .unwrap();
    let got = tree.get("abc123").unwrap().expect("migrated row");
    let parsed: serde_json::Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(parsed["id"], "abc123");
    assert_eq!(parsed["status"], "created");

    // destructive cleanup optional
    let removed = vision_market::market::cash_store::cleanup_legacy_prefix().unwrap();
    assert_eq!(removed, 1);
    let after = vision_market::market::cash_store::db_owned()
        .get(key)
        .unwrap();
    assert!(after.is_none());

    // restore cwd
    std::env::set_current_dir(orig).unwrap();
}
