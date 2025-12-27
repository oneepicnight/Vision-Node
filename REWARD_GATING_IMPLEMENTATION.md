# Reward Gating System - Implementation Complete

## Overview

Prevents "fake rich" scenarios on testnets by gating block rewards based on sync health, peer connectivity, and network participation. Miners only receive rewards when their node is properly synced and well-connected to the network.

## Implementation Summary

### Part 1: Configuration (src/config/miner.rs)

Added `RewardEligibilityConfig` struct to `MinerConfig`:

```rust
pub struct RewardEligibilityConfig {
    /// Minimum number of connected peers required before we pay block rewards.
    pub min_peers_for_rewards: u16,
    /// Maximum allowed desync between local tip and network estimated height
    /// before we pay block rewards.
    pub max_reward_desync_blocks: u64,
    /// Height below which we do not pay any block subsidy (warm-up era).
    pub reward_warmup_height: u64,
}
```

**Defaults** (testnet-friendly):
- `min_peers_for_rewards: 3`
- `max_reward_desync_blocks: 5`
- `reward_warmup_height: 0`

### Part 2: Eligibility Helper (src/config/miner.rs)

```rust
pub struct SyncHealthSnapshot {
    pub connected_peers: u16,
    pub p2p_health: String,      // "ok", "stable", "isolated", etc.
    pub sync_height: u64,
    pub network_estimated_height: u64,
}

pub fn is_reward_eligible(
    cfg: &RewardEligibilityConfig,
    snapshot: &SyncHealthSnapshot,
    current_height: u64,
) -> bool
```

**Eligibility Rules:**
1. ✅ Block height >= `reward_warmup_height`
2. ✅ Connected peers >= `min_peers_for_rewards`
3. ✅ P2P health is "ok", "stable", or "immortal"
4. ✅ Desync <= `max_reward_desync_blocks`

### Part 3: Sync Health Snapshot (src/main.rs - Chain impl)

Added method to Chain struct:

```rust
pub fn current_sync_health(&self) -> config::miner::SyncHealthSnapshot {
    // Fetches peer info from PEER_MANAGER
    // Calculates p2p_health based on connected peer count
    // Returns snapshot for eligibility check
}
```

**P2P Health Calculation:**
- `isolated`: 0 peers
- `weak`: 1 peer
- `ok`: 2-7 peers
- `stable`: 8-31 peers
- `immortal`: 32+ peers

### Part 4: Reward Gating Logic (src/main.rs - apply_tokenomics)

Modified `apply_tokenomics()` function:

```rust
// Check eligibility
let sync_health = chain.current_sync_health();
let miner_cfg = config::miner::MinerConfig::load_or_create("miner.json").unwrap_or_default();
let eligible = config::miner::is_reward_eligible(
    &miner_cfg.reward_eligibility,
    &sync_health,
    height,
);

// Gate miner emission
if !eligible {
    miner_emission = 0;
}

// Redirect miner tithe share to vault if not eligible
let tithe_miner = if eligible { tithe_miner_base } else { 0 };
let tithe_vault = tithe_vault_base + if !eligible { tithe_miner_base } else { 0 };
```

### Part 5: State Change Logging (src/main.rs - apply_tokenomics)

Uses atomic boolean to track eligibility state changes:

```rust
static LAST_ELIGIBILITY_STATE: std::sync::atomic::AtomicBool = ...;

if eligible != last_eligible {
    if !eligible {
        tracing::warn!(
            target: "rewards",
            "⚠️ Block rewards DISABLED: node not yet eligible (unsynced / insufficient peers)"
        );
    } else {
        tracing::info!(
            target: "rewards",
            "✅ Block rewards ENABLED: node is now eligible for rewards"
        );
    }
}
```

## Behavior

### When Rewards ENABLED (eligible = true)
- ✅ Miner receives full emission (e.g., 5 LAND per block)
- ✅ Miner receives 20% of 2 LAND block tithe
- ✅ Foundation addresses receive their shares (Vault/Fund/Treasury)
- ✅ Transaction fees distributed normally

### When Rewards DISABLED (eligible = false)
- ⛔ Miner emission = 0 LAND
- ⛔ Miner tithe share redirected to Vault
- ✅ Foundation addresses still receive their tithe shares (Vault gets extra 20%)
- ✅ Transaction fees still distributed (foundation needs funding)
- ⚠️ Warning logged with diagnostic info (peers, height, health)

