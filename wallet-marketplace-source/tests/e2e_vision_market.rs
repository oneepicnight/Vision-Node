use serde_json::Value;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;
mod common;

fn spawn_vision_market_with_env(db_path: &str, electrum_mock_url: &str) -> Child {
    // use platform-specific binary name to avoid falling back to `cargo run`
    // (on Unix the binary has no .exe extension). If the built binary
    // exists in target/debug we'll spawn it directly; otherwise fall back
    // to `cargo run` which builds and runs the binary.
    let exe_path = if cfg!(windows) {
        "target/debug/vision-market.exe"
    } else {
        "target/debug/vision-market"
    };
    let exe = std::path::Path::new(exe_path);
    let mut cmd = if exe.exists() {
        Command::new(exe)
    } else {
        let mut c = Command::new("cargo");
        // allow enabling testhooks feature via env for cargo-run fallback
        if std::env::var("VISION_TEST_HOOKS").ok().as_deref() == Some("1") {
            c.args(["run", "--bin", "vision-market", "--features", "testhooks"]);
        } else {
            c.args(["run", "--bin", "vision-market"]);
        }
        c
    };
    cmd.env("VISION_DB_PATH", db_path);
    cmd.env("VISION_ELECTRUM_BTC", electrum_mock_url);
    if let Ok(v) = std::env::var("VISION_WATCH_POLL_SECS") {
        cmd.env("VISION_WATCH_POLL_SECS", v);
    }
    if let Ok(p) = std::env::var("VISION_PORT") {
        cmd.env("VISION_PORT", p);
    }
    if let Ok(url) = std::env::var("VISION_SERVER_URL") {
        cmd.env("VISION_SERVER_URL", url);
    }
    cmd.spawn().expect("failed to spawn vision-market")
}

fn start_mock_electrum_server(
    registered: Arc<Mutex<std::collections::HashSet<String>>>,
    seen: Arc<Mutex<Vec<String>>>,
) -> (String, tokio::sync::oneshot::Sender<()>) {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Method, Request, Response, Server, StatusCode};
    use serde_json::json;
    use std::convert::Infallible;
    use tokio::runtime::Runtime;

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("find free port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    std::thread::spawn(move || {
        let rt = Runtime::new().expect("tokio rt");
        rt.block_on(async move {
            let addr = ([127,0,0,1], port).into();
            let make_svc = make_service_fn(move |_conn| {
                // clone per-connection
                let registered_conn = registered.clone();
                let seen_conn = seen.clone();
                async move {
                    Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                        // clone per-request
                        let registered = registered_conn.clone();
                        let seen = seen_conn.clone();
                        async move {
                            if req.method() == Method::POST {
                                let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap_or_default();
                                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                                    if let Some(m) = v.get("method").and_then(|x| x.as_str()) {
                                        if m == "blockchain.address.get_history" {
                                            if let Some(params) = v.get("params").and_then(|p| p.as_array()) {
                                                if let Some(addr_val) = params.first().and_then(|a| a.as_str()) {
                                                    let reg = registered.lock().unwrap();
                                                    if reg.contains(addr_val) {
                                                        let mut s = seen.lock().unwrap();
                                                        s.push(addr_val.to_string());
                                                        let res = json!({
                                                            "jsonrpc": "2.0",
                                                            "result": [ { "height": 600000, "tx_hash": format!("mockedtx_for_{}", addr_val) } ],
                                                            "id": v.get("id").cloned().unwrap_or(serde_json::Value::Number(1.into()))
                                                        });
                                                        return Ok::<_, Infallible>(Response::new(Body::from(res.to_string())));
                                                    }
                                                }
                                            }
                                            let res = json!({ "jsonrpc": "2.0", "result": [], "id": v.get("id").cloned().unwrap_or(serde_json::Value::Number(1.into())) });
                                            return Ok::<_, Infallible>(Response::new(Body::from(res.to_string())));
                                        }
                                    }
                                }
                                let not_impl = Response::builder().status(StatusCode::NOT_IMPLEMENTED).body(Body::from("not implemented")).unwrap();
                                Ok::<_, Infallible>(not_impl)
                            } else {
                                Ok::<_, Infallible>(Response::new(Body::from("ok")))
                            }
                        }
                    }))
                }
            });

            let server = Server::bind(&addr).serve(make_svc);
            let graceful = server.with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
            let _ = graceful.await;
        });
    });

    (format!("http://127.0.0.1:{}", port), shutdown_tx)
}

