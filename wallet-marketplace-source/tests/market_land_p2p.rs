// reqwest is used fully-qualified below
use serde_json::json;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn listing_endpoints_smoke() {
    // Start a temporary server instance for the test
    let td = tempdir().expect("tempdir");
    let db_path = td.path().to_str().unwrap().to_string();

    let exe = Path::new("target/debug/vision-market.exe");
    let mut cmd = if exe.exists() {
        Command::new(exe)
    } else {
        let mut c = Command::new("cargo");
        c.args(["run", "--bin", "vision-market"]);
        c
    };
    cmd.env("VISION_DB_PATH", &db_path);
    let mut child = cmd.spawn().expect("failed to spawn vision-market");

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(800)).await;

    let client = reqwest::Client::new();

    let create = client
        .post("http://127.0.0.1:8080/market/land/list")
        .json(&json!({
            "seller_addr": "0xSeller",
            "qty_land": 100u64,
            "price_amount": 100000u64,
            "price_chain": "BTC"
        }))
        .send()
        .await
        .expect("request failed");

    assert!(create.status().is_success());

    let list = client
        .get("http://127.0.0.1:8080/market/land/listings")
        .send()
        .await
        .expect("request failed");

    assert!(list.status().is_success());

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
}
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[test]
    fn listing_lifecycle_smoke() {
        // This is a smoke test scaffold. Run with a proper Cargo project.
        let tmp = tempfile::tempdir().unwrap();
        let db = sled::open(tmp.path()).unwrap();
        let db = Arc::new(db);

        // Create a listing
        // Use HTTP client or directly call functions in a fully wired project
        assert!(db.open_tree("market_land_listings").is_ok());
    }
}
