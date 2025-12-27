# Chain Safety Guards v2.5.0

## Overview

VisionNode v2.5.0 introduces comprehensive chain wipe protection to prevent accidental loss of blockchain data. This document details the three-tier safety guard system.

---

## 🛡️ Safety Features

### 1. Full Chain Reset Protection

**Purpose**: Prevent automatic chain resets on non-trivial chains

**Implementation**:
- `Chain::allow_full_chain_reset()` method checks if chain reset is allowed
- Returns `true` only if:
  - Chain height is 0 (no data to lose), OR
  - `VISION_FORCE_FULL_RESYNC` environment variable is set

**Usage**:
```bash
# Normal operation - resets blocked on chains with data
./vision-node --guardian

# Force full resync (when intentional chain wipe is needed)
VISION_FORCE_FULL_RESYNC=1 ./vision-node --guardian
```

**Logs**:
```
⚠️ FORCING FULL CHAIN RESET due to VISION_FORCE_FULL_RESYNC
❌ REFUSING automatic full chain reset on non-zero height chain
```

---

### 2. Auto-Sync Protection

**Purpose**: Prevent backward sync or syncing from corrupted peers

**Checks**:

1. **Height-0 Peer Protection**
   - Refuses to sync from a peer at height 0 when local chain has data
   - Prevents catastrophic backward sync
   
2. **Backward Sync Protection**
   - Refuses to sync from peers significantly behind (5+ blocks)
   - Uses `SAFETY_MARGIN = 5` blocks for legitimate reorgs

**Logs**:
```
⚠️ AUTO-SYNC: Refusing to sync from peer at height 0
⚠️ AUTO-SYNC: Refusing to sync from peer significantly behind us
```

**Location**: `src/auto_sync.rs`

---

### 3. Height Change Logging

**Purpose**: Audit trail for all height changes with automatic alerts

**Tracking Points**:
- Chain initialization: Logs starting height
- Block mining: Tracks height after mining
- Block acceptance: Tracks height after accepting from network
- Reorg operations: Tracks all rollback and re-apply operations
- Sync/recovery: Tracks height during chain synchronization

**Alert Levels**:

| Condition | Level | Message |
|-----------|-------|---------|
| Height drops to 0 | ❌ ERROR | `HEIGHT DROPPED TO ZERO - This should NEVER happen without explicit reset!` |
| Height drops >5 blocks | ⚠️ ERROR | `BLOCK HEIGHT DECREASED SIGNIFICANTLY` |
| Normal height increase | ℹ️ INFO | Standard block acceptance |

**Log Examples**:
```
📊 Chain initialized with 1234 blocks
✅ HEIGHT: 1234 → 1235 [mine_block]
✅ HEIGHT: 1235 → 1236 [accept_block]
📊 REORG: Height changed from 1236 to 1235 (ancestor at 1234)
❌ HEIGHT DROPPED TO ZERO - This should NEVER happen without explicit reset!
⚠️ BLOCK HEIGHT DECREASED SIGNIFICANTLY
```

**Implementation**:
- `Chain::current_height()` - Get current chain height
- `Chain::log_height_change(old, new, context)` - Log and alert on height changes

---

## 🔍 Context Tags

Height change logs include context tags to identify the operation:

| Context | Description |
|---------|-------------|
| `mine_block` | Block mined by local node |
| `accept_block` | Block accepted from network peer |
| `reorg_accept_block` | Block accepted during reorganization |
| `sync_recovery_block` | Block applied during sync/recovery |
| `reorg_snapshot_restore` | Chain restored from snapshot during reorg |

---

## 🚨 Response to Alerts

### Height Dropped to Zero
**Severity**: CRITICAL

**Possible Causes**:
- Software bug in chain initialization
- Corrupted database
- Accidental chain wipe attempt

**Actions**:
1. Stop the node immediately
2. Investigate logs before the drop
3. Check for database corruption
4. Restore from backup if available
5. Report bug to development team

### Height Decreased Significantly
**Severity**: WARNING

**Possible Causes**:
- Large reorganization (>5 blocks)
- Time travel attack attempt
- Peer consensus issues

**Actions**:
1. Verify peer connections (check peer heights)
2. Monitor if height recovers naturally
3. Consider stopping node if continuing to drop
4. Check network consensus status

---

## 🧪 Testing Safety Guards

### Test 1: Verify Reset Protection
```bash
# Start node with existing data
./vision-node --guardian

# Should see in logs:
# 📊 Chain initialized with <N> blocks (where N > 0)

# Try to trigger reset (should be blocked)
# Guards will log: ❌ REFUSING automatic full chain reset
```

### Test 2: Force Reset (When Needed)
```bash
# Set environment variable to allow reset
VISION_FORCE_FULL_RESYNC=1 ./vision-node --guardian

# Should see in logs:
# ⚠️ FORCING FULL CHAIN RESET due to VISION_FORCE_FULL_RESYNC
```

