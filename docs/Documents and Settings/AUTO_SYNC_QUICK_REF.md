# Auto-Sync Independent - Quick Reference

## Core Rule
**Mining eligibility NEVER controls auto-sync.**

## What It Does
- Checks every 10 seconds: "Am I behind?"
- If >5 blocks behind → Syncs immediately
- Never checks mining flags, quorum, or eligibility

## Files Changed
1. `src/p2p/peer_manager.rs` - Added `best_remote_height()`
2. `src/vision_constants.rs` - Added auto-sync constants
3. `src/auto_sync.rs` - Complete rewrite (mining-independent)

## Key Methods

### PeerManager::best_remote_height()
```rust
// Returns highest peer height (simple max)
let height = PEER_MANAGER.best_remote_height().await;
```

### spawn_auto_sync_task()
```rust
// Starts background sync loop (never stops)
auto_sync::spawn_auto_sync_task(AutoSyncConfig::default());
```

## Configuration

### Constants (vision_constants.rs)
```rust
AUTO_SYNC_INTERVAL_SECS = 10      // Check every 10s
AUTO_SYNC_MAX_LAG_BLOCKS = 5      // Sync if >5 behind
```

### Tuning
**Faster sync**: Lower interval, lower lag threshold  
**Slower sync**: Higher interval, higher lag threshold

## Behavior Matrix

| Scenario | Mining Eligible? | Has Peers? | Auto-Sync Action |
|----------|------------------|------------|------------------|
| Synced node mining | ✅ Yes | ✅ Yes | Monitor (no sync) |
| Synced node NOT mining | ❌ No | ✅ Yes | Monitor (no sync) |
| Behind node mining | ✅ Yes | ✅ Yes | **SYNC** |
| Behind node NOT mining | ❌ No | ✅ Yes | **SYNC** |
| Behind, quorum failed | ❌ No | ✅ Yes | **SYNC** |
| Isolated node | ❌ No | ❌ No | Wait for peers |

**Key**: Auto-sync only checks "Has Peers?" and "Am I Behind?" - Nothing else!

## Log Messages

### ✅ Synced
```
AUTO-SYNC: at tip or ahead (local=1050, remote=1050)
```

### ⏸️ Small lag (monitoring)
```
AUTO-SYNC: small lag=2 (< 5), staying in monitor mode
```

### 🔄 Syncing
```
AUTO-SYNC: behind by 15 blocks (local=1035, remote=1050), starting catch-up
AUTO-SYNC: pulled 15 blocks from http://peer:7070
```

### ⚠️ No peers
```
AUTO-SYNC: no peers with known height
```

## Testing Commands

### Check sync status
```powershell
curl http://localhost:7070/api/v1/info | ConvertFrom-Json | Select height
```

### Watch sync logs
```powershell
# In node terminal, should see "AUTO-SYNC:" messages
```

### Test sync without mining
```powershell
# Start node, disable auto-mine in miner.json
# Node should still sync to network
```

## Common Issues

### "Auto-sync not running"
- Check if `start_autosync()` called at startup (it is)
- Check if node has any peers connected
- Enable trace logs: `RUST_LOG=auto_sync=trace`

### "Syncing too often"
- Increase `AUTO_SYNC_MAX_LAG_BLOCKS` (e.g., to 10)
- Increase `AUTO_SYNC_INTERVAL_SECS` (e.g., to 30)

### "Not syncing fast enough"
- Decrease `AUTO_SYNC_MAX_LAG_BLOCKS` (e.g., to 2)
- Decrease `AUTO_SYNC_INTERVAL_SECS` (e.g., to 5)

## Architecture

```
Auto-Sync Loop (every 10s):
  1. Get best_remote_height() from PEER_MANAGER
  2. Get local height from CHAIN
  3. Calculate lag = remote - local
  4. If lag > 5 → Call /sync/pull
  5. Never check mining eligibility!
```

## Differences from Mining Quorum

| Feature | Auto-Sync | Mining Gate |
|---------|-----------|-------------|
| **Purpose** | Keep synced | Gate mining |
| **Height Check** | Max peer height | Consensus height |
| **Threshold** | >5 blocks behind | 2+ peers in quorum |
| **Checks Mining?** | ❌ Never | ✅ Yes |
| **Can Block?** | ❌ No | ✅ Yes (intended) |

## Build Status
✅ Compiles  
✅ Tests pass  
✅ Ready to use  

---
**See `AUTO_SYNC_INDEPENDENT.md` for full documentation.**
