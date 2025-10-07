#[tokio::test]
async fn admin_bearer() {
    use std::net::TcpListener;
    use std::process::Command;
    use std::time::Duration;
    use std::{thread, env};
    use reqwest::Client;

    // find free port
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    // get binary path provided by cargo
    let bin = match env::var("CARGO_BIN_EXE_vision-node") {
        Ok(p) => p,
        Err(_) => {
            // fallback to target/debug/vision-node
            let mut path = env::current_exe().expect("cwd");
            for _ in 0..3 { path.pop(); }
            path.push("debug"); path.push("vision-node");
            path.to_string_lossy().to_string()
        }
    };

    // spawn the server
    let mut child = Command::new(bin)
        .env("VISION_PORT", port.to_string())
        .env("VISION_ADMIN_TOKEN", "testtoken")
        .spawn()
        .expect("spawn server");

    // wait for server
    let client = Client::new();
    let base = format!("http://127.0.0.1:{}", port);
    let mut ok = false;
    for _ in 0..40 {
        if let Ok(r) = client.get(format!("{}/livez", base)).send().await {
            if r.status().is_success() { ok = true; break; }
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ok, "server did not start");

    // Bearer header with correct token
    let r = client.get(format!("{}/admin/ping", base))
        .header("Authorization", "Bearer testtoken")
        .send().await.unwrap();
    assert!(r.status().is_success());

    // Bearer header with wrong token
    let r = client.get(format!("{}/admin/ping", base))
        .header("Authorization", "Bearer wrong")
        .send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::UNAUTHORIZED);

    // shutdown
    let _ = child.kill();
    let _ = child.wait();
}
