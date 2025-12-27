// COPILOT TASK: Implement simple price oracle with multi-chain support
//
// Goal:
// - Introduce a PriceOracle that periodically fetches asset prices (BTC/BCH/DOGE vs USD)
//   and exposes them to the rest of the node for LAND valuation and fee logic.
//
// Steps:
// 1. Create a new module `src/oracle.rs` with:
//      - `#[derive(Clone)] pub struct PriceOracle { ... }`
//      - An internal `Arc<RwLock<HashMap<String, f64>>>` to store symbol->price (e.g. "BTCUSD").
// 2. Implement `impl PriceOracle` with methods:
//      - `pub fn new(http: reqwest::Client) -> Self`
//      - `pub async fn refresh(&self) -> anyhow::Result<()>`
//        - Fetch BTC/BCH/DOGE prices in USD from one or two public HTTP APIs (e.g. CoinGecko).
//        - Parse JSON, update the internal map.
//      - `pub fn get(&self, symbol: &str) -> Option<f64>` to read cached price.
// 3. In `main.rs` (or app state module):
//      - Construct a shared `PriceOracle` with a `reqwest::Client`.
//      - Spawn a background task using `tokio::spawn` that:
//            loop {
//                if let Err(e) = oracle.refresh().await {
//                    warn!("price oracle refresh failed: {:?}", e);
//                }
//                tokio::time::sleep(Duration::from_secs(30)).await;
//            }
// 4. Extend your AppState (or equivalent) to hold an `Arc<PriceOracle>`:
//      - Pass it into API handlers needing pricing (e.g., fiat onramp, LAND valuation).
// 5. Add a read-only HTTP endpoint, e.g. `GET /oracle/prices`, that returns the current map
//    as JSON so operators can see what the oracle thinks.
//
// Result:
// - The node maintains a cached view of BTC/BCH/DOGE → USD prices and exposes them
//   for fee calculation, LAND valuation, and diagnostics.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time;
use axum::{
    extract::State,
    response::Json,
};

/// Price data from CoinGecko API
#[derive(Debug, Deserialize)]
struct CoinGeckoResponse {
    bitcoin: Option<CoinPrice>,
    #[serde(rename = "bitcoin-cash")]
    bitcoin_cash: Option<CoinPrice>,
    dogecoin: Option<CoinPrice>,
}

#[derive(Debug, Deserialize)]
struct CoinPrice {
    usd: Option<f64>,
}

/// Multi-currency price oracle
#[derive(Clone)]
pub struct PriceOracle {
    http: reqwest::Client,
    prices: Arc<RwLock<HashMap<String, f64>>>,
    last_update: Arc<RwLock<Option<std::time::SystemTime>>>,
}

impl PriceOracle {
    /// Create new price oracle with HTTP client
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            prices: Arc::new(RwLock::new(HashMap::new())),
            last_update: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Fetch latest prices from CoinGecko
    pub async fn refresh(&self) -> Result<()> {
        tracing::debug!("Refreshing price oracle from CoinGecko...");
        
        // CoinGecko simple price endpoint (free tier, no API key required)
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,bitcoin-cash,dogecoin&vs_currencies=usd";
        
        let response = self.http
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch prices: {}", e))?;
        
        if !response.status().is_success() {
            return Err(anyhow!("CoinGecko API returned status {}", response.status()));
        }
        
        let data: CoinGeckoResponse = response.json().await
            .map_err(|e| anyhow!("Failed to parse CoinGecko response: {}", e))?;
        
        // Update prices map
        let mut prices = self.prices.write().unwrap();
        
        if let Some(btc) = data.bitcoin.and_then(|p| p.usd) {
            prices.insert("BTCUSD".to_string(), btc);
            tracing::debug!("Updated BTC price: ${:.2}", btc);
        }
        
        if let Some(bch) = data.bitcoin_cash.and_then(|p| p.usd) {
            prices.insert("BCHUSD".to_string(), bch);
            tracing::debug!("Updated BCH price: ${:.2}", bch);
        }
        
        if let Some(doge) = data.dogecoin.and_then(|p| p.usd) {
            prices.insert("DOGEUSD".to_string(), doge);
            tracing::debug!("Updated DOGE price: ${:.2}", doge);
        }
        
        // Update last refresh time
        let mut last_update = self.last_update.write().unwrap();
        *last_update = Some(std::time::SystemTime::now());
        
        tracing::info!(
            "✅ Price oracle refreshed: BTC=${:.2}, BCH=${:.2}, DOGE=${:.4}",
            prices.get("BTCUSD").unwrap_or(&0.0),
            prices.get("BCHUSD").unwrap_or(&0.0),
            prices.get("DOGEUSD").unwrap_or(&0.0)
        );
        
        Ok(())
    }
    
