# Guardian Launch Sequence

## Overview

The Guardian Launch Sequence is a ceremonial process that only applies to the **mainnet-full** network, ensuring an orderly start to the production blockchain. During this sequence, only the designated Guardian node is permitted to mine the first 3 blocks, after which the network opens to all Constellation nodes.

**Important**: The testnet network remains completely unchanged and continues to allow any node to mine freely (wild playground behavior).

## Network Types

### Testnet (Default)
- **Behavior**: Unchanged - any node can mine any block
- **Purpose**: Testing, experimentation, wild playground
- **Configuration**: `VISION_NETWORK=testnet` (default if not set)
- **Launch Sequence**: Disabled

### Mainnet-Full
- **Behavior**: Guardian mines blocks 1-3, then retires
- **Purpose**: Production network with formal ceremony
- **Configuration**: `VISION_NETWORK=mainnet-full`
- **Launch Sequence**: Enabled with `VISION_LAUNCH_GUARDIAN_ENABLED=true`

## Launch Block Rules

When the Guardian Launch Sequence is active (`mainnet-full` + launch enabled):

### Blocks 1, 2, and 3 (Launch Blocks)
- ✅ **Guardian Node**: Mines these blocks exclusively
- ⏱️ **Constellation Nodes**: Wait patiently, mining blocked
- 🚫 **Block Validation**: Rejects any block 1-3 not mined by Guardian address

### Block 4 and Beyond
- 🎖️ **Guardian Node**: Retires from mining (unless emergency override)
- ⚡ **Constellation Nodes**: Begin mining normally
- 🌐 **Network**: Operates as decentralized PoW network

## Configuration

### Guardian Node Configuration

The Guardian node mines the first 3 blocks and then retires.

**Environment Variables:**
```bash
VISION_NETWORK=mainnet-full                      # Use production network
VISION_LAUNCH_GUARDIAN_ENABLED=true              # Enable launch sequence
VISION_GUARDIAN_ADDRESS=LAND1234...               # Guardian's reward address
VISION_IS_GUARDIAN=true                          # Mark this as the Guardian node
```

**Example .env file:**
```env
VISION_NETWORK=mainnet-full
VISION_LAUNCH_GUARDIAN_ENABLED=true
VISION_GUARDIAN_ADDRESS=LAND1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7
VISION_IS_GUARDIAN=true
VISION_WALLET_PATH=./guardian-wallet.json
```

**Log Output During Launch:**
```
[guardian_launch] 🎖️  [MINING_GATE] Guardian mining launch block 1
[guardian_launch] ✅ Guardian launch block 1 validated
[guardian_launch] 🎖️  [MINING_GATE] Guardian mining launch block 2
[guardian_launch] ✅ Guardian launch block 2 validated
[guardian_launch] 🎖️  [MINING_GATE] Guardian mining launch block 3
[guardian_launch] ✅ Guardian launch block 3 validated
[guardian_launch] 🎖️  [MINING_GATE] Guardian launch complete at height 3 – standing down miner (mainnet-full).
```

### Constellation Node Configuration

Constellation nodes wait for the Guardian to complete the launch, then begin mining.

**Environment Variables:**
```bash
VISION_NETWORK=mainnet-full                      # Use production network
VISION_LAUNCH_GUARDIAN_ENABLED=true              # Enable launch sequence (wait mode)
VISION_GUARDIAN_ADDRESS=LAND1234...               # Guardian's address (for validation)
VISION_IS_GUARDIAN=false                         # This is NOT the Guardian (default)
```

**Example .env file:**
```env
VISION_NETWORK=mainnet-full
VISION_LAUNCH_GUARDIAN_ENABLED=true
VISION_GUARDIAN_ADDRESS=LAND1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t1u2v3w4x5y6z7
VISION_WALLET_PATH=./constellation-wallet.json
```

**Log Output During Launch:**
```
[guardian_launch] ⏱️  [MINING_GATE] Waiting for Guardian launch blocks (0/3) on mainnet-full...
[guardian_launch] ⏱️  [MINING_GATE] Waiting for Guardian launch blocks (1/3) on mainnet-full...
[guardian_launch] ⏱️  [MINING_GATE] Waiting for Guardian launch blocks (2/3) on mainnet-full...
[vision_node::miner] Mining started with 8 threads
```

### Testnet Configuration (Unchanged)

Testnet nodes operate normally with no Guardian restrictions.

**Environment Variables:**
```bash
VISION_NETWORK=testnet                           # Default - wild playground
# No other Guardian variables needed
```

**Example .env file:**
```env
VISION_NETWORK=testnet
VISION_WALLET_PATH=./testnet-wallet.json
```

## Emergency Override

In rare circumstances where the Guardian must mine after block 3 (e.g., network emergency, debugging), the Guardian can enable emergency mining:

**Guardian Emergency Override:**
```bash
VISION_ALLOW_GUARDIAN_EMERGENCY_MINING=true
```

This allows the Guardian to continue mining beyond block 3. **Use with caution** - this breaks the ceremonial retirement and should only be used when necessary.

## Implementation Details

### Part 1: Configuration Infrastructure
- **File**: `src/config/network.rs`
- **Constants**: `src/constants.rs` (LAUNCH_BLOCK_COUNT = 3)
- **Fields Added**:
  - `network: String` - "testnet" or "mainnet-full"
  - `launch_guardian_enabled: bool` - Enable launch sequence
  - `guardian_address: String` - Guardian's LAND address
  - `is_guardian: bool` - Is this node the Guardian?
  - `allow_guardian_emergency_mining: bool` - Emergency override

