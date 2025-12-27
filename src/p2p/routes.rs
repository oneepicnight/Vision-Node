#![allow(dead_code)]
//! P2P HTTP Routes for Headers-First Sync

use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::sync::Arc;

use base64::Engine as _;

use super::orphans::{OrphanPool, SeenFilters};
use super::protocol::*;

/// Global orphan pool
static ORPHAN_POOL: Lazy<Arc<Mutex<OrphanPool>>> =
    Lazy::new(|| Arc::new(Mutex::new(OrphanPool::new(512))));

/// Global seen filters
static SEEN_FILTERS: Lazy<Arc<Mutex<SeenFilters>>> =
    Lazy::new(|| Arc::new(Mutex::new(SeenFilters::new(8192))));

/// Create P2P router
pub fn p2p_router() -> Router {
    Router::new()
        // Signed hello handshake (v2.7.0)
        .route("/p2p/hello", post(handle_hello))
        // Headers-first sync (Phase 1)
        .route("/p2p/announce", post(handle_announce))
        .route("/p2p/get_headers", post(handle_get_headers))
        .route("/p2p/headers", get(handle_headers_endpoint))
        .route("/p2p/get_blocks", post(handle_get_blocks))
        .route("/p2p/blocks", get(handle_blocks_endpoint))
        // INV/GETDATA protocol (Phase 2)
        .route("/p2p/inv", post(handle_inv))
        .route("/p2p/getdata", post(handle_getdata))
        // Compact blocks (Phase 2)
        .route("/p2p/compact_block", post(handle_compact_block))
        .route("/p2p/get_block_txs", post(handle_get_block_txs))
        // Direct tx/block send
        .route("/p2p/tx", post(handle_tx))
        .route("/p2p/block", post(handle_block))
    // Peer discovery removed - now handled by p2p/api.rs
}

/// Handle signed hello handshake
/// Verifies Ed25519 signature, checks timestamp/nonce freshness
async fn handle_hello(
    Json(body): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};
    use std::collections::HashSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Nonce cache (replay protection)
    static NONCE_CACHE: Lazy<Arc<Mutex<HashSet<String>>>> =
        Lazy::new(|| Arc::new(Mutex::new(HashSet::with_capacity(1024))));

    // Check if debug mode is enabled (allows unsigned hello)
    let debug_mode = std::env::var("VISION_P2P_DEBUG_ALLOW_ALL").is_ok();

    if debug_mode {
        eprintln!("⚠️ P2P DEBUG MODE: Accepting unsigned hello");
        return respond_hello();
    }

    // Extract required fields
    let from_node_id = match body.get("from_node_id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing from_node_id"
                })),
            )
        }
    };

    let pubkey_b64 = match body.get("pubkey_b64").and_then(|v| v.as_str()) {
        Some(pk) => pk,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing pubkey_b64"
                })),
            )
        }
    };

    let ts_unix = match body.get("ts_unix").and_then(|v| v.as_u64()) {
        Some(ts) => ts,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing ts_unix"
                })),
            )
        }
    };

    let nonce_hex = match body.get("nonce_hex").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing nonce_hex"
                })),
            )
        }
    };

    let signature_hex = match body.get("signature_hex").and_then(|v| v.as_str()) {
        Some(sig) => sig,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Missing signature_hex"
                })),
            )
        }
    };

    // 1. Check timestamp freshness (±120s window)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if ts_unix.abs_diff(now) > 120 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Timestamp out of range (±120s)",
                "server_time": now,
                "client_time": ts_unix
            })),
        );
    }

    // 2. Check nonce uniqueness (replay protection)
    {
        let mut cache = NONCE_CACHE.lock();
        if cache.contains(nonce_hex) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Nonce already used (replay attack?)"
                })),
            );
        }
        cache.insert(nonce_hex.to_string());

        // Keep cache size bounded (evict oldest if > 1024)
        if cache.len() > 1024 {
            // Simple strategy: clear half the cache
            let to_remove: Vec<String> = cache.iter().take(512).cloned().collect();
            for n in to_remove {
                cache.remove(&n);
            }
        }
    }

    // 3. Parse public key
    let pubkey_bytes = match base64::engine::general_purpose::STANDARD.decode(pubkey_b64) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid base64 pubkey"
                })),
            )
        }
    };

    if pubkey_bytes.len() != 32 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid pubkey length (expected 32 bytes)"
            })),
        );
    }

    let pubkey = match VerifyingKey::from_bytes(&pubkey_bytes) {
        Ok(pk) => pk,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid Ed25519 public key"
                })),
            )
        }
    };

    // 4. Verify node_id matches pubkey
    let derived_id = crate::identity::node_id_from_pubkey(&pubkey_bytes);
    if derived_id != from_node_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "node_id does not match pubkey",
                "expected": derived_id,
                "provided": from_node_id
            })),
        );
    }

    // 5. Parse signature
    let sig_bytes = match hex::decode(signature_hex) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid hex signature"
                })),
            )
        }
    };

    if sig_bytes.len() != 64 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid signature length (expected 64 bytes)"
            })),
        );
    }

    let signature = match Signature::try_from(&sig_bytes.try_into().map_err(|_| ()).unwrap()) {
        Ok(sig) => sig,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid Ed25519 signature"
                })),
            )
        }
    };

    // 6. Build canonical payload and verify signature
    let payload = format!("{}|{}|{}", from_node_id, ts_unix, nonce_hex);

    if let Err(_) = pubkey.verify(payload.as_bytes(), &signature) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Invalid signature"
            })),
        );
    }

    // ✅ Signature valid - respond with our identity
    respond_hello()
}

