# Vision LAND System: Digital Property on the Blockchain

**Status**: Active (Mainnet-Ready)  
**Token Type**: LAND  
**Total Supply**: Fixed at genesis (based on `airdrop.csv`)  
**Standard**: Custom (on-chain state, consensus-validated)  

---

## 🏗️ Overview

The LAND system represents **digital property rights** in the Vision blockchain ecosystem. Each LAND token corresponds to a **plot of virtual real estate** in the GTA V-integrated game world, enabling players to own, trade, and develop in-game properties with blockchain-backed ownership.

### Core Principles

1. **Scarcity**: Fixed supply minted only at genesis
2. **Ownership**: Transferable via blockchain transactions
3. **Utility**: Grants gameplay rights (building, income, voting)
4. **Immutability**: Ownership records stored on-chain, not in game state

---

## 📊 Token Economics

### Supply Model

| Metric | Value |
|--------|-------|
| **Total LAND Supply** | Fixed at genesis (see `airdrop.csv`) |
| **Mint Mechanism** | One-time genesis allocation only |
| **Destruction** | None (LAND is never burned) |
| **Divisibility** | Whole units only (no fractions) |

### Genesis Distribution

LAND tokens were distributed at **block 0 (genesis)** to early supporters, testers, and contributors:

```rust
// From genesis_state() in main.rs
pub fn genesis_state() -> Chain {
    // ... initialize chain ...
    
    // Distribute genesis land deeds from airdrop.csv
    if let Ok(csv_data) = std::fs::read_to_string("airdrop.csv") {
        for line in csv_data.lines().skip(1) { // Skip header
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let address = parts[0].trim();
                let amount = parts[1].trim().parse::<u128>().unwrap_or(10_000);
                
                land_balances.insert(
                    format!("acct:{}:land", address),
                    amount
                );
                
                tracing::info!("Genesis land deed: {} -> {} LAND", address, amount);
            }
        }
    }
    
    // ...
}
```

**No future LAND minting** - This was a one-time distribution.

---

## 🎮 LAND Usage in Gameplay

### 1. **Property Ownership**

Each LAND token represents ownership of a specific property in Los Santos:

- **Residential**: Houses, apartments, condos
- **Commercial**: Stores, warehouses, nightclubs
- **Industrial**: Factories, garages, drug labs
- **Special**: Gang safehouses, government buildings

### 2. **Income Generation**

LAND owners earn **passive income** through:

- **Rent Collection**: NPCs or players pay rent to use properties
- **Business Profits**: Operating stores, nightclubs, or services
- **Mission Hubs**: Completing missions from your property yields bonuses

```rust
// Example game hook integration (from game_hooks.rs)
pub fn on_land_use(
    player: &str,
    plot_id: u64,
    action: &str
) -> anyhow::Result<()> {
    // Called when player interacts with owned property
    tracing::info!(
        "Land use: player={}, plot={}, action={}",
        player, plot_id, action
    );
    
    // Future: Trigger rent collection, access control, etc.
    Ok(())
}
```

### 3. **Building & Development**

LAND owners can **upgrade properties** using CASH:

- **Level 1 (Basic)**: Standard property, minimal income
- **Level 2 (Improved)**: Enhanced income, new features
- **Level 3 (Maxed)**: Maximum income, exclusive access

Upgrades persist on-chain and are visible in-game.

### 4. **Governance Participation**

LAND tokens grant **voting power** in:

- **Map Expansion**: Vote on new areas to unlock
- **Game Rules**: Decide on PvP zones, economy adjustments
- **Treasury Allocation**: Direct community funds

(Governance system in development - see `docs/GOVERNANCE_OVERVIEW.md`)

---

## 🔗 On-Chain Integration

### Smart Contract Modules

LAND functionality is implemented in the `land` module:

```rust
// Module: land
// Methods:
//   - transfer(to: String, amount: u128) -> Result<(), String>
//   - get_balance(address: String) -> u128
//   - get_property_details(plot_id: u64) -> PropertyInfo
//   - upgrade_property(plot_id: u64, level: u8) -> Result<(), String>
```

### Access Control

The blockchain enforces ownership:

```rust
// Example: Only LAND owner can upgrade property
fn upgrade_property(state: &mut State, tx: &Tx, args: UpgradeArgs) -> TxResult {
    let owner_key = format!("acct:{}:land", tx.sender_pubkey);
    let balance = state.land_balances.get(&owner_key).copied().unwrap_or(0);
    
    if balance == 0 {
        return TxResult::Err("No LAND ownership".into());
    }
    
    // Verify plot ownership via property_owners tree
    let plot_owner_key = format!("plot:{}:owner", args.plot_id);
    let current_owner = state.property_owners.get(&plot_owner_key);
    
    if current_owner != Some(&tx.sender_pubkey) {
        return TxResult::Err("Not property owner".into());
    }
    
    // Charge CASH for upgrade
    let upgrade_cost = 50_000u128 * (args.level as u128);
    // ... deduct CASH, upgrade property ...
    
    TxResult::Ok
}
```

---

## 💰 LAND Trading

### On-Chain Market

LAND can be traded via:

1. **Peer-to-Peer**: Direct wallet-to-wallet transfers
2. **DEX Integration**: Automated market makers (future)
3. **NFT Marketplaces**: Cross-chain bridging (planned)

### Example Trade Transaction

