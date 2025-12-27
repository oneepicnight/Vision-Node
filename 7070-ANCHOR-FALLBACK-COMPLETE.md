# 7070 Anchor Fallback Implementation

## ✅ Complete - All Changes Applied

### Summary
Nodes now always have anchors to probe on HTTP 7070, even without VISION_ANCHOR_SEEDS set in .env.

---

## Changes Made

### 1. Added `default_anchor_seeds()` Helper
**File:** `src/p2p/seed_peers.rs`

```rust
/// Get default anchor seeds for 7070 HTTP control plane
/// Returns the same IPs from INITIAL_SEEDS (without ports)
/// The control plane will add :7070 automatically
pub fn default_anchor_seeds() -> Vec<String> {
    INITIAL_SEEDS
        .iter()
        .map(|(ip, _port)| ip.to_string())
        .collect()
}
```

**Behavior:**
- Takes genesis seed IPs: `("69.173.206.211", 7072)`
- Returns IP-only: `"69.173.206.211"`
- Control plane adds `:7070` automatically

---

### 2. Updated `parse_anchor_seeds()` with Fallback
**File:** `src/control_plane.rs`

**Old Behavior:**
- Read `VISION_ANCHOR_SEEDS` env var
- Return empty list if not set
- Probe loop would wait indefinitely

**New Behavior:**
- Read `VISION_ANCHOR_SEEDS` env var first
- If set → use it (with logging)
- If empty → fallback to `default_anchor_seeds()`
- Always returns anchors (never empty)

**Logging:**
```
[BACKBONE] Using 7 anchors from VISION_ANCHOR_SEEDS (probing HTTP 7070): 16.163.123.221, ...
[BACKBONE] Using 7 default anchor seeds (probing HTTP 7070): 16.163.123.221, ...
```

---

### 3. Added Explicit VISION_ANCHOR_SEEDS to .env
**File:** `.env`

```bash
# 🌐 HTTP CONTROL PLANE ANCHORS (7070) - v2.7.0+
# Comma-separated list of anchor IPs (ports auto-added as :7070)
# Falls back to genesis seed IPs if not set
VISION_ANCHOR_SEEDS=16.163.123.221,69.173.206.211,69.173.207.135,74.125.212.204,75.128.156.69,98.97.137.74,182.106.66.15
```

**Note:** No ports in the list - control plane adds `:7070` automatically.

---

### 4. Boot Logging
**On Startup:**

```
[BACKBONE] 🌐 Starting 7070 control plane probe loop
[BACKBONE] Using 7 anchors from VISION_ANCHOR_SEEDS (probing HTTP 7070): 16.163.123.221, ...
[BACKBONE] ✅ Connected to http://16.163.123.221:7070 (45ms) - tip=12345 peers=8
```

**Or if connection fails:**
```
[BACKBONE] ⚠️ Failed to probe http://16.163.123.221:7070: ...
[BACKBONE] ⚠️ Failed to probe http://69.173.206.211:7070: ...
[BACKBONE] ⚠️ All anchors unreachable - retrying in 10s
```

---

## Quick Sanity Check

### Expected Startup Sequence (within 5 seconds):

1. **Probe Loop Starts:**
   ```
   [BACKBONE] 🌐 Starting 7070 control plane probe loop
   ```

2. **Anchors Resolved:**
   ```
   [BACKBONE] Using 7 anchors from VISION_ANCHOR_SEEDS (probing HTTP 7070): ...
   ```
   OR
   ```
   [BACKBONE] Using 7 default anchor seeds (probing HTTP 7070): ...
   ```

3. **Connection Result:**
   - **Success:** `[BACKBONE] ✅ Connected to http://X.X.X.X:7070 ...`
   - **Failure:** `[BACKBONE] ⚠️ Failed to probe ...` (but at least it tries)

---

## Key Benefits

1. **No Silent Failures:** Nodes always attempt 7070 connections
2. **Clear Logging:** Can't confuse HTTP 7070 with P2P 7072
3. **Explicit Configuration:** .env shows exactly what's being probed
4. **Fallback Safety:** Works even if .env is missing VISION_ANCHOR_SEEDS

---

## Testing

### Test 1: With Explicit Anchors
```powershell
# .env has VISION_ANCHOR_SEEDS set
.\target\release\vision-node.exe
```

**Expected Log:**
```
[BACKBONE] Using 7 anchors from VISION_ANCHOR_SEEDS (probing HTTP 7070): ...
```

### Test 2: Without Explicit Anchors
```powershell
# Comment out VISION_ANCHOR_SEEDS in .env
# VISION_ANCHOR_SEEDS=...
.\target\release\vision-node.exe
```

**Expected Log:**
```
[BACKBONE] Using 7 default anchor seeds (probing HTTP 7070): 16.163.123.221, ...
```

### Test 3: Connection Success
**Within 5-10 seconds:**
```
[BACKBONE] ✅ Connected to http://16.163.123.221:7070 (45ms) - tip=12345 peers=8
```

---

## Build Status

```
✅ cargo check - Finished successfully
✅ All changes applied
✅ Ready for testing
```

---

## Files Modified

- `src/p2p/seed_peers.rs` - Added `default_anchor_seeds()`
- `src/control_plane.rs` - Updated `parse_anchor_seeds()` with fallback + logging
- `.env` - Added explicit `VISION_ANCHOR_SEEDS` with all 7 genesis IPs

---

**Version:** v2.7.0+  
**Status:** ✅ Complete  
**Next Step:** Test startup and verify 7070 connection logs
