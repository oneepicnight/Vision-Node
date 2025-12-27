# Testnet v2.0 Auto-Stamp Implementation Guide

## Overview

The Testnet Auto-Stamp feature enables designated seed nodes to automatically stamp the first 3 blocks on the v2.0 testnet, establishing a canonical starting chain that all other nodes sync from. This ensures network identity is locked in and all miners start with the same canonical blockchain.

**Key Points:**
- **Testnet-only**: Only applies when `network = "testnet"`
- **Seed nodes only**: Only nodes with `is_testnet_seed = true` participate
- **Automatic**: Happens on first startup with empty/incomplete chain
- **Safe**: Won't overwrite existing blocks if height >= 3
- **Separate from mainnet**: Completely independent of Guardian launch logic

---

## Implementation Summary

### 1. Configuration Fields

**File**: `src/config/network.rs`

Added two new fields to `NetworkConfig`:

```rust
/// Whether this node is an official testnet seed node
pub is_testnet_seed: bool,

/// Whether this node should auto-stamp the first blocks on testnet
pub auto_stamp_testnet_blocks: bool,
```

**Environment Variables:**
- `VISION_IS_TESTNET_SEED` - Set to `"true"` for official seed nodes
- `VISION_AUTO_STAMP_TESTNET` - Set to `"true"` to enable auto-stamping

**Defaults**: Both default to `false` (disabled)

### 2. Constant

**File**: `src/constants.rs`

```rust
/// Number of blocks to auto-stamp on testnet seed nodes
pub const TESTNET_STAMP_BLOCK_COUNT: u64 = 3;
```

### 3. Bootstrap Function

**File**: `src/main.rs` (around line 10617)

```rust
fn bootstrap_testnet_stamp(
    chain: &mut Chain,
    cfg: &config::network::NetworkConfig
) -> Result<(), String>
```

**Behavior:**
- Checks network == "testnet"
- Checks is_testnet_seed && auto_stamp_testnet_blocks
- Only runs if current_height < TESTNET_STAMP_BLOCK_COUNT
- Sequentially mines 3 empty blocks using `execute_and_mine()`
- Appends through normal block append path (state updates, difficulty, etc.)
- Logs each step clearly with `[TESTNET_STAMP]` prefix

### 4. Startup Integration

**File**: `src/main.rs` (around line 5221)

Called early in `main()` before Phase 3 Guardian initialization:

```rust
// Testnet v2.0 Bootstrap: Auto-stamp first 3 blocks on seed nodes
{
    let network_cfg = config::network::NetworkConfig::from_env();
    
    if network_cfg.network == constants::NETWORK_TESTNET {
        if network_cfg.is_testnet_seed && network_cfg.auto_stamp_testnet_blocks {
            let mut chain = CHAIN.lock();
            match bootstrap_testnet_stamp(&mut chain, &network_cfg) {
                Ok(()) => { /* Success */ }
                Err(e) => { /* Log error but don't exit */ }
            }
            drop(chain);
        }
    }
}
```

---

## Configuration Examples

### For Testnet Seed Nodes (Your 3 Seeds)

**Example `.env` file:**

```bash
# Network configuration
VISION_NETWORK=testnet

# Testnet seed configuration
VISION_IS_TESTNET_SEED=true
VISION_AUTO_STAMP_TESTNET=true

# Miner address for stamped blocks
VISION_MINER_ADDRESS=land1seed1qxxxxxxxxxxxxxxxxxxxxxxxxxx

# Standard configuration
VISION_PORT=7070
VISION_P2P_PORT=7072
RUST_LOG=info
```

**Startup command:**

```bash
# Linux
./vision-node

# Windows
.\vision-node.exe
```

### For Regular Testnet Miners

**Example `.env` file:**

```bash
# Network configuration
VISION_NETWORK=testnet

# Not a seed node - will sync from seeds
# VISION_IS_TESTNET_SEED=false  # (default, can omit)
# VISION_AUTO_STAMP_TESTNET=false  # (default, can omit)

# Miner address
VISION_MINER_ADDRESS=land1minerxxxxxxxxxxxxxxxxxxxxxxxxx

# Standard configuration
VISION_PORT=7070
VISION_P2P_PORT=7072
RUST_LOG=info
```

### For Mainnet Nodes (Guardian Launch)

**Example `.env` file:**

