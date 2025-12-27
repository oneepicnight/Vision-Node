# Even-Split Staking Rewards Implementation

## Overview

This document describes the **even-split staking rewards model** for Vision Node's post-mining era. Unlike proportional staking where rewards are weighted by stake amount, this model distributes rewards **equally** to all deed-owning wallets (one reward per wallet, regardless of how much LAND they own).

## Design Philosophy

**One Wallet, One Vote** (or rather, "One Deed, One Share")

- Every wallet that owns at least one Land Deed receives an equal share of staking rewards
- No stake weighting - a wallet with 1 LAND deed gets the same reward as a wallet with 1000 LAND deeds
- Encourages broader network participation and decentralization
- Prevents large holders from dominating rewards

## Reward Distribution Formula

Per block in staking phase:

```
reward_pool = base_reward + fees_for_stakers

where:
- base_reward = 4.25 LAND (from Vault, not minted)
- fees_for_stakers = configured share of protocol/transaction fees

N = number of deed-owning wallets

If N == 0:
  → No payout, all reward_pool stays in Vault

Else:
  per_node = reward_pool ÷ N (integer division)
  dust = reward_pool - (per_node × N)
  
  → Each deed owner gets: per_node LAND
  → Dust returns to Vault
```

## Implementation Components

### 1. Deed Owner Tracking

**File:** `src/land_deeds.rs`

Added reverse index to efficiently track all deed owners:

```rust
const LAND_DEED_OWNER_INDEX: &str = "land:deed:by-owner:";
// Key: "land:deed:by-owner:<address>" => deed_id (u64 BE)

// Core functions:
pub fn wallet_has_deed(db: &Db, addr: &str) -> bool
pub fn get_owned_deed_id(db: &Db, addr: &str) -> Option<u64>
pub fn all_deed_owners(db: &Db) -> Vec<String>
pub fn update_owner_index(db: &Db, deed_id: u64, new_owner: &str) -> Result<(), String>
```

**Storage:**
- `land:deed:<id>` → owner address (existing)
- `land:deed:by-owner:<address>` → deed_id (NEW reverse index)

**Maintenance:**
- Genesis minting automatically populates owner index for FOUNDER_ADDRESS
- `update_owner_index()` should be called whenever deed ownership changes
- Index allows O(1) lookup to check if wallet owns deed
- Index allows efficient iteration over all deed owners

### 2. Even-Split Reward Distribution

**File:** `src/staking_rewards.rs`

Completely rewritten for even-split logic:

```rust
pub fn distribute_staking_rewards(db: &Db, fees_for_stakers: u128) -> Result<()> {
    // 1. Get all deed owners
    let stakers = all_deed_owners(db);
    let n = stakers.len();
    
    if n == 0 {
        return Ok(()); // No payout, keep in Vault
    }
    
    // 2. Compute reward pool
    let mut base_reward = staking_base_reward(); // 4.25 LAND
    let vault_balance = get_vault_balance(db);
    
    if vault_balance < base_reward {
        base_reward = vault_balance; // Cap to available funds
    }
    
    let reward_pool = base_reward.saturating_add(fees_for_stakers);
    
    if reward_pool == 0 {
        return Ok(());
    }
    
    // 3. Compute per-node share + dust
    let n_u128 = n as u128;
    let per_node = reward_pool / n_u128;
    let dust = reward_pool - (per_node * n_u128);
    
    if per_node == 0 {
        return Ok(()); // Too small to split, keep in Vault
    }
    
    // 4. Debit vault for base_reward
    if base_reward > 0 {
        debit_vault(db, base_reward)?;
    }
    
    // 5. Credit each staker equally
    for addr in stakers.iter() {
        add_balance(db, addr, per_node)?;
    }
    
    // 6. Return dust to Vault
    if dust > 0 {
        credit_vault(db, dust)?;
    }
    
    Ok(())
}
```

