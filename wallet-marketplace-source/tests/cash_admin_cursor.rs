use chrono::Utc;
use tempfile::TempDir;

use vision_market::market::{cash_store, cursor};

#[test]
fn pagination_logic_produces_expected_pages() {
    // temp sled + chdir so code writes to wallet_data/market
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    // Insert 75 fake orders descending timestamps
    for i in 0..75 {
        let id = format!("id{i:03}");
        let now = Utc::now().timestamp() - i as i64;
        let mut o = cash_store::new_pending(
            id.clone(),
            format!("V1User{i}"),
            1000 + i as u64,
            100 + i as u64,
            None,
            None,
        );
        o.created_at = now;
        o.updated_at = now;
        cash_store::put(&o).unwrap();
    }

    // Simulate handler's list behavior
    let mut all = cash_store::list_all().unwrap_or_default();
    all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));
    let limit = 50usize;
    let page1: Vec<_> = all.iter().take(limit).cloned().collect();
    assert_eq!(page1.len(), 50);
    let remaining: Vec<_> = all.iter().skip(limit).cloned().collect();
    assert_eq!(remaining.len(), 25);

    // Next cursor encoding/decoding round-trip sanity
    let last = page1.last().unwrap();
    let cursor_str = cursor::encode_cursor(last.updated_at, &last.id);
    let decoded = cursor::decode_cursor(&cursor_str).unwrap();
    assert_eq!(decoded.0, last.updated_at);
    assert_eq!(decoded.1, last.id);
}