/// Build hello response with our node identity
fn respond_hello() -> (StatusCode, Json<serde_json::Value>) {
    use crate::identity::node_id::try_local_node_identity;

    // Get identity safely (might not be initialized yet)
    let (node_id, pubkey_b64) = if let Some(identity_arc) = try_local_node_identity() {
        let guard = identity_arc.read();
        (guard.node_id.clone(), guard.pubkey_b64.clone())
    } else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Node identity not initialized yet"
            })),
        );
    };

    let chain_guard = crate::CHAIN.lock();
    let genesis_hash = if !chain_guard.blocks.is_empty() {
        chain_guard.blocks[0].header.pow_hash.clone()
    } else {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    };
    let chain_height = chain_guard.current_height();
    drop(chain_guard);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "node_id": node_id,
            "pubkey_b64": pubkey_b64,
            "chain_id": "vision-mainnet",
            "genesis_hash": genesis_hash,
            "protocol_version": 1,
            "node_version": crate::vision_constants::VISION_VERSION,
            "is_anchor": false,
            "advertised_ip": None::<String>,
            "advertised_port": None::<u16>,
            "height": chain_height,
        })),
    )
}

/// Handle block announcement (lightweight tip notification)
async fn handle_announce(
    Json(announce): Json<AnnounceBlock>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check if already seen
    let mut filters = SEEN_FILTERS.lock();
    if filters.mark_header_seen(&announce.hash) {
        // Already seen, drop silently and increment dupe counter
        crate::PROM_P2P_DUPES_DROPPED.inc();
        return (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "duplicate" })),
        );
    }
    drop(filters);

    // Update P2P metrics
    crate::PROM_P2P_ANNOUNCES_RECEIVED.inc();

    // Enqueue for header/block sync (don't pull full block yet)
    // This is handled by the sync task
    eprintln!(
        "📡 Received block announcement: height={}, hash={}",
        announce.height,
        &announce.hash[..8]
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "queued" })),
    )
}

/// Handle headers request
async fn handle_get_headers(Json(req): Json<GetHeaders>) -> (StatusCode, Json<Headers>) {
    let g = crate::CHAIN.lock();

    // Find first common block from locator
    let mut start_height = 0;
    for locator_hash in &req.locator {
        // Find this hash in our chain
        if let Some(pos) = g
            .blocks
            .iter()
            .position(|b| &b.header.pow_hash == locator_hash)
        {
            start_height = pos + 1; // Start from next block
            break;
        }
    }

    // Collect headers starting from start_height
    let mut headers = Vec::new();
    let max_headers = req.max.min(2000);

    for i in start_height..g.blocks.len() {
        if headers.len() >= max_headers {
            break;
        }

        let block = &g.blocks[i];

        // Validate header before sending
        if let Err(e) = validate_header_for_send(block, i, &g) {
            eprintln!("⚠️ Skipping invalid header at height {}: {}", i, e);
            continue;
        }

        headers.push(LiteHeader::from_block(block));

        // Stop at requested hash
        if let Some(stop_hash) = &req.stop {
            if &block.header.pow_hash == stop_hash {
                break;
            }
        }
    }

    drop(g);

    eprintln!(
        "📤 Sending {} headers starting from height {}",
        headers.len(),
        start_height
    );

    // Update metrics
    crate::PROM_P2P_HEADERS_SENT.inc_by(headers.len() as u64);

    (StatusCode::OK, Json(Headers { headers }))
}

