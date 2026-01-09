# Critical Sync Fixes - Deployment Guide

## The Real Bug (Now Fixed!)

**Your node was trying to "adopt heavier tip" BEFORE verifying the candidate chain's parents exist locally.**

Result:
```
reorg: adopting heavier tip heaviest=<some_hash>
<tries to build reorg path>
Error: "missing parent for candidate tip"
→ marks peer as incompatible
→ disconnects peer
→ never syncs
```

## Fixes Deployed

### ✅ Fix 1: Heaviest Tip Must Be CONNECTED

**Before:**
```rust
// Bad: Consider ALL blocks with cumulative_work
for (hash, weight) in cumulative_work.iter() {
    if weight > heaviest_work {
        heaviest_hash = hash;  // Even if orphaned!
    }
}
```

**After:**
```rust
// Good: Only consider blocks with CONNECTED ancestry
for (hash, weight) in cumulative_work.iter() {
    if weight > heaviest_work {
        // Walk back to verify parent chain exists
        if is_connected_to_main_chain(hash) {
            heaviest_hash = hash;  // Only if connected!
        } else {
            // Skip disconnected/orphaned chains
        }
    }
}
```

**What this fixes:**
- Orphaned blocks no longer trigger reorg attempts
- Only fully-connected alternative chains can become new tip
- "Missing parent for candidate tip" should never happen

### ✅ Fix 2: Missing Parent → Fetch, Not Fail

**Before:**
```rust
if parent_missing {
    return Err("missing parent for candidate tip");
    // → peer marked incompatible
    // → disconnected
}
```

**After:**
```rust
if parent_missing {
    tracing::warn!("[REORG-DEFERRED] Missing parent - fetching");
    spawn_fetch_parent_task(missing_parent, peer);
    return Ok(());  // Defer reorg until parent arrives
}
```

**What this fixes:**
- Missing parents trigger fetch request instead of failure
- Peer stays connected
- Reorg will succeed once parent chain completes

### ✅ Fix 3: Bootstrap Mode (from previous deployment)

Allows deep reorgs when:
- `height < 128` (initial sync)
- `behind_by > 24` (catching up)

Max reorg: 2048 blocks (vs 36 in normal mode)

## Combined Effect

**Old behavior:**
1. Receive block #101 from peer
2. Compute it as "heaviest" (even though parents missing)
3. Try to reorg
4. Hit "missing parent for candidate tip"
5. Disconnect peer as incompatible
6. Never sync

**New behavior:**
1. Receive block #101 from peer
2. Check if it's connected to known chain
3. If not connected → skip it for heaviest tip selection
4. If it WAS selected but parents missing → fetch parents
5. Keep peer connected
6. Once parent chain completes → reorg succeeds
7. **Sync completes!**

## Deployment Steps

### Step 1: Stop Your Current Node
```powershell
# Find and stop the process
Stop-Process -Name "vision-node" -Force
```

### Step 2: Deploy New Binary
```powershell
Copy-Item -Path "target\release\vision-node.exe" -Destination "C:\vision-node\vision-node-v1.0-windows-mainnet\vision-node.exe" -Force
```

### Step 3: Start Node
```powershell
cd C:\vision-node\vision-node-v1.0-windows-mainnet
.\vision-node.exe
```

### Step 4: Watch Logs

**Success indicators:**
```
[HEAVIEST-TIP] Updated to connected heavier chain
[REORG-BOOTSTRAP] Initial sync mode active - allowing deep reorg
[ORPHAN-POOL] processed children of accepted block
✅ inserted height=72 hash=...
✅ inserted height=73 hash=...
```

**Old errors should NOT appear:**
```
❌ Won't see: "missing parent for candidate tip"
❌ Won't see: "reorg too large: 69 > max 36"
❌ Won't see: "Incompatible chain; disconnecting peer"
```

## Verification

### Check Peer Diagnostics (from previous feature)
```powershell
Get-Content .\vision_data_7070\public\peer_store_stats.json | ConvertFrom-Json
Get-Content .\vision_data_7070\public\peer_connect_reasons.json | ConvertFrom-Json
```

