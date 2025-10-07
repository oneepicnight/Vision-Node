
// auto_sync.rs
// Background auto-sync task that periodically calls this node's /sync/pull
// to fetch from known peers. Keeps chain caught up and lets reorg logic kick in
// inside the existing sync implementation.
//
// Drop this file at: src/auto_sync.rs
// Add to main.rs: `mod auto_sync;` (top) and `auto_sync::start_autosync();` (in main()).

use std::time::Duration;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::json;

static HTTP: Lazy<Client> = Lazy::new(|| Client::new());

fn autosync_secs() -> u64 {
    std::env::var("VISION_AUTOSYNC_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(3)
}

fn local_port() -> u16 {
    std::env::var("VISION_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(7070)
}

/// Start the background autosync loop.
pub fn start_autosync() {
    tokio::spawn(async move {
        let my_port = local_port();
        let url = format!("http://127.0.0.1:{}/sync/pull", my_port);

        loop {
            // Snapshot peers under lock-less HTTP by calling our own /peers endpoint.
            // We use HTTP rather than referencing crate internals to keep this module decoupled.
            let peers_url = format!("http://127.0.0.1:{}/peers", my_port);
            let peers: Vec<String> = match HTTP.get(&peers_url).send().await {
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(v) => v.get("peers")
                              .and_then(|x| x.as_array())
                              .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|t| t.to_string())).collect())
                              .unwrap_or_else(|| vec![]),
                    Err(_) => vec![],
                },
                Err(_) => vec![],
            };

            // Try to pull from each peer; /sync/pull figures out ranges & reorgs.
            for src in peers {
                let body = json!({ "src": src });
                let _ = HTTP.post(&url).json(&body).send().await;
            }

            tokio::time::sleep(Duration::from_secs(autosync_secs())).await;
        }
    });
}
