# Vision CASH System: The Native Currency

**Status**: Active (Mainnet-Ready)  
**Token Type**: CASH  
**Total Supply**: Dynamic (emission-based)  
**Emission Model**: Block rewards with halvings  
**Standard**: Native blockchain token (consensus-enforced)  

---

## 🪙 Overview

**CASH** is the native currency of the Vision blockchain, functioning as:

1. **Transaction Fees**: Pay for all on-chain operations
2. **Miner Rewards**: Incentivize block production
3. **In-Game Currency**: Purchase items, upgrades, and services
4. **Governance Token**: Weighted voting in DAO proposals
5. **Store of Value**: Deflationary supply via fee burning

---

## 📊 Token Economics

### Emission Schedule

CASH is minted via **block rewards** following a predictable schedule:

| Epoch | Blocks | Reward per Block | Total Minted | Duration* |
|-------|--------|------------------|--------------|-----------|
| 1 | 0 - 999,999 | 10,000 CASH | 10B CASH | ~23 days |
| 2 | 1M - 1,999,999 | 5,000 CASH | 5B CASH | ~23 days |
| 3 | 2M - 2,999,999 | 2,500 CASH | 2.5B CASH | ~23 days |
| 4 | 3M - 3,999,999 | 1,250 CASH | 1.25B CASH | ~23 days |
| 5 | 4M - 4,999,999 | 625 CASH | 625M CASH | ~23 days |
| ... | ... | ... | ... | ... |
| ∞ | 21M+ | 1 CASH | Tail emission | Forever |

\* *Based on 2-second block time*

**Halving Formula**:

```rust
pub fn emission_at_height(height: u64) -> u128 {
    const INITIAL_REWARD: u128 = 10_000;
    const HALVING_INTERVAL: u64 = 1_000_000;
    const MIN_REWARD: u128 = 1;
    
    let halvings = height / HALVING_INTERVAL;
    let reward = INITIAL_REWARD >> halvings; // Bit shift = divide by 2^halvings
    
    reward.max(MIN_REWARD) // Never go below 1 CASH
}
```

### Total Supply Model

```
Total Supply (after N halvings) = Σ(halving_reward * 1,000,000 blocks)

Approximate Max Supply = 20,000,000,000 CASH (20B)
Tail Emission Forever = 1 CASH per block (perpetual inflation ~1.5%/year)
```

---

## 💸 Block Reward Distribution

Each mined block distributes CASH to multiple stakeholders:

| Recipient | Percentage | Purpose |
|-----------|------------|---------|
| **Miner** | 70% | PoW reward (incentivizes security) |
| **Vault** | 10% | Reserve fund (future liquidity) |
| **Community Fund** | 10% | Grants, partnerships, marketing |
| **Treasury** | 5% | DAO-governed spending |
| **Founder 1** | 2.5% | Early development compensation |
| **Founder 2** | 2.5% | Early development compensation |

**Example** (Epoch 1, 10,000 CASH per block):

```
Miner:          7,000 CASH
Vault:          1,000 CASH
Community Fund: 1,000 CASH
Treasury:         500 CASH
Founder 1:        250 CASH
Founder 2:        250 CASH
----------------------------
TOTAL:         10,000 CASH
```

### Configuration

Emission percentages are hardcoded in `config/token_accounts.toml`:

```toml
# Token Account Configuration (IMMUTABLE after genesis)
vault_address = "0xVaultAddress..."
fund_address = "0xFundAddress..."
founder1_address = "0xFounder1..."
founder2_address = "0xFounder2..."

vault_pct = 10
fund_pct = 10
treasury_pct = 5
founder1_pct = 2.5
founder2_pct = 2.5
# Miner gets remainder (70%)
```

**⚠️ Important**: These percentages are **immutable after genesis**. The removed `/admin/token-accounts/set` endpoint prevents tampering.

---

## 🔥 Deflationary Mechanics

### Fee Burning

A portion of transaction fees is **burned** (destroyed forever):

```rust
pub fn process_transaction_fees(tx: &Tx, state: &mut State) {
    let total_fee = tx.base_fee + tx.priority_fee;
    
    // 50% to miner (incentive)
    let miner_fee = total_fee / 2;
    state.add_balance(&tx.miner, miner_fee);
    
    // 50% burned (deflationary)
    let burn_amount = total_fee - miner_fee;
    state.total_supply -= burn_amount;
    
    tracing::info!("🔥 Burned {} CASH from fees", burn_amount);
}
```

### Supply Reduction Formula

