# Vision Node v1.0.1 Network Reset Guide

## Overview
v1.0.1 includes a clean network reset to ensure all nodes run compatible code with the new miner identity system. This reset prevents "missing field miner" errors and creates a fresh chain state.

## What Changed
1. **New Network Identity**: Updated genesis hash, bootstrap prefix, and version
2. **Backward Compatible Miner Field**: `miner` field now uses `#[serde(default)]` for graceful degradation
3. **Clean State**: All nodes start from block 0 with identical configuration

## Network Reset Checklist

### 1. New Network Identity ✅
The following constants have been updated to create a new network identity:

- **GENESIS_HASH**: `e7580fd06f67c98ab5e912f51c63b4f013a7bcbe37693fe9ec9cac57f5b8bb24`
- **VISION_BOOTSTRAP_PREFIX**: `vision-constellation-v1.0.1`
- **VISION_VERSION**: `v1.0.1`

**Result**: Old nodes will handshake-reject new nodes with clear version/genesis mismatch instead of cryptic "missing field" errors.

### 2. Wipe Local State (REQUIRED)

**On every seed node and validator:**

```bash
# Stop the node
pkill vision-node

# Delete chain database
rm -rf vision_data_*

# Delete peer book database
rm -rf pb_vision-*
rm -rf peerbook/

# Delete mempool persistence (if exists)
rm -rf mempool_*

# Restart with v1.0.1 binary
./vision-node
```

**Windows:**
```powershell
# Stop the node
taskkill /F /IM vision-node.exe

# Delete chain database
Remove-Item -Recurse -Force vision_data_*

# Delete peer book database
Remove-Item -Recurse -Force pb_vision-*
Remove-Item -Recurse -Force peerbook/

# Restart with v1.0.1 binary
.\vision-node.exe
```

**Why this matters:**
- Ensures all nodes start at height 0
- Clears peer lists of dead/incompatible entries
- Prevents accidental fork anchors
- No ghost state from old network

### 3. Seed Node Deployment Order

**Critical**: Deploy to ALL seeds before opening to public.

```bash
# 1. Build v1.0.1 binary
cd vision-node
cargo build --release

# 2. Deploy to seed servers (do this for each seed)
scp target/release/vision-node seed1.example.com:/opt/vision/
scp target/release/vision-node seed2.example.com:/opt/vision/
scp target/release/vision-node seed3.example.com:/opt/vision/

# 3. On each seed: wipe state and restart
ssh seed1.example.com "cd /opt/vision && rm -rf vision_data_* pb_vision-* && ./vision-node"
ssh seed2.example.com "cd /opt/vision && rm -rf vision_data_* pb_vision-* && ./vision-node"
ssh seed3.example.com "cd /opt/vision && rm -rf vision_data_* pb_vision-* && ./vision-node"
```

**Verification:**
```bash
# Check all seeds are on v1.0.1
curl http://seed1.example.com:7072/api/health/public | jq '.version'
curl http://seed2.example.com:7072/api/health/public | jq '.version'
curl http://seed3.example.com:7072/api/health/public | jq '.version'

# All should return: "v1.0.1"
```

### 4. Update Hardcoded Seed List

**In your config or deployment:**

```toml
# config/bootstrap.toml (example)
seeds = [
    "seed1.visionnetwork.io:7072",
    "seed2.visionnetwork.io:7072",
    "seed3.visionnetwork.io:7072"
]

# Ensure these point ONLY to v1.0.1 seeds
# Remove any old/incompatible seed addresses
```

**If using DNS seeds:**
```bash
# Update DNS records to point only to v1.0.1 seeds
# Example:
seed.visionnetwork.io. IN A 192.0.2.1
seed.visionnetwork.io. IN A 192.0.2.2
seed.visionnetwork.io. IN A 192.0.2.3
```

### 5. Startup Order (Critical)

Follow this sequence to ensure clean network formation:

```bash
# 1. Start seed nodes (let them stabilize for 30-60 seconds)
# Seeds should discover each other and sync to height 0

# 2. Start genesis guide node (optional, for controlled mining)
# This can be your "official" first miner

# 3. Start 1-2 test validators
# Verify they can sync and see height 0

# 4. Open to public
# Announce availability on Discord/Twitter/etc.
```

**Validation checklist:**
- [ ] All seeds running v1.0.1
- [ ] All seeds showing height 0 or 1
- [ ] Seeds can peer with each other (check `/api/net/info`)
- [ ] Test node can connect and sync
- [ ] Mining starts cleanly with miner addresses populated

## Technical Implementation Details

### Backward Compatible Miner Field

The `miner` field in `BlockHeader` now uses `#[serde(default)]`:

