//! External blockchain RPC client management for BTC, BCH, DOGE, etc.
#![allow(dead_code)]

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// RPC client health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStatus {
    pub configured: bool,
    pub ok: bool,
    pub last_error: Option<String>,
}

/// Supported external blockchains
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExternalChain {
    #[serde(rename = "btc")]
    Btc,
    #[serde(rename = "bch")]
    Bch,
    #[serde(rename = "doge")]
    Doge,
}

impl ExternalChain {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExternalChain::Btc => "btc",
            ExternalChain::Bch => "bch",
            ExternalChain::Doge => "doge",
        }
    }
}

/// Configuration for a single blockchain RPC endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct ChainRpcConfig {
    pub rpc_url: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_retries: Option<u32>,
    pub fallback_urls: Option<Vec<String>>,
}

/// External RPC configuration container
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExternalRpcConfig {
    pub btc: Option<ChainRpcConfig>,
    pub bch: Option<ChainRpcConfig>,
    pub doge: Option<ChainRpcConfig>,
}

/// RPC client for a specific blockchain
pub struct RpcClient {
    pub chain: ExternalChain,
    pub primary_url: String,
    pub fallback_urls: Vec<String>,
    pub http: Client,
    pub username: Option<String>,
    pub password: Option<String>,
    pub max_retries: u32,
}

impl RpcClient {
    /// Create RPC client from chain config
    pub fn from_chain_cfg(chain: ExternalChain, cfg: &ChainRpcConfig) -> Result<Self> {
        let timeout = cfg.timeout_ms.unwrap_or(8000);
        let max_retries = cfg.max_retries.unwrap_or(3);

        let http = Client::builder()
            .timeout(std::time::Duration::from_millis(timeout))
            .build()?;

        Ok(Self {
            chain,
            primary_url: cfg.rpc_url.clone(),
            fallback_urls: cfg.fallback_urls.clone().unwrap_or_default(),
            http,
            username: cfg.username.clone(),
            password: cfg.password.clone(),
            max_retries,
        })
    }

    /// Call RPC method with automatic failover and exponential backoff
    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let mut urls: Vec<String> = std::iter::once(&self.primary_url)
            .chain(self.fallback_urls.iter())
            .cloned()
            .collect();

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let max_attempts = 1 + self.fallback_urls.len();
        let mut backoff_ms = 100u64; // Start with 100ms backoff

        for (attempt, url) in urls.drain(..).enumerate() {
            // Apply exponential backoff (except on first attempt)
            if attempt > 0 {
                let delay = tokio::time::Duration::from_millis(backoff_ms);
                tracing::debug!(
                    chain = ?self.chain,
                    backoff_ms = backoff_ms,
                    "Backing off before retry"
                );
                tokio::time::sleep(delay).await;

                // Exponential backoff with cap at 10 seconds
                backoff_ms = (backoff_ms * 2).min(10_000);
            }

            let mut req = self.http.post(&url).json(&body);

            if let (Some(ref user), Some(ref pass)) = (&self.username, &self.password) {
                req = req.basic_auth(user, Some(pass));
            }

            match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        tracing::warn!(
                            chain = ?self.chain,
                            url = %url,
                            status = %resp.status(),
                            attempt = attempt + 1,
                            max_attempts,
                            "RPC endpoint returned error status"
                        );
                        if attempt + 1 >= max_attempts {
                            return Err(anyhow!(
                                "All RPC endpoints failed after {} attempts",
                                max_attempts
                            ));
                        }
                        continue;
                    }

                    let json: serde_json::Value = resp.json().await?;

                    if let Some(err) = json.get("error") {
                        if !err.is_null() {
                            tracing::warn!(
                                chain = ?self.chain,
                                error = %err,
                                "RPC returned error response"
                            );
                            return Err(anyhow!("RPC error: {}", err));
                        }
                    }

                    // Success - log if we recovered after failures
                    if attempt > 0 {
                        tracing::info!(
                            chain = ?self.chain,
                            url = %url,
                            attempt = attempt + 1,
                            "RPC request succeeded after {} retries", attempt
                        );
                    }

                    return Ok(json["result"].clone());
                }
                Err(e) => {
                    tracing::warn!(
                        chain = ?self.chain,
                        url = %url,
                        error = %e,
                        attempt = attempt + 1,
                        max_attempts,
                        "RPC request failed, will retry with backoff"
                    );

                    if attempt + 1 >= max_attempts {
                        return Err(anyhow!(
                            "All RPC endpoints failed after {} attempts: {}",
                            max_attempts,
                            e
                        ));
                    }
                    // Continue to next URL with backoff
                }
            }
        }

        Err(anyhow!("All RPC endpoints exhausted"))
    }

    /// Convenience method for calling with array params
    pub async fn call_array(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.call(method, serde_json::Value::Array(params)).await
    }

    /// Convenience method for calling with no params
    pub async fn call_no_params(&self, method: &str) -> Result<serde_json::Value> {
        self.call(method, serde_json::Value::Array(vec![])).await
    }
}