```
Net Supply Change per Block = Emission - Fee Burns

If Fee Burns > Emission: Supply decreases (deflationary)
If Fee Burns < Emission: Supply increases (inflationary)
```

As network usage grows, fee burning accelerates, potentially leading to **net deflation** despite ongoing emission.

---

## 💰 CASH Use Cases

### 1. **Transaction Fees** (Base Layer)

Every on-chain operation requires CASH:

| Operation | Typical Fee |
|-----------|-------------|
| Simple Transfer | 10-100 CASH |
| Smart Contract Call | 100-5,000 CASH |
| LAND Transfer | 100-500 CASH |
| Property Upgrade | 50,000 CASH |
| DAO Vote | 1,000 CASH |

**EIP-1559-Style Fee Market**:

```rust
// Fee = base_fee + priority_fee
// base_fee adjusts dynamically based on congestion
// priority_fee goes to miner
```

### 2. **In-Game Purchases**

CASH is the currency for GTA V integration:

- **Weapons & Ammo**: 50-5,000 CASH
- **Vehicles**: 10,000-500,000 CASH
- **Property Upgrades**: 50,000-1,000,000 CASH
- **Cosmetics**: 1,000-50,000 CASH

**Blockchain-Game Bridge**:

```rust
// Example: Buy weapon in-game
pub fn on_cash_mint(
    amount: u128,
    recipient: &str,
    block_height: u64
) -> anyhow::Result<()> {
    // Notify GTA V mod of CASH balance change
    game_server::update_player_balance(recipient, amount)?;
    
    tracing::info!("🎮 Player {} received {} CASH in-game", recipient, amount);
    Ok(())
}
```

### 3. **Staking & Yield** (Planned)

Future staking mechanisms:

- **Validator Staking**: Lock CASH to run consensus nodes
- **Liquidity Mining**: Provide CASH-ETH liquidity for rewards
- **LAND Staking**: Boost property income with CASH collateral

### 4. **Governance Participation**

CASH holders can vote on:

- **Protocol Upgrades**: Consensus rule changes
- **Treasury Spending**: Community fund allocations
- **Game Features**: In-game economy adjustments

**Voting Power** = `sqrt(CASH_balance) * time_locked`

---

## 🔗 CASH Transfers

### Wallet-to-Wallet Transfer

```json
POST /wallet/transfer
{
  "from": "0xAlice...",
  "to": "0xBob...",
  "amount": "5000",
  "fee": "50",
  "memo": "For that car you sold me",
  "nonce": 42,
  "public_key": "0x...",
  "signature": "0x..."
}
```

### Smart Contract Interaction

```json
POST /submit_tx
{
  "nonce": 42,
  "sender_pubkey": "0xAlice...",
  "module": "cash",
  "method": "transfer",
  "args": {
    "to": "0xBob...",
    "amount": "5000"
  },
  "fee_limit": 1000,
  "max_fee_per_gas": 100,
  "max_priority_fee_per_gas": 10,
  "tip": 50,
  "sig": "0x..."
}
```

---

## 📈 CASH Price Discovery

### Market Dynamics

CASH value is determined by:

1. **Mining Cost**: Electricity + hardware depreciation
2. **Utility Demand**: Game adoption drives transaction volume
3. **Speculative Premium**: Expectations of future growth
4. **Supply Scarcity**: Halvings reduce new supply
5. **Fee Burning**: Net deflation increases scarcity

### Price Catalysts

**Bullish**:
- ✅ Halving events (reduced supply)
- ✅ Game launch / viral adoption
- ✅ CEX listings (increased liquidity)
- ✅ High fee burning (deflation)

**Bearish**:
- ⚠️ Network exploits / bugs
- ⚠️ Competing games / blockchains
- ⚠️ Regulatory crackdowns
- ⚠️ Low transaction volume (weak fees)

---

## 🛡️ Security & Anti-Inflation

### Supply Audit

Query current supply via API:

```bash
curl http://localhost:7070/status

{
  "height": 250000,
  "total_supply": "2500000000000", // 2.5T CASH
  "emission_per_block": "10000",
  "next_halving_height": 1000000,
  "blocks_until_halving": 750000
}
```

### Emission Verification

Anyone can verify emission schedule:

```rust
// Audit total minted CASH
let mut expected_supply = 0u128;
for height in 0..current_height {
    expected_supply += emission_at_height(height);
}

assert_eq!(actual_supply, expected_supply, "Supply mismatch!");
```

### Founder Vesting (Transparency)

Founder allocations are **not vested** - they receive 2.5% per block immediately. This is transparent and consensus-enforced:

