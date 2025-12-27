# Vision Network Tokenomics

## Overview

The Vision Network uses a dual-token model:
1. **Native Token**: PoW-mined utility token for network operations
2. **CASH Token**: Game-integrated currency launched at mainnet block 1,000,000

This document details the economic model, emission schedule, and token distribution.

## Native Token (PoW Mined)

### Supply and Emission

#### Initial Supply
- **Genesis**: 0 tokens (pure PoW launch)
- **Pre-mine**: None
- **Founder allocation**: 0% (fair launch)

#### Emission Schedule
```rust
pub fn land_block_reward(height: u64) -> u128 {
    // After max mining block, emission is 0 forever
    if height >= MAX_MINING_BLOCK { return 0; }
    
    // Calculate which era we're in (0-indexed)
    let era = height / BLOCKS_PER_ERA;
    
    match era {
        0 => land_amount(34.0),   // Era 1: 34 LAND/block
        1 => land_amount(17.0),   // Era 2: 17 LAND/block
        2 => land_amount(8.5),    // Era 3: 8.5 LAND/block
        3 => land_amount(4.25),   // Era 4: 4.25 LAND/block
        _ => 0,
    }
}

// Constants:
// BLOCKS_PER_ERA = 15,768,000 (1 year at 2s blocks)
// MAX_MINING_BLOCK = 63,072,000 (4 years total)
// PROTOCOL_FEE_LAND = 2.0 LAND (deducted from emission)
```

#### Emission Chart
| Era | Block Range | Reward per Block | Protocol Fee | Miner Gets | Era Total Supply | Cumulative Supply |
|-----|-------------|------------------|--------------|------------|------------------|-------------------|
| 1 | 0 - 15,767,999 | 34 LAND | 2 LAND | 32 LAND | 536,112,000 | 536,112,000 |
| 2 | 15,768,000 - 31,535,999 | 17 LAND | 2 LAND | 15 LAND | 268,056,000 | 804,168,000 |
| 3 | 31,536,000 - 47,303,999 | 8.5 LAND | 2 LAND | 6.5 LAND | 134,028,000 | 938,196,000 |
| 4 | 47,304,000 - 63,071,999 | 4.25 LAND | 2 LAND | 2.25 LAND | 67,014,000 | 1,005,210,000 |
| Post-Mining | 63,072,000+ | 0 LAND | 0 LAND | 0 LAND | 0 | 1,005,210,000 |

**Total Hard Cap**: 1,005,210,000 LAND (fixed forever after block 63,072,000)  
**Mining Duration**: 4 years (63,072,000 blocks at 2 seconds each)  
**No Tail Emission**: Mining ends permanently at block 63,072,000

### Block Time and Production Rate
- **Target**: 2 seconds per block
- **Adjustment**: Difficulty retargets every 100 blocks
- **Daily Production**: ~43,200 blocks/day
  - Era 1: 1,468,800 LAND/day (34 LAND/block)
  - Era 2: 734,400 LAND/day (17 LAND/block)
  - Era 3: 367,200 LAND/day (8.5 LAND/block)
  - Era 4: 183,600 LAND/day (4.25 LAND/block)

### Token Distribution

#### Protocol Fee (Fixed 2 LAND per block)
The 2 LAND protocol fee is deducted from each block's emission and split:
- **Vault**: 50% (1 LAND)
- **Founder**: 30% (0.6 LAND)
- **Ops**: 20% (0.4 LAND)

#### Miner Rewards
Miner receives: `Block Emission - Protocol Fee`

Example for Era 1 (34 LAND block):
- Total Emission: 34 LAND
- Protocol Fee: 2 LAND
  - Vault: 1 LAND (50%)
  - Founder: 0.6 LAND (30%)
  - Ops: 0.4 LAND (20%)
- **Miner Gets**: 32 LAND

#### Transaction Fee Distribution
Transaction fees are split among foundation addresses (miner gets block reward only):
- Vault: 50%
- Founder: 30%
- Ops: 20%

### Era Transitions

