# POW_MESSAGE_BYTES STABILITY FIX

## Problem
The `pow_message_bytes()` function was using `.as_bytes()` directly on hash string fields (parent_hash, state_root, tx_root, receipts_root, da_commitment). This caused **consensus forks** because:

1. **Locally-mined blocks** use `hex32()` which produces lowercase hex without "0x" prefix
2. **Blocks from P2P/JSON** could have "0x" prefix or uppercase hex
3. **Different string encodings** = different PoW message = different digest = fork!

Example fork scenario:
```rust
// Node A mines block:
parent_hash = "abc123..."  // from hex32()
msg_bytes = "abc123...".as_bytes()
digest_A = visionx_hash(msg_bytes, nonce)

// Node B receives same block from JSON with "0x" prefix:
parent_hash = "0xabc123..."  // from serde_json deserialization
msg_bytes = "0xabc123...".as_bytes()  // DIFFERENT!
digest_B = visionx_hash(msg_bytes, nonce)  // DIFFERENT!

// Result: Node B rejects block, fork occurs
```

## Solution
Added `normalize_hash()` function that:
1. Strips "0x" or "0X" prefix if present
2. Converts to lowercase
3. Applied to all hash fields before encoding

## Changes
**File:** `src/main.rs`

**Added function (before pow_message_bytes):**
```rust
/// Normalize hash string: strip "0x" prefix, convert to lowercase.
/// Ensures deterministic encoding across different JSON sources (Windows vs Linux).
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

**Modified pow_message_bytes():**
- All hash fields now call `normalize_hash()` before `.as_bytes()`
- parent_hash: `normalize_hash(&h.parent_hash)`
- state_root: `normalize_hash(&h.state_root)`
- tx_root: `normalize_hash(&h.tx_root)`
- receipts_root: `normalize_hash(&h.receipts_root)`
- da_commitment: `normalize_hash(da)` if present

## Verification
**Test file:** `pow_message_bytes_test.rs`

**Tests:**
1. ✅ Same block with/without "0x" prefix produces identical message bytes
2. ✅ Uppercase hex normalized to lowercase
3. ✅ Big-endian numeric encoding verified
4. ✅ Optional da_commitment handled correctly

**Test results:**
```
=== pow_message_bytes Cross-Platform Stability Test ===

Test 1: Hash prefix variations
  No prefix:    80 bytes
  With '0x':    80 bytes
  Uppercase:    80 bytes
  ✅ PASS: All three encodings are identical

Test 2: Parent hash encoding breakdown
  h1.parent_hash = "abc123"
  h2.parent_hash = "0xabc123"
  h3.parent_hash = "0xABC123"
  After normalize:
    norm1 = "abc123"
    norm2 = "abc123"
    norm3 = "abc123"
  ✅ PASS: All normalize to same value

Test 3: Numeric field endianness
  u64 0x0102030405060708 -> BE bytes: [01, 02, 03, 04, 05, 06, 07, 08]
  ✅ PASS: Big-endian encoding correct

Test 4: Optional field handling
  No da_commitment:   68 bytes
  With da_commitment: 77 bytes
  ✅ PASS: da_commitment changes message length correctly

=== ALL TESTS PASSED ===
```

## Cross-Platform Guarantee
The test can be run on Windows and Linux to verify identical output:
```powershell
# Windows
rustc --edition 2021 pow_message_bytes_test.rs -o pow_message_bytes_test.exe
.\pow_message_bytes_test.exe

# Linux
rustc --edition 2021 pow_message_bytes_test.rs -o pow_message_bytes_test
./pow_message_bytes_test
```

Both should produce identical message bytes and pass all tests.

## Impact
- **Before fix:** Risk of fork from "0x" prefix or case variations in P2P blocks
- **After fix:** All blocks produce identical PoW digest regardless of JSON formatting
- **Breaking change:** None - all existing locally-mined blocks already use normalized format

## Related Code
- `hex32()` (line 5306): Produces normalized format (lowercase, no "0x")
- `chain::accept::apply_block()` (line 44): Calls `pow_message_bytes()` for validation
- BlockHeader deserialization: Uses `#[derive(Deserialize)]` without custom deserializers

## Audit Checklist
✅ Field order stable (hardcoded)
✅ Endianness fixed (big-endian for all numeric fields)
✅ No string formatting/JSON/debug in message
✅ Excludes pow_hash (correct - it's the output)
✅ Includes all critical fields (parent, state_root, tx_root, receipts_root, number, timestamp, difficulty)
✅ Hash normalization prevents fork from string format variations
✅ Test vectors pass on Windows (verified)
⏳ Test vectors pass on Linux (can be verified by running pow_message_bytes_test)

## Production Deployment
1. Deploy updated binary to all nodes
2. Monitor for PoW validation errors (should be none)
3. Run cross-platform test on representative machines
4. Verify no fork events in metrics

## Future Improvements
Consider normalizing hash strings at deserialization time (custom serde deserializer) to catch issues earlier.
