use crate::config::get_app_cfg;
use crate::crypto::addr::{address_to_script_any, scripthash_hex};
use crate::util::log_throttle::warn_throttled;
use anyhow::{anyhow, Result};
use hex;
use log::{info, warn};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};
use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName};
use tokio_rustls::TlsConnector;

#[derive(Clone)]
pub struct ChainConf {
    pub name: &'static str, // "BTC"|"BCH"|"DOGE"
    pub host: String,
    pub port_tls: u16,
    pub conf_req: u64,
    pub http_url: Option<String>,
    pub plaintext: bool,
}

fn load_chain_confs() -> Vec<ChainConf> {
    // Use the app resolved config (vision.toml + env overrides) and also allow
    // an explicit VISION_ELECTRUM_<CHAIN> HTTP URL override for test mocks.
    let cfg = get_app_cfg();
    let btc_http = std::env::var("VISION_ELECTRUM_BTC").ok();
    let bch_http = std::env::var("VISION_ELECTRUM_BCH").ok();
    let doge_http = std::env::var("VISION_ELECTRUM_DOGE").ok();
    // prefer resolved config value
    let app_cfg = crate::config::get_app_cfg();
    let plaintext = app_cfg.electrum_plaintext;

    vec![
        ChainConf {
            name: "BTC",
            host: cfg.btc_host.clone(),
            port_tls: cfg.btc_port,
            conf_req: cfg.btc_conf,
            http_url: btc_http,
            plaintext,
        },
        ChainConf {
            name: "BCH",
            host: cfg.bch_host.clone(),
            port_tls: cfg.bch_port,
            conf_req: cfg.bch_conf,
            http_url: bch_http,
            plaintext,
        },
        ChainConf {
            name: "DOGE",
            host: cfg.doge_host.clone(),
            port_tls: cfg.doge_port,
            conf_req: cfg.doge_conf,
            http_url: doge_http,
            plaintext,
        },
    ]
}

pub fn generate_invoice_address(chain: &str, listing_id: &str) -> String {
    let seed = std::env::var("VISION_INVOICE_SEED").unwrap_or("VisionSeedDefault".into());
    let hash = Sha256::digest(format!("{}:{}:{}", chain, listing_id, seed).as_bytes());
    format!("{}_{}", chain.to_lowercase(), &hex::encode(hash)[..16])
}

// --- TLS client for Electrum (line-delimited JSON) ---
async fn electrum_call_tls(cfg: &ChainConf, method: &str, params: Value) -> Result<Value> {
    // If an http_url is provided (test mock), use HTTP JSON-RPC POST instead of TLS TCP socket.
    if let Some(url) = &cfg.http_url {
        let client = reqwest::Client::new();
        let req = json!({"id": 1, "jsonrpc": "2.0", "method": method, "params": params});
        let resp = client.post(url).json(&req).send().await?;
        let v: Value = resp.json().await?;
        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
    }
    // If configured for plaintext Electrum (test mocks), open a plain TCP socket
    if cfg.plaintext {
        let addr = format!("{}:{}", cfg.host, cfg.port_tls);
        let mut stream = TcpStream::connect(&addr).await?;
        let req = json!({"id": 1, "jsonrpc": "2.0", "method": method, "params": params});
        let line = format!("{}\n", req);
        stream.write_all(line.as_bytes()).await?;

        // read single line
        let mut reader = tokio::io::BufReader::new(stream);
        let mut resp_line = String::new();
        reader.read_line(&mut resp_line).await?;
        if resp_line.is_empty() {
            return Ok(Value::Null);
        }
        let v: Value = serde_json::from_str(&resp_line)?;
        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
    }
    let host = &cfg.host;
    let port = cfg.port_tls;
    // TLS setup
    let mut roots = RootCertStore::empty();
    roots.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject.as_ref().to_vec(),
            ta.subject_public_key_info.as_ref().to_vec(),
            ta.name_constraints.clone().map(|c| c.as_ref().to_vec()),
        )
    }));
    let cfg = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(cfg));

    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).await?;
    let dns = ServerName::try_from(host.as_str()).map_err(|_| anyhow!("dns name"))?;
    let mut tls = connector.connect(dns, stream).await?;

    // Send request
    let req = json!({"id": 1, "jsonrpc": "2.0", "method": method, "params": params});
    let line = format!("{}\n", req);
    tls.write_all(line.as_bytes()).await?;

    // Read single line response (Electrum replies line-by-line)
    let mut buf = Vec::with_capacity(4096);
    loop {
        let mut byte = [0u8; 1];
        if tls.read(&mut byte).await? == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        buf.push(byte[0]);
        if buf.len() > 1_000_000 {
            return Err(anyhow!("electrum response too big"));
        }
    }

    let v: Value = serde_json::from_slice(&buf)?;
    Ok(v.get("result").cloned().unwrap_or(Value::Null))
}

