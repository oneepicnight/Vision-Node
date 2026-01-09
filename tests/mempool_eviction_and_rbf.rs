// use Duration if needed in harness respawn backoffs
use std::net::TcpListener;
use tempfile::TempDir;
// Spawn handled by harness.rs
#[path = "harness.rs"]
mod harness;
use serde_json::json;
// use serial_test::serial;  // Not in Cargo.toml dependencies
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

/// Simple client-side tx struct to match the server's JSON format and
/// reproduce the canonical bytes used for signatures (sig must be empty
/// when building the signable bytes).
#[derive(serde::Serialize)]
struct TestTx<'a> {
    nonce: u64,
    sender_pubkey: &'a str,
    access_list: Vec<&'a str>,
    module: &'a str,
    method: &'a str,
    args: Vec<u8>,
    tip: u64,
    fee_limit: u64,
    sig: &'a str,
    max_priority_fee_per_gas: u64,
    max_fee_per_gas: u64,
}

fn signable_bytes(tx: &TestTx) -> Vec<u8> {
    // Serialize with empty sig to replicate server signable_tx_bytes behavior
    let tmp = TestTx {
        nonce: tx.nonce,
        sender_pubkey: tx.sender_pubkey,
        access_list: tx.access_list.clone(),
        module: tx.module,
        method: tx.method,
        args: tx.args.clone(),
        tip: tx.tip,
        fee_limit: tx.fee_limit,
        sig: "",
        max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
        max_fee_per_gas: tx.max_fee_per_gas,
    };
    serde_json::to_vec(&tmp).unwrap()
}

fn build_signed_tx(sk: &SigningKey, nonce: u64, tip: u64, fee_limit: u64) -> serde_json::Value {
    let sender = hex::encode(sk.verifying_key().to_bytes());
    let tx = TestTx {
        nonce,
        sender_pubkey: &sender,
        access_list: Vec::new(),
        module: "noop",
        method: "ping",
        args: Vec::new(),
        tip,
        fee_limit,
        sig: "",
        max_priority_fee_per_gas: 0,
        max_fee_per_gas: 0,
    };
    let to_sign = signable_bytes(&tx);
    let sig = sk.sign(&to_sign);
    let mut j = serde_json::to_value(&tx).unwrap();
    j["sig"] = serde_json::Value::String(hex::encode(sig.to_bytes()));
    j
}

// Spawn node and return base url + child
// Use the test harness spawn helper in `tests/harness.rs`

#[tokio::test]
// #[serial]  // serial_test not in Cargo.toml
async fn mempool_eviction_prefers_bulk_and_accepts_high_tip() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_string_lossy().to_string();

    let mempool_max = 10;
    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, Some(mempool_max), Some(1000), true, true)
            .await;
    let client = reqwest::Client::new();

    // Build mempool_max low-tip txs from distinct senders
    for i in 0..mempool_max {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let tx_json = build_signed_tx(&sk, 0, 1, 1000);
        let resp = client
            .post(format!("{}/submit_tx", base))
            .json(&json!({"tx": tx_json}))
            .send()
            .await
            .expect("submit");
        assert!(
            resp.status().is_success(),
            "Expected success for low-tip tx {}: {}",
            i,
            resp.status()
        );
    }

    // Confirm mempool count equals mempool_max
    let stats = client
        .get(format!("{}/mempool", base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        stats["stats"]["total_count"].as_u64().unwrap(),
        mempool_max as u64
    );

    // Submit a new high-tip tx that should evict a low-tip one
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let high_tx = build_signed_tx(&sk, 0, 5000, 1000);
    let resp = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": high_tx}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let mempool_after = client
        .get(format!("{}/mempool", base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        mempool_after["stats"]["total_count"].as_u64().unwrap(),
        mempool_max as u64
    );
    // Ensure at least one critical tx present (our high tip should be critical)
    let mut found_high = false;
    for txv in mempool_after["transactions"].as_array().unwrap() {
        if txv["tip"].as_u64().unwrap_or(0) >= 5000 {
            found_high = true;
            break;
        }
    }
    assert!(
        found_high,
        "High tip tx should be present in mempool (critical)"
    );

    // Shutdown
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[tokio::test]
// #[serial]  // serial_test not in Cargo.toml
async fn mempool_rejects_low_tip_when_full() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mempool_max = 5;
    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, Some(mempool_max), Some(1000), true, true)
            .await;
    let client = reqwest::Client::new();

    // Fill with bulk low-tip txs
    for _ in 0..mempool_max {
        let mut rng = OsRng;
        let sk = SigningKey::generate(&mut rng);
        let tx_json = build_signed_tx(&sk, 0, 1, 1000);
        let resp = client
            .post(format!("{}/submit_tx", base))
            .json(&json!({"tx": tx_json}))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
    }
    // Submitting another low-tip should be rejected with mempool_full
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let tx_json = build_signed_tx(&sk, 0, 1, 1000);
    let resp = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": tx_json}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"].as_str().unwrap_or(""), "mempool_full");

    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[tokio::test]
// #[serial]  // serial_test not in Cargo.toml
async fn rbf_replace_and_reject_behavior() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mempool_max = 20;
    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, Some(mempool_max), Some(1000), true, true)
            .await;
    let client = reqwest::Client::new();

    // Choose a sender and submit a low-tip tx
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let tx_low = build_signed_tx(&sk, 0, 1, 1000);
    let resp_low = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": tx_low}))
        .send()
        .await
        .unwrap();
    assert!(resp_low.status().is_success());

    // Submit higher tip for same sender/nonce: should replace (OK)
    let tx_high = build_signed_tx(&sk, 0, 100, 1000);
    let resp_high = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": tx_high}))
        .send()
        .await
        .unwrap();
    assert!(resp_high.status().is_success());
    // Verify mempool shows one tx for this sender with tip=100
    let mem = client
        .get(format!("{}/mempool?limit=100", base))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let mut seen = false;
    for t in mem["transactions"].as_array().unwrap() {
        if t["sender"].as_str() == Some(hex::encode(sk.verifying_key().to_bytes()).as_str()) {
            assert_eq!(t["tip"].as_u64().unwrap_or(0), 100);
            seen = true;
        }
    }
    assert!(seen);

    // Submit lower tip for same sender/nonce: should be rejected (409 rbf_tip_too_low)
    let tx_lower = build_signed_tx(&sk, 0, 10, 1000);
    let resp_lower = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": tx_lower}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_lower.status(), reqwest::StatusCode::CONFLICT);
    let j: serde_json::Value = resp_lower.json().await.unwrap();
    assert_eq!(j["error"]["code"].as_str().unwrap_or(""), "rbf_tip_too_low");

    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[tokio::test]
// #[serial]  // serial_test not in Cargo.toml
async fn submit_tx_rate_limit_headers_present() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_string_lossy().to_string();
    let mempool_max = 20;
    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, Some(mempool_max), Some(100), true, true)
            .await;
    let client = reqwest::Client::new();
    let mut rng = OsRng;
    let sk = SigningKey::generate(&mut rng);
    let tx_json = build_signed_tx(&sk, 0, 1, 1000);
    let resp = client
        .post(format!("{}/submit_tx", base))
        .json(&json!({"tx": tx_json}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    // Check headers
    assert!(resp.headers().get("x-ratelimit-limit").is_some());
    assert!(resp.headers().get("x-ratelimit-remaining").is_some());
    assert!(resp.headers().get("x-ratelimit-reset").is_some());

    let _ = child.kill().await;
    let _ = child.wait().await;
}
