# FORK PREVENTION QUICK REF

## Critical Fixes Applied

### 1. Block Acceptance - Single Track Only ✅
**Rule:** ALL blocks MUST go through `chain::accept::apply_block()`

**Violation Example:**
```rust
// ❌ NEVER DO THIS
g.blocks.push(block);
```

**Correct Pattern:**
```rust
// ✅ ALWAYS DO THIS
chain::accept::apply_block(g, &block)?;
```

**Files to Check:**
- `src/main.rs` - execute_and_mine() (line 12100)
- `src/p2p/routes.rs` - All block reception handlers
- `src/p2p/connection.rs` - Compact block handling

---

### 2. Reorg Orchestration - Single Source of Truth ✅
**Rule:** ONLY `apply_block()` performs reorgs

**Violation Example:**
```rust
// ❌ NEVER DO THIS
handle_reorg(g, &block)?;
```

**Correct Pattern:**
```rust
// ✅ ALWAYS DO THIS
chain::accept::apply_block(g, &block)?;
// apply_block handles reorg internally if needed
```

**Deprecated Functions:**
- `handle_reorg()` - DO NOT USE

---

### 3. Sync Resilience - Continue on Failure ✅
**Rule:** Sync MUST continue through single block failures

**Violation Example:**
```rust
// ❌ NEVER DO THIS
if fetch_fails {
    return Err("abort entire sync");
}
```

**Correct Pattern:**
```rust
// ✅ ALWAYS DO THIS
if fetch_fails {
    warn!("Block {} failed, continuing", height);
    failed_blocks.push(height);
    continue;
}
```

**Files to Check:**
- `src/main.rs` - sync_pull() (line 13336)

---

### 4. VisionX Params - Consensus Only ✅
**Rule:** ALL validation MUST use `VISIONX_CONSENSUS_PARAMS`

**Violation Example:**
```rust
// ❌ NEVER DO THIS
let params = VisionXParams::default();
```

**Correct Pattern:**
```rust
// ✅ ALWAYS DO THIS
let params = consensus_params_to_visionx(&VISIONX_CONSENSUS_PARAMS);
```

**Files to Check:**
- `src/chain/accept.rs` - Block validation (line 26)
- `src/pool/worker_client.rs` - PoolWorker (line 51)
- Any new PoW validation code

---

### 5. PoW Message Encoding - Normalize Hashes ✅
**Rule:** ALL hash strings MUST be normalized before encoding

**Violation Example:**
```rust
// ❌ NEVER DO THIS
let bytes = h.parent_hash.as_bytes();  // May have "0x" prefix
```

**Correct Pattern:**
```rust
// ✅ ALWAYS DO THIS
let normalized = normalize_hash(&h.parent_hash);
let bytes = normalized.as_bytes();
```

**Helper Function:**
```rust
#[inline]
fn normalize_hash(s: &str) -> String {
    let trimmed = if s.starts_with("0x") || s.starts_with("0X") {
        &s[2..]
    } else {
        s
    };
    trimmed.to_lowercase()
}
```

**Files to Check:**
- `src/main.rs` - pow_message_bytes() (line 5313)

---

## Testing

### Cross-Platform Stability Test
```powershell
# Windows
rustc --edition 2021 pow_message_bytes_test.rs -o pow_message_bytes_test.exe
.\pow_message_bytes_test.exe

# Linux
rustc --edition 2021 pow_message_bytes_test.rs -o pow_message_bytes_test
./pow_message_bytes_test
```

**Expected Output:**
```
=== pow_message_bytes Cross-Platform Stability Test ===
Test 1: Hash prefix variations
  ✅ PASS: All three encodings are identical
Test 2: Parent hash encoding breakdown
  ✅ PASS: All normalize to same value
Test 3: Numeric field endianness
  ✅ PASS: Big-endian encoding correct
Test 4: Optional field handling
  ✅ PASS: da_commitment changes message length correctly
=== ALL TESTS PASSED ===
```

---

## Code Review Checklist

When reviewing new code, check for:

- [ ] All block inserts go through `apply_block()`
- [ ] No direct calls to `handle_reorg()` (deprecated)
- [ ] Sync loops use `continue` on failure, not `return Err`
- [ ] PoW validation uses `VISIONX_CONSENSUS_PARAMS`
- [ ] Hash strings normalized before `.as_bytes()`
- [ ] No "0x" prefix assumptions in PoW message
- [ ] Big-endian encoding for all numeric fields in PoW message
- [ ] No JSON/string formatting in PoW message

---

## Production Monitoring

### Key Metrics to Watch

**Fork Detection:**
- Chain height consistency across nodes
- Cumulative work divergence
- PoW validation failure rate

**Sync Health:**
- Sync completion rate
- Failed block retry rate
- Peer rejection rate

**PoW Validation:**
- Digest mismatch rate (should be 0)
- Target failure rate (normal for valid PoW)
- Validation latency

### Alert Conditions

**Critical:**
- PoW digest mismatch > 0/hour → **FORK RISK**
- Chain height divergence > 5 blocks → **FORK DETECTED**
- Cumulative work divergence > 1% → **FORK DETECTED**

**Warning:**
- Sync failure rate > 10% → Check network
- Peer rejection rate > 20% → Check P2P protocol
- Validation latency > 1s → Check dataset cache

---

## Rollback Procedure

If fork detected:

1. **Immediate:**
   ```powershell
   # Stop all nodes
   Stop-Process -Name "vision-node" -Force
   
   # Restore previous binary
   Copy-Item "vision-node.backup.exe" "vision-node.exe"
   
   # Restart nodes
   .\vision-node.exe
   ```

2. **Investigation:**
   - Check logs for "pow_hash mismatch" errors
   - Compare digests across nodes
   - Verify hash string formats in database

3. **Fix and Redeploy:**
   - Identify root cause
   - Add test case
   - Re-run all tests
   - Deploy with monitoring

---

## Reference

**Documentation:**
- [POW_MESSAGE_BYTES_STABILITY_FIX.md](POW_MESSAGE_BYTES_STABILITY_FIX.md) - Detailed fix
- [FORK_PREVENTION_AUDIT_FINAL_SUMMARY.md](FORK_PREVENTION_AUDIT_FINAL_SUMMARY.md) - Complete audit

**Test Files:**
- [pow_message_bytes_test.rs](pow_message_bytes_test.rs) - Cross-platform test

**Key Functions:**
- `chain::accept::apply_block()` - Single block acceptance entry point
- `normalize_hash()` - Hash string normalization
- `pow_message_bytes()` - Stable PoW message encoding
- `sync_pull()` - Resilient block sync

**Constants:**
- `VISIONX_CONSENSUS_PARAMS` - Authoritative consensus parameters
  - dataset_size: 256MB (268435456 bytes)
  - scratchpad_size: 32MB (33554432 bytes)
  - mix_iters: 65536

---

**Last Updated:** 2024-01-XX  
**Version:** Vision Node v3.0.0  
**Status:** Production Ready ✅
