use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use tokio::time::sleep;

#[tokio::test]
async fn simple_server_smoke() {
    // Prefer the platform-specific built binary so tests don't fall back to
    // `cargo run` (which can trigger a build and cause timing/race issues).
    let exe_path = if cfg!(windows) {
        "target/debug/simple_server.exe"
    } else {
        "target/debug/simple_server"
    };
    let exe = Path::new(exe_path);
    let mut child = if exe.exists() {
        Command::new(exe)
            .spawn()
            .expect("failed to spawn simple_server")
    } else {
        // Fallback: run cargo run --bin simple_server
        Command::new("cargo")
            .args(["run", "--bin", "simple_server"])
            .spawn()
            .expect("failed to spawn cargo run simple_server")
    };

    // wait for the server to bind by attempting TCP connect (robust vs fixed sleep)
    let addr = "127.0.0.1:8080";
    let start = Instant::now();
    let mut bound = false;
    while start.elapsed() < Duration::from_secs(5) {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            bound = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    assert!(
        bound,
        "simple_server did not bind to 127.0.0.1:8080 in time"
    );

    let client = reqwest::Client::new();
    let resp = client
        .get("http://127.0.0.1:8080/")
        .send()
        .await
        .expect("request failed");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "expected 200 from simple server"
    );

    // Kill the child process
    let _ = child.kill();
    let _ = child.wait();
}
