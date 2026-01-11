//! Anchor Client - HTTP-based Truth from Backbone Nodes
//!
//! Queries anchor nodes over HTTP (port 7070) to get canonical network state.
//! This decouples mining eligibility from P2P peer gossip chaos.

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Remote status from an anchor node via HTTP
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemoteStatus {
    pub height: u64,
    pub tip_hash: String,
    pub chain_id: String,
    pub genesis_hash: String,
}

/// Fetch status from a single anchor via HTTP
///
/// # Arguments
/// * `base_url` - HTTP base URL like "http://16.163.123.221:7070"
///
/// # Returns
/// * `Ok(RemoteStatus)` - Anchor responded with valid status
/// * `Err(_)` - Network error, timeout, or invalid response
pub async fn fetch_anchor_status(base_url: &str) -> anyhow::Result<RemoteStatus> {
    let url = format!("{}/api/status", base_url);

    debug!("[ANCHOR_CLIENT] Querying {}", url);

    // Use shorter timeout for anchor queries (don't block mining checks)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()?;

    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!("Anchor returned status {}", resp.status()));
    }

    // Parse /api/status response
    let status_value: serde_json::Value = resp.json().await?;

    // Extract fields (adapt to your actual /api/status schema)
    let height = status_value
        .get("height")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("Missing height field"))?;

    let tip_hash = status_value
        .get("tip_hash")
        .or_else(|| status_value.get("latest_block_hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let chain_id = status_value
        .get("chain_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let genesis_hash = status_value
        .get("genesis_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    debug!(
        "[ANCHOR_CLIENT] ✅ {} responded: height={}, chain_id={}",
        base_url, height, chain_id
    );

    Ok(RemoteStatus {
        height,
        tip_hash,
        chain_id,
        genesis_hash,
    })
}

/// Query multiple anchors and return consensus view
///
/// # Arguments
/// * `anchor_seeds` - List of anchor hosts/IPs (HTTP truth sources on port 7070)
/// * `our_chain_id` - Expected chain ID to filter anchors
/// * `our_genesis_hash` - Expected genesis hash to filter anchors
///
/// # Returns
/// * `(network_height, network_hash, anchors_sampled)` - Consensus from anchors
pub async fn query_anchor_consensus(
    anchor_seeds: &[String],
    our_chain_id: &str,
    our_genesis_hash: &str,
) -> (u64, String, usize) {
    if anchor_seeds.is_empty() {
        debug!("[ANCHOR_CLIENT] No anchor seeds configured");
        return (0, String::new(), 0);
    }

    info!(
        "[ANCHOR_CLIENT] 🛰️ Querying {} anchors for network consensus",
        anchor_seeds.len()
    );

    let mut valid_responses = Vec::new();

    // Query up to 3 anchors (don't spam all if we have many)
    let anchors_to_query = anchor_seeds.iter().take(3);

    for seed in anchors_to_query {
        // Parse "ip:port" -> "http://ip:7070"
        let parts: Vec<&str> = seed.split(':').collect();
        if parts.is_empty() {
            continue;
        }

        let ip = parts[0];
        let base_url = format!("http://{}:7070", ip);

        match fetch_anchor_status(&base_url).await {
            Ok(status) => {
                // Filter: only accept anchors on same chain
                if status.chain_id == our_chain_id && status.genesis_hash == our_genesis_hash {
                    debug!(
                        "[ANCHOR_CLIENT] ✅ Anchor {} validated: height={}",
                        ip, status.height
                    );
                    valid_responses.push(status);
                } else {
                    warn!(
                        "[ANCHOR_CLIENT] ⚠️ Anchor {} on different chain (chain_id={}, expected={})",
                        ip, status.chain_id, our_chain_id
                    );
                }
            }
            Err(e) => {
                debug!("[ANCHOR_CLIENT] ❌ Anchor {} failed: {}", ip, e);
            }
        }
    }

    if valid_responses.is_empty() {
        warn!("[ANCHOR_CLIENT] ⚠️ No anchors responded - using fallback");
        return (0, String::new(), 0);
    }

    // Consensus = highest height from valid anchors
    let max_height = valid_responses.iter().map(|r| r.height).max().unwrap_or(0);

    // Get tip hash from anchor(s) at max height
    let tip_hash = valid_responses
        .iter()
        .find(|r| r.height == max_height)
        .map(|r| r.tip_hash.clone())
        .unwrap_or_default();

    info!(
        "[ANCHOR_CLIENT] 🎯 Network consensus from {} anchors: height={}, tip_hash={}",
        valid_responses.len(),
        max_height,
        &tip_hash[..tip_hash.len().min(16)]
    );

    (max_height, tip_hash, valid_responses.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_anchor_seed() {
        let seed = "16.163.123.221";
        let parts: Vec<&str> = seed.split(':').collect();
        assert_eq!(parts[0], "16.163.123.221");
    }
}