```json
{
  "nonce": 42,
  "sender_pubkey": "0xAlice...",
  "module": "land",
  "method": "transfer",
  "args": {
    "to": "0xBob...",
    "amount": "1",
    "plot_id": 12345
  },
  "fee_limit": 1000,
  "tip": 500,
  "sig": "0x..."
}
```

**Important**: Always verify plot ownership before trading!

---

## 🔍 Querying LAND Balances

### REST API

```bash
# Get LAND balance for an address
curl http://localhost:7070/balance/land/0xYourAddress

# Response:
{
  "address": "0xYourAddress",
  "token": "land",
  "balance": "5"
}
```

### CLI Query

```powershell
# Using curl (Windows PowerShell)
$address = "0xYourAddress"
$response = Invoke-RestMethod -Uri "http://localhost:7070/balance/land/$address"
Write-Host "LAND Balance: $($response.balance)"
```

### GraphQL (Future)

```graphql
query GetLandHoldings($address: String!) {
  account(address: $address) {
    land {
      balance
      properties {
        plot_id
        location
        level
        income_per_block
      }
    }
  }
}
```

---

## 🏆 LAND Leaderboard

### Top Holders (Example)

| Rank | Address | LAND | Properties Owned |
|------|---------|------|------------------|
| 1 | 0xWhale... | 250 | 250 |
| 2 | 0xHolder... | 180 | 180 |
| 3 | 0xInvestor... | 120 | 120 |
| ... | ... | ... | ... |

Query leaderboard via:

```bash
curl http://localhost:7070/land/leaderboard?limit=100
```

---

## 🛡️ Security & Anti-Fraud

### Ownership Verification

Before accepting LAND transfers:

1. **Check Balance**: Verify sender has sufficient LAND
2. **Verify Plot Mapping**: Confirm plot_id maps to LAND token
3. **Validate Signature**: Ensure transaction is signed by owner
4. **Check Nonce**: Prevent replay attacks

### Scam Prevention

**Common Scams**:

- **Fake Properties**: Always verify `plot_id` exists on-chain
- **Double-Selling**: Check blockchain state, not game state
- **Phishing**: Never share private keys, even with "support"

**Protection**:

```rust
// All LAND transfers are atomic and consensus-validated
// Once a block is finalized, ownership is immutable
```

---

## 📈 LAND Value Proposition

### Why Hold LAND?

1. **Passive Income**: Earn CASH from rent & businesses
2. **Governance Power**: Vote on game & economy changes
3. **Status Symbol**: Flex your virtual real estate portfolio
4. **Scarcity**: Fixed supply ensures long-term value
5. **Interoperability**: Future cross-game integrations

### Growth Catalysts

- **Game Adoption**: More players = higher demand
- **Feature Expansion**: New property types & gameplay
- **Cross-Chain Bridges**: Trade on ETH/BSC/SOL DEXes
- **Metaverse Integration**: Virtual world interoperability

---

## 🚀 Future Roadmap

### Phase 1: Mainnet Launch (Current)

- [x] Genesis LAND distribution
- [x] On-chain balance tracking
- [x] Basic transfer functionality

### Phase 2: Gameplay Integration (Q2 2024)

- [ ] In-game property visualization
- [ ] Rent collection automation
- [ ] Property upgrade system
- [ ] Income dashboards

### Phase 3: Advanced Features (Q3 2024)

- [ ] LAND staking for yield
- [ ] Fractional ownership (LAND shards)
- [ ] Cross-game property portals
- [ ] DAO-governed expansions

### Phase 4: Metaverse (Q4 2024+)

- [ ] VR property tours
- [ ] Cross-chain LAND bridges
- [ ] Real-world asset tokenization
- [ ] Global property marketplace

---

## 📚 Technical Reference

### State Trees

```
balances/
  acct:<address>:land -> u128 (LAND balance)

property_owners/
  plot:<plot_id>:owner -> String (owner address)
  plot:<plot_id>:level -> u8 (upgrade level)
  plot:<plot_id>:income -> u128 (CASH per block)

land_metadata/
  plot:<plot_id>:location -> String (GPS coords)
  plot:<plot_id>:type -> String (residential/commercial/etc.)
```

### Transaction Fees

| Operation | Base Fee | Gas Units |
|-----------|----------|-----------|
| LAND Transfer | 100 CASH | 10,000 |
| Property Upgrade | 50,000 CASH | 25,000 |
| Metadata Update | 500 CASH | 5,000 |

---

## 🤝 Support & Community

- **Discord**: [discord.gg/vision-blockchain](https://discord.gg/vision-blockchain)
- **Telegram**: [@VisionLandHolders](https://t.me/VisionLandHolders)
- **GitHub**: [github.com/vision-node/land-system](https://github.com/vision-node/land-system)
- **Docs**: [docs.vision-blockchain.io/land](https://docs.vision-blockchain.io/land)

---

## 📝 Related Documentation

- [TOKENOMICS.md](TOKENOMICS.md) - CASH token economics
- [CASH_SYSTEM.md](CASH_SYSTEM.md) - In-depth CASH guide
- [GOVERNANCE_OVERVIEW.md](GOVERNANCE_OVERVIEW.md) - Voting & proposals
- [GENESIS.md](GENESIS.md) - Genesis block details

---

**Vision LAND: Own the virtual world, backed by blockchain.**
