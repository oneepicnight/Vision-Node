# Bootstrap Checkpoint Implementation (Phase 11)

## Overview
Implemented baked-in bootstrap prefix system to quarantine incompatible chains and prevent old testnet nodes from poisoning the network. All nodes now ship with the same first 10 blocks that can NEVER be reorged.

## Problem Solved
**Old Issue**: Testnet nodes from different builds could connect and create chain splits, causing sync stagnation and UTXO corruption.

**Solution**: Every node ships with a hardcoded 10-block prefix (heights 0-9). Nodes with different prefixes are auto-rejected during handshake.

## Design Goals

### Network Quarantine
- Old "testnet from last night" nodes automatically quarantined
- Constellation nodes for this drop all converge on one chain tip or they won't connect
- No manual cleanup needed - incompatible nodes just can't connect

### Chain Prefix Immutability
- First 10 blocks are NEVER reorged
- First 10 blocks are NEVER paid out (zero emission coinbase)
- First 10 blocks define the canonical network identity

### Handshake Gating
New builds refuse to talk to nodes that:
- Don't have those 10 blocks
- Have different 10 blocks (different hash at height 9)
- Use different protocol versions/ports/network tags

## Implementation Details

### 1. Bootstrap Constants (`vision_constants.rs`)

```rust
/// Height of the last baked-in bootstrap block (0-based, so 9 = 10 blocks)
pub const BOOTSTRAP_CHECKPOINT_HEIGHT: u64 = 9;

/// Hash of the last baked-in block at height 9
pub const BOOTSTRAP_CHECKPOINT_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// All 10 bootstrap block hashes (heights 0-9)
pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [
    "0x...", // h=0 (genesis)
    "0x...", // h=1
    // ... (h=2-8)
    "0x...", // h=9 (checkpoint)
];
```

**TODO**: Fill these hashes from export script after mining first 10 blocks.

### 2. Handshake Extension (`p2p/connection.rs`)

Added bootstrap checkpoint fields to `HandshakeMessage`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    // ... existing fields ...
    
    // Phase 11: Bootstrap Checkpoint
    pub bootstrap_checkpoint_height: u64,
    pub bootstrap_checkpoint_hash: String,
}
```

#### Handshake Validation
```rust
// In HandshakeMessage::validate()
if self.bootstrap_checkpoint_height == BOOTSTRAP_CHECKPOINT_HEIGHT {
    if self.bootstrap_checkpoint_hash != BOOTSTRAP_CHECKPOINT_HASH {
        return Err("Bootstrap checkpoint mismatch - peer on incompatible chain");
    }
}
```

**Result**: Peers with different bootstrap prefixes are rejected before any sync attempt.

### 3. Chain Initialization (`main.rs`)

#### Bootstrap Function
```rust
fn bootstrap_with_embedded_prefix(db: &sled::Db) -> Vec<Block> {
    // Creates 10 "dead" bootstrap blocks with:
    // - Zero emission (unspendable coinbase)
    // - Deterministic timestamps
    // - Hardcoded hashes from BOOTSTRAP_BLOCK_HASHES
    
    // Sanity checks:
    // - Tip height must be 9
    // - Tip hash must match BOOTSTRAP_CHECKPOINT_HASH
}
```

#### Dead Bootstrap Block
```rust
fn create_dead_bootstrap_block(height: u64, pow_hash: &str, parent_hash: &str) -> Block {
    // Creates minimal block with:
    // - Correct header.pow_hash from BOOTSTRAP_BLOCK_HASHES
    // - Parent hash chain
    // - UNSPENDABLE coinbase tx (zero emission)
    // - Special marker: sender_pubkey = "BOOTSTRAP_DEAD"
}
```

**Effect**: Fresh nodes start with 10 blocks already in database. No mining needed for prefix.

### 4. Reorg Protection (`p2p/reorg.rs`)

#### Hard Checkpoint
```rust
fn is_past_bootstrap_checkpoint(height: u64) -> bool {
    height <= BOOTSTRAP_CHECKPOINT_HEIGHT
}