| Era | Block Height | Approx Date | Block Reward | Miner Gets | Event |
|-----|--------------|-------------|--------------|------------|-------|
| 1 | 0 - 15,767,999 | Launch - Year 1 | 34 LAND | 32 LAND | Network start |
| 2 | 15,768,000 - 31,535,999 | Year 1-2 | 17 LAND | 15 LAND | First halving |
| 3 | 31,536,000 - 47,303,999 | Year 2-3 | 8.5 LAND | 6.5 LAND | Second halving |
| 4 | 47,304,000 - 63,071,999 | Year 3-4 | 4.25 LAND | 2.25 LAND | Third halving |
| Post-Mining | 63,072,000+ | After Year 4 | 0 LAND | 0 LAND | Mining ends, staking begins |

*Each era is exactly 15,768,000 blocks (365.25 days at 2s blocks)*  
*Total mining duration: 4 years*

## CASH Token

### Genesis Event

#### Activation Height
- **Mainnet Only**: Block 1,000,000
- **Testnet**: CASH not activated (testnet sunsets at 1M)

#### Initial Supply Formula
```rust
pub fn calculate_cash_genesis_supply(
    total_land_staked: u64,
    pioneer_count: u64,
    block_height: u64
) -> u128 {
    // Base supply scales with network adoption
    let base = 1_000_000_000_000_000_000_000_000u128; // 1M CASH (18 decimals)
    
    // Bonus for early land staking (up to 2x multiplier)
    let stake_multiplier = (total_land_staked * 10_000 / GENESIS_LAND_DEED_TOTAL).min(20_000);
    
    // Bonus for pioneer participation (up to 1.5x multiplier)
    let pioneer_multiplier = (pioneer_count * 5_000 / 1_000).min(15_000);
    
    let total_multiplier = (10_000 + stake_multiplier + pioneer_multiplier) as u128;
    
    base * total_multiplier / 10_000
}
```

**Expected Range**: 1M - 3.5M CASH at genesis (depending on adoption)

#### Distribution Method
1. **Land Deed Holders**: Pro-rata by staked land deeds
2. **Pioneer Miners**: Bonus for miners who found blocks 1-999,999
3. **Treasury**: 10% of initial supply reserved for governance

### CASH Utility

#### In-Game Integration
- **GTA V Currency**: Integrated with FiveM server
- **Property Purchases**: Buy in-game properties with CASH
- **Services**: Pay for jobs, races, businesses
- **Trading**: P2P transfers via blockchain

#### Game Hooks
```rust
// Minting event (Genesis only)
game_hooks::on_cash_mint(player, amount, source);

// Land usage (ongoing)
game_hooks::on_land_use(player, plot_id, action);

// Property interactions
game_hooks::on_property_damage(player, property_id, damage);

// Job completions
game_hooks::on_job_result(player, job_type, success, reward);

// Race completions
game_hooks::on_race_completed(player, race_id, position, prize);
```

### Post-Genesis CASH

#### No Additional Minting
- **One-time Genesis**: CASH minted once at block 1,000,000
- **Fixed Supply**: No inflation; circulating supply fixed at genesis amount
- **Deflationary Pressure**: Lost keys reduce effective supply over time

#### Secondary Distribution
- **Trading**: Users trade CASH peer-to-peer
- **Earning**: Complete in-game activities for CASH rewards (funded by treasury)
- **Marketplaces**: Buy/sell goods and services with CASH

## Economic Security

### Mining Incentives

#### Security Budget Calculation
```
Daily Security = (Miner Emission Value) + (Transaction Fees Value)
               = (43,200 blocks * miner_reward * price) + daily_fees
```

Example at $1/LAND:
- Era 1: $1,382,400/day (32 LAND/block * 43,200 blocks)
- Era 2: $648,000/day (15 LAND/block * 43,200 blocks)
- Era 3: $280,800/day (6.5 LAND/block * 43,200 blocks)
- Era 4: $97,200/day (2.25 LAND/block * 43,200 blocks)
- Post-Mining: Transaction fees only (staking rewards from vault)

### Attack Cost Analysis

