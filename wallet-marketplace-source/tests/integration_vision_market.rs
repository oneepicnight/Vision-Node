use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use tokio::time::sleep;

#[tokio::test]
async fn vision_market_smoke() {
    // Start the real vision-market binary (or fallback to `cargo run`)
    let exe = if cfg!(windows) {
        Path::new("target/debug/vision-market.exe")
    } else {
        Path::new("target/debug/vision-market")
    };

    let mut child: Child = if exe.exists() {
        Command::new(exe)
            .spawn()
            .expect("failed to spawn vision-market executable")
    } else {
        Command::new("cargo")
            .args(["run", "--bin", "vision-market"])
            .spawn()
            .expect("failed to spawn `cargo run --bin vision-market`")
    };

    // Poll for TCP readiness (avoid fixed sleeps)
    let addr = "127.0.0.1:8080";
    let start = Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(5) {
        match tokio::net::TcpStream::connect(addr).await {
            Ok(_) => {
                ready = true;
                break;
            }
            Err(_) => sleep(Duration::from_millis(100)).await,
        }
    }
    assert!(ready, "server did not start in time");

    let client = reqwest::Client::new();
    let resp = client
        .get("http://127.0.0.1:8080/market/land/listings")
        .send()
        .await
        .expect("request failed");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "expected 200 from /market/land/listings",
    );

    // Tear down
    let _ = child.kill();
    let _ = child.wait();
}