```rust
pub struct BlockHeader {
    pub parent_hash: String,
    pub number: u64,
    pub timestamp: u64,
    pub difficulty: u64,
    pub nonce: u64,
    pub pow_hash: String,
    pub state_root: String,
    pub tx_root: String,
    pub receipts_root: String,
    pub da_commitment: Option<String>,
    /// Miner address for block rewards (backward compatible)
    #[serde(default)]
    pub miner: String,
    /// EIP-1559 style base fee per gas (dynamic fee market)
    #[serde(default)]
    pub base_fee_per_gas: u128,
}
```

**Benefits:**
- Old peers missing `miner` won't crash you
- Connection stays alive during sync
- You can still enforce "miner required" at validation time
- Graceful degradation for emergency compatibility

**Validation enforcement:**
```rust
// At block validation time, you can still require miner
if blk.header.miner.is_empty() {
    return Err(anyhow!("Block rejected: miner address required"));
}
```

### Network Identity Computation

The chain ID is now computed from v1.0.1 constants:

```rust
pub fn expected_chain_id() -> String {
    let material = format!(
        "Vision|chain_id_v={}|genesis={}|block_time_secs={}|bootstrap_ckpt_h={}|bootstrap_ckpt_hash={}",
        CHAIN_ID_VERSION,
        crate::genesis::GENESIS_HASH,  // New hash
        BLOCK_TIME_SECS,
        VISION_BOOTSTRAP_HEIGHT,
        VISION_BOOTSTRAP_HASH,
    );
    hex::encode(blake3::hash(material.as_bytes()).as_bytes())
}
```

**Result**: Old nodes compute a different chain ID and are rejected during handshake.

## Troubleshooting

### "Genesis hash mismatch"
**Cause**: Node is running old binary or has mixed v1.0.0/v1.0.1 state.

**Fix**:
```bash
# Stop node
pkill vision-node

# Verify binary version
./vision-node --version  # Should show v1.0.1

# Rebuild if needed
cargo build --release

# Wipe state completely
rm -rf vision_data_* pb_vision-* peerbook/

# Restart
./vision-node
```

### "Handshake rejected: version mismatch"
**Cause**: Trying to connect to old v1.0.0 seed.

**Fix**: Update your seed list to only include v1.0.1 seeds.

### "Block validation failed: miner required"
**Cause**: Node received a block without miner field (shouldn't happen with v1.0.1 network reset, but possible during transition).

**Fix**: This is expected behavior. The node will reject invalid blocks and continue syncing from valid peers.

### "Stuck at height 0, no blocks"
**Cause**: Not enough miners running, or miners don't have sufficient peers.

**Fix**:
```bash
# Check peer count
curl http://localhost:7072/api/net/info | jq '.peer_count'

# Need at least 3 peers for mining
# Check mining status
curl http://localhost:7072/api/miner/status | jq
```

### "Peer count stuck at 0"
**Cause**: Seed list empty or seeds unreachable.

**Fix**:
```bash
# Check seed configuration
cat config/bootstrap.toml

# Test seed connectivity
nc -zv seed1.visionnetwork.io 7072
nc -zv seed2.visionnetwork.io 7072

# Verify firewall allows port 7072 (both TCP and UDP)
```

## Post-Reset Validation

After network reset is complete, verify:

```bash
# 1. Check version
curl http://localhost:7072/api/health/public | jq '.version'
# Expected: "v1.0.1"

# 2. Check chain height (should be growing)
curl http://localhost:7072/api/health/public | jq '.height'

# 3. Check peer count (should be >= 3)
curl http://localhost:7072/api/net/info | jq '.peer_count'

# 4. Check mining status
curl http://localhost:7072/api/miner/status

# 5. Verify miner addresses in blocks
curl http://localhost:7072/api/chain/blocks?limit=5 | jq '.[].header.miner'
# Should show actual miner addresses, not empty strings
```

## Migration from v1.0.0

**There is no migration.** v1.0.1 is a clean network reset.

Users must:
1. Back up any important data (wallets, keys)
2. Stop v1.0.0 node
3. Wipe all state directories
4. Install v1.0.1 binary
5. Restart from height 0

**Airdrop balances** are preserved in the wallet (not on-chain), so users won't lose their initial LAND/GAME/CASH allocations.

## Summary

v1.0.1 network reset provides:
- ✅ Clean break from v1.0.0 (prevents "missing field" errors)
- ✅ Backward compatible miner field (future-proof)
- ✅ Updated network identity (genesis + bootstrap prefix)
- ✅ Clear migration path (wipe state, restart)
- ✅ Improved peer compatibility checking

This reset establishes a stable foundation for mainnet launch.
