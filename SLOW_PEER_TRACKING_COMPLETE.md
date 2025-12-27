# Slow Peer Tracking & Balanced Mining Fairness - Implementation Complete

## Overview
Implemented comprehensive peer health monitoring with automatic catch-up assistance, balanced mining fairness (no historical penalties), and weighted gossip for helping slow nodes sync faster.

## âœ… Feature 1: Slow Peer Tracking & Metrics

### New Fields in `Peer` struct (`src/p2p/peer_manager.rs`)
```rust
pub last_reported_height: Option<u64>,              // Most recently reported height
pub last_height_updated_at: Option<Instant>,         // When height was last updated (non-serializable)
```

### New Methods in `Peer`
```rust
pub fn update_reported_height(&mut self, height: u64)
// Call this whenever handshake/status message received

pub fn lag_blocks(&self, network_estimated_height: u64) -> u64
// Returns how many blocks behind this peer is

pub fn is_slow(&self, network_estimated_height: u64) -> bool
// Returns true if lag >= SLOW_PEER_LAG_BLOCKS (64 blocks)
```

### Extended `SyncHealthSnapshot` (`src/auto_sync.rs`)
**New fields:**
```rust
pub behind_blocks: i64,           // Negative = ahead, positive = behind
pub slow_peers: usize,            // Count of peers lagging >= 64 blocks
pub avg_peer_lag_blocks: f32,     // Average lag across all connected peers
```

**Automatic calculation in `SyncHealthSnapshot::current()`:**
- Iterates through all connected peers
- Calculates individual peer lag: `network_height - peer_reported_height`
- Counts slow peers (lag >= 64 blocks)
- Computes average lag for visibility
- Logs slow peers at debug level

### Constants (`src/vision_constants.rs`)
```rust
pub const SLOW_PEER_LAG_BLOCKS: u64 = 64;        // Threshold for "slow" peer
pub const MAX_DESYNC_FOR_MINING: u64 = 2;        // Max desync allowed to mine
```

### API Exposure (`src/api/website_api.rs`)
**Added to `/status` endpoint response:**
```json
{
  "behind_blocks": -1,           // This node's position relative to network
  "slow_peers": 2,               // How many peers are struggling
  "avg_peer_lag_blocks": 12.5    // Average lag across all peers
}
```

## âœ… Feature 2: Balanced Mining Fairness

### New Mining Check: `can_mine_fair()` (`src/auto_sync.rs`)
**No historical penalties - only current state matters:**

```rust
pub fn can_mine_fair(&self, min_peers: u16) -> bool {
    // âœ… Chain ID matches
    // âœ… Min peers connected (e.g., >= 3)
    // âœ… Not actively syncing
    // âœ… Not diverged (too far ahead)
    // âœ… Within MAX_DESYNC_FOR_MINING (â‰¤2 blocks)
    
    // If ALL pass â†’ FULL mining ticket, no throttling!
    true
}
```

**Key principle:** Once you're caught up and healthy â†’ equal chance in mining. No "you were behind earlier" penalty.

### Mining Status Helper: `mining_status_message()`
Returns user-friendly status:
- `"â›” Mining disabled: Chain ID mismatch"`
- `"â›” Mining disabled: Need 3 peers (have 1)"`
- `"â³ Mining disabled: Syncing (47 blocks behind)"`
- `"âš ï¸  Mining disabled: Too far ahead (85 blocks ahead of network)"`
- `"â³ Mining disabled: Desync too large (5 blocks)"`
- `"âœ… Mining ready: Full mining eligibility"`

### Rules Summary
| Condition | Result |
|-----------|--------|
| Behind by 3+ blocks | No mining ticket |
| Caught up (â‰¤2 blocks behind/ahead) | **Full mining ticket** |
| Was behind 10 min ago, now synced | **Full mining ticket** (no memory) |
| Ahead by 51+ blocks (diverged) | No mining ticket |

## âœ… Feature 3: Auto-Boost Weighted Gossip

### Peer Weight Calculation (`src/p2p/peer_manager.rs`)
```rust
pub fn compute_peer_weight(peer: &Peer, network_estimated_height: u64) -> f32 {
    let mut weight = 1.0;
    
    if peer.lag_blocks(network_height) >= SLOW_PEER_LAG_BLOCKS {
        weight *= 2.0;  // 2x boost for slow peers
        
        tracing::debug!(
            "Boosting slow peer {} with extra gossip weight (lag: {} blocks)",
            peer.ebid, lag
        );
    }
    
    weight
}
```

### Weighted Random Selection
```rust
pub async fn select_weighted_peers(
    &self, 
    count: usize, 
    network_estimated_height: u64
) -> Vec<String> {
    // 1. Calculate weights for all connected peers
    // 2. Slow peers get 2x weight â†’ higher probability
    // 3. Weighted random sampling without replacement
    // 4. Returns list of peer EBIDs to send block/tx to
}
```