```bash
# Network configuration
VISION_NETWORK=mainnet-full

# Guardian launch configuration (if Guardian)
VISION_LAUNCH_GUARDIAN_ENABLED=true
VISION_GUARDIAN_ADDRESS=land1guardxxxxxxxxxxxxxxxxxxxxxxxxx
VISION_IS_GUARDIAN=true

# Testnet auto-stamp does NOT apply here (different network)
# VISION_IS_TESTNET_SEED=false  # (ignored on mainnet-full)
# VISION_AUTO_STAMP_TESTNET=false  # (ignored on mainnet-full)

# Standard configuration
VISION_PORT=7070
VISION_P2P_PORT=7072
RUST_LOG=info
```

---

## Expected Behavior

### Testnet Seed Node (First Startup, Empty DB)

```
[VISION] Node Version v2.0 - Testnet Reset
[VISION] Protocol Version: 2
🌐 Network: testnet
⚠️  Testnet Mode: Only protocol v2 nodes accepted

[TESTNET_STAMP] 🌱 This is a testnet seed node with auto-stamp enabled
[TESTNET_STAMP] 🌱 Auto-stamping 3 blocks for v2.0 Testnet on seed node...
[TESTNET_STAMP] 🔨 Stamping block 1 of 3...
[TESTNET_STAMP] ✅ Block 1 stamped successfully (height: 1)
[TESTNET_STAMP] 🔨 Stamping block 2 of 3...
[TESTNET_STAMP] ✅ Block 2 stamped successfully (height: 2)
[TESTNET_STAMP] 🔨 Stamping block 3 of 3...
[TESTNET_STAMP] ✅ Block 3 stamped successfully (height: 3)
[TESTNET_STAMP] 🎉 Testnet auto-stamp complete. Best height now 3.
[TESTNET_STAMP] ✅ Testnet bootstrap complete. Chain height: 3

[P2P] 🔌 Starting P2P listener on 0.0.0.0:7072
[P2P] ✅ P2P listener started - ready for peer connections
Vision node HTTP API listening on 0.0.0.0:7070
```

### Testnet Seed Node (Restart After Stamping)

```
[VISION] Node Version v2.0 - Testnet Reset
[VISION] Protocol Version: 2
🌐 Network: testnet
⚠️  Testnet Mode: Only protocol v2 nodes accepted

[TESTNET_STAMP] 🌱 This is a testnet seed node with auto-stamp enabled
[TESTNET_STAMP] Not required (current_height = 3, target = 3).
[TESTNET_STAMP] ✅ Testnet bootstrap complete. Chain height: 3

[P2P] 🔌 Starting P2P listener on 0.0.0.0:7072
Vision node HTTP API listening on 0.0.0.0:7070
```

### Regular Testnet Miner (Syncs from Seeds)

```
[VISION] Node Version v2.0 - Testnet Reset
[VISION] Protocol Version: 2
🌐 Network: testnet
⚠️  Testnet Mode: Only protocol v2 nodes accepted

[TESTNET_STAMP] ℹ️  Regular testnet node - will sync from seeds
[BOOTSTRAP] 🚀 Executing unified bootstrap...
[BOOTSTRAP] 📡 Connecting to testnet seeds...
[BOOTSTRAP] ✅ Connected to 69.173.206.211:7070
[BOOTSTRAP] ✅ Bootstrap completed successfully

[SYNC] Syncing blocks from peers...
[SYNC] ✅ Synced block 1 (hash: abc123...)
[SYNC] ✅ Synced block 2 (hash: def456...)
[SYNC] ✅ Synced block 3 (hash: ghi789...)
[SYNC] ✅ Chain synchronized. Height: 3
```

### Mainnet Guardian Node (Separate Logic)

```
[VISION] Node Version v2.0 - Testnet Reset
[VISION] Protocol Version: 2
🌐 Network: mainnet-full

[GUARDIAN_LAUNCH] 🛡️  Guardian launch sequence active
[GUARDIAN_LAUNCH] Only Guardian can mine blocks 1-3
[GUARDIAN_LAUNCH] Guardian address: land1guardxxxxxxxxx

Vision node HTTP API listening on 0.0.0.0:7070
```

---

## Logging Reference

All testnet auto-stamp logs use the `[TESTNET_STAMP]` prefix:

| Log Message | Meaning |
|-------------|---------|
| `🌱 This is a testnet seed node with auto-stamp enabled` | Seed node detected, about to check if stamping needed |
| `🌱 Auto-stamping 3 blocks for v2.0 Testnet on seed node...` | Stamping process starting |
| `🔨 Stamping block N of 3...` | Currently mining block N |
| `✅ Block N stamped successfully (height: N)` | Block N successfully mined and appended |
| `🎉 Testnet auto-stamp complete. Best height now N.` | All blocks stamped successfully |
| `Not required (current_height = N, target = 3).` | Chain already has 3+ blocks, skipping stamp |
| `ℹ️  Testnet seed node, but auto-stamp disabled` | Seed flag set but auto-stamp disabled |
| `ℹ️  Regular testnet node - will sync from seeds` | Not a seed node, will sync normally |
| `❌ Failed to auto-stamp testnet blocks: <error>` | Error occurred during stamping (non-fatal) |

---

## Verification Steps

### 1. Check Seed Node Stamped Blocks

```bash
# Query chain height
curl http://localhost:7070/height

# Expected: 3 (or higher if mining continued)

# Query block 1
curl http://localhost:7070/block/1

# Check miner field matches VISION_MINER_ADDRESS

# Query block 2
curl http://localhost:7070/block/2

# Query block 3
curl http://localhost:7070/block/3
```

### 2. Verify Miner Can Sync

```bash
# On miner node, check peers
curl http://localhost:7070/peers

# Should see seed nodes listed

# Check chain height
curl http://localhost:7070/height

# Should be 3+ (synced from seeds)

# Verify blocks match seed blocks
curl http://localhost:7070/block/1
# Compare hash with seed node block 1
```

### 3. Verify Protocol Enforcement

```bash
# Old v1.x node attempting to connect
# Should see rejection in logs:

[P2P] ❌ Testnet Reset: Peer protocol 1 is outdated. 
       Minimum required: 2. Please upgrade to v2.0.
```

---

## Troubleshooting

### Seed Node Not Stamping

**Symptoms**: Seed node starts but doesn't stamp blocks

**Check:**

1. Environment variables set correctly:
   ```bash
   echo $VISION_NETWORK  # Should be "testnet"
   echo $VISION_IS_TESTNET_SEED  # Should be "true"
   echo $VISION_AUTO_STAMP_TESTNET  # Should be "true"
   ```

2. Check logs for:
   ```
   [TESTNET_STAMP] ℹ️  Testnet seed node, but auto-stamp disabled
   ```
   This means `VISION_AUTO_STAMP_TESTNET` is not set to `"true"`

3. Check logs for:
   ```
   [TESTNET_STAMP] Not required (current_height = N, target = 3).
   ```
   This means chain already has 3+ blocks (normal on restart)

**Solution**: 
- If fresh install: Set both `VISION_IS_TESTNET_SEED=true` and `VISION_AUTO_STAMP_TESTNET=true`
- If restart: Normal behavior - stamping only happens once

### Miner Not Syncing Stamped Blocks

**Symptoms**: Miner starts but has height 0 or doesn't sync

**Check:**

1. Network configuration:
   ```bash
   echo $VISION_NETWORK  # Should be "testnet"
   ```

2. Bootstrap peers configured:
   ```bash
   # Should have TESTNET_DEFAULT_SEEDS or manual bootstrap
   curl http://localhost:7070/peers
   ```

3. Protocol version:
   ```bash
   # v2.0 nodes only
   curl http://localhost:7070/api/status | jq .protocol_version
   # Should be 2
   ```

**Solution**:
- Ensure `VISION_NETWORK=testnet`
- Verify seed nodes are accessible on port 7070
- Check firewall/network connectivity

### Blocks Mined with Wrong Address

**Symptoms**: Stamped blocks show unexpected miner address

**Check:**
```bash
echo $VISION_MINER_ADDRESS
```

**Solution**: Set `VISION_MINER_ADDRESS` to your seed node address before first start

---

## Deployment Checklist

### Testnet Seed Deployment

- [ ] Set `VISION_NETWORK=testnet`
- [ ] Set `VISION_IS_TESTNET_SEED=true`
- [ ] Set `VISION_AUTO_STAMP_TESTNET=true`
- [ ] Set `VISION_MINER_ADDRESS=<seed_address>`
- [ ] Set `VISION_PORT=7070`
- [ ] Set `VISION_P2P_PORT=7072`
- [ ] Deploy v2.0 binary to all 3 seed nodes
- [ ] Start seed nodes simultaneously
- [ ] Verify all 3 seed nodes stamp blocks 1-3
- [ ] Check all 3 have identical block hashes

