use axum::{routing::post, Json, Router};
use axum_server::Server;
use chrono::Utc;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

/// Minimal plaintext Electrum server: supports headers.subscribe and address.get_history
async fn serve_electrum(
    listener: TcpListener,
    tip: u64,
    target_addr: String,
    tx_height: u64,
    txid: &'static str,
) {
    loop {
        let (sock, _) = listener.accept().await.unwrap();
        let a = target_addr.clone();
        tokio::spawn(handle_client(sock, tip, a, tx_height, txid));
    }
}

async fn handle_client(
    sock: TcpStream,
    tip: u64,
    target_addr: String,
    tx_h: u64,
    txid: &'static str,
) {
    let (r, mut w) = sock.into_split();
    let mut lines = BufReader::new(r).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let v: Value = serde_json::from_str(&line).unwrap_or(json!({}));
        let method = v.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = v.get("id").cloned().unwrap_or(json!(1));
        let res = match method {
            "blockchain.headers.subscribe" => json!({"height": tip, "hex": "00"}),
            "blockchain.address.get_history" => {
                let params = v
                    .get("params")
                    .and_then(|p| p.as_array())
                    .cloned()
                    .unwrap_or_default();
                let addr = params.first().and_then(|x| x.as_str()).unwrap_or("");
                if addr == target_addr {
                    json!([ { "tx_hash": txid, "height": tx_h } ])
                } else {
                    json!([])
                }
            }
            _ => Value::Null,
        };
        let reply = json!({"jsonrpc":"2.0","id":id,"result":res}).to_string();
        let _ = w.write_all(format!("{}\n", reply).as_bytes()).await;
    }
}

/// Counts callbacks per listing id
struct Counters {
    btc: AtomicUsize,
    bch: AtomicUsize,
    doge: AtomicUsize,
}
static COUNTS: Counters = Counters {
    btc: AtomicUsize::new(0),
    bch: AtomicUsize::new(0),
    doge: AtomicUsize::new(0),
};

async fn confirm_sink(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    match body.get("listing_id").and_then(|x| x.as_str()) {
        Some("Lbtc") => {
            COUNTS.btc.fetch_add(1, Ordering::SeqCst);
        }
        Some("Lbch") => {
            COUNTS.bch.fetch_add(1, Ordering::SeqCst);
        }
        Some("Ldoge") => {
            COUNTS.doge.fetch_add(1, Ordering::SeqCst);
        }
        _ => {}
    }
    Json(json!({"ok": true}))
}

#[tokio::test]
async fn confirms_all_three_chains_with_exact_confs() {
    // 1) start confirm sink (axum)
    let app = Router::new().route("/_market/land/confirm", post(confirm_sink));
    // use a std TcpListener for axum_server::Server::from_tcp
    let confirm_listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let confirm_addr = confirm_listener.local_addr().unwrap();
    tokio::spawn(async move {
        Server::from_tcp(confirm_listener)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // 2) three plaintext Electrum servers, each with tip/tx heights that satisfy thresholds
    // BTC: conf req 2 -> tip 101, tx 100  => 2 confs
    // BCH: conf req 2 -> tip 305, tx 304 => 2 confs
    // DOGE: conf req 40 -> tip 2000, tx 1961 => 40 confs
    let btc_l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let bch_l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let doge_l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let btc_addr = btc_l.local_addr().unwrap();
    let bch_addr = bch_l.local_addr().unwrap();
    let doge_addr = doge_l.local_addr().unwrap();

    tokio::spawn(async move {
        serve_electrum(btc_l, 101, "addr_btc".into(), 100, "txbtc").await;
    });
    tokio::spawn(async move {
        serve_electrum(bch_l, 305, "addr_bch".into(), 304, "txbch").await;
    });
    tokio::spawn(async move {
        serve_electrum(doge_l, 2000, "addr_doge".into(), 1961, "txdoge").await;
    });

    // 3) set test env so watcher posts back to our sink & allows plaintext electrum
    std::env::set_var("ELECTRUM_PLAINTEXT", "1");
    // watcher posts confirmations to VISION_SERVER_URL/_market/land/confirm
    std::env::set_var("VISION_SERVER_URL", format!("http://{}", confirm_addr));

    // ensure MOCK_CHAIN is not enabled for this test (we want real electrum mocks)
    std::env::remove_var("MOCK_CHAIN");
    // install resolved app config so watcher code can read flags
    let app_cfg = vision_market::config::AppConfig::default().resolved();
    vision_market::config::set_app_cfg(app_cfg).expect("set app cfg for test");

    // 4) seed three open listings into sled
    // access sled via the library crate
    let db = vision_market::market::cash_store::db_owned();
    let tree = db.open_tree("market_land_listings").unwrap();
    for (id, chain, pay_to) in [
        ("Lbtc", "BTC", "addr_btc"),
        ("Lbch", "BCH", "addr_bch"),
        ("Ldoge", "DOGE", "addr_doge"),
    ] {
        let now = Utc::now().timestamp();
        // ensure distinct timestamps
        let listing = serde_json::json!({
            "id": id,
            "seller_addr":"S",
            "qty_land": 1_u64,
            "price_amount": 10_u64,
            "price_chain": chain,
            "pay_to": pay_to,
            "status": "open",
            "created_at": now,
        });
        // distinct timestamps not critical here; ignore decrement
        tree.insert(id.as_bytes(), serde_json::to_vec(&listing).unwrap())
            .unwrap();
    }
    tree.flush().unwrap();

    // 5) build chain configs for each chain using plaintext + mock ports
    use vision_market::market::crypto_watch::ChainConf;
    let c_btc = ChainConf {
        name: "BTC",
        host: btc_addr.ip().to_string(),
        port_tls: btc_addr.port(),
        conf_req: 2,
        http_url: None,
        plaintext: true,
    };
    let c_bch = ChainConf {
        name: "BCH",
        host: bch_addr.ip().to_string(),
        port_tls: bch_addr.port(),
        conf_req: 2,
        http_url: None,
        plaintext: true,
    };
    let c_doge = ChainConf {
        name: "DOGE",
        host: doge_addr.ip().to_string(),
        port_tls: doge_addr.port(),
        conf_req: 40,
        http_url: None,
        plaintext: true,
    };

    // 6) run one cycle per chain (serial keeps the sled happy on Windows)
    vision_market::market::crypto_watch::one_cycle_for_test(c_btc)
        .await
        .unwrap();
    vision_market::market::crypto_watch::one_cycle_for_test(c_bch)
        .await
        .unwrap();
    vision_market::market::crypto_watch::one_cycle_for_test(c_doge)
        .await
        .unwrap();

    // 7) assert each listing triggered exactly one confirm callback
    assert_eq!(COUNTS.btc.load(Ordering::SeqCst), 1, "BTC confirm count");
    assert_eq!(COUNTS.bch.load(Ordering::SeqCst), 1, "BCH confirm count");
    assert_eq!(COUNTS.doge.load(Ordering::SeqCst), 1, "DOGE confirm count");
}