// HTTP fallback (per-chain)
async fn http_address_history(chain: &str, address: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let url = match chain {
        "BTC" => format!("https://blockstream.info/api/address/{address}/txs"),
        "BCH" => format!("https://api.fullstack.cash/v5/electrumx/transactions/{address}"), // example
        "DOGE" => format!("https://dogechain.info/api/v1/address/txs/{address}"),
        _ => return Err(anyhow!("unknown chain")),
    };
    let r = client.get(url).send().await?;
    let v = r.json::<Value>().await?;
    Ok(v)
}

// Utility: confirmation count from history entry
fn is_confirmed_for_btc_like(chain: &str, entry: &Value, conf_req: u64) -> Option<(String, u64)> {
    if entry.get("height").is_some() && entry.get("tx_hash").is_some() {
        let height = entry["height"].as_i64().unwrap_or(0);
        if height > 0 {
            let txid = entry["tx_hash"].as_str()?.to_string();
            // If an explorer returns a height for the tx, treat it as meeting
            // the required confirmations (use conf_req) so mocks which report
            // inclusion will immediately count.
            return Some((txid, conf_req));
        }
    }
    if let Some(true) = entry
        .get("status")
        .and_then(|s| s.get("confirmed"))
        .and_then(|b| b.as_bool())
    {
        let txid = entry
            .get("txid")
            .and_then(|x| x.as_str())
            .or_else(|| entry.get("tx_hash").and_then(|x| x.as_str()))?
            .to_string();
        return Some((txid, conf_req));
    }
    if chain == "DOGE" {
        if let Some(confs) = entry.get("confirmations").and_then(|c| c.as_u64()) {
            if confs >= conf_req {
                let txid = entry
                    .get("txid")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string();
                return Some((txid, confs));
            }
        }
    }
    None
}

// --- public API ---
pub async fn spawn_crypto_watchers() {
    let confs = load_chain_confs();
    for cc in confs {
        let cfg = cc.clone();
        tokio::spawn(async move { run_chain_watcher(cfg).await });
    }
}

