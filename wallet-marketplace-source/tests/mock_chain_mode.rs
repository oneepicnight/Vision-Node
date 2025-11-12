use axum::{routing::post, Json, Router};
use axum_server::Server;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

static CALLED: AtomicUsize = AtomicUsize::new(0);

async fn confirm_sink(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    if body.get("listing_id").and_then(|x| x.as_str()) == Some("Lmock") {
        CALLED.fetch_add(1, Ordering::SeqCst);
    }
    Json(json!({"ok": true}))
}

#[tokio::test]
async fn mock_chain_confirms_immediately() {
    let app = Router::new().route("/_market/land/confirm", post(confirm_sink));
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        Server::from_tcp(listener)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
    std::env::set_var(
        "MARKET_CONFIRM_URL",
        format!("http://{}/_market/land/confirm", addr),
    );
    std::env::set_var("MOCK_CHAIN", "1");

    // install resolved app config so watcher code can read flags
    let app_cfg = vision_market::config::AppConfig::default().resolved();
    vision_market::config::set_app_cfg(app_cfg).expect("set app cfg for test");

    // seed a mock listing
    let db = vision_market::market::cash_store::db_owned();
    let listing = serde_json::json!({
        "id":"Lmock","seller_addr":"S","qty_land":1u64,"price_amount":10u64,
        "price_chain":"BTC","pay_to":"mock:anything","status":"open","created_at":0i64
    });
    db.insert("listing:Lmock", serde_json::to_vec(&listing).unwrap())
        .unwrap();
    db.flush().unwrap();

    let cfg = vision_market::market::crypto_watch::ChainConf {
        name: "BTC",
        host: "ignored".into(),
        port_tls: 0,
        conf_req: 1,
        http_url: None,
        plaintext: true,
    };
    vision_market::market::crypto_watch::one_cycle_for_test(cfg)
        .await
        .unwrap();

    assert_eq!(
        CALLED.load(Ordering::SeqCst),
        1,
        "mock confirm not called exactly once"
    );
}