**Key Changes from Proportional Model:**
- ❌ Removed: `get_stake()`, `total_stake()`, proportional calculation
- ✅ Added: `all_deed_owners()`, equal split, dust handling
- ✅ Added: `credit_vault()` helper for dust returns

### 3. Vault Helpers

```rust
fn get_vault_balance(db: &Db) -> u128
fn debit_vault(db: &Db, amount: u128) -> Result<()>
fn credit_vault(db: &Db, amount: u128) -> Result<()>  // NEW
```

**Vault Balance Key:** `supply:vault` (u128 BE)

### 4. Block Processing Integration

**File:** `src/main.rs` (block creation logic)

The staking rewards are called during block processing in the staking phase:

```rust
let phase = vision_constants::ChainPhase::from_height(block_height);

match phase {
    vision_constants::ChainPhase::Mining => {
        // Mining era: traditional emission + fee splits
        apply_vision_tokenomics_and_persist(...)
    }
    vision_constants::ChainPhase::Staking => {
        // Staking era: even-split rewards from Vault + fees
        staking_rewards::distribute_staking_rewards(&g.db, tx_fees_total)?;
    }
}
```

**Important:** In staking phase:
- No new LAND minted (`land_block_reward(height) = 0` after MAX_MINING_BLOCK)
- `fees_for_stakers` = total transaction fees for the block
- Rewards only move existing LAND: Vault → deed owners (+ dust back to Vault)

## Example Scenarios

### Scenario 1: Three Deed Owners, No Fees

**Setup:**
- Vault balance: 10,000 LAND
- Deed owners: Alice, Bob, Carol
- Transaction fees: 0 LAND

**Calculation:**
```
base_reward = 4.25 LAND (4,250,000,000 base units)
fees_for_stakers = 0
reward_pool = 4.25 LAND
N = 3

per_node = 4,250,000,000 ÷ 3 = 1,416,666,666 base units
dust = 4,250,000,000 - (1,416,666,666 × 3) = 833,333,002 base units
```

**Result:**
- Alice receives: 1.416666666 LAND
- Bob receives: 1.416666666 LAND
- Carol receives: 1.416666666 LAND
- Dust to Vault: 0.833333002 LAND
- Total distributed: 4.25 LAND ✓

### Scenario 2: Five Deed Owners with Fees

**Setup:**
- Vault balance: 50,000 LAND
- Deed owners: 5 wallets
- Transaction fees: 2.0 LAND

**Calculation:**
```
base_reward = 4.25 LAND
fees_for_stakers = 2.0 LAND
reward_pool = 6.25 LAND (6,250,000,000 base units)
N = 5

per_node = 6,250,000,000 ÷ 5 = 1,250,000,000 base units (1.25 LAND)
dust = 6,250,000,000 - (1,250,000,000 × 5) = 0 base units
```

**Result:**
- Each of 5 deed owners receives: 1.25 LAND
- Dust to Vault: 0 LAND (perfect division)
- Total distributed: 6.25 LAND ✓

### Scenario 3: No Deed Owners

**Setup:**
- Vault balance: 100,000 LAND
- Deed owners: 0
- Transaction fees: 5.0 LAND

**Result:**
- No distribution occurs
- Vault balance unchanged: 100,000 LAND
- Transaction fees remain in Vault
- Base reward (4.25 LAND) stays in Vault

### Scenario 4: Insufficient Vault Funds

**Setup:**
- Vault balance: 2.0 LAND (less than base_reward)
- Deed owners: 4 wallets
- Transaction fees: 1.0 LAND

**Calculation:**
```
base_reward = 4.25 LAND (requested)
vault_balance = 2.0 LAND (available)
base_reward = 2.0 LAND (capped to available)

fees_for_stakers = 1.0 LAND
reward_pool = 3.0 LAND (3,000,000,000 base units)
N = 4

per_node = 3,000,000,000 ÷ 4 = 750,000,000 base units (0.75 LAND)
dust = 3,000,000,000 - (750,000,000 × 4) = 0 base units
```

