#[tokio::test]
async fn admin_smoke() {
    use reqwest::Client;
    use std::net::TcpListener;
    use std::process::Command;
    use std::time::Duration;
    use std::{env, thread};

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
            // go up from deps/<exe>.exe to target/debug
            for _ in 0..3 {
                path.pop();
            }
            path.push("debug");
            path.push("vision-node");
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
            if r.status().is_success() {
                ok = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(ok, "server did not start");

    // readyz
    let r = client.get(format!("{}/readyz", base)).send().await.unwrap();
    assert!(r.status().is_success());

    // admin ping without token -> 401
    let r = client
        .get(format!("{}/admin/ping", base))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::UNAUTHORIZED);

    // wrong token via header -> 401
    let r = client
        .get(format!("{}/admin/ping", base))
        .header("x-admin-token", "wrong")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::UNAUTHORIZED);

    // with header
    let r = client
        .get(format!("{}/admin/ping", base))
        .header("x-admin-token", "testtoken")
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());

    // with query
    let r = client
        .get(format!("{}/admin/ping?token=testtoken", base))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());

    // metrics should show at least 2 admin pings
    let r = client
        .get(format!("{}/metrics.prom", base))
        .send()
        .await
        .unwrap();
    let body = r.text().await.unwrap();
    assert!(body.contains("vision_admin_ping_total"));
    // crude parse to check numeric value >= 2
    if let Some(line) = body
        .lines()
        .find(|l| l.starts_with("vision_admin_ping_total "))
    {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            let v: u64 = parts[1].parse().unwrap_or(0);
            assert!(v >= 2, "expected admin ping counter >= 2, got {}", v);
        }
    }

    // metrics
    // (metrics already checked above)

    // admin info
    let r = client
        .get(&format!("{}/admin/info", base))
        .header("x-admin-token", "testtoken")
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());

    // shutdown
    let _ = child.kill();
    let _ = child.wait();
}