fn start_confirm_receiver() -> (
    String,
    Arc<Mutex<Vec<serde_json::Value>>>,
    tokio::sync::oneshot::Sender<()>,
) {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Method, Request, Response, Server, StatusCode};
    use std::convert::Infallible;
    use tokio::runtime::Runtime;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let seen = Arc::new(Mutex::new(Vec::new()));

    // pick an ephemeral port
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("find free port");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let seen_clone = seen.clone();
    std::thread::spawn(move || {
        let rt = Runtime::new().expect("tokio rt");
        rt.block_on(async move {
            let addr = ([127, 0, 0, 1], port).into();
            let make_svc = make_service_fn(move |_conn| {
                let seen_conn = seen_clone.clone();
                async move {
                    Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                        let seen = seen_conn.clone();
                        async move {
                            if req.method() == Method::POST
                                && req.uri().path() == "/_market/land/confirm"
                            {
                                let bytes = hyper::body::to_bytes(req.into_body())
                                    .await
                                    .unwrap_or_default();
                                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                                    let mut s = seen.lock().unwrap();
                                    s.push(v);
                                    return Ok::<_, Infallible>(Response::new(Body::from("ok")));
                                }
                                return Ok::<_, Infallible>(
                                    Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(Body::from("bad json"))
                                        .unwrap(),
                                );
                            }
                            Ok::<_, Infallible>(Response::new(Body::from("ok")))
                        }
                    }))
                }
            });

            let server = Server::bind(&addr).serve(make_svc);
            let graceful = server.with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });
            let _ = graceful.await;
        });
    });

    (format!("http://127.0.0.1:{}", port), seen, shutdown_tx)
}

#[tokio::test]
async fn e2e_create_signal_confirm_with_watcher() {
    common::init_test_tracing();
    use tempfile::tempdir;
    let td = tempdir().expect("tempdir");
    let db_path = td.path().to_str().unwrap().to_string();

    let registered = Arc::new(Mutex::new(std::collections::HashSet::new()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (mock_url, mock_shutdown) = start_mock_electrum_server(registered.clone(), seen.clone());

    // wait for mock to be ready (tcp connect)
    let mock_port: u16 = mock_url.split(':').next_back().unwrap().parse().unwrap();
    let addr = format!("127.0.0.1:{}", mock_port);
    let start = std::time::Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(10) {
        if tokio::net::TcpStream::connect(&addr).await.is_ok() {
            ready = true;
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }
    assert!(ready, "mock did not become ready");

    // start confirm receiver and instruct child to post to it
    std::env::set_var("VISION_WATCH_POLL_SECS", "1");
    let (confirm_url, confirm_seen, confirm_shutdown) = start_confirm_receiver();
    std::env::set_var("VISION_SERVER_URL", &confirm_url);
    sleep(Duration::from_millis(200)).await;

    let mut child = spawn_vision_market_with_env(&db_path, &mock_url);
    // wait for server to accept connections on 127.0.0.1:8080 (timeout 10s)
    let start = std::time::Instant::now();
    let mut server_ready = false;
    while start.elapsed() < Duration::from_secs(10) {
        if tokio::net::TcpStream::connect("127.0.0.1:8080")
            .await
            .is_ok()
        {
            server_ready = true;
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }
    assert!(server_ready, "server did not start in time");

    let client = reqwest::Client::new();
    let create_body = serde_json::json!({
        "seller_addr": "seller1",
        "qty_land": 100,
        "price_amount": 1000,
        "price_chain": "BTC"
    });
    let resp = client
        .post("http://127.0.0.1:8080/market/land/list")
        .json(&create_body)
        .send()
        .await
        .expect("create failed");
    assert_eq!(resp.status().as_u16(), 201);
    let created: Value = resp.json().await.expect("invalid create json");
    let pay_to = created["pay_to"].as_str().unwrap().to_string();

    {
        let mut reg = registered.lock().unwrap();
        reg.insert(pay_to.clone());
    }

    // Nudge the watcher via testhook endpoint if available
    if std::env::var("VISION_TEST_HOOKS").ok().as_deref() == Some("1") {
        let base = "http://127.0.0.1:8080".to_string();
        let _ = reqwest::Client::new()
            .post(format!("{}/__test/watcher_tick", base))
            .send()
            .await;
    }

    // wait up to 20s for confirm POST -> then assert listing is settled by checking sled DB directly
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut got_confirm = false;
    while std::time::Instant::now() < deadline {
        {
            let confirms = confirm_seen.lock().unwrap();
            if !confirms.is_empty() {
                got_confirm = true;
            }
        }

        // Query the running server for the listing status instead of opening sled (avoids DB locking between processes on Windows)
        if let Ok(resp) = client
            .get(format!(
                "http://127.0.0.1:8080/market/land/listings/{}",
                created["listing_id"].as_str().unwrap()
            ))
            .send()
            .await
        {
            if let Ok(listing_json) = resp.json::<serde_json::Value>().await {
                if listing_json.get("status").and_then(|s| s.as_str()) == Some("settled")
                    && got_confirm
                {
                    break;
                }
            }
        }

        sleep(Duration::from_millis(200)).await;
    }
    assert!(
        got_confirm,
        "confirm receiver did not get a POST from the watcher"
    );

    let seen_addrs = seen.lock().unwrap();
    assert!(
        seen_addrs.iter().any(|a| a == &pay_to),
        "mock electrum did not receive a request for the pay_to address"
    );

    // Clean up
    let _ = child.kill();
    let _ = child.wait();
    let _ = mock_shutdown.send(());
    let _ = confirm_shutdown.send(());
}