    /// Get cached price for a symbol
    pub fn get(&self, symbol: &str) -> Option<f64> {
        let prices = self.prices.read().unwrap();
        prices.get(symbol).copied()
    }
    
    /// Get all cached prices
    pub fn get_all(&self) -> HashMap<String, f64> {
        let prices = self.prices.read().unwrap();
        prices.clone()
    }
    
    /// Get time of last successful refresh
    pub fn last_update(&self) -> Option<std::time::SystemTime> {
        let last_update = self.last_update.read().unwrap();
        *last_update
    }
    
    /// Check if prices are stale (older than 5 minutes)
    pub fn is_stale(&self) -> bool {
        match self.last_update() {
            Some(time) => {
                match time.elapsed() {
                    Ok(duration) => duration > Duration::from_secs(300),
                    Err(_) => true,
                }
            }
            None => true,
        }
    }
}

/// Response for /oracle/prices endpoint
#[derive(Debug, Serialize)]
pub struct PricesResponse {
    pub prices: HashMap<String, f64>,
    pub last_update: Option<String>,
    pub stale: bool,
}

/// HTTP handler to get current prices
pub async fn get_prices_handler(
    State(oracle): State<Arc<PriceOracle>>,
) -> Json<PricesResponse> {
    let prices = oracle.get_all();
    let last_update = oracle.last_update().map(|t| {
        // Format as ISO 8601
        match t.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => {
                let secs = d.as_secs();
                format!("{}", chrono::DateTime::from_timestamp(secs as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string()))
            }
            Err(_) => "unknown".to_string(),
        }
    });
    let stale = oracle.is_stale();
    
    Json(PricesResponse {
        prices,
        last_update,
        stale,
    })
}

/// Start background price refresh task
pub fn start_price_refresh_task(oracle: Arc<PriceOracle>, interval_secs: u64) {
    tokio::spawn(async move {
        tracing::info!("Starting price oracle refresh task (interval: {}s)", interval_secs);
        
        // Initial refresh
        if let Err(e) = oracle.refresh().await {
            tracing::warn!("Initial price oracle refresh failed: {}", e);
        }
        
        // Periodic refresh loop
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = oracle.refresh().await {
                tracing::warn!("Price oracle refresh failed: {}", e);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_oracle_creation() {
        let client = reqwest::Client::new();
        let oracle = PriceOracle::new(client);
        
        assert!(oracle.get("BTCUSD").is_none());
        assert!(oracle.is_stale());
    }
    
    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_oracle_refresh() {
        let client = reqwest::Client::new();
        let oracle = PriceOracle::new(client);
        
        let result = oracle.refresh().await;
        assert!(result.is_ok(), "Refresh failed: {:?}", result);
        
        // Should have prices now
        assert!(oracle.get("BTCUSD").is_some());
        assert!(oracle.get("BCHUSD").is_some());
        assert!(oracle.get("DOGEUSD").is_some());
        
        // Should not be stale immediately after refresh
        assert!(!oracle.is_stale());
    }
}