/// Validate header integrity for P2P transmission
fn validate_header_for_send(
    block: &crate::Block,
    height: usize,
    chain: &crate::Chain,
) -> Result<(), String> {
    // Check height monotonicity
    if block.header.number != height as u64 {
        return Err(format!(
            "Height mismatch: block says {}, position is {}",
            block.header.number, height
        ));
    }

    // Check parent linkage (except genesis)
    if height > 0 {
        if let Some(parent) = chain.blocks.get(height - 1) {
            if block.header.parent_hash != parent.header.pow_hash {
                return Err("Parent hash mismatch".to_string());
            }
        }
    }

    // Check timestamp sanity
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Not too far in future (allow 10s clock drift)
    if block.header.timestamp > now + 10 {
        return Err(format!(
            "Timestamp too far in future: {} vs now {}",
            block.header.timestamp, now
        ));
    }

    // Check against median of last 11 blocks
    if height >= 11 {
        let mut recent_times: Vec<u64> = chain.blocks[height.saturating_sub(11)..height]
            .iter()
            .map(|b| b.header.timestamp)
            .collect();
        recent_times.sort_unstable();
        let median = recent_times[recent_times.len() / 2];

        if block.header.timestamp < median {
            return Err(format!(
                "Timestamp below median: {} < {}",
                block.header.timestamp, median
            ));
        }
    }

    // Target bounds check (ensure difficulty is reasonable)
    if block.header.difficulty == 0 {
        return Err("Zero difficulty".to_string());
    }

    Ok(())
}

/// Placeholder endpoint (actual responses via POST /p2p/get_headers)
async fn handle_headers_endpoint() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "info": "Use POST /p2p/get_headers to request headers"
        })),
    )
}

/// Handle block batch request
async fn handle_get_blocks(Json(req): Json<GetBlocks>) -> (StatusCode, Json<Blocks>) {
    let g = crate::CHAIN.lock();
    let mut blocks = Vec::new();

    for hash in &req.hashes {
        // Find block by hash
        if let Some(block) = g.blocks.iter().find(|b| &b.header.pow_hash == hash) {
            // Serialize block to JSON then base64
            if let Ok(json) = serde_json::to_string(block) {
                use base64::{engine::general_purpose, Engine as _};
                let raw = general_purpose::STANDARD.encode(json.as_bytes());
                blocks.push(BlockEnvelope {
                    hash: hash.clone(),
                    raw,
                });
            }
        }
    }

    drop(g);

    eprintln!("📤 Sending {} blocks", blocks.len());

    // Update metrics
    crate::PROM_P2P_BLOCKS_SENT.inc_by(blocks.len() as u64);

    (StatusCode::OK, Json(Blocks { blocks }))
}

/// Placeholder endpoint
async fn handle_blocks_endpoint() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "info": "Use POST /p2p/get_blocks to request blocks"
        })),
    )
}

/// Helper: Build and send block announcement to peers
pub async fn announce_block_to_peers(block: &crate::Block) {
    let announce = AnnounceBlock {
        height: block.header.number,
        hash: block.header.pow_hash.clone(),
        prev: block.header.parent_hash.clone(),
    };

    let peers: Vec<String> = {
        let g = crate::CHAIN.lock();
        g.peers.iter().cloned().collect()
    };

    for peer in peers {
        let url = format!("{}/p2p/announce", peer.trim_end_matches('/'));
        let announce_clone = announce.clone();

        tokio::spawn(async move {
            let _ = crate::HTTP
                .post(&url)
                .json(&announce_clone)
                .timeout(std::time::Duration::from_secs(3))
                .send()
                .await;
        });
    }

    crate::PROM_P2P_ANNOUNCES_SENT.inc();
}

/// Helper: Fetch headers from peer
pub async fn fetch_headers_from_peer(
    peer_url: &str,
    locator: Vec<String>,
) -> Result<Vec<LiteHeader>, String> {
    let req = GetHeaders::new(locator);
    let url = format!("{}/p2p/get_headers", peer_url.trim_end_matches('/'));

    let start = std::time::Instant::now();
    let response = crate::HTTP
        .post(&url)
        .json(&req)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let rtt_ms = start.elapsed().as_millis() as f64;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let headers_resp: Headers = response
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;

    eprintln!(
        "📥 Received {} headers from {} (RTT: {:.0}ms)",
        headers_resp.headers.len(),
        peer_url,
        rtt_ms
    );

    crate::PROM_P2P_HEADERS_RECEIVED.inc_by(headers_resp.headers.len() as u64);

    Ok(headers_resp.headers)
}

/// Helper: Fetch blocks from peer (windowed)
pub async fn fetch_blocks_from_peer(
    peer_url: &str,
    hashes: Vec<String>,
) -> Result<Vec<BlockEnvelope>, String> {
    let req = GetBlocks {
        hashes: hashes.clone(),
    };
    let url = format!("{}/p2p/get_blocks", peer_url.trim_end_matches('/'));

    let start = std::time::Instant::now();
    let response = crate::HTTP
        .post(&url)
        .json(&req)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let rtt_ms = start.elapsed().as_millis() as f64;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let blocks_resp: Blocks = response
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;

    eprintln!(
        "📥 Received {} blocks from {} (RTT: {:.0}ms)",
        blocks_resp.blocks.len(),
        peer_url,
        rtt_ms
    );

    crate::PROM_P2P_BLOCKS_RECEIVED.inc_by(blocks_resp.blocks.len() as u64);

    Ok(blocks_resp.blocks)
}

// ============================================================================
// Phase 2: INV/GETDATA Protocol + Compact Blocks
// ============================================================================

