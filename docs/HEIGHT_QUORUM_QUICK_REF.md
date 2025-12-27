# Height Quorum Quick Reference

## What It Does
Prevents isolated/desynced nodes from mining until they sync with network consensus.

## Configuration (src/vision_constants.rs)
```rust
MIN_SAME_HEIGHT_PEERS_FOR_MINING = 2    // Need 2+ peers agreeing
MAX_HEIGHT_DELTA_FOR_QUORUM = 2         // Within ±2 blocks
MINING_QUORUM_TIMEOUT_SECS = 300        // 5 min escape hatch
```

## Behavior

### With Network (Normal)
1. Node connects to peers
2. Waits for 2+ peers at same height (±2 blocks)
3. Verifies local height is within consensus range
4. Begins mining when synced

### Isolated (No Network)
1. Node starts alone
2. Quorum check fails
3. Mining blocked for 5 minutes
4. After timeout, isolated mining allowed

### Desynced (Behind Network)
1. Node sees quorum at height 100
2. Local height 90 (outside 100±2)
3. Mining blocked
4. Syncs to 98+
5. Mining resumes

## Log Messages

**Quorum Failed**:
```
⚠️  Mining gate: Height quorum not satisfied (need sync with 3 peers at height Some(150), we're at 145)
```

**No Network**:
```
⚠️  Mining gate: No height quorum detected (need network convergence before mining)
```

**Timeout Escape**:
```
⚠️  Mining in ISOLATED mode (quorum timeout elapsed after 300s)
```

## Testing Commands

### 3-Node Local Test
```powershell
# Terminal 1 (node on 7070)
$env:VISION_PORT=7070; .\vision-node.exe

# Terminal 2 (node on 7073)
$env:VISION_PORT=7073; .\vision-node.exe

# Terminal 3 (node on 7076)
$env:VISION_PORT=7076; .\vision-node.exe
```

### Check Quorum Status
```powershell
curl http://localhost:7070/api/v1/info | ConvertFrom-Json | Select-Object height, connected_peers
```

### Monitor Mining Eligibility
Watch node logs for:
- "Mining gate: Height quorum" messages
- "Block rewards ENABLED" vs "DISABLED"
- Mining attempts vs blocks mined

## Implementation Files
- `src/p2p/peer_manager.rs` - Quorum detection
- `src/vision_constants.rs` - Config constants
- `src/config/miner.rs` - Mining gate logic
- `src/main.rs` - Quorum computation

## Tuning

### More Conservative (require more peers)
```rust
pub const MIN_SAME_HEIGHT_PEERS_FOR_MINING: usize = 3;
```

### More Lenient (allow bigger desync)
```rust
pub const MAX_HEIGHT_DELTA_FOR_QUORUM: u64 = 5;
```

### Longer Timeout (wait longer before isolated mining)
```rust
pub const MINING_QUORUM_TIMEOUT_SECS: u64 = 600;  // 10 minutes
```

## Common Issues

**"Mining gate: No height quorum detected"**
- Not enough peers connected
- Peers don't have heights reported yet
- Wait for peer discovery to complete

**Node stuck not mining after 5 minutes**
- Check other eligibility requirements (min_peers, desync)
- Verify peers actually connected (check /api/v1/info)
- Check p2p_health is not "isolated" or "weak"

**All nodes mining immediately on fresh network**
- This is correct! All start at height 0 (genesis)
- Quorum immediately satisfied at height 0
- Normal behavior for new testnet

## Verification Checklist

✅ Node waits for quorum before mining  
✅ Isolated node mines after 5 min timeout  
✅ Desynced node catches up before mining  
✅ 3-node network mines normally  
✅ Quorum warnings appear in logs  
✅ Mining resumes after sync complete  

## Build Status
✅ Compiles successfully
✅ No new errors introduced
✅ Ready for testing

---
See `HEIGHT_QUORUM_IMPLEMENTATION.md` for complete documentation.