## Configuration

Edit `miner.json`:

```json
{
  "reward_address": "land1...",
  "auto_mine": false,
  "max_txs": 1000,
  "reward_eligibility": {
    "min_peers_for_rewards": 3,
    "max_reward_desync_blocks": 5,
    "reward_warmup_height": 0
  }
}
```

### Recommended Settings

**Testnet:**
```json
{
  "min_peers_for_rewards": 3,
  "max_reward_desync_blocks": 5,
  "reward_warmup_height": 0
}
```

**Mainnet:**
```json
{
  "min_peers_for_rewards": 5,
  "max_reward_desync_blocks": 10,
  "reward_warmup_height": 1000
}
```

## Testing

### Test Scenario 1: Isolated Node
1. Start node with no peers
2. Mine blocks
3. **Expected:** `⚠️ Block rewards DISABLED` (p2p_health: "isolated", peers: 0)
4. Miner balance remains 0 LAND

### Test Scenario 2: Network Join
1. Node connects to 3+ peers
2. Syncs to network height
3. **Expected:** `✅ Block rewards ENABLED`
4. Miner starts earning rewards

### Test Scenario 3: Desync
1. Node falls >5 blocks behind network
2. **Expected:** `⚠️ Block rewards DISABLED` (desync > threshold)
3. Node catches up
4. **Expected:** `✅ Block rewards ENABLED`

### Test Scenario 4: Peer Loss
1. Node drops to <3 peers
2. **Expected:** `⚠️ Block rewards DISABLED` (insufficient peers)
3. Peers reconnect
4. **Expected:** `✅ Block rewards ENABLED`

## Monitoring

Watch logs for reward status changes:

```bash
# Look for state changes
grep "Block rewards" vision-node.log

# Check current eligibility
grep "rewards" vision-node.log | tail -n 20
```

Expected log output:
```
[WARN] rewards: ⚠️ Block rewards DISABLED: node not yet eligible (unsynced / insufficient peers)
  height=150
  connected_peers=1
  p2p_health=weak
  sync_height=150
  network_estimated_height=155
```

```
[INFO] rewards: ✅ Block rewards ENABLED: node is now eligible for rewards
  height=156
  connected_peers=4
  p2p_health=ok
```

## API Endpoints

Check sync health via existing endpoints:

```bash
# Check constellation status
curl http://127.0.0.1:7070/constellation/status | jq

# Response includes:
# - connected_peers
# - p2p_health
# - sync_height
# - network_estimated_height
# - is_syncing
```

## Files Modified

1. **src/config/miner.rs**
   - Added `RewardEligibilityConfig` struct
   - Added `SyncHealthSnapshot` struct
   - Added `is_reward_eligible()` function
   - Updated `MinerConfig::default()` to include `reward_eligibility`

2. **src/main.rs**
   - Added `Chain::current_sync_health()` method
   - Modified `apply_tokenomics()` to check eligibility
   - Added eligibility state change logging
   - Gated `miner_emission` based on eligibility
   - Redirected `tithe_miner` to vault when not eligible

3. **src/p2p/peer_manager.rs**
   - Added `get_all_peers_blocking()` method for sync contexts

## Benefits

1. **Prevents Fake Rich Scenarios:** Solo miners can't earn unlimited testnet tokens
2. **Encourages Network Participation:** Rewards require being connected and synced
3. **Foundation Funding Protected:** Vault/Fund/Treasury still receive shares even when miner disabled
4. **Flexible Configuration:** Easy to adjust thresholds for different networks
5. **Clear Diagnostics:** Log messages explain why rewards are disabled
6. **Fair Transition:** Rewards automatically enable when node becomes healthy

## Future Enhancements

Potential improvements:
- [ ] Gradual reward scaling (50% at 2 peers, 75% at 3 peers, 100% at 5+ peers)
- [ ] Time-based requirements (must be synced for N minutes before rewards)
- [ ] Reputation-based requirements (must have good peer score)
- [ ] Guardian verification (guardians verify constellation eligibility)
- [ ] Historical uptime tracking (require X% uptime over Y blocks)

## Implementation Date
December 5, 2025

## Status
✅ Complete and ready for testing
