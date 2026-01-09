# Connection Timeout Fix - CRITICAL PATCH

## Problem Identified

**Root Cause**: TCP connection attempts to unreachable seed peers were blocking for 21+ seconds (Windows OS default timeout), preventing the node from trying subsequent seeds in the same 30-second connection cycle.

**Symptom**: All 7 seed operators reported the same behavior:
- Node tries first available seed peer
- Connection times out after ~20 seconds  
- Node doesn't move to next seed in the list
- 30-second cycle repeats with same behavior

## Technical Details

**File**: `src/p2p/connection.rs` line 2498

**Before**:
```rust
let stream = match TcpStream::connect(&dial_addr).await {
    Ok(s) => s,
    Err(e) => { /* ... */ }
};
```

**After**:
```rust
use tokio::time::{timeout, Duration};
let connect_result = timeout(Duration::from_secs(5), TcpStream::connect(&dial_addr)).await;
let stream = match connect_result {
    Ok(Ok(s)) => s,           // Connected successfully
    Ok(Err(e)) => { /* ... */ }, // Connection error
    Err(_) => { /* ... */ }   // 5-second timeout
};
```

## Impact

**Before Fix**:
- Connection attempt: 21 seconds (OS default)
- Attempts per 30s cycle: 1 seed maximum
- Time to try all 7 seeds: 7 × 30s = 3.5 minutes minimum

**After Fix**:
- Connection attempt: 5 seconds (explicit timeout)
- Attempts per 30s cycle: 3 seeds (MAX_NEW_DIALS_PER_CYCLE)
- Time to try all 7 seeds: ~1 minute (3 attempts every 30s)

## Why This Fixes the Issue

1. **Sequential Blocking**: Connection attempts are sequential (`.await` in loop)
2. **Cycle Time Budget**: 30-second maintenance cycle
3. **21s Timeout**: First seed timeout consumed 70% of cycle time
4. **5s Timeout**: Now 3 seeds × 5s = 15s, leaving 15s buffer

With all 7 seed operators starting simultaneously:
- All firewalls are open (confirmed)
- All nodes have correct VISION_EXTERNAL_IP set
- Some seeds will connect within 5s
- Others timeout quickly and move to next seed
- Network forms within 1-2 minutes instead of never

## Deployment Instructions

**Package**: `vision-node-v1.0-windows-mainnet-CONNECTION-TIMEOUT-FIX.zip`

1. Stop all running nodes
2. Replace `vision-node.exe` with new binary
3. Coordinate restart of all 7 seed operators (within 5-minute window)
4. Check logs for `✅ Connected to seed: <ip>:<port>`
5. Expected: 3-6 connections per seed within 2 minutes

## Testing Verification

Test by checking logs after startup:

```
[CONN_MAINTAINER] 🌱 Trying 7 seed peers
[p2p::connect] Attempting seed connection (maintainer) seed=16.163.123.221:7072
[p2p::connect] Connection attempt timed out after 5 seconds  // <-- FAST TIMEOUT
[CONN_MAINTAINER] 📖 Trying peers from peer store
[p2p::connect] Attempting seed connection (maintainer) seed=69.173.206.211:7072
[p2p::connect] ✅ Connected to seed: 69.173.206.211:7072      // <-- SUCCESS
```

## Notes

- **No configuration changes needed** - same startup scripts work
- **Backward compatible** - regular operators can upgrade anytime
- **Coordination required** - All 7 seed operators should coordinate restart
- Seeds that timeout after 5s are skipped, allowing network formation
- Once 2-3 seeds connect to each other, the network stabilizes rapidly

## Version

- **Build**: Release + full features
- **File Modified**: src/p2p/connection.rs
- **Commit**: Connection timeout fix (5s explicit timeout)
- **Date**: 2025-01-XX
