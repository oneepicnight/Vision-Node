# Height Quorum Mining Gate - Implementation Complete

## Overview
Height quorum system prevents isolated or desynced nodes from mining blocks until they converge with the network consensus. This prevents fake-rich scenarios on testnets and ensures nodes mine on the canonical chain.

## Implementation Summary

### Phase 1: Peer Height Tracking (✅ Complete)
**File**: `src/p2p/peer_manager.rs`

1. **Extended PeerSnapshotEntry** (Lines 29-33)
   ```rust
   pub struct PeerSnapshotEntry {
       pub peer_id: String,
       pub addr: SocketAddr,
       pub protocol_version: u32,
       pub remote_height: Option<u64>,  // NEW: Track peer's chain height
   }
   ```

2. **Updated snapshot() method** (Lines 428-432)
   - Populates `remote_height` from peer state
   - Used by quorum detection algorithm

3. **Added best_height_quorum() helper** (Lines 492-544)
   - Clusters peer heights into buckets
   - Finds dominant height band within `MAX_HEIGHT_DELTA_FOR_QUORUM` tolerance
   - Returns `(consensus_height, peer_count)` if quorum exists
   - Uses HashMap-based algorithm to detect network convergence

### Phase 2: Configuration Constants (✅ Complete)
**File**: `src/vision_constants.rs` (Lines 290-304)

```rust
// MINING HEIGHT QUORUM
pub const MIN_SAME_HEIGHT_PEERS_FOR_MINING: usize = 2;
pub const MAX_HEIGHT_DELTA_FOR_QUORUM: u64 = 2;
pub const MINING_QUORUM_TIMEOUT_SECS: u64 = 300;
```

**Constants Explained**:
- **MIN_SAME_HEIGHT_PEERS_FOR_MINING = 2**: Need at least 2 peers agreeing on height
- **MAX_HEIGHT_DELTA_FOR_QUORUM = 2**: Peers within ±2 blocks count as "same height"
- **MINING_QUORUM_TIMEOUT_SECS = 300**: After 5 minutes, allow isolated mining (escape hatch)

### Phase 3: Sync Health Extension (✅ Complete)
**File**: `src/config/miner.rs` (Lines 41-48)

Extended `SyncHealthSnapshot` with quorum fields:
```rust
pub struct SyncHealthSnapshot {
    pub connected_peers: u16,
    pub p2p_health: String,
    pub sync_height: u64,
    pub network_estimated_height: u64,
    // NEW QUORUM FIELDS:
    pub height_quorum_ok: bool,            // Are we synced with consensus?
    pub height_quorum_peers: usize,        // How many peers in quorum
    pub height_quorum_height: Option<u64>, // Network consensus height
}
```

### Phase 4: Quorum Computation (✅ Complete)
**File**: `src/main.rs` (Lines 4018-4090, updated)

Updated `current_sync_health()` to compute quorum:
1. Calls `PEER_MANAGER.best_height_quorum(MAX_HEIGHT_DELTA_FOR_QUORUM)`
2. Checks if quorum threshold met (≥ MIN_SAME_HEIGHT_PEERS_FOR_MINING)
3. Verifies local height is within tolerance of cluster height
4. Populates 3 new snapshot fields

**Quorum Logic**:
```rust
if cluster_peers >= MIN_SAME_HEIGHT_PEERS_FOR_MINING {
    // Check if we're close to the consensus cluster
    if sync_height >= cluster_height - MAX_HEIGHT_DELTA_FOR_QUORUM
        && sync_height <= cluster_height + MAX_HEIGHT_DELTA_FOR_QUORUM
    {
        height_quorum_ok = true;
    }
}
```

### Phase 5: Mining Gate (✅ Complete)
**File**: `src/config/miner.rs` (Lines 51-115, updated)

**Extended RewardEligibilityConfig**:
```rust
pub struct RewardEligibilityConfig {
    // ... existing fields ...
    #[serde(skip)]
    pub node_started_at: Option<Instant>,  // NEW: For timeout escape
}
```

**Updated is_reward_eligible()**:
- Added Check 5: Height quorum gate
- Implements timeout escape (after 300 seconds)
- Logs warnings when mining without quorum
- Returns false if quorum not satisfied and timeout not elapsed

**Timeout Escape Logic**:
```rust
if !snapshot.height_quorum_ok {
    let timeout_elapsed = if let Some(start_time) = cfg.node_started_at {
        start_time.elapsed().as_secs() >= MINING_QUORUM_TIMEOUT_SECS
    } else {
        false
    };
    
    if !timeout_elapsed {
        return false;  // Block mining
    } else {
        // Allow isolated mining after timeout
    }
}
```

### Phase 6: Node Start Time Tracking (✅ Complete)
**File**: `src/main.rs`

1. **Global NODE_START_TIME** (Line 2950)
   ```rust
   pub static NODE_START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
   ```