/// Handle INV message (inventory announcement)
async fn handle_inv(
    Json(inv): Json<super::tx_relay::Inv>,
) -> (StatusCode, Json<serde_json::Value>) {
    use crate::PROM_TX_INV_RECEIVED;

    tracing::debug!(
        target: "p2p::inv",
        count = inv.objects.len(),
        "Received INV message"
    );

    // Track metrics for tx INV messages
    if inv
        .objects
        .iter()
        .any(|item| matches!(item.inv_type, super::tx_relay::InvType::Tx))
    {
        PROM_TX_INV_RECEIVED.inc();
    }

    // Get peer from request (in real implementation, extract from headers)
    let peer = "http://localhost:7000".to_string();

    // Process INV in background
    tokio::spawn(async move {
        super::tx_relay::handle_inv(peer, inv).await;
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "processing" })),
    )
}

/// Handle GETDATA request (request for specific inventory items)
async fn handle_getdata(
    Json(getdata): Json<super::tx_relay::GetData>,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::debug!(
        target: "p2p::getdata",
        count = getdata.objects.len(),
        "Received GETDATA request"
    );

    let peer = "http://localhost:7000".to_string();

    // Process GETDATA in background
    tokio::spawn(async move {
        super::tx_relay::handle_getdata(peer, getdata).await;
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "processing" })),
    )
}

/// Handle compact block reception (direct call from TCP P2P)
pub async fn handle_compact_block_direct(
    compact: super::compact::CompactBlock,
) -> Result<(), String> {
    let (status, response) = handle_compact_block(Json(compact)).await;

    if status.is_success() {
        Ok(())
    } else {
        let error = response.0["error"].as_str().unwrap_or("unknown error");
        let reason = response.0["reason"].as_str().unwrap_or("");
        if reason.is_empty() {
            Err(error.to_string())
        } else {
            Err(format!("{}: {}", error, reason))
        }
    }
}

/// Handle compact block reception (HTTP endpoint)
pub async fn handle_compact_block(
    Json(compact): Json<super::compact::CompactBlock>,
) -> (StatusCode, Json<serde_json::Value>) {
    tracing::info!(
        target: "p2p::compact",
        hash = %compact.header.hash,
        height = compact.header.height,
        parent_hash = %compact.header.prev,
        tx_count = compact.short_tx_ids.len(),
        prefilled = compact.prefilled_txs.len(),
        compact_size = compact.size_bytes(),
        "Received compact block from peer"
    );

    // Update metrics
    crate::PROM_COMPACT_BLOCKS_RECEIVED.inc();

    // Log current chain state for debugging
    let current_height = {
        let g = crate::CHAIN.lock();
        g.blocks.last().map(|b| b.header.number).unwrap_or(0)
    };

    let height_diff = (compact.header.height as i64) - (current_height as i64);

    tracing::debug!(
        target: "p2p::compact",
        current_height = current_height,
        incoming_height = compact.header.height,
        height_diff = height_diff,
        "Processing compact block relative to current chain"
    );

    // Try to reconstruct the block from mempool
    match super::mempool_sync::reconstruct_block(&compact) {
        super::mempool_sync::ReconstructResult::Complete(block) => {
            tracing::info!(
                target: "p2p::compact",
                hash = %block.header.pow_hash,
                "Successfully reconstructed block from compact representation"
            );

            crate::PROM_COMPACT_BLOCK_RECONSTRUCTIONS.inc();

            // Check if block extends current tip or requires reorg
            let current_tip = {
                let g = crate::CHAIN.lock();
                g.blocks.last().map(|b| b.header.pow_hash.clone())
            };

            match current_tip {
                Some(tip_hash) if block.header.parent_hash == tip_hash => {
                    // Block extends current tip
                    let mut g = crate::CHAIN.lock();
                    let new_height = block.header.number;

                    match crate::chain::accept::apply_block(&mut g, &block) {
                        Ok(()) => {
                            drop(g);

                            tracing::info!(
                                target: "p2p::compact",
                                height = new_height,
                                hash = %block.header.pow_hash,
                                "✅ Compact block accepted and integrated - chain extended"
                            );

                            eprintln!(
                                "✅ Received and integrated block #{} from peer via compact block",
                                new_height
                            );
                        }
                        Err(e) => {
                            drop(g);
                            tracing::error!(
                                target: "p2p::compact",
                                error = %e,
                                hash = %block.header.pow_hash,
                                "Block rejected by validation"
                            );
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({
                                    "status": "rejected",
                                    "error": e,
                                    "hash": block.header.pow_hash
                                })),
                            );
                        }
                    }

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "accepted",
                            "hash": block.header.pow_hash,
                            "extended_tip": true
                        })),
                    )
                }
                Some(_) => {
                    // Block doesn't extend tip - let apply_block handle reorg if needed
                    let mut g = crate::CHAIN.lock();
                    match crate::chain::accept::apply_block(&mut g, &block) {
                        Ok(()) => {
                            drop(g);

                            tracing::info!(
                                target: "p2p::compact",
                                hash = %block.header.pow_hash,
                                "Block accepted (may have triggered reorg)"
                            );

                            (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "status": "accepted",
                                    "hash": block.header.pow_hash
                                })),
                            )
                        }
                        Err(e) => {
                            drop(g);

                            // If block is not on heaviest chain, it will be stored in side_blocks
                            // and may trigger reorg later
                            tracing::debug!(
                                target: "p2p::compact",
                                error = %e,
                                hash = %block.header.pow_hash,
                                "Block not accepted (may be orphaned or invalid)"
                            );

                            (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "status": "deferred",
                                    "hash": block.header.pow_hash,
                                    "reason": e
                                })),
                            )
                        }
                    }
                }
                None => {
                    // Empty chain
                    let mut g = crate::CHAIN.lock();

                    match crate::chain::accept::apply_block(&mut g, &block) {
                        Ok(()) => {
                            drop(g);

                            (
                                StatusCode::OK,
                                Json(serde_json::json!({
                                    "status": "accepted",
                                    "hash": block.header.pow_hash
                                })),
                            )
                        }
                        Err(e) => {
                            drop(g);
                            tracing::error!(
                                target: "p2p::compact",
                                error = %e,
                                hash = %block.header.pow_hash,
                                "Block rejected by validation"
                            );
                            (
                                StatusCode::BAD_REQUEST,
                                Json(serde_json::json!({
                                    "status": "rejected",
                                    "error": e,
                                    "hash": block.header.pow_hash
                                })),
                            )
                        }
                    }
                }
            }
        }
        super::mempool_sync::ReconstructResult::NeedTxs(indices) => {
            tracing::debug!(
                target: "p2p::compact",
                missing_count = indices.len(),
                "Need to fetch missing transactions"
            );

            // Enterprise: Request missing TXs from peer via GetBlockTxns message
            // This will be handled by the compact block protocol
            let block_hash = compact.header.hash.clone();

            // Queue the missing TX request
            tokio::spawn(async move {
                if let Err(e) = request_missing_block_txs(block_hash, indices).await {
                    tracing::warn!(
                        target: "p2p::compact",
                        error = %e,
                        "Failed to request missing transactions"
                    );
                }
            });

            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({
                    "status": "need_txs",
                    "message": "Requesting missing transactions from peer"
                })),
            )
        }
        super::mempool_sync::ReconstructResult::Failed(err) => {
            tracing::warn!(
                target: "p2p::compact",
                error = %err,
                "Failed to reconstruct block"
            );

            crate::PROM_COMPACT_BLOCK_RECONSTRUCTION_FAILURES.inc();

            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "status": "failed",
                    "error": err
                })),
            )
        }
    }
}