**Effect:**
- Slow peer at height 1000 (64 blocks behind 1064) â†’ weight = 2.0
- Healthy peer at height 1063 (1 block behind) â†’ weight = 1.0
- Slow peer gets picked ~2x more often for block/tx gossip
- Catches up faster â†’ rejoins mining mining sooner

### Integration Points (For Future Use)
When broadcasting blocks or transactions, replace:
```rust
// OLD: Flat random selection
let random_peers = peer_manager.connected_peers().await
    .choose_multiple(&mut rng, 5)
    .collect();

// NEW: Weighted selection helps slow peers
let weighted_peers = peer_manager
    .select_weighted_peers(5, network_height)
    .await;
```

## Flow Diagrams

### Slow Peer Detection Flow
```
Handshake/Status Message
    â†“
peer.update_reported_height(remote_height)
    â†“
SyncHealthSnapshot::current()
    â†“
For each peer: calculate lag = network_height - peer_height
    â†“
If lag >= 64 â†’ slow_peers count++
    â†“
Expose in /status API: { "slow_peers": 2, "avg_peer_lag_blocks": 15.3 }
```

### Balanced Mining Fairness Flow
```
Mining Attempt
    â†“
snapshot = SyncHealthSnapshot::current()
    â†“
can_mine_fair(min_peers=3)?
    â†“
Check CURRENT state only:
  âœ“ Chain ID matches?
  âœ“ >= 3 peers connected?
  âœ“ Not syncing right now?
  âœ“ Not diverged?
  âœ“ Within Â±2 blocks?
    â†“
ALL YES â†’ Full mining ticket! ðŸŽ«
ANY NO  â†’ No mining (with helpful status message)
```

### Weighted Gossip Flow
```
New Block Mined
    â†“
Need to broadcast to 5 peers
    â†“
select_weighted_peers(5, network_height)
    â†“
Calculate weights:
  - Peer A (lag=70) â†’ weight=2.0 â­ (slow, needs help)
  - Peer B (lag=1)  â†’ weight=1.0
  - Peer C (lag=65) â†’ weight=2.0 â­ (slow, needs help)
  - Peer D (lag=0)  â†’ weight=1.0
    â†“
Weighted random sample â†’ likely picks A, C more often
    â†“
Send block to selected peers
    â†“
Slow peers get more updates â†’ catch up faster
```

## Usage Examples

### Check If Node Can Mine
```rust
use crate::auto_sync::SyncHealthSnapshot;

let snapshot = SyncHealthSnapshot::current();

if snapshot.can_mine_fair(3) {
    println!("âœ… Ready to mine!");
} else {
    println!("{}", snapshot.mining_status_message(3));
}
```

### Get Slow Peer Count
```rust
let snapshot = SyncHealthSnapshot::current();

println!("Slow peers needing help: {}", snapshot.slow_peers);
println!("Average peer lag: {:.1} blocks", snapshot.avg_peer_lag_blocks);
println!("This node is {} blocks behind network", snapshot.behind_blocks);
```

### Update Peer Height (In Handshake Handler)
```rust
// When receiving handshake or status message
peer.update_reported_height(remote_chain_height);

// This automatically:
// 1. Updates last_reported_height
// 2. Updates last_height_updated_at timestamp
// 3. Updates legacy height field for compatibility
```

### Select Weighted Peers for Gossip
```rust
let peer_manager = Arc::clone(&PEER_MANAGER);
let network_height = snapshot.network_estimated_height;

// Get 5 peers with bias toward slow ones
let peer_ebids = peer_manager
    .select_weighted_peers(5, network_height)
    .await;

for ebid in peer_ebids {
    send_block_to_peer(&ebid, &new_block).await?;
}
```

## Monitoring & Debugging

### Log Messages

**Slow Peer Detection:**
```
[vision_node::auto_sync] Peer is slow and needs help catching up
  peer_ebid: "abc123...", lag_blocks: 78
```

**Weighted Gossip Boost:**
```
[vision_node::p2p::gossip] Boosting slow peer with extra gossip weight
  peer_ebid: "abc123...", lag_blocks: 78, weight: 2.0
```

**Weighted Selection:**
```
[vision_node::p2p::gossip] Selected weighted peers for gossip
  selected_count: 5, total_peers: 12
```

### API Monitoring
```bash
# Check slow peer metrics
curl http://localhost:7070/status | jq '{
  slow_peers,
  avg_peer_lag_blocks,
  behind_blocks,
  can_mine
}'
```

**Example output:**
```json
{
  "slow_peers": 2,
  "avg_peer_lag_blocks": 18.5,
  "behind_blocks": -1,
  "can_mine": true
}
```

## Configuration

### Tunable Constants (vision_constants.rs)
```rust
SLOW_PEER_LAG_BLOCKS = 64        // Threshold for "slow" classification
MAX_DESYNC_FOR_MINING = 2         // Max desync to participate in mining
MAX_BLOCKS_AHEAD_OF_CONSENSUS = 50  // Too far ahead = diverged
```