2. **Set on config load** (Lines 4134, 4204)
   - Both mining locations set `node_started_at = Some(*NODE_START_TIME)`
   - Enables timeout calculation in eligibility checks

## Behavioral Changes

### Before Height Quorum
- Isolated node could mine immediately
- Risk of fake-rich scenarios on testnet
- No network consensus verification

### After Height Quorum
1. **Normal Operation** (with network):
   - Node starts, connects to peers
   - Waits for height quorum (2+ peers agree on height)
   - Verifies local height is within ±2 blocks of consensus
   - Begins mining only when synced

2. **Isolated Operation** (no network):
   - Node starts without peers
   - Quorum check fails (no consensus detected)
   - Mining blocked for 5 minutes
   - After timeout, isolated mining allowed (escape hatch)
   - Logs warning: "Mining in ISOLATED mode"

3. **Desynced Node** (behind network):
   - Node connects to network
   - Detects quorum at height N
   - Local height at N-10 (behind)
   - Mining blocked until synced to N±2
   - Syncs blocks, then mining resumes

## Log Messages

### Quorum Not Satisfied
```
⚠️  Mining gate: Height quorum not satisfied (need sync with 3 peers at height Some(150), we're at 145)
```

### No Quorum Detected
```
⚠️  Mining gate: No height quorum detected (need network convergence before mining)
```

### Isolated Mining (Timeout)
```
⚠️  Mining in ISOLATED mode (quorum timeout elapsed after 300s)
```

## Testing Scenarios

### Test 1: Fresh 3-Node Network
**Setup**: Start 3 nodes from genesis
**Expected**:
- Nodes connect to each other
- All start at height 0 (genesis)
- Quorum immediately satisfied (3 peers at height 0)
- All nodes can mine block 1

### Test 2: Isolated Node
**Setup**: Start single node with no peers
**Expected**:
- No peers detected, quorum fails
- Mining blocked for 5 minutes
- After 300s, timeout escape triggers
- Node mines in isolated mode

### Test 3: Desynced Node Joins Network
**Setup**: 2 nodes at height 100, 1 node at height 80
**Expected**:
- Desynced node sees quorum at height 100
- Mining blocked (80 is outside 100±2 range)
- Node syncs to height 98+
- Quorum satisfied, mining resumes

### Test 4: Network Split Recovery
**Setup**: 2 nodes at height 100, 2 nodes at height 105
**Expected**:
- Each group has quorum within their cluster
- Mining continues on both forks
- When network merges, nodes reorganize to longest chain
- Mining resumes on canonical chain

## Configuration Tuning

### Adjust Quorum Threshold
Edit `src/vision_constants.rs`:
```rust
pub const MIN_SAME_HEIGHT_PEERS_FOR_MINING: usize = 3;  // Require 3 peers instead of 2
```

### Adjust Height Tolerance
```rust
pub const MAX_HEIGHT_DELTA_FOR_QUORUM: u64 = 5;  // Allow ±5 blocks instead of ±2
```

### Adjust Timeout
```rust
pub const MINING_QUORUM_TIMEOUT_SECS: u64 = 600;  // 10 minutes instead of 5
```

## Architecture Notes

### Why Timeout Escape?
Without timeout, a node with no peers would never mine. The 5-minute timeout allows:
- Development/testing on isolated machines
- Recovery from network outages
- Solo mining for genesis/testnet operators

### Why ±2 Block Tolerance?
Network propagation delays mean honest nodes may be 1-2 blocks apart. Strict equality would frequently fail quorum on real networks.

### Why HashMap Clustering?
The `best_height_quorum()` algorithm uses HashMap to find dominant height cluster:
1. Group peer heights into buckets
2. For each bucket, expand by ±MAX_HEIGHT_DELTA
3. Count peers in expanded range
4. Return largest cluster

This handles gradual height divergence better than simple "most common height" approach.

## Files Modified

1. `src/p2p/peer_manager.rs` - Peer height tracking, quorum detection
2. `src/vision_constants.rs` - Quorum configuration constants
3. `src/config/miner.rs` - Sync health extension, mining gate
4. `src/main.rs` - Quorum computation, node start time tracking

## Build Status
✅ **Compiled successfully** (warnings pre-existing, not from this feature)

## Next Steps
1. Test with 3-node local network (different ports)
2. Verify isolated node timeout escape
3. Test network split/merge scenarios
4. Monitor logs for quorum warnings
5. Adjust constants based on network behavior

## Related Documentation
- `BOOTSTRAP_SNAPSHOT_IMPLEMENTATION.md` - Bootstrap checkpoint system
- `BEACON_P2P_BOOTSTRAP_STATUS.md` - P2P network architecture
- Port configuration: HTTP=7070, P2P=7072

---
**Implementation Date**: 2024 (continuation of bootstrap work)
**Status**: Complete, ready for testing
**Build**: Compiles clean with existing 24 warnings (unrelated)