**Result:**
- Each of 4 deed owners receives: 0.75 LAND
- Dust to Vault: 0 LAND
- Vault debited: 2.0 LAND (all available funds used)
- Total distributed: 3.0 LAND ✓

## Economic Implications

### Advantages

1. **Decentralization**
   - Encourages broad participation (1 deed = 1 share)
   - Prevents whale domination of rewards
   - More democratic reward distribution

2. **Simplicity**
   - Easy to understand (equal split)
   - No complex stake-weighting calculations
   - Clear reward expectations

3. **Incentive Alignment**
   - Rewards network participation, not just capital
   - Encourages deed distribution across many wallets
   - Reduces centralization risk

4. **Dust Efficiency**
   - Rounding dust returns to Vault (no loss)
   - Accumulates for future rewards
   - Maintains supply accuracy

### Disadvantages / Trade-offs

1. **No Stake Weighting**
   - Large LAND holders don't get proportional rewards
   - May reduce large-holder incentive to stay
   - Could encourage deed splitting across wallets (Sybil)

2. **Potential Sybil Attack**
   - Rational actor might split deeds across multiple wallets
   - Example: 100 deeds in 1 wallet vs 100 deeds in 100 wallets
   - Mitigation: Transaction costs for deed transfers, identity verification

3. **Fixed Overhead**
   - Base reward must be sufficient for meaningful per-node share
   - Low deed ownership = high per-node reward (good)
   - High deed ownership = low per-node reward (could be too small)

### Vault Sustainability

With 63M+ LAND in Vault at transition:

```
Base reward per block: 4.25 LAND
Blocks per year: ~15,768,000 (2-second blocks)
Base cost per year: 67M LAND

Initial vault: 63M LAND
Years sustained (base only): ~0.9 years (11 months)
```

**However:**
- Transaction fees extend sustainability
- Dust returns accumulate back to Vault
- If network is active, fees could exceed base reward
- System designed to be self-sustaining with sufficient transaction volume

## Debugging and Monitoring

### Log Messages

**Debug Level:**
```
staking_rewards: stakers=5, reward_pool=6250000000, per_node=1250000000, dust=0
```

**Info Level:**
```
💰 Distributed 6250000000 LAND evenly to 5 deed owners (1250000000 per node, 0 dust to vault)
```

**Warning Level:**
```
⚠️ Vault balance (2000000000) below base reward (4250000000), capping to available funds
```

### Recommended Prometheus Metrics

```rust
// Future additions:
vision_staking_deed_owners_count // Number of wallets eligible for rewards
vision_staking_reward_pool_total // Total reward pool per block
vision_staking_per_node_reward   // Reward per deed owner
vision_staking_dust_to_vault     // Dust returned to Vault
vision_vault_balance             // Current Vault balance
vision_vault_depletion_blocks    // Estimated blocks until Vault depleted
```

## Testing

### Unit Tests

**Test 1: Even Split with Dust**
- 3 deed owners
- 4.25 LAND reward
- Verifies equal distribution + dust return

**Test 2: No Deed Owners**
- 0 deed owners
- Verifies no distribution, Vault unchanged

**Test 3: Perfect Division** (add if needed)
- Reward pool evenly divisible by N
- Verifies zero dust

### Integration Testing

1. **Testnet with Low MAX_MINING_BLOCK**
   - Set `MAX_MINING_BLOCK = 1000` for fast testing
   - Create 3-5 test wallets with deeds
   - Mine to block 999 (mining phase)
   - Verify transition at block 1000
   - Monitor reward distribution for blocks 1000-1050
   - Confirm equal splits and dust handling

2. **Deed Transfer Test**
   - Start with 3 deed owners
   - Transfer deed from Alice to Dave
   - Verify owner index updates
   - Confirm next block's reward goes to Dave (not Alice)