#### 51% Attack
Required hashrate: >50% of network  
Daily cost: >50% of daily security budget  
**Defense**: High mining profitability attracts honest miners

#### Double-Spend Attack
Confirmation depth: 6 blocks (12 seconds)  
Reorg protection: MAX_REORG_DEPTH = 64 blocks  
**Defense**: Deep confirmations + reorg limits

#### Long-Range Attack
Genesis hash: Hardcoded in software  
Checkpoints: Static checkpoints at milestones  
**Defense**: New nodes reject alternative histories

## Treasury Management

### Funding Sources
1. **Ops/Treasury Split**: 20% of all mining rewards
2. **Transaction Fees**: 20% of all tx fees  
3. **CASH Reserve**: 10% of CASH genesis supply

### Allocation Strategy
- **Development**: 40% (core protocol, testing, audits)
- **Marketing**: 20% (community growth, partnerships)
- **Operations**: 25% (servers, infrastructure, monitoring)
- **Reserve**: 15% (emergency fund, legal, unexpected costs)

### Governance
- **Proposal System**: Token-weighted voting for treasury spending
- **Multisig**: 3-of-5 founder multisig controls treasury
- **Transparency**: All transactions published on-chain

## Inflationary Dynamics

### Native Token Inflation Schedule
| Era | Block Range | Era Supply | % of Total | Notes |
|-----|-------------|------------|------------|-------|
| 1 | 0 - 15.7M | 536M LAND | 53.3% | Initial distribution |
| 2 | 15.7M - 31.5M | 268M LAND | 26.7% | First halving |
| 3 | 31.5M - 47.3M | 134M LAND | 13.3% | Second halving |
| 4 | 47.3M - 63M | 67M LAND | 6.7% | Final mining era |
| Post | 63M+ | 0 LAND | 0% | Mining ended, fixed supply |

**Total Supply**: 1,005,210,000 LAND (fixed forever)  
**No Inflation**: After block 63,072,000, supply is permanently capped

### CASH Token Inflation
- **Year 1-∞**: 0% (fixed supply after genesis)

## Comparison to Other Networks

| Metric | Bitcoin | Ethereum | Vision Native | Vision CASH |
|--------|---------|----------|---------------|-------------|
| Initial Supply | 0 | 72M | 0 | 1-3.5M |
| Consensus | PoW | PoS | PoW | PoW (inherited) |
| Block Time | 10min | 12s | 2s | - |
| Supply Cap | 21M | ∞ | 1.005B | Fixed |
| Tail Emission | No | No | No | No |
| Era Interval | 210k blocks | N/A | 15.768M blocks | N/A |
| Mining Duration | ~140 years | Ended | 4 years | N/A |

## Future Considerations

### Potential Adjustments (via Governance)
- **Emission Curve**: Modify halving schedule if security budget insufficient
- **Fee Structure**: Dynamic fee market (EIP-1559 style)
- **Treasury Allocation**: Reallocate based on network needs
- **CASH Expansion**: Secondary CASH mints (requires supermajority vote)

### Upgrade Paths
- **Merged Mining**: Allow Bitcoin miners to mine Vision blocks
- **Cross-Chain Bridges**: Connect to other networks for liquidity
- **DeFi Integration**: Lending, borrowing, AMM with CASH/native tokens

## Audit and Transparency

### Emission Verification
```bash
# Verify emission schedule
cargo run --bin verify-emissions

# Check total supply
curl http://localhost:7070/supply
```

### Treasury Transparency
- **Public Addresses**: All treasury addresses published
- **Block Explorer**: All transactions visible on-chain
- **Quarterly Reports**: Treasury activity reported to community

## References

- [GENESIS.md](./GENESIS.md) - Genesis block and initial distribution
- [CASH_SYSTEM.md](./CASH_SYSTEM.md) - CASH token deep dive
- [GOVERNANCE_OVERVIEW.md](./GOVERNANCE_OVERVIEW.md) - Voting and proposals
- [Emission Code](../src/main.rs) - `calculate_block_reward()` implementation