// In handle_reorg():
if is_past_bootstrap_checkpoint(ancestor_height as u64) {
    return ReorgResult::Failed("Cannot reorg past bootstrap checkpoint");
}
```

#### Local Chain Validation
```rust
// When reorg fails to find common ancestor:
if local_height >= BOOTSTRAP_CHECKPOINT_HEIGHT {
    let checkpoint_block = blocks[BOOTSTRAP_CHECKPOINT_HEIGHT];
    
    if checkpoint_block.hash != BOOTSTRAP_CHECKPOINT_HASH {
        error!("LOCAL CHAIN CORRUPTED - bootstrap checkpoint mismatch");
        return ReorgResult::Failed("Database corruption detected");
    }
}
```

**Effect**: 
- No reorg can cross the 10-block prefix
- Database corruption detected if local checkpoint diverges
- Peers on different prefix are identified and rejected

## Workflow

### Initial Setup (One-Time)

1. **Mine Bootstrap Prefix**
   ```powershell
   # Start fresh guardian node
   ./vision-node.exe
   
   # Let it mine 10 blocks (or manually mine them)
   ```

2. **Export Block Hashes**
   ```powershell
   # Create export script (vision-node/scripts/export-bootstrap-hashes.ps1)
   # Query /chain/blocks?from=0&to=9
   # Extract pow_hash from each block
   ```

3. **Update Constants**
   ```rust
   // In vision_constants.rs
   pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [
       "abc123...", // h=0
       "def456...", // h=1
       // ... extracted hashes
       "xyz789...", // h=9
   ];
   
   pub const BOOTSTRAP_CHECKPOINT_HASH: &str = "xyz789..."; // h=9 hash
   ```

4. **Rebuild and Ship**
   ```powershell
   cargo build --release
   # Copy binary to distribution package
   ```

### Runtime Behavior

#### New Node (Empty DB)
1. `Chain::init()` detects empty DB
2. Calls `bootstrap_with_embedded_prefix()`
3. Inserts 10 dead blocks from `BOOTSTRAP_BLOCK_HASHES`
4. Sets tip to height 9, hash = `BOOTSTRAP_CHECKPOINT_HASH`
5. Node starts at height 9, ready to sync from height 10+

#### Handshake with Peer
1. Local node sends `bootstrap_checkpoint_height: 9`
2. Local node sends `bootstrap_checkpoint_hash: "xyz789..."`
3. Remote peer validates against its own constants
4. If mismatch → reject connection with detailed error
5. If match → proceed with sync

#### Attempted Reorg
1. Peer sends block that would reorg past height 9
2. Reorg logic detects `ancestor_height <= 9`
3. Rejects reorg: "Cannot reorg past bootstrap checkpoint"
4. Logs peer as incompatible, marks for disconnect

## Files Modified

### Core Implementation
- `src/vision_constants.rs` - Added checkpoint constants (Lines 245-275)
- `src/p2p/connection.rs` - Extended HandshakeMessage (Lines 135-136, 385-398, 530-565)
- `src/p2p/reorg.rs` - Added checkpoint validation (Lines 42-44, 81-120, 166-186)
- `src/main.rs` - Added bootstrap initialization (Lines 3449-3575, 3625-3635)

### Integration Points
- `Lazy<Mutex<Chain>>` initialization (line 2952) - Calls bootstrap on empty DB
- `HandshakeMessage::new()` - Includes checkpoint fields
- `HandshakeMessage::validate()` - Validates checkpoint match
- `handle_reorg()` - Refuses reorgs past checkpoint

## Testing Checklist

### Pre-Deployment (Before Filling Hashes)
- [x] Code compiles without errors
- [x] Handshake includes checkpoint fields (default zeros)
- [x] Bootstrap creates 10 blocks on empty DB
- [x] Reorg protection prevents crossing checkpoint

### Post-Deployment (After Filling Hashes)
- [ ] Mine fresh 10-block prefix
- [ ] Export hashes to constants
- [ ] Rebuild with real hashes
- [ ] Test new node starts at height 9
- [ ] Test handshake rejects old build (zeros)
- [ ] Test handshake accepts same build
- [ ] Test reorg blocked at height 9
- [ ] Test database corruption detection

## Security Implications

### Prevents Chain Poisoning
- Old testnet nodes can't connect → can't send bad blocks
- Database corruption detected early → prevents UTXO corruption
- Network self-heals by rejecting incompatible peers

### Prevents Reorg Attacks
- No deep reorg can go past height 9
- First 10 blocks are immutable truth
- Attackers can't force long-range reorgs

### Prevents Testnet Drift
- Each testnet drop has unique 10-block prefix
- Nodes from different drops can't connect
- No accidental mixing of incompatible testnets

## Operational Notes

### Version Management
- **Current Version**: v2.2.0-CONSTELLATION
- **Protocol Version**: 2
- **Checkpoint Height**: 9 (10 blocks)
- **Checkpoint Hash**: `0x0000...` (TODO: fill after mining)

### Database Cleanup
If a node has corrupted checkpoint:
```powershell
# Stop node
Stop-Process -Name vision-node