/// Handle request for missing block transactions
async fn handle_get_block_txs(
    Json(req): Json<super::compact::GetBlockTxns>,
) -> (StatusCode, Json<super::compact::BlockTxns>) {
    tracing::debug!(
        target: "p2p::compact",
        block = %req.block_hash,
        count = req.tx_indices.len(),
        "Received GetBlockTxns request"
    );

    // Find the block
    let g = crate::CHAIN.lock();
    let block = g
        .blocks
        .iter()
        .find(|b| b.header.pow_hash == req.block_hash);

    if let Some(block) = block {
        // Extract requested transactions
        let mut txs = Vec::new();
        for &idx in &req.tx_indices {
            if idx < block.txs.len() {
                txs.push(block.txs[idx].clone());
            }
        }

        drop(g);

        tracing::debug!(
            target: "p2p::compact",
            found = txs.len(),
            "Sending missing transactions"
        );

        (
            StatusCode::OK,
            Json(super::compact::BlockTxns {
                block_hash: req.block_hash,
                txs,
            }),
        )
    } else {
        drop(g);

        tracing::warn!(
            target: "p2p::compact",
            block = %req.block_hash,
            "Block not found for GetBlockTxns"
        );

        (
            StatusCode::NOT_FOUND,
            Json(super::compact::BlockTxns {
                block_hash: req.block_hash,
                txs: vec![],
            }),
        )
    }
}

