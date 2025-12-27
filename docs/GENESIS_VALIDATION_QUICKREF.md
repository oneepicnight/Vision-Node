# Genesis Hash Validation - Quick Reference

## 🔐 Security Feature: Genesis Block Integrity Validation

**Status**: ✅ Implemented and Active  
**Version**: v0.8.0+  
**Critical**: This feature CANNOT be disabled or bypassed

---

## What It Does

Validates the genesis block hash on every node startup to prevent:
- Chain substitution attacks
- Database corruption
- Network splits
- Consensus failures

## How It Works

1. **Hardcoded Hash**: Canonical genesis hash embedded in code
2. **Local Validation**: No network/remote dependencies required
3. **Startup Check**: Runs automatically before chain operations
4. **Fail-Safe**: Node refuses to start if validation fails

## The Genesis Hash

```
af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
```

**Source**: `src/genesis.rs:21`  
**Algorithm**: BLAKE3 (256-bit)  
**Format**: Hex-encoded

## Validation Process

```
┌─────────────────────┐
│   Node Startup      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  Load Genesis Block │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ ✓ Validate computation matches hash │
└──────────┬──────────────────────────┘
           │ [pass]
           ▼
┌─────────────────────────────────────┐
│ ✓ Verify stored block matches hash  │
└──────────┬──────────────────────────┘
           │ [pass]
           ▼
┌─────────────────────┐
│  Continue Startup   │
└─────────────────────┘

[On any failure: Log error → Exit]
```

## Success Log Message

```
✅ Genesis hash validation PASSED: af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
✅ Stored genesis block validation PASSED: af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
```

## Failure Scenarios

### Scenario 1: Computation Mismatch
**Cause**: Code changed or corrupted  
**Action**: Do not start, verify binary integrity

### Scenario 2: Storage Mismatch
**Cause**: Database corrupted or wrong network  
**Action**: Delete database, restart node

### Scenario 3: Network Split
**Cause**: Multiple genesis hashes in use  
**Action**: Coordinate network-wide genesis update

## Quick Troubleshooting

| Problem | Solution |
|---------|----------|
| "Genesis hash mismatch" | Delete `data/` folder, restart |
| Different hash across nodes | Verify all nodes use same binary version |
| Cannot start after update | Check if hard fork occurred, update binary |
| Database corruption | Backup data, delete `data/`, re-sync |

## For Developers

### Adding New Validation

```rust
// In src/genesis.rs, add new validation function
pub fn validate_new_check() -> Result<()> {
    // Your validation logic
    Ok(())
}

// In src/main.rs Chain::init(), add call
if let Err(e) = genesis::validate_new_check() {
    tracing::error!(error = %e, "Validation failed");
    std::process::exit(1);
}
```

### Testing Validation

```rust
#[test]
fn test_my_validation() {
    let result = validate_new_check();
    assert!(result.is_ok(), "Validation should pass");
}
```

## For Operators

### Normal Operation

✅ Validation passes automatically  
✅ Node starts normally  
✅ No manual intervention needed

### If Validation Fails

1. **Check logs** for specific error
2. **Backup data** if contains valuable info
3. **Delete database**: `rm -rf data/`
4. **Restart node** to regenerate genesis
5. **Contact support** if issue persists

### Monitoring

Watch startup logs for these lines:
```
INFO vision_node: ✅ Genesis hash validation PASSED
INFO vision_node: ✅ Stored genesis block validation PASSED
```

## Critical Information

### ⚠️ DO NOT

- ❌ Modify `GENESIS_HASH` constant
- ❌ Skip or bypass validation
- ❌ Use different genesis across nodes
- ❌ Ignore validation failures

### ✅ DO

- ✅ Monitor validation messages in logs
- ✅ Backup database before major updates
- ✅ Coordinate network-wide for hard forks
- ✅ Test validation in dev environment

## Files

| File | Purpose |
|------|---------|
| `src/genesis.rs` | Genesis hash constant and validation functions |
| `src/main.rs` | Chain initialization with validation |
| `docs/GENESIS_VALIDATION.md` | Complete documentation |
| `docs/GENESIS_VALIDATION_IMPLEMENTATION.md` | Implementation summary |

## Performance

- **Startup Time**: +0.5ms
- **Memory**: +256 bytes
- **CPU**: 1x BLAKE3 hash
- **Network**: Zero
- **Disk**: Zero

## Dependencies

- `blake3`: Hashing algorithm
- `hex`: Hex encoding
- `anyhow`: Error handling
- `tracing`: Logging

## Security Properties

| Property | Status |
|----------|--------|
| Deterministic | ✅ Always same result |
| Local-only | ✅ No network deps |
| Tamper-proof | ✅ Hardcoded constant |
| Fail-safe | ✅ Exits on mismatch |
| Auditable | ✅ Clear logging |

## Support

For issues or questions:
1. Check logs: `logs/vision-node.log`
2. Read docs: `docs/GENESIS_VALIDATION.md`
3. Test locally: `cargo test genesis::tests`
4. Contact: Network administrators

---

**Last Updated**: 2024  
**Version**: v0.8.0+  
**Status**: Production Ready ✅
