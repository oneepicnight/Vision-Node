use std::net::TcpListener;
use std::time::Duration;
use tempfile::TempDir;
// Spawn handled by harness.rs
#[path = "harness.rs"]
mod harness;

use serde_json::json;

/// Basic test verifying per-IP gossip token bucket rate limiting.
#[tokio::test]
async fn gossip_ip_rate_limit_triggers() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let tmp = TempDir::new().expect("tmpdir");
    let data_dir = tmp.path().to_string_lossy().to_string();

    // Tune the gossip rate low for the test
    let rate = 2u64; // allow 2 rps
    std::env::set_var("VISION_PORT", port.to_string());
    std::env::set_var("VISION_DATA_DIR", &data_dir);
    std::env::set_var("VISION_DEV", "1");
    std::env::set_var("VISION_DEV_TOKEN", "devtest-token");
    std::env::set_var("VISION_RATE_GOSSIP_RPS", rate.to_string());

    let bin_path = if cfg!(windows) {
        "target/debug/vision-node.exe"
    } else {
        "target/debug/vision-node"
    };
    if !std::path::Path::new(bin_path).exists() {
        eprintln!("Skipping integration test: {} not found", bin_path);
        return;
    }

    let (base, mut child) =
        harness::spawn_node_with_opts(port, &data_dir, None, Some(rate), true, true).await;

    let client = reqwest::Client::new();
    // base is returned from spawn_node_with_opts
    // Wait until server is reachable
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

    // Build a minimal Tx envelope that can be parsed by gossip_tx handler
    let tx = json!({
        "nonce": 0u64,
        "sender_pubkey": "dev",
        "access_list": [],
        "module": "noop",
        "method": "ping",
        "args": [],
        "tip": 0u64,
        "fee_limit": 0u64,
        "sig": "",
    });
    let env = json!({ "tx": tx });

    // Send requests upto rate + 1 and expect the final one to be rate-limited
    for i in 0..(rate + 2) {
        let r = client
            .post(format!("{}/gossip/tx", &base))
            .json(&env)
            .send()
            .await
            .expect("gossip request");
        if i < rate {
            // first `rate` requests should be accepted or at least not rate-limited
            assert!(
                r.status() != reqwest::StatusCode::TOO_MANY_REQUESTS,
                "early rate-limited"
            );
        } else {
            // Expect this to be rate-limited eventually
            assert_eq!(r.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);
            let j: serde_json::Value = r.json().await.expect("json");
            assert_eq!(j["code"], "rate_limited");
            assert_eq!(j["reason"], "ip_rate_limit");
            break;
        }
    }

    // Shutdown child process
    let _ = child.kill().await;
    let _ = child.wait().await;
}
