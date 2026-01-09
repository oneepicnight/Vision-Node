use std::net::TcpListener;
use std::time::Duration;
use tempfile::TempDir;
#[path = "harness.rs"]
mod harness;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use serde_json::json;

#[tokio::test]
async fn run_node_and_transfer_over_http() {
    // Reserve a free port to avoid collisions
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    // Create a temporary data dir to isolate the node database
    let tmp = TempDir::new().expect("tmpdir");
    let data_dir = tmp.path().to_string_lossy().to_string();

    // We'll spawn a node subprocess in isolation using the test harness

    // Ensure binary exists
    let bin_path = if cfg!(windows) {
        "target/debug/vision-node.exe"
    } else {
        "target/debug/vision-node"
    };
    if !std::path::Path::new(bin_path).exists() {
        eprintln!("Skipping integration test: target/debug/vision-node not found");
        return;
    }

    // Spawn the node binary as a subprocess so it creates its own CHAIN state.
    // This provides an integration-level test that exercises the binary's
    // full startup and HTTP stack.
    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, None, None, true, true).await;

    // Wait until server is reachable
    let client = reqwest::Client::new();
    // base is returned by spawn_node_with_opts
    let mut ready = false;
    for _ in 0..30 {
        if let Ok(r) = client.get(format!("{}/health", &base)).send().await {
            if r.status().is_success() {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ready, "server did not start in time");

    // Create a new keypair and mint funds to it via dev faucet
    let mut rng = OsRng;
    let kp = SigningKey::generate(&mut rng);
    let sender = hex::encode(kp.verifying_key().to_bytes());
    let recipient = hex::encode([0x02u8; 32]);

    // Call dev faucet mint
    let mint_body = json!({ "to": sender, "amount": 100000u128 });
    let mint_url = format!("{}/dev/faucet_mint?dev_token=devtest-token", base);
    let r = client
        .post(&mint_url)
        .json(&mint_body)
        .send()
        .await
        .expect("mint request");
    assert!(r.status().is_success(), "mint failed: {}", r.status());

    // Read balance to confirm
    let bal_resp = client
        .get(format!("{}/wallet/{}/balance", base, sender))
        .send()
        .await
        .expect("balance get");
    assert!(bal_resp.status().is_success());
    let bal_json: serde_json::Value = bal_resp.json().await.expect("parse json");
    // balances are strings in API
    assert_eq!(bal_json["balance"].as_str().unwrap(), "100000");

    // Prepare transfer
    let mut sign_msg = Vec::new();
    sign_msg.extend_from_slice(&hex::decode(&sender).unwrap());
    sign_msg.extend_from_slice(&hex::decode(&recipient).unwrap());
    sign_msg.extend_from_slice(&50000u128.to_le_bytes());
    sign_msg.extend_from_slice(&100u128.to_le_bytes());
    sign_msg.extend_from_slice(&1u64.to_le_bytes());
    let sig = kp.sign(&sign_msg);

    let transfer_body = serde_json::json!({
        "from": sender,
        "to": recipient,
        "amount": 50000u128.to_string(),
        "fee": 100u128.to_string(),
        "nonce": 1u64,
        "public_key": hex::encode(kp.verifying_key().to_bytes()),
        "signature": hex::encode(sig.to_bytes()),
    });

    let r = client
        .post(format!("{}/wallet/transfer", base))
        .json(&transfer_body)
        .send()
        .await
        .expect("transfer request");
    assert!(r.status().is_success(), "transfer failed: {}", r.status());

    // Confirm balances changed
    let sender_bal_resp = client
        .get(format!("{}/wallet/{}/balance", base, sender))
        .send()
        .await
        .expect("balance get");
    assert!(sender_bal_resp.status().is_success());
    let sender_bal_json: serde_json::Value = sender_bal_resp.json().await.expect("parse json");
    // expected sender: 100000 - 50000 - 100 = 49800
    assert_eq!(sender_bal_json["balance"].as_str().unwrap(), "49800");

    let recip_bal_resp = client
        .get(format!("{}/wallet/{}/balance", base, recipient))
        .send()
        .await
        .expect("balance get");
    assert!(recip_bal_resp.status().is_success());
    let recip_bal_json: serde_json::Value = recip_bal_resp.json().await.expect("parse json");
    assert_eq!(recip_bal_json["balance"].as_str().unwrap(), "50000");

    // Confirm receipts contains our transfer
    let receipts_resp = client
        .get(format!("{}/receipts/latest?limit=10", base))
        .send()
        .await
        .expect("receipts get");
    assert!(receipts_resp.status().is_success());
    let receipts_json: serde_json::Value = receipts_resp.json().await.expect("parse json");
    assert!(receipts_json.is_array());
    let mut found = false;
    for rec in receipts_json.as_array().unwrap() {
        if rec["from"].as_str() == Some(sender.as_str()) && rec["to"].as_str() == Some(recipient.as_str()) {
            found = true;
            break;
        }
    }
    assert!(found, "transfer receipt not found in receipts");

    // Shutdown child process
    let _ = child.kill().await;
    let _ = child.wait().await;
}