/// Container for all external RPC clients
#[derive(Clone, Default)]
pub struct RpcClients {
    inner: HashMap<ExternalChain, Arc<RpcClient>>,
}

impl RpcClients {
    /// Create RPC clients from config
    pub fn new(cfg: &ExternalRpcConfig) -> Result<Self> {
        let mut inner = HashMap::new();

        if let Some(btc) = &cfg.btc {
            let client = RpcClient::from_chain_cfg(ExternalChain::Btc, btc)?;
            inner.insert(ExternalChain::Btc, Arc::new(client));
            tracing::info!("✅ Bitcoin RPC configured: {}", btc.rpc_url);
        }

        if let Some(bch) = &cfg.bch {
            let client = RpcClient::from_chain_cfg(ExternalChain::Bch, bch)?;
            inner.insert(ExternalChain::Bch, Arc::new(client));
            tracing::info!("✅ Bitcoin Cash RPC configured: {}", bch.rpc_url);
        }

        if let Some(doge) = &cfg.doge {
            let client = RpcClient::from_chain_cfg(ExternalChain::Doge, doge)?;
            inner.insert(ExternalChain::Doge, Arc::new(client));
            tracing::info!("✅ Dogecoin RPC configured: {}", doge.rpc_url);
        }

        Ok(Self { inner })
    }

    /// Get RPC client for a specific chain
    pub fn get(&self, chain: ExternalChain) -> Option<Arc<RpcClient>> {
        self.inner.get(&chain).cloned()
    }

    /// Check if a chain is configured
    pub fn has(&self, chain: ExternalChain) -> bool {
        self.inner.contains_key(&chain)
    }

    /// Get all configured chains
    pub fn configured_chains(&self) -> Vec<ExternalChain> {
        self.inner.keys().copied().collect()
    }

    /// Check status of all configured RPC clients
    pub async fn check_status(&self) -> HashMap<ExternalChain, RpcStatus> {
        let mut status_map = HashMap::new();

        for (chain, client) in &self.inner {
            let status = match client.call_no_params("getblockcount").await {
                Ok(_) => RpcStatus {
                    configured: true,
                    ok: true,
                    last_error: None,
                },
                Err(e) => RpcStatus {
                    configured: true,
                    ok: false,
                    last_error: Some(e.to_string()),
                },
            };

            status_map.insert(*chain, status);
        }

        status_map
    }

    /// Apply environment variable overrides to config
    pub fn apply_env_overrides(cfg: &mut ExternalRpcConfig) {
        // Bitcoin overrides
        if let Some(ref mut btc) = cfg.btc {
            if let Ok(url) = std::env::var("VISION_RPC_BTC_URL") {
                btc.rpc_url = url;
            }
            if let Ok(user) = std::env::var("VISION_RPC_BTC_USER") {
                btc.username = Some(user);
            }
            if let Ok(pass) = std::env::var("VISION_RPC_BTC_PASS") {
                btc.password = Some(pass);
            }
        }

        // Bitcoin Cash overrides
        if let Some(ref mut bch) = cfg.bch {
            if let Ok(url) = std::env::var("VISION_RPC_BCH_URL") {
                bch.rpc_url = url;
            }
            if let Ok(user) = std::env::var("VISION_RPC_BCH_USER") {
                bch.username = Some(user);
            }
            if let Ok(pass) = std::env::var("VISION_RPC_BCH_PASS") {
                bch.password = Some(pass);
            }
        }

        // Dogecoin overrides
        if let Some(ref mut doge) = cfg.doge {
            if let Ok(url) = std::env::var("VISION_RPC_DOGE_URL") {
                doge.rpc_url = url;
            }
            if let Ok(user) = std::env::var("VISION_RPC_DOGE_USER") {
                doge.username = Some(user);
            }
            if let Ok(pass) = std::env::var("VISION_RPC_DOGE_PASS") {
                doge.password = Some(pass);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_enum() {
        assert_eq!(ExternalChain::Btc.as_str(), "btc");
        assert_eq!(ExternalChain::Bch.as_str(), "bch");
        assert_eq!(ExternalChain::Doge.as_str(), "doge");
    }

    #[test]
    fn test_empty_config() {
        let cfg = ExternalRpcConfig::default();
        let clients = RpcClients::new(&cfg).unwrap();
        assert!(!clients.has(ExternalChain::Btc));
        assert!(!clients.has(ExternalChain::Bch));
        assert!(!clients.has(ExternalChain::Doge));
    }
}