### Test 3: Auto-Sync Protection
```bash
# Start node and monitor auto-sync logs
# If malicious peer tries to sync backwards:
# ⚠️ AUTO-SYNC: Refusing to sync from peer at height 0
# ⚠️ AUTO-SYNC: Refusing to sync from peer significantly behind us
```

### Test 4: Height Change Tracking
```bash
# Mine some blocks and watch logs
# Each block should show:
# ✅ HEIGHT: <old> → <new> [mine_block]

# Accept blocks from network:
# ✅ HEIGHT: <old> → <new> [accept_block]
```

---

## 📝 Configuration

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `VISION_FORCE_FULL_RESYNC` | unset | Allow full chain reset on non-zero chains |

**Example**:
```bash
# PowerShell
$env:VISION_FORCE_FULL_RESYNC = "1"
./vision-node --guardian

# Bash
export VISION_FORCE_FULL_RESYNC=1
./vision-node --guardian
```

---

## 🔧 Maintenance

### When to Use VISION_FORCE_FULL_RESYNC

✅ **Safe Scenarios**:
- Fresh deployment after corruption
- Intentional chain restart for testing
- Database migration/upgrade
- Network-wide reset (coordinated)

❌ **Dangerous Scenarios**:
- Production node with live data
- During normal operation
- As a "quick fix" for sync issues
- Without backing up existing chain

### Backup Before Reset

```powershell
# Windows PowerShell
$port = 7070  # or your VISION_PORT
Copy-Item -Recurse ".\vision_data_$port" ".\vision_data_${port}_backup_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
```

---

## 📊 Monitoring

### Key Metrics

1. **Height Changes**: Monitor logs for unexpected drops
2. **Reorg Frequency**: Excessive reorgs indicate network issues
3. **Auto-Sync Rejections**: High rejection rate suggests peer problems
4. **Force Reset Usage**: Track VISION_FORCE_FULL_RESYNC usage

### Prometheus Metrics

Related metrics (already existing):
- `vision_reorgs_total` - Total reorg count
- `vision_reorg_length_total` - Blocks switched during reorgs
- `vision_chain_reorg_blocks_rolled_back_total` - Rollback count

---

## 🐛 Troubleshooting

### Problem: Node refuses to start after crash

**Solution**:
```bash
# Check if database is corrupted
# If needed, force clean start:
VISION_FORCE_FULL_RESYNC=1 ./vision-node --guardian
```

### Problem: Auto-sync keeps rejecting peers

**Cause**: Local chain ahead of network or peers corrupted

**Solution**:
1. Check local height vs network consensus
2. If local chain is corrupted, use VISION_FORCE_FULL_RESYNC
3. If peers are corrupted, wait for network to heal

### Problem: Frequent height warnings

**Cause**: Large reorganizations or network instability

**Solution**:
1. Monitor peer connections
2. Check network consensus
3. May indicate attack or network partition

---

## 🔒 Security Implications

### Protection Against

1. **Time Travel Attacks**: Auto-sync refuses backward sync
2. **Malicious Peers**: Height-0 peer protection
3. **Accidental Wipes**: Reset protection requires explicit flag
4. **Data Loss**: Complete audit trail of all height changes

### Attack Vectors Still Possible

1. **Eclipse Attack**: If all peers are malicious (mitigated by diverse peer selection)
2. **51% Attack**: Consensus-level attack (not preventable at node level)

---

## 📚 Developer Notes

### Adding New Chain Modifications

When adding code that modifies `chain.blocks`:

1. **Capture old height**:
   ```rust
   let old_height = g.current_height();
   ```

2. **Perform modification**:
   ```rust
   g.blocks.push(new_block);
   // or g.blocks.pop();
   // or g.blocks = new_vec;
   ```

3. **Log height change**:
   ```rust
   let new_height = g.current_height();
   g.log_height_change(old_height, new_height, "context_name");
   ```

### Testing New Features

Always test with non-empty chain to ensure safety guards trigger properly:
```rust
#[test]
fn test_my_feature_with_existing_chain() {
    let mut chain = fresh_chain();
    // Mine some blocks first
    mine_test_blocks(&mut chain, 10);
    
    // Now test your feature
    // ...
}
```

---

## 📖 Version History

**v2.5.0** (2025-12-10)
- Initial implementation of three-tier safety guard system
- Added `allow_full_chain_reset()`, `log_height_change()`, `current_height()`
- Protected auto-sync from backward sync
- Height tracking at all block push locations

---

## ⚠️ Important Notes

1. **VISION_FORCE_FULL_RESYNC is destructive** - Only use when you understand the consequences
2. **Backup before using VISION_FORCE_FULL_RESYNC** - Chain data will be lost
3. **Height drops are never normal** - Investigate immediately if you see ERROR logs
4. **Auto-sync rejections are protective** - Do not disable without understanding why they occur

---

## 🆘 Support

If you encounter issues with the safety guards:

1. Check logs for specific error messages
2. Verify environment variables are set correctly
3. Ensure database is not corrupted
4. Consult this documentation for common scenarios
5. Report bugs with full log context

---

**Remember**: These safety guards are designed to prevent data loss. If they trigger, investigate before bypassing them.
