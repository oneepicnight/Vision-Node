# Slow Peer Tracking - Quick Reference

## At a Glance

âœ… **Identify slow peers:** Track who's lagging â‰¥64 blocks behind  
âœ… **Balanced mining fairness:** Current state only, no historical penalties  
âœ… **Auto-boost gossip:** Slow peers get 2x priority to catch up faster

## Key Metrics (Exposed in `/status` API)

```json
{
  "behind_blocks": -1,           // Your position: negative = ahead, positive = behind
  "slow_peers": 2,               // How many peers are struggling (lag â‰¥64 blocks)
  "avg_peer_lag_blocks": 12.5,   // Average lag across all connected peers
  "can_mine": true               // Mining eligibility (snapshot-based, fair)
}
```

## Mining Fairness Rules

| Your Status | Can Mine? | Reason |
|-------------|-----------|--------|
| Within Â±2 blocks of network | âœ… Yes | Full mining ticket |
| 3+ blocks behind | âŒ No | Must sync first |
| 51+ blocks ahead (diverged) | âŒ No | Chain mismatch |
| < 3 peers connected | âŒ No | Need consensus |
| Chain ID mismatch | âŒ No | Wrong network |

**Key principle:** Once you're caught up â†’ equal chance. No memory of past struggles.

## Weighted Gossip Priority

```
Peer A: 64+ blocks behind â†’ Weight = 2.0 â­ (2x priority)
Peer B: < 64 blocks behind â†’ Weight = 1.0 (normal)
Peer C: 64+ blocks behind â†’ Weight = 2.0 â­ (2x priority)
```

When broadcasting block/tx to 5 peers â†’ slow peers A & C more likely to be selected â†’ catch up faster.

## Quick Code Snippets

### Check Mining Eligibility
```rust
let snapshot = SyncHealthSnapshot::current();
if snapshot.can_mine_fair(3) {
    start_mining();
} else {
    println!("{}", snapshot.mining_status_message(3));
}
```

### Get Slow Peer Count
```rust
let snapshot = SyncHealthSnapshot::current();
println!("Slow peers: {}", snapshot.slow_peers);
println!("Avg lag: {:.1} blocks", snapshot.avg_peer_lag_blocks);
```

### Update Peer Height (Handshake Handler)
```rust
peer.update_reported_height(remote_height);
// Automatically updates: last_reported_height, last_height_updated_at, height
```

### Weighted Peer Selection (Gossip)
```rust
let peer_ebids = PEER_MANAGER
    .select_weighted_peers(5, network_height)
    .await;
// Returns 5 peer EBIDs with bias toward slow ones
```

## Constants (Tunable)

```rust
SLOW_PEER_LAG_BLOCKS = 64      // Threshold for "slow" (can increase/decrease)
MAX_DESYNC_FOR_MINING = 2       // Max blocks behind/ahead to mine
MAX_BLOCKS_AHEAD_OF_CONSENSUS = 50  // Divergence threshold
```

## Monitoring Commands

```bash
# Check status
curl http://localhost:7070/status | jq '{slow_peers, avg_peer_lag_blocks, can_mine}'

# Watch for slow peers
watch -n 5 'curl -s http://localhost:7070/status | jq .slow_peers'

# Monitor your position
curl http://localhost:7070/status | jq '.behind_blocks'
```

## Log Patterns

```
[auto_sync] Peer is slow and needs help catching up
  peer_ebid: "abc123", lag_blocks: 78

[p2p::gossip] Boosting slow peer with extra gossip weight
  peer_ebid: "abc123", lag_blocks: 78, weight: 2.0

[p2p::gossip] Selected weighted peers for gossip
  selected_count: 5, total_peers: 12
```

## Integration Points (TODO)

1. **Handshake/Status Handler:** Call `peer.update_reported_height(remote_height)`
2. **Block Broadcast:** Use `select_weighted_peers()` instead of random selection
3. **TX Gossip:** Use `select_weighted_peers()` instead of random selection
4. **Mining Gate:** Use `snapshot.can_mine_fair()` for eligibility

## Benefits

- ðŸ“Š **Visibility:** Know exactly which peers are struggling
- âš–ï¸ **Fairness:** No permanent penalties, current state = truth
- ðŸš€ **Auto-healing:** Network automatically helps stragglers catch up
- ðŸŽ¯ **Faster convergence:** More nodes ready to mine = stronger decentralization

## Files Modified

```
src/vision_constants.rs      - Constants
src/p2p/peer_manager.rs      - Tracking + weighted selection
src/auto_sync.rs             - Snapshot + fairness checks
src/api/website_api.rs       - API exposure
```

---

**Version:** 2.7.0 | **Status:** âœ… Ready for integration

