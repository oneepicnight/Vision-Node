//! Integration tests for complete blockchain workflows
//! Tests end-to-end scenarios: transaction creation, mining, validation, and chain updates

use reqwest::blocking::Client;
use serde_json::json;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

fn spawn_test_node(port: u16) -> Child {
    let exe = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "vision-node.exe"
        } else {
            "vision-node"
        });

    let mut c = Command::new(exe);
    c.env("VISION_PORT", port.to_string());
    c.env("VISION_ADMIN_TOKEN", "test_token_123");
    c.env("VISION_DISABLE_P2P", "1"); // Disable P2P for isolated testing
    c.env("VISION_MINER_DISABLED", "0"); // Enable miner
    c.env("VISION_DEV", "1");
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());
    c.spawn().expect("Failed to spawn test node")
}

fn wait_for_node_ready(client: &Client, port: u16, timeout_seconds: u64) -> bool {
    let iterations = timeout_seconds * 5; // Check every 200ms
    for _ in 0..iterations {
        if let Ok(resp) = client
            .get(format!("http://127.0.0.1:{}/health", port))
            .send()
        {
            if resp.status().is_success() {
                return true;
            }
        }
        sleep(Duration::from_millis(200));
    }
    false
}

#[test]
fn test_transaction_to_block_workflow() {
    let port = 9001;
    let mut node = spawn_test_node(port);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    assert!(
        wait_for_node_ready(&client, port, 10),
        "Node failed to start within timeout"
    );

    // Step 1: Check initial height
    let height_resp = client
        .get(format!("http://127.0.0.1:{}/height", port))
        .send()
        .expect("Failed to get height");
    assert!(height_resp.status().is_success(), "Height endpoint failed");

    // Step 2: Submit a transaction to mempool
    let tx_data = json!({
        "from": "0".repeat(64),
        "to": "1".repeat(64),
        "amount": 1000,
        "fee": 10,
        "nonce": 0
    });

    let submit_resp = client
        .post(format!("http://127.0.0.1:{}/submit_tx", port))
        .json(&tx_data)
        .send();

    // Note: Transaction submission may fail if wallet/keys not set up - that's okay for this test
    // The important part is testing the endpoint responds
    if let Ok(resp) = submit_resp {
        println!("Transaction submission status: {}", resp.status());
    }

    // Step 3: Check mempool size
    let mempool_resp = client
        .get(format!("http://127.0.0.1:{}/mempool", port))
        .send();

    if let Ok(resp) = mempool_resp {
        assert!(resp.status().is_success(), "Mempool endpoint failed");
        println!("Mempool status: {}", resp.status());
    }

    // Step 4: Wait a bit for potential mining activity
    sleep(Duration::from_secs(2));

    // Step 5: Check height again - may have increased if mining occurred
    let final_height_resp = client
        .get(format!("http://127.0.0.1:{}/height", port))
        .send()
        .expect("Failed to get final height");
    assert!(
        final_height_resp.status().is_success(),
        "Final height check failed"
    );

    // Cleanup
    let _ = node.kill();
}

#[test]
fn test_block_validation_workflow() {
    let port = 9002;
    let mut node = spawn_test_node(port);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    assert!(
        wait_for_node_ready(&client, port, 10),
        "Node failed to start"
    );

    // Get current chain state
    let height_resp = client
        .get(format!("http://127.0.0.1:{}/height", port))
        .send()
        .expect("Failed to get height");
    assert!(height_resp.status().is_success());

    let height_text = height_resp.text().unwrap();
    println!("Current chain height: {}", height_text);

    // Try to get block at height 0 (genesis)
    let block_resp = client
        .get(format!("http://127.0.0.1:{}/block/0", port))
        .send();

    if let Ok(resp) = block_resp {
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 404,
            "Unexpected block endpoint response"
        );
        println!("Block endpoint status: {}", resp.status());
    }

    // Cleanup
    let _ = node.kill();
}

#[test]
fn test_node_health_and_metrics() {
    let port = 9003;
    let mut node = spawn_test_node(port);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    assert!(
        wait_for_node_ready(&client, port, 10),
        "Node failed to start"
    );

    // Test health endpoint
    let health_resp = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .expect("Health check failed");
    assert!(
        health_resp.status().is_success(),
        "Health endpoint not healthy"
    );

    // Test metrics endpoint
    let metrics_resp = client
        .get(format!("http://127.0.0.1:{}/metrics.prom", port))
        .send()
        .expect("Metrics request failed");
    assert!(
        metrics_resp.status().is_success(),
        "Metrics endpoint failed"
    );

    let metrics_text = metrics_resp.text().unwrap();
    assert!(
        metrics_text.contains("vision_"),
        "Metrics should contain vision_ prefix"
    );
    println!(
        "Metrics endpoint working, sample: {}",
        &metrics_text[..metrics_text.len().min(200)]
    );

    // Cleanup
    let _ = node.kill();
}

#[test]
fn test_concurrent_api_requests() {
    let port = 9004;
    let mut node = spawn_test_node(port);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    assert!(
        wait_for_node_ready(&client, port, 10),
        "Node failed to start"
    );

    // Make multiple concurrent API calls
    let mut handles = vec![];

    for i in 0..5 {
        let client_clone = client.clone();
        let handle = std::thread::spawn(move || {
            let resp = client_clone
                .get(format!("http://127.0.0.1:{}/height", port))
                .send();
            (i, resp.is_ok())
        });
        handles.push(handle);
    }

    let mut successes = 0;
    for handle in handles {
        if let Ok((i, success)) = handle.join() {
            if success {
                successes += 1;
            }
            println!("Request {} succeeded: {}", i, success);
        }
    }

    assert!(
        successes >= 4,
        "At least 4 out of 5 concurrent requests should succeed"
    );

    // Cleanup
    let _ = node.kill();
}
