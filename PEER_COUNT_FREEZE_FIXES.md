# Peer Count & Freeze Fixes - Vision Node v3.0.0

**Problem**: Node showed "Peers: 0" despite successful connections, mining thought it was alone, froze at "Integrating block #10..."

**Root Cause**: Mining queried wrong peer source (PeerManager state vs live TCP connections)

---

## âœ… Fix 1: Peer Count Truth Source

**File**: `src/p2p/connection.rs`

Added methods to `P2PConnectionManager` to expose live connection data:

```rust
/// Fix 1: Get live connected peer addresses (truth source for peer count)
pub async fn connected_peer_addrs(&self) -> Vec<String> {
    let peers = self.peers.lock().await;
    peers.keys().cloned().collect()
}

/// Fix 1: Get live connected peer count (truth source)
pub async fn connected_peer_count(&self) -> usize {
    let peers = self.peers.lock().await;
    peers.len()
}
```

**File**: `src/p2p/peer_manager.rs`

Updated `PeerManager::connected_peer_count()` to delegate to live connection manager:

```rust
/// Fix 1: Get count of connected peers from live connection map (truth source)
pub async fn connected_peer_count(&self) -> usize {
    // Use live P2P connection manager as truth source
    crate::P2P_MANAGER.connected_peer_count().await
}
```

**Impact**: 
- âœ… Peer count now shows actual live TCP connections
- âœ… Telemetry reflects reality

---

## âœ… Fix 2: Mining Pool from Live Connections

**File**: `src/p2p/connection.rs`

Added method to get mining-eligible peer IDs from live connections:

```rust
/// Fix 2: Get live connected peer IDs for mining pool
/// Returns peer addresses (can be enhanced with node_id mapping later)
pub async fn connected_peer_ids(&self) -> Vec<String> {
    let peers = self.peers.lock().await;
    // For now, use normalized address as ID (IP:7072)
    // This is deterministic across all nodes
    peers.keys().map(|addr| {
        // Normalize to IP:7072 format for consistency
        if let Some(ip) = addr.split(':').next() {
            format!("{}:7072", ip)
        } else {
            addr.clone()
        }
    }).collect()
}
```

**File**: `src/main.rs`

Updated mining producer to query live connections instead of PeerManager:

**BEFORE**:
```rust
let mut pool = PEER_MANAGER.eligible_mining_id_pool().await;  // âŒ Stale state
```

**AFTER**:
```rust
// Fix 2: Get eligible pool from LIVE connected peers (truth source)
let mut pool = crate::P2P_MANAGER.connected_peer_ids().await;
pool.push(my_node_id.clone());
pool.sort();
pool.dedup();
```

**Impact**: 
- âœ… Mining sees actual connected peers
- âœ… Won't propose blocks when alone
- âœ… No more "pool size 1" hallucinations

---

## âœ… Fix 3: Broadcast Timeout (Freeze Prevention)

**File**: `src/main.rs`

Wrapped block broadcast in 10-second timeout:

**BEFORE**:
```rust
let (success, failure) = crate::P2P_MANAGER.broadcast_message(msg).await;
```

**AFTER**:
```rust
// Fix 3A: Broadcast block via P2P with timeout (async, no locks)
let broadcast_result = tokio::time::timeout(
    Duration::from_secs(10),
    crate::P2P_MANAGER.broadcast_message(msg)
).await;

match broadcast_result {
    Ok((success, failure)) => {
        tracing::info!(target: "mining",
            "[MINING] ðŸ“¡ Block {} broadcast: {} success, {} failed",
            height, success, failure
        );
    }
    Err(_) => {
        tracing::warn!(target: "mining",
            "[MINING] âš ï¸ Block {} broadcast timeout after 10s",
            height
        );
    }
}
```

**Lock Audit (Fix 3B)**:
- âœ… Verified no `.await` inside CHAIN lock blocks
- âœ… Lock acquired twice, dropped before broadcast
- âœ… All async operations happen unlocked

**Impact**: 
- âœ… Won't freeze on stuck sends
- âœ… Continues producing blocks even if some peers unresponsive
- âœ… 10-second upper bound on broadcast delay

---

## âœ… Fix 4: Gossip Merge (Already Correct)

**File**: `src/p2p/peer_gossip.rs`

Audit confirmed gossip already properly merges peer lists:

```rust
// Build existing peer set
let mut existing_ids: HashSet<String> = peer_store
    .get_all()
    .into_iter()
    .map(|p| p.node_id)
    .collect();

for peer_info in capped_peers {
    // Skip if we already know this peer
    if existing_ids.contains(&peer_info.node_id) {
        continue;  // âœ… Merges, doesn't replace
    }
    
    // ... add new peer ...
    existing_ids.insert(peer_info.node_id.clone());
}
```

**Impact**: 
- âœ… No peer list overwrites
- âœ… Gossip properly accumulates peers

---

## Summary

| Fix | Description | Status |
|-----|-------------|--------|
| **1** | Peer count from live TCP connections | âœ… FIXED |
| **2** | Mining pool from live connections | âœ… FIXED |
| **3A** | Broadcast timeout (10s) | âœ… FIXED |
| **3B** | No `.await` while locked | âœ… VERIFIED |
| **4** | Gossip merge (not replace) | âœ… ALREADY CORRECT |

---

## Expected Behavior After Fixes

**Before**:
```
[INFO] âœ… Connected to peer 192.168.1.100:7072
[INFO] Peers: 0
[WARN] [MINING] Pool size 1 < 2, skipping proposal (waiting for peers)
[INFO] [MINING] ðŸŽ¯ Won slot 0! Proposing block #10...
[INFO] Integrating block #10...
<FREEZE>
```

**After**:
```
[INFO] âœ… Connected to peer 192.168.1.100:7072
[INFO] Peers: 1
[DEBUG] [MINING] Pool built from 1 live connected peers
[INFO] [MINING] Pool size 2 (me + peer)
[INFO] [MINING] ðŸŽ¯ Won slot 0! Proposing block #10...
[INFO] [MINING] âœ… Block 10 created: 0 txs, hash: abc123...
[INFO] [MINING] ðŸ“¡ Block 10 broadcast: 1 success, 0 failed
```

---

## Testing Commands

```powershell
# Start node
.\target\release\vision-node.exe

# Watch for correct peer count
# Should show "Peers: N" matching actual connections

# Watch mining pool building
# Should see "Pool built from N live connected peers"

# Watch block broadcast
# Should see "Block N broadcast: X success, Y failed"
# OR "Block N broadcast timeout after 10s" (if peers frozen)
```

---

## Technical Notes

### Peer Count Architecture

**OLD (WRONG)**:
```
Mining â†’ PeerManager.eligible_mining_id_pool()
         â†“
    PeerManager state (stale)
```

**NEW (CORRECT)**:
```
Mining â†’ P2PConnectionManager.connected_peer_ids()
         â†“
    Live TCP connection map (truth source)
```

### Determinism

The mining pool uses normalized addresses (`IP:7072`) to ensure:
- âœ… All nodes build identical pool from same connections
- âœ… Same winner selected by all nodes
- âœ… Deduplication works correctly

### Freeze Prevention

1. **Send timeout**: 5 seconds per message (already existed)
2. **Broadcast timeout**: 10 seconds total (new)
3. **No lock during I/O**: All `.await` happens unlocked (verified)

---

## Version Info

- **Vision Node**: v3.0.0 MINING TESTNET
- **Date**: 2024
- **Build**: Release (optimized)
- **Compile Status**: âœ… SUCCESS (27 warnings, normal)