### Recommended Settings
- **Testnet:** Keep defaults (64 block lag, 2 block mining tolerance)
- **Mainnet:** May want stricter (32 block lag, 1 block mining tolerance)
- **Private chains:** Can be more lenient (128 block lag, 5 block tolerance)

## Benefits

### 1. Slow Peer Visibility
- **Before:** No idea which peers are struggling
- **After:** Real-time count + average lag visible in API/logs

### 2. Fair Mining
- **Before:** Possible historical penalty systems
- **After:** Pure snapshot-based - caught up = full ticket immediately

### 3. Automatic Network Healing
- **Before:** All peers get equal gossip priority
- **After:** Slow peers automatically get 2x more updates

### 4. Faster Convergence
- Network naturally helps stragglers catch up
- Slow nodes rejoin mining faster
- More participants = healthier decentralization

## Testing Recommendations

### 1. Simulate Slow Peer
```bash
# Start Node A normally
node-a> ./vision-node

# Start Node B with artificial delay (stop syncing for 2 minutes)
node-b> ./vision-node
# (Stop Node B after 30 blocks)
# (Restart Node B after 2 min = ~60 blocks behind)

# Check Node A's status
curl http://node-a:7070/status | jq .slow_peers
# Should show: 1
```

### 2. Test Weighted Gossip
```rust
#[test]
fn test_weighted_peer_selection() {
    let peer_manager = PeerManager::new();
    
    // Add peers at different heights
    add_peer("peer1", 1000);  // Behind by 64 blocks
    add_peer("peer2", 1063);  // Caught up
    add_peer("peer3", 1064);  // Fully synced
    
    let selected = peer_manager.select_weighted_peers(100, 1064).await;
    
    // Count how often each peer was selected
    let peer1_count = selected.iter().filter(|p| *p == "peer1").count();
    let peer2_count = selected.iter().filter(|p| *p == "peer2").count();
    
    // Slow peer should be selected ~2x more
    assert!(peer1_count > peer2_count * 1.5);
}
```

### 3. Test Mining Fairness
```rust
#[test]
fn test_mining_fairness() {
    let snapshot = SyncHealthSnapshot {
        sync_height: 1062,
        network_estimated_height: 1064,
        behind_blocks: 2,
        connected_peers: 5,
        chain_id_matches: true,
        is_syncing: false,
        is_too_far_ahead: false,
        slow_peers: 0,
        avg_peer_lag_blocks: 0.0,
    };
    
    // Within 2 blocks â†’ should be able to mine
    assert!(snapshot.can_mine_fair(3));
    
    // Now simulate falling behind by 3 blocks
    let behind_snapshot = SyncHealthSnapshot {
        behind_blocks: 3,
        ..snapshot
    };
    
    // Beyond 2 block threshold â†’ cannot mine
    assert!(!behind_snapshot.can_mine_fair(3));
}
```

## Integration Checklist

- [x] Add `last_reported_height` and `last_height_updated_at` to `Peer` struct
- [x] Add `update_reported_height()` method to `Peer`
- [x] Extend `SyncHealthSnapshot` with `behind_blocks`, `slow_peers`, `avg_peer_lag_blocks`
- [x] Calculate slow peer metrics in `SyncHealthSnapshot::current()`
- [x] Add `SLOW_PEER_LAG_BLOCKS` and `MAX_DESYNC_FOR_MINING` constants
- [x] Implement `can_mine_fair()` for balanced mining eligibility
- [x] Implement `mining_status_message()` helper
- [x] Add `compute_peer_weight()` for gossip priority
- [x] Add `select_weighted_peers()` for weighted random selection
- [x] Expose slow peer metrics in `/status` API endpoint
- [x] Add debug logging for slow peer detection and boost
- [ ] **TODO:** Call `peer.update_reported_height()` in handshake/status handlers
- [ ] **TODO:** Replace flat peer selection with `select_weighted_peers()` in block/tx broadcast
- [ ] **TODO:** Update mining logic to use `snapshot.can_mine_fair()` instead of old checks

## Files Modified

```
src/vision_constants.rs         - Added SLOW_PEER_LAG_BLOCKS, MAX_DESYNC_FOR_MINING
src/p2p/peer_manager.rs         - Added height tracking fields, weight/selection methods
src/auto_sync.rs                - Extended snapshot, added fairness checks
src/api/website_api.rs          - Exposed slow peer metrics in /status
```

## Version Info
- **Release:** v2.7.0
- **Chain ID:** VISION-CONSTELLATION-V2.7-TESTNET1
- **Package:** VisionNode-Constellation-v2.7.0-WIN64.zip (15.38 MB)
- **Build Date:** December 10, 2025

---

**Status:** âœ… Core infrastructure complete, ready for integration into handshake/gossip logic
**Next Steps:** Wire `update_reported_height()` into P2P message handlers, use `select_weighted_peers()` in broadcast code