/// Handle direct transaction send
async fn handle_tx(Json(tx): Json<crate::Tx>) -> (StatusCode, Json<serde_json::Value>) {
    use crate::{tx_hash, verify_tx, CHAIN, PROM_TX_GOSSIP_DUPLICATES, PROM_TX_GOSSIP_RECEIVED};

    let tx_hash = hex::encode(tx_hash(&tx));

    let _span = tracing::info_span!(
        "p2p_handle_tx",
        tx_hash = %tx_hash,
        sender = %tx.sender_pubkey,
        module = %tx.module,
        method = %tx.method
    )
    .entered();

    tracing::debug!(
        target: "p2p::tx",
        tx_hash = %tx_hash,
        from = %tx.sender_pubkey,
        "Received transaction via gossip"
    );

    // Update metrics
    PROM_TX_GOSSIP_RECEIVED.inc();

    // Check if we already have this transaction
    let already_have = {
        let g = CHAIN.lock();
        g.seen_txs.contains_key(&tx_hash)
    };

    if already_have {
        tracing::debug!(
            target: "p2p::tx",
            tx_hash = %tx_hash,
            "Already have transaction, skipping"
        );
        PROM_TX_GOSSIP_DUPLICATES.inc();
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "duplicate",
                "tx_hash": tx_hash
            })),
        );
    }

    // Verify transaction signature
    if let Err(e) = verify_tx(&tx) {
        tracing::warn!(
            target: "p2p::tx",
            tx_hash = %tx_hash,
            error = ?e,
            "Invalid transaction signature"
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_signature"
            })),
        );
    }

    // Add to mempool
    let mut g = CHAIN.lock();

    // Mark as seen
    g.seen_txs.insert(tx_hash.clone(), ());

    // Determine lane based on tip
    let critical_threshold = 1000;
    if tx.tip >= critical_threshold {
        g.mempool_critical.push_back(tx.clone());
        tracing::debug!(
            target: "p2p::tx",
            tx_hash = %tx_hash,
            tip = tx.tip,
            "Added to critical mempool lane"
        );
    } else {
        g.mempool_bulk.push_back(tx.clone());
        tracing::debug!(
            target: "p2p::tx",
            tx_hash = %tx_hash,
            tip = tx.tip,
            "Added to bulk mempool lane"
        );
    }

    // Track timestamps
    g.mempool_ts.insert(tx_hash.clone(), crate::now_ts());

    // Track block height when transaction enters mempool
    let current_height = g.blocks.last().map(|b| b.header.number);
    if let Some(height) = current_height {
        g.mempool_height.insert(tx_hash.clone(), height);
    }

    drop(g);

    // Propagate to other peers (avoid re-announcing to sender)
    let tx_hash_for_propagation = tx_hash.clone();
    tokio::spawn(async move {
        announce_tx_to_peers(tx_hash_for_propagation).await;
    });

    tracing::info!(
        target: "p2p::tx",
        tx_hash = %tx_hash,
        "Transaction accepted and propagated"
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "accepted",
            "tx_hash": tx_hash
        })),
    )
}

/// Handle direct block send
async fn handle_block(Json(block): Json<crate::Block>) -> (StatusCode, Json<serde_json::Value>) {
    let _span = tracing::info_span!(
        "p2p_handle_block",
        block_hash = %block.header.pow_hash,
        block_height = block.header.number,
        tx_count = block.txs.len(),
        difficulty = block.header.difficulty
    )
    .entered();

    tracing::info!(
        target: "p2p::block",
        hash = %block.header.pow_hash,
        height = block.header.number,
        txs = block.txs.len(),
        "Received full block via P2P"
    );

    // Update metrics
    crate::PROM_P2P_BLOCKS_RECEIVED.inc();

    // Check if block extends current tip or requires reorg
    let current_tip = {
        let g = crate::CHAIN.lock();
        g.blocks.last().map(|b| b.header.pow_hash.clone())
    };

    match current_tip {
        Some(tip_hash) if block.header.parent_hash == tip_hash => {
            // Block extends current tip - normal case
            tracing::debug!(
                target: "p2p::block",
                hash = %block.header.pow_hash,
                "Block extends current tip"
            );

            let mut g = crate::CHAIN.lock();

            match crate::chain::accept::apply_block(&mut g, &block) {
                Ok(()) => {
                    drop(g);

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "accepted",
                            "hash": block.header.pow_hash,
                            "extended_tip": true
                        })),
                    )
                }
                Err(e) => {
                    drop(g);
                    tracing::warn!(
                        target: "p2p::block",
                        hash = %block.header.pow_hash,
                        error = %e,
                        "Block validation failed"
                    );
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "status": "rejected",
                            "error": e,
                            "hash": block.header.pow_hash
                        })),
                    )
                }
            }
        }
        Some(_) => {
            // Block doesn't extend tip - let apply_block handle reorg if needed
            tracing::debug!(
                target: "p2p::block",
                hash = %block.header.pow_hash,
                parent = %block.header.parent_hash,
                "Block does not extend tip - unified acceptance will handle reorg if needed"
            );

            let mut g = crate::CHAIN.lock();
            match crate::chain::accept::apply_block(&mut g, &block) {
                Ok(()) => {
                    drop(g);

                    tracing::info!(
                        target: "p2p::block",
                        hash = %block.header.pow_hash,
                        "Block accepted (may have triggered reorg)"
                    );

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "accepted",
                            "hash": block.header.pow_hash
                        })),
                    )
                }
                Err(e) => {
                    drop(g);

                    tracing::debug!(
                        target: "p2p::block",
                        error = %e,
                        hash = %block.header.pow_hash,
                        "Block not accepted (may be stored in side_blocks for future reorg)"
                    );

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "deferred",
                            "hash": block.header.pow_hash,
                            "reason": e
                        })),
                    )
                }
            }
        }
        None => {
            // Empty chain - this is genesis block
            tracing::info!(
                target: "p2p::block",
                hash = %block.header.pow_hash,
                "Received block for empty chain"
            );

            let mut g = crate::CHAIN.lock();

            match crate::chain::accept::apply_block(&mut g, &block) {
                Ok(()) => {
                    drop(g);

                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "accepted",
                            "hash": block.header.pow_hash,
                            "genesis": true
                        })),
                    )
                }
                Err(e) => {
                    drop(g);
                    tracing::error!(
                        target: "p2p::block",
                        error = %e,
                        hash = %block.header.pow_hash,
                        "Block rejected by validation"
                    );
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "status": "rejected",
                            "error": e,
                            "hash": block.header.pow_hash
                        })),
                    )
                }
            }
        }
    }
}

