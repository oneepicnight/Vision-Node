// ---- Clippy/lints: keep signals high, noise low ----
#![cfg_attr(not(any(test, feature = "dev")), allow(dead_code))]
mod config;

use axum::Router;
use std::net::SocketAddr;

mod crypto;
mod ledger;
mod market;
mod util;
use market::router as market_router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();
    // Load application config (vision.toml + env overrides)
    {
        let cfg = config::AppConfig::load_from("vision.toml")
            .map(|c| c.resolved())
            .expect("load config");
        log::info!(
            "Electrum cfg: BTC {}:{}, BCH {}:{}, DOGE {}:{}, conf[btc:{} bch:{} doge:{}]",
            cfg.btc_host,
            cfg.btc_port,
            cfg.bch_host,
            cfg.bch_port,
            cfg.doge_host,
            cfg.doge_port,
            cfg.btc_conf,
            cfg.bch_conf,
            cfg.doge_conf
        );
        // install globally for other modules to read
        config::set_app_cfg(cfg).expect("set app cfg");
    }
    // Log DB path (resolve to absolute for clarity)
    let db_path = std::path::Path::new("wallet_data/market");
    match std::fs::canonicalize(db_path) {
        Ok(abs) => log::info!("Using sled DB at {}", abs.display()),
        Err(_) => log::info!("Using sled DB at {} (will be created)", db_path.display()),
    }
    if std::env::var("ADMIN_TOKEN").is_ok() {
        log::info!("Admin endpoints require X-Admin-Token header");
    } else {
        log::warn!("ADMIN_TOKEN not set — admin endpoints open (DEV mode). ");
    }

    // --- migrate legacy cash orders -> market_cash_orders ---
    match crate::market::cash_store::migrate_legacy_prefix() {
        Ok(n) => {
            if n > 0 {
                log::warn!("cash_store migration: moved {} legacy 'cash_order:<id>' keys into 'market_cash_orders' tree", n);
            } else {
                log::info!("cash_store migration: no legacy keys found");
            }
        }
        Err(e) => {
            log::error!("cash_store migration failed: {:?}", e);
        }
    }

    // optionally remove legacy keys after a successful migration if explicitly enabled
    if std::env::var("CASH_MIGRATION_DELETE_LEGACY")
        .ok()
        .as_deref()
        == Some("1")
    {
        match crate::market::cash_store::cleanup_legacy_prefix() {
            Ok(rm) => {
                if rm > 0 {
                    log::warn!("cash_store: deleted {} legacy keys after migration", rm);
                }
            }
            Err(e) => log::error!("cash_store cleanup failed: {:?}", e),
        }
    }

    let app = Router::new()
        .merge(market_router())
        .layer(tower_http::cors::CorsLayer::permissive());

    // spawn crypto watchers
    tokio::spawn(async move { crate::market::crypto_watch::spawn_crypto_watchers().await });

    let port: u16 = std::env::var("VISION_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Starting vision-node on {}", addr);

    println!("Server listening on http://{}", addr);

    // Create a make_service from the router and pass it to axum_server.
    let make_svc = app.into_make_service();

    axum_server::bind(addr).serve(make_svc).await?;

    Ok(())
}

// use crate::config::get_app_cfg() from other modules