```rust
// Founder balances are public on-chain
// Anyone can query:
let founder1_balance = get_balance("0xFounder1...");
let founder1_expected = (current_height * 10_000 * 2.5%) / 100;

// Compare actual vs expected to detect any tampering
```

---

## 💎 CASH Scarcity Timeline

### Emission Milestones

| Milestone | Height | Supply Minted | % of Max Supply |
|-----------|--------|---------------|-----------------|
| Genesis | 0 | 0 CASH | 0% |
| First Halving | 1M | 10B CASH | 50% |
| Second Halving | 2M | 15B CASH | 75% |
| Third Halving | 3M | 17.5B CASH | 87.5% |
| Fourth Halving | 4M | 18.75B CASH | 93.75% |
| 99% Mined | ~14M | 19.8B CASH | 99% |
| Tail Emission | 21M+ | ~20B CASH | Perpetual 1.5%/yr inflation |

**Key Insight**: 75% of all CASH will be minted in the first **2 million blocks** (~46 days at 2s block time).

---

## 🔍 Querying CASH Balances

### REST API

```bash
# Get CASH balance
curl http://localhost:7070/balance/0xYourAddress

# Get supply metrics
curl http://localhost:7070/metrics | grep cash_total_supply
```

### PowerShell Example

```powershell
# Check your CASH balance
$address = "0xYourAddress"
$balance = (Invoke-RestMethod "http://localhost:7070/balance/$address").balance
Write-Host "Your CASH balance: $balance"

# Calculate USD value (assuming $0.01 per CASH)
$usd_value = [decimal]$balance * 0.01
Write-Host "Estimated value: $$usd_value USD"
```

---

## 🚀 Future Roadmap

### Phase 1: Mainnet Launch (Current)

- [x] Block emission system
- [x] Halving schedule
- [x] Fee market (EIP-1559)
- [x] Fee burning

### Phase 2: DeFi Integration (Q2 2024)

- [ ] CASH-ETH liquidity pools (Uniswap)
- [ ] Cross-chain bridges (BSC, Polygon)
- [ ] Lending protocols (Aave-style)
- [ ] Yield aggregators

### Phase 3: Advanced Features (Q3 2024)

- [ ] CASH staking for validator nodes
- [ ] Dynamic supply cap (DAO-voted)
- [ ] Algorithmic stability mechanisms
- [ ] Real-time supply dashboard

### Phase 4: Mass Adoption (Q4 2024+)

- [ ] CEX listings (Binance, Coinbase)
- [ ] Fiat on/off ramps
- [ ] Mobile wallet (iOS/Android)
- [ ] In-game CASH ATMs (GTA V)

---

## 📚 Technical Reference

### State Trees

```
balances/
  acct:<address> -> u128 (CASH balance)

emission/
  meta:total_supply -> u128 (global supply)
  meta:burned_fees -> u128 (cumulative burns)
  meta:miner_rewards -> u128 (cumulative miner earnings)

vault/
  vault_balance -> u128
  fund_balance -> u128
  treasury_balance -> u128
  founder1_balance -> u128
  founder2_balance -> u128
```

### Transaction Structure

```rust
pub struct Tx {
    pub nonce: u64,
    pub sender_pubkey: String,
    pub module: String,       // "cash"
    pub method: String,        // "transfer"
    pub args: Vec<u8>,         // JSON serialized
    pub fee_limit: u128,       // Max fee willing to pay
    pub max_fee_per_gas: u128, // Base + priority fee cap
    pub max_priority_fee_per_gas: u128, // Tip to miner
    pub tip: u128,             // Additional miner tip
    pub sig: String,           // Ed25519 signature
}
```

---

## 🤝 Support & Community

- **Discord**: [discord.gg/vision-blockchain](https://discord.gg/vision-blockchain)
- **Telegram**: [@VisionCashHolders](https://t.me/VisionCashHolders)
- **GitHub**: [github.com/vision-node/cash-system](https://github.com/vision-node/cash-system)
- **Docs**: [docs.vision-blockchain.io/cash](https://docs.vision-blockchain.io/cash)

---

## 📝 Related Documentation

- [TOKENOMICS.md](TOKENOMICS.md) - Complete emission model
- [LAND_DEEDS.md](LAND_DEEDS.md) - LAND token system
- [GOVERNANCE_OVERVIEW.md](GOVERNANCE_OVERVIEW.md) - DAO voting
- [GENESIS.md](GENESIS.md) - Genesis block details

---

**Vision CASH: Power the game, secure the chain, earn rewards.**