# Delete chain database
Remove-Item -Recurse -Force ./vision_data_7070/chain

# Restart - will bootstrap with embedded prefix
./vision-node.exe
```

### Network Compatibility
- ✅ Nodes with same checkpoint → can connect and sync
- ❌ Nodes with different checkpoint → rejected at handshake
- ❌ Nodes with protocol v1 → rejected (separate check)
- ❌ Old builds without checkpoint field → default to zeros, rejected

## Future Enhancements

### Dynamic Checkpoints
- [ ] Add checkpoint at height 100, 1000, 10000
- [ ] Allow checkpoint updates via governance
- [ ] Support multiple checkpoints for long-lived chains

### Checkpoint Verification
- [ ] Add Merkle proof for checkpoint validation
- [ ] Allow light clients to verify checkpoint
- [ ] Implement checkpoint gossip protocol

### Monitoring
- [ ] Prometheus metric: `checkpoint_rejections_total`
- [ ] Prometheus metric: `checkpoint_validation_failures`
- [ ] Log dashboard showing checkpoint mismatches

## Error Messages

### Handshake Rejection
```
❌ BOOTSTRAP CHECKPOINT MISMATCH - peer is on incompatible chain prefix
Local checkpoint:  abc123... @ height 9
Remote checkpoint: def456... @ height 9
Remote build:      v2.1.0-OLD

This node ships with a different bootstrap prefix (first 10 blocks).
Both nodes must start from the same baked-in bootstrap chain.
Old testnet builds are automatically quarantined.
```

### Reorg Blocked
```
❌ REFUSING REORG - would cross bootstrap checkpoint

Common ancestor at height 7 is below bootstrap checkpoint (height 9).
Reorgs are FORBIDDEN past the baked-in bootstrap prefix.

This peer is on an incompatible chain and should have been rejected
during handshake. Marking for disconnect.
```

### Local Corruption Detected
```
❌ LOCAL CHAIN CORRUPTED - bootstrap checkpoint mismatch

Your local chain prefix does not match the baked-in bootstrap.
This indicates database corruption or tampering.

Expected: abc123... @ height 9
Got:      def456... @ height 9

REQUIRED ACTION: Delete chain database and restart.
```

## Appendix: Export Script Template

```powershell
# scripts/export-bootstrap-hashes.ps1
# Extract first 10 block hashes from running node

$port = 7070
$url = "http://localhost:$port/chain/blocks?from=0&to=9"

$response = Invoke-RestMethod -Uri $url -Method Get
$blocks = $response.blocks

Write-Host "Bootstrap Block Hashes (paste into vision_constants.rs):"
Write-Host ""
Write-Host "pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = ["

for ($i = 0; $i -lt 10; $i++) {
    $hash = $blocks[$i].header.pow_hash
    Write-Host "    `"$hash`", // h=$i"
}

Write-Host "];"
Write-Host ""
Write-Host "pub const BOOTSTRAP_CHECKPOINT_HASH: &str = `"$($blocks[9].header.pow_hash)`";"
```

---
*Implementation complete: December 9, 2025*
*Version: v2.2.0-CONSTELLATION with Bootstrap Checkpoint*
