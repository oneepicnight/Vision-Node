# Sync & Mining Gate Audit - v1.0.1

## Current Implementation Status

### SYNC GATE (Auto-Sync)
**Location**: [src/auto_sync.rs#L491-L510](src/auto_sync.rs#L491-L510)

**Current Rule**: Requires **3 peers minimum** to start syncing
```rust
pub const MIN_PEERS_FOR_SYNC: u32 = 3;

if connected_peers < MIN_PEERS_FOR_SYNC as usize {
    tracing::debug!(
        "[SYNC-GATE] Waiting for minimum peers: have={} need={}",
        connected_peers, MIN_PEERS_FOR_SYNC
    );
    return Ok(());
}
```

**What It Checks**:
1. ✅ **Peer count**: Must have 3+ connected peers
2. ❌ **NOT CHECKED**: Whether peers are on same chain/height
3. ❌ **NOT CHECKED**: Consensus quorum (majority agreement)

**Issue**: The sync gate only checks RAW peer count, not if they agree on chain state!

---

### MINING GATE (Mining Readiness)
**Location**: [src/mining_readiness.rs#L40-L149](src/mining_readiness.rs#L40-L149)

**Current Rules**:
```rust
pub const MAINNET_MIN_PEERS_FOR_MINING: u32 = 3;
const MAX_DESYNC_FOR_MINING: u64 = 5;
const ACTIVE_SYNC_THRESHOLD: u64 = 2;
```

**Gate Checks** (in order):
1. ✅ **Wallet configured**: `MINER_ADDRESS` must be set
2. ✅ **Peer count**: 3+ connected peers (mainnet floor)
3. ✅ **Not actively syncing**: Must be within 2 blocks of network tip
4. ✅ **Desync tolerance**: Within 5 blocks of observed tip (if height > 100)
5. ❌ **NOT CHECKED**: Consensus quorum (peers on same chain)

**What "Connected Peers" Means**:
```rust
let connected_peers = crate::globals::P2P_MANAGER.clone_inner().try_get_peer_count() as u32;
```
This is just TCP connection count - does NOT verify chain compatibility!

---

## The Problem: No Chain Consensus Validation

### Current Vulnerability
Both gates check **peer count** but NOT **peer agreement**:
- ❌ 3 peers could be on different forks
- ❌ 3 peers could be at different heights
- ❌ 3 peers could have different chain IDs
- ❌ Miner could build on wrong fork

### What SHOULD Happen
**Sync Gate**: Only start syncing if 3+ peers agree on the same chain/height
**Mining Gate**: Only allow mining if 3+ peers are on same chain AND within height tolerance

---

## Consensus Quorum Infrastructure (Already Exists!)

### Chain Compatibility Check
**Location**: [src/p2p/peer_manager.rs#L205-L240](src/p2p/peer_manager.rs#L205-L240)

```rust
pub fn is_chain_compatible(
    &self,
    chain_id: &str,           // Must match exactly
    bootstrap_prefix: &str,   // Must match exactly
    min_proto: u32,           // Protocol version range
    max_proto: u32,
    min_node_version: &str,   // Node version minimum
) -> bool {
    // Chain id and prefix must match exactly
    match (&self.chain_id, &self.bootstrap_prefix) {
        (Some(cid), Some(prefix)) if cid == chain_id && prefix == bootstrap_prefix => {}
        _ => return false,
    }
    
    // Protocol version must be present and within range
    if let Some(pv) = self.protocol_version {
        if pv < min_proto || pv > max_proto {
            return false;
        }
    } else {
        return false;
    }
    
    // Node version check
    if let Some(ver) = &self.node_version {
        if ver.as_str() < min_node_version {
            return false;
        }
    } else {
        return false;
    }
    
    true
}
```

### Consensus Quorum API
**Location**: [src/p2p/peer_manager.rs#L750-L794](src/p2p/peer_manager.rs#L750-L794)

```rust
pub struct ConsensusQuorum {
    pub compatible_peers: usize,        // Peers on same chain
    pub incompatible_peers: usize,      // Peers on different chain
    pub min_compatible_height: Option<u64>,
    pub max_compatible_height: Option<u64>,
}

pub async fn consensus_quorum(&self) -> ConsensusQuorum {
    let expected_chain_id = expected_chain_id();
    let peers = self.peers.read().await;
    
    for p in peers.values() {
        if p.state != PeerState::Connected {
            continue;
        }
        
        if p.is_chain_compatible(
            &expected_chain_id,
            VISION_BOOTSTRAP_PREFIX,
            VISION_MIN_PROTOCOL_VERSION,
            VISION_MAX_PROTOCOL_VERSION,
            VISION_MIN_NODE_VERSION,
        ) {
            compatible_peers += 1;
            // Track height range of compatible peers
        } else {
            incompatible_peers += 1;
        }
    }
    
    ConsensusQuorum {
        compatible_peers,
        incompatible_peers,
        min_compatible_height,
        max_compatible_height,
    }
}
```

**This infrastructure EXISTS but is NOT USED by sync/mining gates!**

---

## Recommended Fix

### 1. Update Sync Gate
**Before**:
```rust
if connected_peers < MIN_PEERS_FOR_SYNC as usize {
    return Ok(());
}
```

**After**:
```rust
// Get consensus quorum (peers on same chain)
let quorum = crate::PEER_MANAGER.consensus_quorum().await;

if quorum.compatible_peers < MIN_PEERS_FOR_SYNC as usize {
    tracing::debug!(
        "[SYNC-GATE] Waiting for consensus: compatible={} incompatible={} need={}",
        quorum.compatible_peers, quorum.incompatible_peers, MIN_PEERS_FOR_SYNC
    );
    return Ok(());
}

// Optional: Also check height spread (prevent syncing from wildly divergent peers)
if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
    const MAX_HEIGHT_SPREAD: u64 = 100; // Don't sync if compatible peers span > 100 blocks
    if max_h.saturating_sub(min_h) > MAX_HEIGHT_SPREAD {
        tracing::warn!(
            "[SYNC-GATE] Compatible peers have divergent heights: min={} max={} spread={}",
            min_h, max_h, max_h - min_h
        );
        return Ok(());
    }
}
```

### 2. Update Mining Gate
**Before**:
```rust
let connected_peers = crate::globals::P2P_MANAGER.clone_inner().try_get_peer_count() as u32;

if connected_peers < effective_min_peers {
    return false;
}
```

**After**:
```rust
// CRITICAL: Check consensus quorum, not just TCP connections
let quorum = crate::PEER_MANAGER.consensus_quorum_blocking(); // Need blocking version for is_mining_eligible

if quorum.compatible_peers < effective_min_peers as usize {
    static QUORUM_LOG_THROTTLE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let last_log = QUORUM_LOG_THROTTLE.load(std::sync::atomic::Ordering::Relaxed);
    if now > last_log + 15 {
        QUORUM_LOG_THROTTLE.store(now, std::sync::atomic::Ordering::Relaxed);
        tracing::info!(
            "[MINING-GATE] Need consensus quorum: compatible={} incompatible={} need={}",
            quorum.compatible_peers, quorum.incompatible_peers, effective_min_peers
        );
    }
    return false;
}

// Optional: Verify compatible peers are at similar heights (prevent mining on outlier chain)
if let (Some(min_h), Some(max_h)) = (quorum.min_compatible_height, quorum.max_compatible_height) {
    const MAX_QUORUM_HEIGHT_SPREAD: u64 = 10; // Peers must be within 10 blocks of each other
    if max_h.saturating_sub(min_h) > MAX_QUORUM_HEIGHT_SPREAD {
        tracing::warn!(
            "[MINING-GATE] Compatible peers have divergent heights: min={} max={} spread={}",
            min_h, max_h, max_h - min_h
        );
        return false;
    }
}
```

### 3. Add Blocking Quorum Method
**Location**: [src/p2p/peer_manager.rs](src/p2p/peer_manager.rs)

```rust
/// Blocking version of consensus_quorum for non-async contexts like mining eligibility
pub fn consensus_quorum_blocking(&self) -> ConsensusQuorum {
    // Use tokio::runtime::Handle::current() to call async from sync context
    // OR implement a direct lock-based version that doesn't await
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(self.consensus_quorum())
    })
}
```

---

## Impact Analysis

### Without Fix (Current State)
**Scenario**: Node has 3 TCP connections to peers
- Peer A: On `vision-constellation-v1.0.1` at height 1000
- Peer B: On `vision-constellation-v1.0.0` at height 500 (old network)
- Peer C: On `vision-test-fork` at height 2000 (different fork)

**Current Behavior**:
- ✅ Sync gate passes (3 peers connected)
- ✅ Mining gate passes (3 peers connected)
- ❌ **Result**: Node might sync from wrong fork OR mine on isolated chain!

### With Fix (Proposed)
**Same Scenario**:
- Peer A: Compatible (same chain id + bootstrap prefix)
- Peer B: Incompatible (old bootstrap prefix)
- Peer C: Incompatible (different chain id)

**Fixed Behavior**:
- ❌ Sync gate BLOCKS (only 1 compatible peer, need 3)
- ❌ Mining gate BLOCKS (only 1 compatible peer, need 3)
- ✅ **Result**: Node waits for more compatible peers before syncing/mining

---

## Testing Plan

### 1. Multi-Node Testnet
```bash
# Terminal 1: Seed on v1.0.1
VISION_BOOTSTRAP_PREFIX=vision-constellation-v1.0.1 ./vision-node

# Terminal 2: Peer on v1.0.1 (should connect and sync)
VISION_BOOTSTRAP=http://localhost:7070 VISION_PORT=7071 ./vision-node

# Terminal 3: Peer on v1.0.0 (should be rejected as incompatible)
VISION_BOOTSTRAP_PREFIX=vision-constellation-bootstrap-1 VISION_BOOTSTRAP=http://localhost:7070 VISION_PORT=7072 ./vision-node

# Expected:
# - Nodes 1 & 2: Form consensus quorum, sync and mine
# - Node 3: Marked as incompatible, cannot join
```

### 2. Height Divergence Test
```bash
# Seed at height 1000
# Peer A at height 1005 (within tolerance)
# Peer B at height 1200 (too far ahead)

# Expected:
# - Quorum with Peer A only (Peer B too far ahead)
# - If need 3 peers, must wait for more at similar heights
```

### 3. Fork Test
```bash
# Seed on chain A
# Peer 1 on chain A (same)
# Peer 2 on chain B (fork)

# Expected:
# - Only Peer 1 compatible
# - Sync/mining blocked until 3+ compatible peers
```

---

## Summary

### Current State
- ✅ Peer count checks exist (3 minimum)
- ✅ Height lag checks exist (desync tolerance)
- ❌ **NO consensus validation** (peers on same chain)
- ❌ **NO quorum verification** (majority agreement)

### What Needs Fixing
1. **Sync Gate**: Use `consensus_quorum()` instead of raw peer count
2. **Mining Gate**: Use `consensus_quorum()` instead of raw peer count
3. **Add blocking API**: `consensus_quorum_blocking()` for sync contexts
4. **Optional**: Add height spread validation within compatible peers

### Risk Level
**HIGH** - Current implementation allows syncing/mining with incompatible peers, leading to:
- Wrong chain forks
- Isolated mining (wasted work)
- Consensus instability
- Network splits

### Priority
**CRITICAL** - Should be fixed before mainnet launch or in v1.0.2 hotfix

### Estimated Effort
**~2 hours** - Infrastructure already exists, just needs to be wired into gates
