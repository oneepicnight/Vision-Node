//! Parent block fetching for orphan resolution
//!
//! When an orphan is stored, actively request its parent from peers over P2P.

use crate::*;

/// Check if we can send a parent request (rate limiting)
/// Returns true if request is allowed, false if rate limited
pub fn can_request_parent(g: &mut Chain, peer: &str, parent_hash: &str) -> bool {
    const MAX_REQUESTS_PER_PEER: usize = 10;
    const RATE_LIMIT_WINDOW_SECS: u64 = 60;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Clean up old entries for this peer
    if let Some(requests) = g.parent_request_limiter.get_mut(peer) {
        requests.retain(|(_, ts)| now.saturating_sub(*ts) < RATE_LIMIT_WINDOW_SECS);
        
        // Check if we're at the limit
        if requests.len() >= MAX_REQUESTS_PER_PEER {
            tracing::warn!(
                peer = %peer,
                parent_hash = %parent_hash,
                requests_in_window = requests.len(),
                "[PARENT-FETCH] rate limited - too many requests"
            );
            return false;
        }
        
        // Check if we already requested this specific parent from this peer recently
        if requests.iter().any(|(hash, _)| hash == parent_hash) {
            tracing::debug!(
                peer = %peer,
                parent_hash = %parent_hash,
                "[PARENT-FETCH] already requested from this peer recently"
            );
            return false;
        }
        
        // Add this request
        requests.push((parent_hash.to_string(), now));
    } else {
        // First request from this peer
        g.parent_request_limiter.insert(
            peer.to_string(),
            vec![(parent_hash.to_string(), now)],
        );
    }
    
    true
}

/// Request parent block from a peer over P2P
async fn request_parent_over_p2p(
    parent_hash: &str,
    parent_height: u64,
    peers: &[String],
) -> Result<(), String> {
    use crate::p2p::connection::P2PMessage;
    
    if peers.is_empty() {
        return Err("no peers available".to_string());
    }
    
    let n_peers = peers.len().min(5);
    
    tracing::info!(
        parent_height = parent_height,
        expected_parent = %parent_hash,
        from_peers = n_peers,
        "[PARENT-FETCH] p2p requesting parent"
    );
    
    // Send GetBlocks request to up to 5 peers
    let mut sent_count = 0;
    for peer in peers.iter().take(5) {
        let msg = P2PMessage::GetBlocks {
            start_height: parent_height,
            end_height: parent_height,
        };
        
        match crate::P2P_MANAGER.send_to_peer(peer, msg).await {
            Ok(()) => {
                sent_count += 1;
                tracing::debug!(
                    peer = %peer,
                    parent_height = parent_height,
                    "[PARENT-FETCH] sent GetBlocks request"
                );
            }
            Err(e) => {
                tracing::debug!(
                    peer = %peer,
                    error = %e,
                    "[PARENT-FETCH] failed to send GetBlocks"
                );
            }
        }
    }
    
    if sent_count > 0 {
        Ok(())
    } else {
        Err("failed to send to any peer".to_string())
    }
}

/// Fetch parent for an orphan block
/// Uses P2P GetBlocks to request the parent from connected peers
/// FIX #3: Detects forks (parent height exists but hash mismatch) to stop spam
pub async fn fetch_parent_for_orphan(
    parent_hash: String,
    orphan_height: u64,
    source_peer: String,
) {
    // Calculate parent height
    let parent_height = orphan_height.saturating_sub(1);
    
    // FIX #3: Fork detection - if we have a block at parent_height but hash doesn't match,
    // we're on a different fork. Stop parent-fetch spam and let sync handle it.
    let fork_detected = {
        let g = crate::CHAIN.lock();
        if let Some(our_block_at_height) = g.blocks.get(parent_height as usize) {
            let our_hash = crate::canon_hash(&our_block_at_height.header.pow_hash);
            if our_hash != parent_hash {
                tracing::warn!(
                    orphan_height = orphan_height,
                    parent_height = parent_height,
                    expected_parent = %parent_hash,
                    our_hash_at_height = %our_hash,
                    "[PARENT-FETCH] FORK DETECTED: parent height exists but hash mismatch - switching to sync"
                );
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    
    if fork_detected {
        // Don't spam parent requests for forked chains
        // Let the normal sync process handle catching up to the right fork
        tracing::info!(
            "[PARENT-FETCH] Stopping parent-fetch spam - fork detected, use sync instead"
        );
        return;
    }
    
    // Get list of connected peers
    let connected_peers: Vec<String> = {
        let g = crate::CHAIN.lock();
        g.peers.iter().cloned().collect()
    };
    
    // Try source peer first if it's not "unknown"
    let peers_to_try: Vec<String> = if source_peer != "unknown" && source_peer != "local_miner" {
        let mut peers = vec![source_peer.clone()];
        // Add up to 4 other random peers as fallbacks
        for peer in connected_peers.iter().take(4) {
            if peer != &source_peer {
                peers.push(peer.clone());
            }
        }
        peers
    } else {
        // Source unknown, try first 5 connected peers
        connected_peers.into_iter().take(5).collect()
    };
    
    if peers_to_try.is_empty() {
        tracing::warn!(
            parent_hash = %parent_hash,
            "[PARENT-FETCH] no peers available to request parent"
        );
        return;
    }
    
    // Increment sent counter
    crate::PROM_P2P_PARENT_REQUEST_SENT.inc();
    
    // Request via P2P
    match request_parent_over_p2p(&parent_hash, parent_height, &peers_to_try).await {
        Ok(()) => {
            tracing::info!(
                parent_hash = %parent_hash,
                parent_height = parent_height,
                peers_contacted = peers_to_try.len().min(5),
                "[PARENT-FETCH] ✅ sent P2P parent requests"
            );
        }
        Err(e) => {
            tracing::warn!(
                parent_hash = %parent_hash,
                error = %e,
                "[PARENT-FETCH] ❌ failed to send P2P parent requests"
            );
            crate::PROM_P2P_PARENT_REQUEST_FAILED.inc();
        }
    }
}

