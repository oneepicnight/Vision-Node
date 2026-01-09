# 🔍 CRITICAL DIAGNOSTIC: Chain Height Mismatch Investigation

## Problem Statement

User observed in logs:
- **Node received compact block at height 19**
- **But miner manager logs show `chain_tip_height=0` and mines height=1**

This suggests the miner manager and block validator are reading **different chain states**.

---

## What to Check in the 5 PowerShell Node Windows

### ✅ Node 7070 (MINER) Window

Look for these log patterns:

```
[MINER-JOB] Created mining job chain_tip_height=X chain_tip_hash=0xYYY job_height=Z
```

**Questions:**
1. What is `chain_tip_height`? Is it **0** or is it **increasing** (1, 2, 3...)?
2. What is `job_height`? Should be `chain_tip_height + 1`
3. After `[MINER-FOUND]`, do you see a line like:
   ```
   ✅ Block #N integrated into chain
   ```
4. Is N increasing (1, 2, 3...) or stuck at 1?

**Expected if working correctly:**
- `chain_tip_height` starts at 0, then 1, 2, 3...
- `job_height` starts at 1, then 2, 3, 4...
- After mining, you see "Block #1 integrated", then "Block #2 integrated", etc.

**🚨 Problem indicator:**
- `chain_tip_height` stays at **0** forever
- `job_height` stays at **1** forever  
- But validators (below) are seeing blocks at height 10, 20, 30...

---

### ✅ Node 8080/9090/10100/11110 (VALIDATOR) Windows

Look for these patterns:

```
[COMPACT_BLOCK] Received compact block height=X from peer=...
✅ Block #X integrated into chain
```

**Questions:**
1. What heights are you seeing? 1, 2, 3... or higher like 10, 20, 30?
2. Are they receiving blocks continuously?
3. After "Received compact block", do you see "Block #X integrated"?

**Expected if working correctly:**
- Validators see: "Received compact block height=1", then 2, 3, 4...
- Followed by: "Block #1 integrated", "Block #2 integrated"...

**🚨 Problem indicator:**
- Validators show heights like 19, 20, 21... 
- But miner (node 7070) is still at chain_tip_height=0

---

## Root Cause Hypothesis

### Theory A: Database Scope Mismatch
- Miner is writing to database scope "mainnet"
- Validators are writing to database scope "testnet" (or vice versa)
- **Fix:** Check `chain_db_scope` in logs - must match on all nodes

### Theory B: Stale Chain Lock in Miner
- Miner manager's loop at `main.rs:5041` reads:
  ```rust
  let height = last_block.header.number + 1;
  ```
- But it's reading from an OLD/cached chain state
- While block integrator at `chain/accept.rs:298` is updating the REAL chain:
  ```rust
  g.blocks.push(blk.clone());
  ```
- **Fix:** Miner needs to re-acquire `CHAIN.lock()` after blocks are integrated

### Theory C: Port-Based Database Isolation
- Each node uses: `vision_data_{port}` directory
- Node 7070 → `vision_data_7070`
- Node 8080 → `vision_data_8080`
- Miner at 7070 mines blocks and integrates them to its OWN database
- Validators at 8080/9090/etc receive blocks via P2P and integrate to THEIR databases
- **This is EXPECTED behavior for multi-node testing**
- **Problem:** Miner should see its own height increasing in its database

---

## Diagnostic Commands to Run

### Check Chain Heights via API

```powershell
# Check miner status
(Invoke-WebRequest -Uri "http://127.0.0.1:7070/api/miner/status" -UseBasicParsing).Content | ConvertFrom-Json | Select-Object blocks_found, last_block_height

# Check if there's a chain/height endpoint
(Invoke-WebRequest -Uri "http://127.0.0.1:7070/api/chain/info" -UseBasicParsing -ErrorAction SilentlyContinue).Content
```

### Check Database Files

```powershell
# Node 7070 database
Get-ChildItem c:\vision-node\vision_data_7070\*.sled | Select-Object Name, Length

# Node 8080 database
Get-ChildItem c:\vision-node\vision_data_8080\*.sled | Select-Object Name, Length
```

### Grep Node 7070 Window for Height Progression

In the Node 7070 PowerShell window, press Ctrl+F and search for:
- `chain_tip_height=`
- `Block #` 
- `integrated into chain`

Count how many times you see `chain_tip_height=0` vs `chain_tip_height=1`, `chain_tip_height=2`, etc.

---

## Expected vs Actual Behavior

### ✅ EXPECTED (Working Correctly)

**Node 7070 (miner):**
```
[MINER-JOB] ... chain_tip_height=0 job_height=1
[MINER-FOUND] Solution found height=1
✅ Block #1 integrated into chain
[MINER-JOB] ... chain_tip_height=1 job_height=2
[MINER-FOUND] Solution found height=2
✅ Block #2 integrated into chain
[MINER-JOB] ... chain_tip_height=2 job_height=3
```

**Node 8080 (validator):**
```
[COMPACT_BLOCK] Received compact block height=1 from peer=7072
✅ Block #1 integrated into chain
[COMPACT_BLOCK] Received compact block height=2 from peer=7072
✅ Block #2 integrated into chain
```

### 🚨 ACTUAL (If Broken)

**Node 7070 (miner):**
```
[MINER-JOB] ... chain_tip_height=0 job_height=1
[MINER-FOUND] Solution found height=1
✅ Block #1 integrated into chain
[MINER-JOB] ... chain_tip_height=0 job_height=1  ← STUCK AT 0!
[MINER-FOUND] Solution found height=1
✅ Block #1 integrated into chain
[MINER-JOB] ... chain_tip_height=0 job_height=1  ← STILL STUCK!
```

**Node 8080 (validator):**
```
[COMPACT_BLOCK] Received compact block height=19 from peer=7072  ← HIGH NUMBER!
✅ Block #19 integrated into chain
```

This would confirm the miner is **not seeing its own blocks** after integrating them.

---

## Fix Strategy

### If miner chain_tip_height is stuck at 0:

The miner job creation loop needs to **re-read the chain** after blocks are integrated.

**Problem location:** `src/main.rs:5041`
```rust
let height = last_block.header.number + 1;
```

**Potential issue:** The miner loop acquires the lock, reads the height, releases the lock, then sleeps for 2 seconds. During those 2 seconds, blocks can be integrated, but the miner doesn't see them until the next loop iteration.

**Possible fix:** Add a channel/signal to notify the miner loop when a new block is integrated, so it immediately updates its job instead of waiting 2 seconds.

---

## Action Items

1. **Check Node 7070 window** - Find `chain_tip_height` values in logs
2. **Check Node 8080 window** - Find `Received compact block height=` values  
3. **Report back:** Are heights progressing or stuck?
4. **Database check:** Verify all nodes use same database scope
5. **Code review:** Check if miner loop is reading stale chain state

---

Generated: 2025-12-31