### Part 2: Consensus Validation
- **File**: `src/main.rs`, function `apply_block_from_peer()`
- **Logic**: Validates that blocks 1-3 on mainnet-full are mined by Guardian address
- **Rejection**: Returns error if block 1-3 has wrong miner on mainnet-full
- **Testnet**: Validation bypassed completely (no restrictions)
- **Note**: Added `miner_address` field to `BlockHeader` struct to track block miner

### Part 3: Mining Gate
- **File**: `src/miner/manager.rs`, function `can_mine_with_guardian_launch()`
- **Guardian**: Mines blocks 1-3, then stops (unless emergency override)
- **Constellation**: Blocked until block 3 exists, then mines normally
- **Testnet**: Gate bypassed completely (no restrictions)

### Part 4: Behavior Requirements

#### Network Isolation
- **Testnet**: Completely independent, no Guardian restrictions
- **Mainnet-Full**: Guardian launch active when configured

#### Logging Clarity
All Guardian launch decisions are logged with:
- **Target**: `guardian_launch` (filter with `RUST_LOG=guardian_launch=info`)
- **Context**: height, network, Guardian status
- **Emojis**: 🎖️ (Guardian), ⏱️ (waiting), ✅ (validated), ❌ (rejected)

#### State Transitions
1. **Pre-launch** (height 0): Genesis block, no mining
2. **Launch Phase** (height 1-3): Guardian mines, others wait
3. **Post-launch** (height 4+): All nodes mine, Guardian retires

## Troubleshooting

### Problem: Constellation node won't start mining

**Symptoms**: Node logs "Waiting for Guardian launch blocks" forever

**Solutions**:
1. Verify Guardian node is running and mining
2. Check network connectivity (peer count > 0)
3. Ensure `VISION_GUARDIAN_ADDRESS` matches on all nodes
4. Verify Guardian has mined blocks 1-3 (check blockchain)

### Problem: Guardian won't mine after block 3

**Symptoms**: Guardian logs "standing down miner" and stops

**This is correct behavior!** The Guardian retires after block 3 by design.

**If Guardian must continue mining**:
```bash
VISION_ALLOW_GUARDIAN_EMERGENCY_MINING=true
```

### Problem: "Block X must be mined by Guardian" error

**Symptoms**: Node rejects blocks 1-3 during sync

**Solutions**:
1. Verify `VISION_GUARDIAN_ADDRESS` is set correctly
2. Check that Guardian address matches on all nodes
3. Ensure block was actually mined by correct Guardian
4. If wrong Guardian was used, reset chain and restart launch

### Problem: Testnet has Guardian restrictions

**Symptoms**: Testnet nodes waiting for Guardian

**Solution**: Ensure `VISION_NETWORK=testnet` (or unset, as testnet is default)

## Testing

### Test Scenario 1: Testnet Normal Operation
```bash
# Node 1, 2, 3 all with VISION_NETWORK=testnet
# Expected: All nodes mine freely, no Guardian restrictions
```

### Test Scenario 2: Mainnet Guardian Launch
```bash
# Guardian: VISION_IS_GUARDIAN=true, VISION_NETWORK=mainnet-full
# Constellation: VISION_IS_GUARDIAN=false, VISION_NETWORK=mainnet-full
# Expected: Guardian mines 1-3, Constellation waits, then all mine
```

### Test Scenario 3: Guardian Emergency Mining
```bash
# Guardian: VISION_ALLOW_GUARDIAN_EMERGENCY_MINING=true
# Expected: Guardian continues mining after block 3
```

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│  Network Type Check (is_launch_active)                      │
│  ├── testnet → No restrictions (classic behavior)           │
│  └── mainnet-full + launch_guardian_enabled=true            │
│      ├── Block Validation (apply_block_from_peer)           │
│      │   └── Blocks 1-3: Reject if miner != guardian_address│
│      └── Mining Gate (can_mine_with_guardian_launch)        │
│          ├── Non-Guardian: Wait until height >= 3           │
│          └── Guardian: Mine 1-3, then retire                │
└─────────────────────────────────────────────────────────────┘
```

## Quick Reference

### Environment Variables
| Variable | Guardian | Constellation | Testnet | Purpose |
|----------|----------|---------------|---------|---------|
| `VISION_NETWORK` | `mainnet-full` | `mainnet-full` | `testnet` | Network type |
| `VISION_LAUNCH_GUARDIAN_ENABLED` | `true` | `true` | `false` | Enable launch |
| `VISION_GUARDIAN_ADDRESS` | Your address | Guardian's address | N/A | Guardian LAND address |
| `VISION_IS_GUARDIAN` | `true` | `false` | `false` | Mark as Guardian node |
| `VISION_ALLOW_GUARDIAN_EMERGENCY_MINING` | Optional | N/A | N/A | Emergency override |

### Constants
- **LAUNCH_BLOCK_COUNT**: 3 (blocks 1, 2, 3 are Guardian-only)
- **FIRST_MINEABLE_BLOCK**: 1 (block 0 is genesis)
- **GENESIS_HEIGHT**: 0

### Log Targets
- `guardian_launch` - Launch sequence decisions
- `vision_node::miner` - Mining gate messages

---

**Built with Vision Node v1.1.1**  
**Guardian Launch Sequence Implementation Complete**