### Testnet Miner Deployment

- [ ] Set `VISION_NETWORK=testnet`
- [ ] Set `VISION_MINER_ADDRESS=<miner_address>`
- [ ] Deploy v2.0 binary
- [ ] Start node
- [ ] Verify syncs from seeds (height reaches 3+)
- [ ] Start mining

### Mainnet Guardian Deployment (Separate)

- [ ] Set `VISION_NETWORK=mainnet-full`
- [ ] Set `VISION_LAUNCH_GUARDIAN_ENABLED=true` (Guardian only)
- [ ] Set `VISION_GUARDIAN_ADDRESS=<guardian_address>` (Guardian only)
- [ ] Set `VISION_IS_GUARDIAN=true` (Guardian only)
- [ ] Deploy v2.0 binary
- [ ] Guardian launch sequence activates automatically
- [ ] Testnet auto-stamp does NOT apply (different network)

---

## Architecture Notes

### Why Separate from Guardian Launch?

The Testnet Auto-Stamp and Guardian Launch are **completely independent features**:

| Feature | Network | Purpose | Configuration |
|---------|---------|---------|---------------|
| **Testnet Auto-Stamp** | `testnet` | Establish canonical first 3 blocks on v2.0 testnet | `is_testnet_seed` + `auto_stamp_testnet_blocks` |
| **Guardian Launch** | `mainnet-full` | Ceremonial start for production blockchain | `launch_guardian_enabled` + `guardian_address` |

**Key Differences:**

1. **Network Isolation**: 
   - Testnet stamp only checks `network == "testnet"`
   - Guardian launch only checks `network == "mainnet-full"`
   - They never interfere

2. **Triggering Logic**:
   - Testnet: Automatic on startup if height < 3
   - Mainnet: Guardian mines, others wait for height >= 3

3. **Configuration**:
   - Testnet: `VISION_IS_TESTNET_SEED` + `VISION_AUTO_STAMP_TESTNET`
   - Mainnet: `VISION_LAUNCH_GUARDIAN_ENABLED` + `VISION_IS_GUARDIAN`

4. **Use Case**:
   - Testnet: Development/testing environment reset
   - Mainnet: Production launch ceremony

### Code Flow

```
main() startup
    ↓
Load NetworkConfig
    ↓
[Check if testnet && seed && auto-stamp]
    ↓
bootstrap_testnet_stamp()
    ├── Check current_height < 3
    ├── Mine 3 empty blocks
    ├── Append through normal path
    └── Log completion
    ↓
[Continue normal startup]
    ↓
Guardian consciousness init
    ↓
Network services start
```

### Block Stamping Process

```rust
for block_num in (current_height + 1)..=3 {
    1. Get parent block
    2. Build empty tx list
    3. execute_and_mine() - normal block production
    4. persist_block_only() - save to DB
    5. chain.blocks.push() - add to memory
    6. Update difficulty & EMA - normal retarget
    7. Persist state
    8. Log success
}
```

This ensures stamped blocks go through **exactly the same validation** as regular mined blocks.

---

## Success Criteria

✅ **Testnet seed nodes**:
- Auto-stamp 3 blocks on first start
- Don't re-stamp on restart (height >= 3)
- All 3 seeds have identical block hashes
- Serve stamped blocks to miners

✅ **Testnet miners**:
- Sync stamped blocks from seeds
- Start with height 3 (after sync)
- Can mine new blocks after sync
- Reject v1.x peers

✅ **Mainnet nodes**:
- Completely unaffected
- Guardian launch works as before
- No testnet logic triggered
- Different network identifier

✅ **Code quality**:
- ✅ Compiles without errors
- ✅ Clear logging with [TESTNET_STAMP] prefix
- ✅ Safe failure (non-fatal errors)
- ✅ Testnet-only guards in place
- ✅ Separate from Guardian logic

---

## Summary

The Testnet Auto-Stamp feature provides a clean, automatic way to establish the first 3 blocks on the v2.0 testnet, ensuring:

1. **Network Identity**: All miners start with the same canonical chain
2. **Automatic Bootstrap**: Fresh installs "just work" without manual setup
3. **Safe Operation**: Won't overwrite existing blocks, fails gracefully
4. **Clear Separation**: Independent from mainnet Guardian launch
5. **Production Ready**: Compiles cleanly, logs clearly, handles errors

Your v2.0 testnet is ready for deployment! 🚀