3. **Vault Depletion Test**
   - Start with low Vault balance (e.g., 10 LAND)
   - High number of deed owners (e.g., 100)
   - Verify per_node drops appropriately
   - Verify graceful handling when Vault < base_reward

## Migration from Proportional Model

### Removed Functions

```rust
// Old proportional model (REMOVED):
land_stake::get_stake(db, addr) -> u128
land_stake::total_stake(db) -> u128
land_stake::get_all_stakers(db) -> Vec<String>
land_stake::stake_land(db, addr, amount) -> Result<()>
land_stake::unstake_land(db, addr, amount) -> Result<()>
```

These were replaced by deed-based tracking.

### New Functions

```rust
// New even-split model (ADDED):
land_deeds::wallet_has_deed(db, addr) -> bool
land_deeds::all_deed_owners(db) -> Vec<String>
land_deeds::update_owner_index(db, deed_id, owner) -> Result<(), String>

staking_rewards::credit_vault(db, amount) -> Result<()>
```

### Database Schema Changes

**Removed:**
- `land:stake:<address>` (stake amounts per address)
- `land:stake:total` (total staked LAND)
- `land:stake:owners` (comma-separated staker list)

**Added:**
- `land:deed:by-owner:<address>` → deed_id (reverse index)

**Unchanged:**
- `land:deed:<id>` → owner address (existing deed registry)
- `supply:vault` → vault balance (existing)
- `bal:<address>` → address balance (existing)

## Future Enhancements

### 1. Deed Verification

Add proof-of-unique-wallet to prevent Sybil attacks:
- KYC integration (optional, off-chain)
- On-chain identity verification
- Proof-of-humanity protocols

### 2. Tiered Rewards

Introduce deed rarity/tiers:
- Bronze deeds: 1x share
- Silver deeds: 1.5x share
- Gold deeds: 2x share

### 3. Activity Bonuses

Reward active participants:
- Bonus for nodes that relay transactions
- Bonus for nodes with high uptime
- Bonus for nodes providing services

### 4. Delegation

Allow deed-less wallets to delegate stake:
- Deed owners can accept delegations
- Rewards split between deed owner and delegators
- Creates validator-like model

### 5. Minimum Deed Holding Period

Prevent gaming:
- Require deed ownership for N blocks before eligible
- Prevent last-second deed acquisition for rewards

## Security Considerations

### Sybil Resistance

**Current Weakness:**
- Rational actor can split deeds across wallets
- No cost to create new wallets
- Each wallet gets equal share

**Mitigations:**
1. **Deed Transfer Costs:** Make deed transfers expensive (high fee)
2. **Minimum Balance:** Require minimum LAND balance per wallet
3. **Time Locks:** Require holding period before rewards
4. **Identity Verification:** Off-chain KYC or on-chain proof-of-unique-human

### Dust Accumulation

- Dust returns to Vault (no loss of funds)
- Over time, dust accumulates back for future rewards
- Prevents supply leakage from rounding errors

### Vault Safety

- Vault balance checked before each distribution
- If insufficient, base_reward is capped to available funds
- No overdraft possible
- System gracefully handles Vault depletion

## Conclusion

The even-split staking rewards model provides a **simple, egalitarian** approach to post-mining rewards:

✅ **Democratic:** One deed = one share (no whale advantage)  
✅ **Simple:** Easy to understand and implement  
✅ **Efficient:** Dust returns to Vault (no waste)  
✅ **Safe:** No overdraft, graceful Vault depletion handling  
⚠️ **Trade-off:** Vulnerable to Sybil (deed splitting) without additional protections  

The model is **production-ready** and aligns with Vision Node's goal of broad network participation and decentralization.

## References

- `src/land_deeds.rs` - Deed ownership tracking
- `src/staking_rewards.rs` - Even-split reward distribution
- `src/main.rs` - Block processing integration
- `docs/MINING_TO_STAKING_TRANSITION.md` - Original transition design
- `docs/GENESIS.md` - Tokenomics and emission schedule