/// Send compact block to a specific peer via HTTP API
/// peer can be either HTTP address (host:7070) or P2P address (host:7072)
/// Normalize peer address to HTTP base URL (with scheme exactly once)
fn normalize_http_base(peer: &str) -> String {
    use std::net::SocketAddr;

    // If peer starts with http:// or https://, use it directly
    if peer.starts_with("http://") || peer.starts_with("https://") {
        return peer.trim_end_matches('/').to_string();
    }

    // Try parsing as socket address (ip:port format)
    if let Ok(addr) = peer.parse::<SocketAddr>() {
        let host = addr.ip().to_string();
        let port = addr.port();

        // Map P2P port 7072 to HTTP port 7070
        if port == 7072 {
            let memory = crate::CONSTELLATION_MEMORY.lock();
            if let Some(peer_mem) = memory.find_peer_by_ip(&host) {
                if let Some(http_port) = peer_mem.http_api_port {
                    return format!("http://{}:{}", host, http_port);
                }
            }
            return format!("http://{}:7070", host);
        }

        // Use port as-is
        return format!("http://{}:{}", host, port);
    }

    // Check if peer has a port (host:port without scheme)
    if peer.contains(':') {
        return format!("http://{}", peer);
    }

    // No port specified, add default 7070
    format!("http://{}:7070", peer)
}

pub async fn send_compact_block_to_peer(
    peer: &str,
    compact: &super::compact::CompactBlock,
) -> Result<(), String> {
    // Normalize peer address to HTTP base URL
    let base_url = normalize_http_base(peer);
    // Defensive: trim trailing slash to prevent double slashes in URL
    let url = format!("{}/p2p/compact_block", base_url.trim_end_matches('/'));

    tracing::debug!(
        target: "p2p::compact",
        peer = %peer,
        base_url = %base_url,
        url = %url,
        hash = %compact.header.hash,
        height = compact.header.height,
        "Attempting to send compact block via HTTP API"
    );

    let response = crate::HTTP
        .post(&url)
        .json(compact)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| {
            // Extract detailed error information
            let error_details = if e.is_timeout() {
                "Connection timeout after 5s".to_string()
            } else if e.is_connect() {
                format!("Connection failed: {}", e)
            } else if e.is_request() {
                format!("Request error: {}", e)
            } else {
                format!("Network error: {}", e)
            };

            tracing::error!(
                target: "p2p::compact",
                peer = %peer,
                url = %url,
                error = %error_details,
                "Failed to send compact block - detailed error"
            );

            error_details
        })?;

    if response.status().is_success() {
        crate::PROM_COMPACT_BLOCKS_SENT.inc();
        tracing::info!(
            target: "p2p::compact",
            peer = %peer,
            hash = %compact.header.hash,
            height = compact.header.height,
            "Successfully sent compact block"
        );
        Ok(())
    } else {
        let status = response.status();
        let error_msg = format!("Peer returned error status: {}", status);

        tracing::error!(
            target: "p2p::compact",
            peer = %peer,
            status = %status,
            "Peer rejected compact block"
        );

        Err(error_msg)
    }
}

