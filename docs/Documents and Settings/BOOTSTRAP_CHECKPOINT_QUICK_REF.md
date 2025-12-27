# Bootstrap Checkpoint Quick Reference

## What It Does
**Network Quarantine**: Every node ships with same 10-block prefix. Different prefix = auto-rejected at handshake.

## Constants (`vision_constants.rs`)
```rust
BOOTSTRAP_CHECKPOINT_HEIGHT: u64 = 9;  // Last baked-in block
BOOTSTRAP_CHECKPOINT_HASH: &str = "0x..."; // Hash @ height 9
BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [...]; // All 10 hashes
```

## Handshake Fields (`p2p/connection.rs`)
```rust
pub bootstrap_checkpoint_height: u64,
pub bootstrap_checkpoint_hash: String,
```

## Validation Rules

### Handshake
- ✅ Same checkpoint hash → connect
- ❌ Different checkpoint hash → reject with detailed error
- ❌ Missing checkpoint field (old build) → reject (defaults to zeros)

### Reorg
- ✅ Reorg stays above height 9 → allowed
- ❌ Reorg crosses height 9 → blocked
- ❌ Local checkpoint corrupted → panic with cleanup instructions

## Chain Initialization
```rust
// Empty DB:
bootstrap_with_embedded_prefix() → creates 10 dead blocks

// Dead block characteristics:
- Zero emission (unspendable coinbase)
- sender_pubkey = "BOOTSTRAP_DEAD"
- method = "bootstrap_coinbase"
- Deterministic timestamps (1700000000 + height*60)
```

## Setup Workflow

### 1. Mine Bootstrap Prefix
```powershell
# Fresh node, mine 10 blocks
./vision-node.exe
```

### 2. Export Hashes
```powershell
# Query local API
curl http://localhost:7070/chain/blocks?from=0&to=9

# Extract pow_hash from each block
```

### 3. Update Constants
```rust
// vision_constants.rs
pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [
    "abc123...", // h=0
    "def456...", // h=1
    // ...
    "xyz789...", // h=9
];

pub const BOOTSTRAP_CHECKPOINT_HASH: &str = "xyz789...";
```

### 4. Rebuild & Ship
```powershell
cargo build --release
# Distribute binary
```

## Error Messages

### Handshake Rejection
```
❌ BOOTSTRAP CHECKPOINT MISMATCH
Local:  abc123... @ height 9
Remote: def456... @ height 9

Old testnet builds are automatically quarantined.
```

### Reorg Blocked
```
❌ REFUSING REORG - would cross bootstrap checkpoint
Common ancestor at height 7 < checkpoint 9
```

### Local Corruption
```
❌ LOCAL CHAIN CORRUPTED
Expected: abc123...
Got:      def456...

REQUIRED: Delete chain database and restart.
```

## Files Modified
- `vision_constants.rs` - Constants (Lines 245-275)
- `p2p/connection.rs` - Handshake (Lines 135-136, 385-398, 530-565)
- `p2p/reorg.rs` - Validation (Lines 42-44, 81-120, 166-186)
- `main.rs` - Bootstrap (Lines 3449-3575, 3625-3635)

## Behavior Summary

| Scenario | Result |
|----------|--------|
| New node (empty DB) | Bootstraps with 10 blocks @ height 9 |
| Handshake with same checkpoint | ✅ Connected |
| Handshake with different checkpoint | ❌ Rejected |
| Handshake with old build (zeros) | ❌ Rejected |
| Reorg above height 9 | ✅ Allowed |
| Reorg crossing height 9 | ❌ Blocked |
| Local checkpoint corrupted | ❌ Panic & cleanup required |

## Version
- **v2.2.0-CONSTELLATION**
- Protocol: 2
- Checkpoint Height: 9
- Status: ⚠️ **HASHES NOT YET FILLED** (default zeros)

## Next Steps
1. Mine 10 blocks on fresh node
2. Export hashes using script
3. Fill `BOOTSTRAP_BLOCK_HASHES` array
4. Fill `BOOTSTRAP_CHECKPOINT_HASH` constant
5. Rebuild and distribute
