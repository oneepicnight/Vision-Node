#[tokio::test]
async fn sync_pull_retry_prom() {
    use reqwest::Client;
    use std::net::TcpListener;
    use std::process::Command;
    use std::time::Duration;
    use std::{env, thread};

    // find free port
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    // binary path
    let bin = match env::var("CARGO_BIN_EXE_vision-node") {
        Ok(p) => p,
        Err(_) => {
            let mut path = env::current_exe().expect("cwd");
            for _ in 0..3 {
                path.pop();
            }
            path.push("debug");
            path.push("vision-node");
            path.to_string_lossy().to_string()
        }
    };

    let mut child = Command::new(bin)
        .env("VISION_PORT", port.to_string())
        .spawn()
        .expect("spawn server");

    // wait for server
    let client = Client::new();
    let base = format!("http://127.0.0.1:{}", port);
    let mut ok = false;
    for _ in 0..40 {
        if let Ok(r) = client.get(format!("{}/livez", base)).send().await {
            if r.status().is_success() {
                ok = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ok, "server did not start");

    // Trigger sync pull to unreachable peer
    let body = serde_json::json!({ "src": "http://127.0.0.1:9999" });
    let r = client
        .post(format!("{}/sync/pull", base))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::BAD_GATEWAY);

    // wait briefly to let metrics update
    thread::sleep(Duration::from_millis(200));

    let r = client
        .get(format!("{}/metrics.prom", base))
        .send()
        .await
        .unwrap();
    let body = r.text().await.unwrap();
    // check retries counter exists and is >= 1
    assert!(
        body.contains("vision_sync_pull_retries_total"),
        "missing retries metric"
    );
    if let Some(line) = body
        .lines()
        .find(|l| l.starts_with("vision_sync_pull_retries_total "))
    {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            let v: u64 = parts[1].parse().unwrap_or(0);
            assert!(v >= 1, "expected retries >= 1, got {}", v);
        }
    }
    // check failure metric with a reason label is present
    assert!(
        body.contains("vision_sync_pull_failures_total"),
        "missing failures metric"
    );

    // cleanup
    let _ = child.kill();
    let _ = child.wait();
}