/// Announce compact block to all peers (TCP P2P version)
pub async fn announce_compact_block_to_peers(block: &crate::Block) {
    // Generate compact block
    let compact = super::compact::CompactBlock::from_block_auto(block);

    tracing::info!(
        target: "p2p::compact",
        hash = %block.header.pow_hash,
        height = block.header.number,
        compact_size = compact.size_bytes(),
        "Announcing compact block to TCP peers"
    );

    // Use TCP P2P manager to broadcast
    let msg = super::connection::P2PMessage::CompactBlock {
        compact: compact.clone(),
    };
    let (success, failure) = crate::P2P_MANAGER.broadcast_message(msg).await;

    if success > 0 || failure > 0 {
        tracing::info!(
            target: "p2p::compact",
            hash = %block.header.pow_hash,
            height = block.header.number,
            success = success,
            failure = failure,
            "Compact block announcement completed"
        );
    } else {
        tracing::debug!(
            target: "p2p::compact",
            hash = %block.header.pow_hash,
            height = block.header.number,
            "No TCP peers connected - skipping announcement"
        );
    }

    // Legacy HTTP fallback for backward compatibility
    let http_peers: Vec<String> = {
        let g = crate::CHAIN.lock();
        g.peers.iter().cloned().collect()
    };

    if !http_peers.is_empty() {
        tracing::debug!(
            target: "p2p::compact",
            peer_count = http_peers.len(),
            "Also attempting HTTP fallback to legacy peers"
        );

        for peer in http_peers {
            let compact_clone = compact.clone();
            let peer_clone = peer.clone();

            tokio::spawn(async move {
                match send_compact_block_to_peer(&peer_clone, &compact_clone).await {
                    Ok(()) => {
                        tracing::debug!(
                            target: "p2p::compact",
                            peer = %peer_clone,
                            "HTTP fallback: Compact block sent successfully"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            target: "p2p::compact",
                            peer = %peer_clone,
                            error = %e,
                            height = compact_clone.header.height,
                            hash = %compact_clone.header.hash,
                            "HTTP fallback: Failed to send compact block"
                        );
                    }
                }
            });
        }
    }
}

/// Announce transaction to all peers via INV
pub async fn announce_tx_to_peers(tx_hash: String) {
    use crate::{CHAIN, PROM_TX_INV_SENT};

    let peers: Vec<String> = {
        let g = CHAIN.lock();
        g.peers.iter().cloned().collect()
    };

    if peers.is_empty() {
        tracing::debug!(
            target: "p2p::tx",
            "No peers to announce tx to"
        );
        return;
    }

    tracing::info!(
        target: "p2p::tx",
        tx_hash = %tx_hash,
        peer_count = peers.len(),
        "Announcing transaction to peers"
    );

    let inv = super::tx_relay::Inv {
        objects: vec![super::tx_relay::InventoryItem {
            inv_type: super::tx_relay::InvType::Tx,
            hash: tx_hash,
        }],
    };

    // Track INV sent metric
    PROM_TX_INV_SENT.inc();

    // Spawn tasks to announce to all peers in parallel
    let mut tasks = Vec::new();
    for peer in peers {
        let inv_clone = inv.clone();
        let peer_clone = peer.clone();

        let task = tokio::spawn(async move {
            if let Err(e) =
                super::tx_relay::announce_to_peers(vec![peer_clone.clone()], inv_clone).await
            {
                tracing::warn!(
                    target: "p2p::tx",
                    peer = %peer_clone,
                    error = %e,
                    "Failed to send tx INV"
                );
            }
        });

        tasks.push(task);
    }

    // Wait for all sends to complete
    for task in tasks {
        let _ = task.await;
    }
}

/// Get access to orphan pool
pub fn orphan_pool() -> Arc<Mutex<OrphanPool>> {
    ORPHAN_POOL.clone()
}

/// Get access to seen filters
pub fn seen_filters() -> Arc<Mutex<SeenFilters>> {
    SEEN_FILTERS.clone()
}

/// Handle peer list request for discovery
async fn handle_get_peers() -> (StatusCode, Json<serde_json::Value>) {
    // Return list of known peers from both TCP connections and persisted database
    let tcp_peers = crate::P2P_MANAGER.get_peer_addresses().await;
    let persisted_peers = super::connection::P2PConnectionManager::load_persisted_peers();

    // Combine and deduplicate
    let mut all_peers: Vec<String> = tcp_peers;
    for peer in persisted_peers {
        if !all_peers.contains(&peer) {
            all_peers.push(peer);
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "peers": all_peers,
            "count": all_peers.len()
        })),
    )
}

// ==================== ENTERPRISE P2P VALIDATION HELPERS ====================

// validate_block_before_accept removed - validation now handled by crate::chain::accept::apply_block

// Strict PoW hash parsing (accept optional 0x prefix, require 32 bytes)
fn parse_pow_hash_32(pow_hash: &str) -> Result<[u8; 32], String> {
    let s = pow_hash.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != 64 {
        return Err(format!(
            "Invalid PoW hash length: expected 64 hex chars (32 bytes), got {}",
            s.len()
        ));
    }
    let bytes = hex::decode(s).map_err(|_| "Invalid PoW hash format".to_string())?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Request missing transactions for compact block reconstruction
async fn request_missing_block_txs(block_hash: String, indices: Vec<usize>) -> Result<(), String> {
    use tracing::debug;

    debug!(
        block_hash = %block_hash,
        missing_count = indices.len(),
        "Requesting missing transactions for compact block"
    );

    // This would send a GetBlockTxns message to the peer that sent the compact block
    // For now, log the request - full implementation would track peer source
    // and send appropriate P2P message

    Ok(())
}