async fn run_chain_watcher(cfg: ChainConf) {
    // initial immediate run so short-poll e2e tests don't miss the first cycle
    let poll_secs = std::env::var("VISION_WATCH_POLL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5u64);
    if let Err(e) = one_cycle(&cfg).await {
        warn!("{} watcher initial cycle error: {}", cfg.name, e);
    }

    // adaptive backoff starting from configured poll interval
    let mut backoff = poll_secs;
    loop {
        if let Err(e) = one_cycle(&cfg).await {
            warn!("{} watcher error: {}", cfg.name, e);
            backoff = (backoff * 2).min(300);
        } else {
            backoff = poll_secs;
        }
        sleep(Duration::from_secs(backoff)).await;
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct Listing {
    #[serde(alias = "listing_id")]
    id: String,
    seller_addr: String,
    qty_land: u64,
    price_amount: u64,
    price_chain: String,
    pay_to: String,
    status: String,
    created_at: i64,
}

async fn one_cycle(cfg: &ChainConf) -> Result<()> {
    let app_cfg = crate::config::get_app_cfg();
    info!("{} watcher cycle start", cfg.name);
    if app_cfg.mock_chain {
        // Instantly confirm listings that opt-in to mock/demo pay_to prefixes.
        let server_cb = app_cfg.confirm_callback_url.clone();
        // iterate sled tree directly like the normal path and confirm matching mock/demo pay_to
        let db = crate::market::cash_store::db_owned();
        // 1) check the structured tree if present
        if let Ok(tree) = db.open_tree("market_land_listings") {
            for (_k, val) in tree.iter().flatten() {
                if let Ok(l) = serde_json::from_slice::<Listing>(&val) {
                    if l.status == "open"
                        && (l.pay_to.starts_with("mock:") || l.pay_to.starts_with("demo:"))
                    {
                        let _ = reqwest::Client::new()
                                .post(&server_cb)
                                .json(&json!({ "listing_id": l.id, "txid": "mock-txid", "chain": cfg.name }))
                                .send().await;
                    }
                }
            }
        }
        // 2) also support legacy/root keys like "listing:<id>" for tests that seed the DB directly
        for (_, v) in db.scan_prefix("listing:").flatten() {
            if let Ok(l) = serde_json::from_slice::<Listing>(&v) {
                if l.status == "open"
                    && (l.pay_to.starts_with("mock:") || l.pay_to.starts_with("demo:"))
                {
                    let _ = reqwest::Client::new()
                        .post(&server_cb)
                        .json(
                            &json!({ "listing_id": l.id, "txid": "mock-txid", "chain": cfg.name }),
                        )
                        .send()
                        .await;
                }
            }
        }
        return Ok(());
    }
    // Prefer discovering open listings via the running server's HTTP API (this
    // helps e2e tests which create listings through the HTTP server). If the
    // server is not reachable or returns no listings, fall back to reading the
    // sled tree directly.
    let mut listings: Vec<Listing> = Vec::new();
    let server_base_env =
        std::env::var("VISION_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let client = reqwest::Client::new();

    // 1) Try the local server (127.0.0.1:8080) which is used by tests that
    // spawn the HTTP server in-process. If that returns no listings, fall back
    // to the server base provided by VISION_SERVER_URL.
    let local_list_url = "http://127.0.0.1:8080/market/land/listings".to_string();
    match client.get(&local_list_url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            info!(
                "fetched {} -> status={} body_len={}",
                local_list_url,
                status,
                text.len()
            );
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        if let (Some(id), Some(pay_to), Some(price_chain), Some(status)) = (
                            item.get("listing_id").and_then(|v| v.as_str()),
                            item.get("pay_to").and_then(|v| v.as_str()),
                            item.get("price_chain").and_then(|v| v.as_str()),
                            item.get("status").and_then(|v| v.as_str()),
                        ) {
                            let l = Listing {
                                id: id.to_string(),
                                seller_addr: item
                                    .get("seller_addr")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default()
                                    .to_string(),
                                qty_land: item
                                    .get("qty_land")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0),
                                price_amount: item
                                    .get("price_amount")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0),
                                price_chain: price_chain.to_string(),
                                pay_to: pay_to.to_string(),
                                status: status.to_string(),
                                created_at: item
                                    .get("created_at")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0),
                            };
                            listings.push(l);
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!("failed to GET listings from {}: {}", local_list_url, e);
        }
    }

    // 2) If local returned nothing, try the configured server base (VISION_SERVER_URL)
    if listings.is_empty() {
        let list_url = format!(
            "{}/market/land/listings",
            server_base_env.trim_end_matches('/')
        );
        match client.get(&list_url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                info!(
                    "fetched {} -> status={} body_len={}",
                    list_url,
                    status,
                    text.len()
                );
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            if let (Some(id), Some(pay_to), Some(price_chain), Some(status)) = (
                                item.get("listing_id").and_then(|v| v.as_str()),
                                item.get("pay_to").and_then(|v| v.as_str()),
                                item.get("price_chain").and_then(|v| v.as_str()),
                                item.get("status").and_then(|v| v.as_str()),
                            ) {
                                let l = Listing {
                                    id: id.to_string(),
                                    seller_addr: item
                                        .get("seller_addr")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or_default()
                                        .to_string(),
                                    qty_land: item
                                        .get("qty_land")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0),
                                    price_amount: item
                                        .get("price_amount")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0),
                                    price_chain: price_chain.to_string(),
                                    pay_to: pay_to.to_string(),
                                    status: status.to_string(),
                                    created_at: item
                                        .get("created_at")
                                        .and_then(|v| v.as_i64())
                                        .unwrap_or(0),
                                };
                                listings.push(l);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("failed to GET listings from {}: {}", list_url, e);
            }
        }
    }

    // fallback to direct sled tree iteration when HTTP did not yield listings
    if listings.is_empty() {
        let db = crate::market::cash_store::db_owned(); // reuse one handle
        let tree = db.open_tree("market_land_listings")?;
        for kv in tree.iter() {
            let (_k, val) = kv?;
            let l: Listing = serde_json::from_slice(&val)?;
            if l.status == "open" && l.price_chain.to_uppercase() == cfg.name {
                listings.push(l);
            }
        }
    }

    if listings.is_empty() {
        info!("{}: no open listings", cfg.name);
        return Ok(());
    }

    info!("{}: checking {} listings", cfg.name, listings.len());
    for l in &listings {
        info!("{} listing found id={} pay_to={}", cfg.name, l.id, l.pay_to);
    }

    for l in listings {
        info!(
            "{}: querying history for {} via electrum/http/scripthash",
            cfg.name, l.pay_to
        );
        // prefer scripthash if we can derive a P2PKH script for this address
        let sh_opt = address_to_script_any(cfg.name, &l.pay_to).map(|spk| scripthash_hex(&spk));
        info!(
            "{}: scripthash fast-path available={}",
            cfg.name,
            sh_opt.is_some()
        );

        let hist = if let Some(sh) = sh_opt.clone() {
            match electrum_call_tls(cfg, "blockchain.scripthash.get_history", json!([sh])).await {
                Ok(v) => v,
                Err(e) => {
                    warn_throttled(
                        format!("sh_fail:{}:{}", cfg.name, l.pay_to),
                        std::time::Duration::from_secs(15),
                        format!("{} scripthash get_history failed: {}", cfg.name, e),
                    );
                    // fallback to address.get_history
                    match electrum_call_tls(
                        cfg,
                        "blockchain.address.get_history",
                        json!([l.pay_to.clone()]),
                    )
                    .await
                    {
                        Ok(v) => v,
                        Err(e2) => {
                            warn_throttled(
                                format!("addr_fail:{}:{}", cfg.name, l.pay_to),
                                std::time::Duration::from_secs(15),
                                format!("{} address.get_history failed: {}", cfg.name, e2),
                            );
                            http_address_history(cfg.name, &l.pay_to)
                                .await
                                .unwrap_or(Value::Null)
                        }
                    }
                }
            }
        } else {
            match electrum_call_tls(
                cfg,
                "blockchain.address.get_history",
                json!([l.pay_to.clone()]),
            )
            .await
            {
                Ok(v) => v,
                Err(_) => http_address_history(cfg.name, &l.pay_to)
                    .await
                    .unwrap_or(Value::Null),
            }
        };
        // if history is null, log the event for debugging
        if hist.is_null() {
            warn_throttled(
                format!("parse_warn:{}:{}", cfg.name, l.pay_to),
                std::time::Duration::from_secs(10),
                format!("{}: no parseable history yet for {}", cfg.name, l.pay_to),
            );
        }
        // Log a short summary of the history result and the raw JSON (trimmed)
        let summary = if hist.is_null() {
            "<null>".to_string()
        } else {
            format!("{} entries", hist.as_array().map(|a| a.len()).unwrap_or(0))
        };
        info!(
            "{}: history response for {} -> {}",
            cfg.name, l.pay_to, summary
        );
        if !hist.is_null() {
            // Log a trimmed version to avoid noisy output
            let raw = serde_json::to_string(&hist).unwrap_or_default();
            let trimmed = if raw.len() > 800 {
                format!("{}...<truncated>", &raw[..800])
            } else {
                raw
            };
            info!("{}: history json: {}", cfg.name, trimmed);
        }

        let arr = if hist.is_array() {
            hist.as_array().cloned().unwrap_or_default()
        } else if cfg.name == "BTC" && hist.get("chain_stats").is_some() {
            vec![]
        } else if (cfg.name == "BCH" || cfg.name == "DOGE") && hist.get("transactions").is_some() {
            hist["transactions"].as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        };

        for entry in arr {
            if let Some((txid, confs)) = is_confirmed_for_btc_like(cfg.name, &entry, cfg.conf_req) {
                if confs >= cfg.conf_req {
                    info!("{} listing {} confirmed via tx {}", cfg.name, l.id, txid);
                    let server_base = std::env::var("VISION_SERVER_URL")
                        .unwrap_or("http://127.0.0.1:8080".into());
                    let url = format!("{}/_market/land/confirm", server_base.trim_end_matches('/'));
                    info!("posting confirmation to {}", url);
                    let client = reqwest::Client::new();
                    let test_payload = json!({ "listing_id": l.id, "txid": txid });
                    if let Ok(resp) = client.post(&url).json(&test_payload).send().await {
                        let status = resp.status();
                        // try to read text for debug
                        let txt = resp.text().await.unwrap_or_default();
                        info!(
                            "posted to test receiver {}, status={} body={}",
                            url, status, txt
                        );
                    } else {
                        warn!("failed to post to test receiver {}", url);
                    }

                    // Also POST to the local server endpoint (so the app itself sees the confirm
                    // and can mark the listing settled). Use the server-expected field names.
                    let local_url = "http://127.0.0.1:8080/_market/land/confirm".to_string();
                    let server_payload =
                        json!({ "listing_id": l.id, "observed_txid": txid, "chain": cfg.name });
                    info!("posting confirmation to local server {}", local_url);
                    if let Ok(resp2) = client.post(&local_url).json(&server_payload).send().await {
                        let status2 = resp2.status();
                        let txt2 = resp2.text().await.unwrap_or_default();
                        info!(
                            "posted to local server {}, status={} body={}",
                            local_url, status2, txt2
                        );
                    } else {
                        warn!("failed to post to local server {}", local_url);
                    }
                    break;
                } else {
                    info!(
                        "{} listing {} seen but {}<{}, waiting",
                        cfg.name, l.id, confs, cfg.conf_req
                    );
                }
            }
        }
    }

    Ok(())
}

// --- optional test hook ---
#[allow(dead_code)]
pub fn with_test_notify(_hook: fn()) {
    // intentionally left as a test/dev hook
}

// Exported test wrapper that allows integration tests to run a single watcher cycle
#[allow(dead_code)]
pub async fn one_cycle_for_test(cfg: ChainConf) -> anyhow::Result<()> {
    one_cycle(&cfg).await
}
