# Guardian Launch - Quick Reference

## Implementation Complete ✅

### What Was Built

**4-Part Guardian Launch Sequence** (mainnet-full only, testnet unchanged):

1. **Config** - Guardian launch settings in NetworkConfig
2. **Consensus** - Block validation for Guardian-only blocks 1-3
3. **Mining Gate** - Non-Guardian miners wait, Guardian retires after block 3
4. **Behavior** - Clear logs, network isolation, state transitions

### Files Modified

- `src/constants.rs` (NEW) - LAUNCH_BLOCK_COUNT = 3, network identifiers
- `src/config/network.rs` (EXTENDED) - Guardian launch fields, env vars, helpers
- `src/config/mod.rs` - Registered network module
- `src/main.rs` - Added constants module, Guardian validation in apply_block_from_peer()
- `src/miner/manager.rs` - Added mining gate in ActiveMiner::start()

### Build Status

✅ **Build Successful**
- Binary: `target/release/vision-node.exe`
- Size: 24.75 MB
- Timestamp: 2025-12-05 09:34:26

## Network Behavior

### Testnet (Default)
```bash
VISION_NETWORK=testnet  # or unset
# Anyone can mine any block - wild playground
```

### Mainnet-Full (Guardian Launch)
```bash
# Guardian Node:
VISION_NETWORK=mainnet-full
VISION_LAUNCH_GUARDIAN_ENABLED=true
VISION_GUARDIAN_ADDRESS=LAND1234...
VISION_IS_GUARDIAN=true

# Constellation Node:
VISION_NETWORK=mainnet-full
VISION_LAUNCH_GUARDIAN_ENABLED=true
VISION_GUARDIAN_ADDRESS=LAND1234...  # Same as Guardian
VISION_IS_GUARDIAN=false
```

## Launch Sequence Flow

```
Height 0 (Genesis):
  - Genesis block exists, no mining

Height 1-3 (Launch Phase):
  - Guardian: ✅ Mines blocks
  - Constellation: ⏱️  Waits (mining blocked)
  - Validation: ❌ Rejects non-Guardian blocks

Height 4+ (Post-Launch):
  - Guardian: 🎖️  Retires (mining stops)
  - Constellation: ⚡ Mines normally
  - Validation: ✅ Accepts any miner
```

## Key Functions

### validate_guardian_launch() - src/main.rs
- Called during block validation
- Checks blocks 1-3 on mainnet-full
- Rejects if miner != guardian_address
- Bypassed completely on testnet

### can_mine_with_guardian_launch() - src/miner/manager.rs
- Called before starting mining
- Non-Guardian waits until height >= 3
- Guardian mines 1-3, then stops
- Bypassed completely on testnet

### is_launch_active() - src/config/network.rs
```rust
pub fn is_launch_active(&self) -> bool {
    self.is_mainnet_full() && self.launch_guardian_enabled
}
```

## Constants (src/constants.rs)

```rust
pub const LAUNCH_BLOCK_COUNT: u64 = 3;           // Guardian mines blocks 1, 2, 3
pub const NETWORK_MAINNET_FULL: &str = "mainnet-full";
pub const NETWORK_TESTNET: &str = "testnet";
pub const GENESIS_HEIGHT: u64 = 0;
pub const FIRST_MINEABLE_BLOCK: u64 = 1;
```

## Testing Checklist

- [ ] Testnet: Multiple nodes mine freely without Guardian restrictions
- [ ] Mainnet Guardian: Mines blocks 1, 2, 3 successfully
- [ ] Mainnet Constellation: Waits during launch, mines after block 3
- [ ] Block Validation: Rejects non-Guardian blocks 1-3 on mainnet-full
- [ ] Block Validation: Accepts any miner on testnet
- [ ] Guardian Retirement: Guardian stops after block 3 (no emergency override)
- [ ] Emergency Override: Guardian continues with VISION_ALLOW_GUARDIAN_EMERGENCY_MINING=true
- [ ] Logging: Clear messages with guardian_launch target

## Common Issues

**Constellation won't mine:**
- Check height >= 3 (Guardian must mine 1-3 first)
- Verify peer connections > 0

**Guardian keeps mining after block 3:**
- Check VISION_ALLOW_GUARDIAN_EMERGENCY_MINING=false (should be false or unset)

**Testnet has restrictions:**
- Ensure VISION_NETWORK=testnet or unset (testnet is default)

**Block validation fails:**
- Verify all nodes use same VISION_GUARDIAN_ADDRESS
- Check Guardian actually mined the blocks

## Log Examples

### Guardian Mining Launch Block
```
[guardian_launch] 🎖️  [MINING_GATE] Guardian mining launch block 1
[guardian_launch] ✅ Guardian launch block 1 validated
```

### Constellation Waiting
```
[guardian_launch] ⏱️  [MINING_GATE] Waiting for Guardian launch blocks (1/3) on mainnet-full...
```

### Guardian Retiring
```
[guardian_launch] 🎖️  [MINING_GATE] Guardian launch complete at height 3 – standing down miner (mainnet-full).
```

### Validation Rejection
```
[guardian_launch] ❌ Guardian launch validation failed
Block 2 must be mined by Guardian (LAND1234...) on mainnet-full, got LAND5678...
```

---

**Full Documentation**: See GUARDIAN_LAUNCH_SEQUENCE.md

**Implementation**: 4 parts complete, build successful (24.75 MB)

**Network Strategy**: Testnet = wild playground | Mainnet-Full = ceremonial launch