Should see:
- `connected: 4-5` (not disconnecting)
- Low or zero `HandshakeFailed_IncompatibleChain` count

### Watch Height Progress
```powershell
# Watch node logs
# Should see height incrementing: 71 → 72 → 73 → ... → 101
```

### Confirm Sync Complete
```
Synced to network height: local=101 network=101
🎯 Mining block #102...
```

## If Still Not Working (Unlikely)

### Nuclear Option: Fresh Sync
```powershell
# Stop node
Stop-Process -Name "vision-node" -Force

# Backup wallet/keys (if you have funds!)
Copy-Item ".\vision_data_7070\mainnet\keys.json" ".\keys_backup.json"

# Delete chain DB only
Remove-Item -Path ".\vision_data_7070\mainnet\*.sled" -Recurse -Force

# Start node - will sync from scratch
.\vision-node.exe
```

**Why this works:**
- Fresh node starts at height 0
- Bootstrap mode active (`height < 128`)
- No conflicting local chain to reorg from
- Clean sync to height 101

**When to use:**
- Only if the new binary STILL fails to sync
- You have no funds on the local chain (or backed up keys)
- You want immediate sync vs debugging further

## What Changed Technically

### File: `src/chain/accept.rs`

**Lines ~290-350: Heaviest tip selection**
- Added ancestry verification loop
- Walks parent chain to verify connection to main chain
- Skips disconnected candidates with debug log
- Safety limit: 10000 blocks max depth check

**Lines ~585-605: Reorg path building**
- Changed "missing parent" from hard error to deferred fetch
- Spawns parent fetch task
- Returns Ok() instead of Err()
- Allows reorg to retry once parent arrives

**Lines ~610-650: Bootstrap mode**
- Already deployed in previous fix
- Allows deep reorgs during initial sync
- Max 2048 vs 36 blocks

## Logs Explained

### `[HEAVIEST-TIP] Skipping disconnected candidate`
**Good**: System is correctly ignoring orphaned blocks that can't be adopted yet.

### `[HEAVIEST-TIP] Updated to connected heavier chain`
**Good**: Found a valid alternative chain that's heavier and connected.

### `[REORG-DEFERRED] Missing parent for candidate tip - fetching`
**Good**: Found a gap in reorg chain, fetching missing parents instead of failing.

### `[REORG-BOOTSTRAP] Initial sync mode active`
**Good**: System detected you're behind, allowing deep reorg for catch-up.

### `[ORPHAN-POOL] processed children of accepted block`
**Good**: After accepting a block, orphaned children are being integrated.

## Testing the Fix

### Scenario 1: Your Current Situation
- Height 71, network at 101
- Bootstrap mode: ACTIVE
- Deep reorg: ALLOWED (69 blocks < 2048)
- Ancestry check: PASS (blocks 1-101 all connected)
- **Result: Should sync successfully**

### Scenario 2: Malicious Deep Reorg (after sync)
- Height 200, malicious peer tries 100-block reorg
- Bootstrap mode: INACTIVE
- Deep reorg: BLOCKED (100 blocks > 36)
- **Result: Rejected, peer marked incompatible**

### Scenario 3: Orphaned Blocks
- Receive block #105 when at height #100
- Parents #101-104 not received yet
- Heaviest tip check: SKIP (not connected)
- Orphan pool: STORED
- Parent fetch: TRIGGERED
- **Result: Once #101-104 arrive, all integrate**

## Performance Impact

**Negligible**:
- Ancestry check: O(depth) pointer chasing
- Typical depth: 1-10 blocks (side chain)
- Max depth: 10000 (safety limit)
- Only runs when heavier side chain detected
- Not in hot path of normal operation

## Summary

**Old Code:**
```
Orphan block → compute as heaviest → try reorg → missing parent → FAIL
```

**New Code:**
```
Orphan block → check connectivity → not connected → skip for now → fetch parent → retry later → SUCCESS
```

**Bottom line:**
The node will now **gracefully handle** incomplete chains instead of treating them as incompatible networks.

**Restart your node with the new binary and it should sync!** 🚀
