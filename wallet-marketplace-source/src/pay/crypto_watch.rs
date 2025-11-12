use chrono::Utc;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use sled::Db;
use std::time::Duration;
use tokio::time::sleep;
use bs58;

pub fn generate_invoice_address(chain: &str, listing_id: &str) -> String {
    let seed = std::env::var("VISION_INVOICE_SEED").unwrap_or_else(|_| "VisionSeedDefault".to_string());
    let hash = Sha256::digest(format!("{}:{}:{}", chain, listing_id, seed).as_bytes());
    let base58 = bs58::encode(hash).into_string();
    format!("{}_{}", chain.to_lowercase(), &base58[..32])
}

pub trait CryptoBackend: Send + Sync + 'static {
    fn watch(&self);
}

pub fn start_watchers(db: Arc<Db>) {
    println!("[crypto_watch] started at {}", Utc::now());
    // Spawn simple watchers for demo
    let db_clone = db.clone();
    tokio::spawn(async move { watch_chain("BTC", db_clone.clone()).await });
    let db_clone = db.clone();
    tokio::spawn(async move { watch_chain("BCH", db_clone.clone()).await });
    let db_clone = db.clone();
    tokio::spawn(async move { watch_chain("DOGE", db_clone.clone()).await });
}

async fn watch_chain(chain: &str, _db: Arc<Db>) {
    loop {
        log::info!("Watching {} transactions...", chain);
        // TODO: Replace with Electrum JSON-RPC or API calls
        sleep(Duration::from_secs(60)).await;
    }
}
