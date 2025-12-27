# Vision Blockchain Governance: Community-Driven Decision Making

**Status**: Phase 1 (Foundation) - Governance framework under development  
**Governance Model**: Progressive decentralization (DAO evolution)  
**Voting Token**: CASH + LAND (weighted)  
**Treasury**: On-chain (5% of block rewards)  

---

## 🏛️ Overview

Vision blockchain governance enables **token holders to control the protocol's future** through:

1. **Protocol Upgrades**: Consensus rule changes (block size, fees, etc.)
2. **Treasury Management**: Spending community funds (~500M CASH/year)
3. **Economic Parameters**: Emission schedules, fee structures
4. **Game Integration**: In-game features, economy adjustments
5. **Emergency Actions**: Security responses, bug fixes

### Core Principles

- **Transparency**: All proposals public on-chain
- **Meritocracy**: Voting power tied to stake + participation
- **Security**: Multi-sig + timelocks prevent hasty changes
- **Inclusivity**: Low barriers to proposal submission

---

## 🗳️ Governance Architecture

### Phase 1: Foundation (Current)

**Status**: Centralized development, community feedback loops

- **Decision Makers**: Core team + founding contributors
- **Community Input**: Discord polls, GitHub discussions
- **Treasury**: Accumulating (not yet spent)
- **Upgrades**: Manual deployment with community notice

**Rationale**: Allows rapid iteration during early growth phase.

### Phase 2: Transition (Target: Q2 2024)

**Status**: Gradual power handoff to token holders

- **On-Chain Voting**: Snapshot-style governance (off-chain signaling)
- **Advisory Votes**: Non-binding proposals guide core team
- **Treasury Grants**: Community votes on funding requests
- **Multi-Sig**: 3-of-5 multisig for treasury withdrawals

### Phase 3: Full DAO (Target: Q4 2024)

**Status**: Complete decentralization

- **Binding Votes**: On-chain proposals execute automatically
- **No Core Team Veto**: Community has final say
- **Delegated Voting**: Token holders can delegate votes
- **Governor Contracts**: Timelock + execution framework

---

## 📊 Voting Power Model

### Hybrid Token-Weighted System

Voting power is calculated from **both CASH and LAND holdings**:

```rust
pub fn calculate_voting_power(address: &str) -> u128 {
    let cash_balance = get_balance(address, "cash");
    let land_balance = get_balance(address, "land");
    
    // Formula: sqrt(CASH) + (LAND * 10,000)
    // This balances large CASH holders with land property owners
    
    let cash_weight = (cash_balance as f64).sqrt() as u128;
    let land_weight = land_balance * 10_000;
    
    cash_weight + land_weight
}
```

**Example**:

| Holder | CASH | LAND | Voting Power |
|--------|------|------|--------------|
| Alice | 1M | 10 | 1,000 + 100,000 = **101,000** |
| Bob | 10M | 0 | 3,162 + 0 = **3,162** |
| Carol | 100K | 50 | 316 + 500,000 = **500,316** |

**Rationale**:
- `sqrt(CASH)` prevents whale dominance (quadratic voting)
- `LAND * 10,000` rewards long-term property owners
- Combined model balances speculative traders vs. ecosystem builders

### Time-Lock Bonuses

Voters who **lock tokens** for governance gain multipliers:

| Lock Duration | Multiplier | Unlock Time |
|---------------|------------|-------------|
| None | 1x | Immediate |
| 3 months | 1.5x | ~3.8M blocks |
| 6 months | 2x | ~7.8M blocks |
| 12 months | 3x | ~15.8M blocks |
| 24 months | 5x | ~31.5M blocks |

**Implementation** (Future):

```rust
pub fn locked_voting_power(address: &str, lock_duration_blocks: u64) -> u128 {
    let base_power = calculate_voting_power(address);
    
    let multiplier = match lock_duration_blocks {
        0..=3_800_000 => 1.0,
        3_800_001..=7_800_000 => 1.5,
        7_800_001..=15_800_000 => 2.0,
        15_800_001..=31_500_000 => 3.0,
        _ => 5.0,
    };
    
    (base_power as f64 * multiplier) as u128
}
```

---

## 📝 Proposal Lifecycle

### 1. **Discussion Phase** (Off-Chain)

**Duration**: 7 days  
**Platform**: Discord #governance, GitHub Discussions  

**Requirements**:
- Clear problem statement
- Proposed solution with rationale
- Cost-benefit analysis (if treasury spend)
- Community feedback integration

**Template**:

```markdown
## [GIP-XXX] Proposal Title

**Author**: @username  
**Date**: YYYY-MM-DD  
**Category**: Protocol Upgrade | Treasury | Economic | Game  

### Summary
One-paragraph overview of the proposal.

### Motivation
Why is this change needed? What problem does it solve?

### Specification
Technical details of the proposed change.

### Cost
CASH required from treasury (if applicable).

### Risks
Potential downsides or attack vectors.

### Timeline
Implementation schedule.
```

### 2. **Proposal Submission** (On-Chain)

**Duration**: Instant  
**Cost**: 10,000 CASH (refunded if >10% quorum)  

**Smart Contract Call**:

```json
POST /submit_tx
{
  "module": "governance",
  "method": "submit_proposal",
  "args": {
    "title": "Increase block size to 2MB",
    "description_url": "https://github.com/vision-node/GIPs/blob/main/GIP-042.md",
    "proposal_type": "protocol_upgrade",
    "execution_data": "0x...", // Bytecode to execute if passed
    "voting_duration_blocks": 86400 // ~2 days
  },
  "fee_limit": 50000
}
```

### 3. **Voting Period** (On-Chain)

**Duration**: 2-7 days (configurable)  
**Quorum**: 10% of circulating supply  
**Threshold**: 51% approval (simple majority)  

**Vote Options**:
- ✅ **For**: Support the proposal
- ❌ **Against**: Oppose the proposal
- ⏸️ **Abstain**: Participate in quorum without taking a side

**Voting Transaction**:

```json
POST /submit_tx
{
  "module": "governance",
  "method": "cast_vote",
  "args": {
    "proposal_id": 42,
    "vote": "for", // "for" | "against" | "abstain"
    "reason": "This change improves scalability without security tradeoffs"
  }
}
```

### 4. **Execution** (Timelock)

**Delay**: 48 hours (emergency proposals: 6 hours)  
**Guardian**: Multi-sig can veto during timelock (Phase 2 only)  

**Automatic Execution**:

```rust
pub fn execute_proposal(proposal_id: u64, state: &mut State) -> Result<(), String> {
    let proposal = state.get_proposal(proposal_id)?;
    
    // Verify vote passed
    if proposal.for_votes <= proposal.against_votes {
        return Err("Proposal did not pass".into());
    }
    
    // Verify quorum met
    let total_votes = proposal.for_votes + proposal.against_votes + proposal.abstain_votes;
    if total_votes < state.quorum_threshold() {
        return Err("Quorum not met".into());
    }
    
    // Verify timelock elapsed
    if state.current_height < proposal.timelock_end_height {
        return Err("Timelock still active".into());
    }
    
    // Execute proposal (upgrade, transfer, parameter change)
    proposal.execute(state)?;
    
    tracing::info!("✅ Proposal {} executed successfully", proposal_id);
    Ok(())
}
```

---

## 💰 Treasury Management

### Treasury Funding

**Source**: 5% of every block reward flows to on-chain treasury

**Annual Accumulation** (Epoch 1):

```
Blocks per year = 15,768,000 (2s blocks)
Reward per block = 10,000 CASH
Treasury % = 5%

Annual Treasury Income = 15,768,000 * 10,000 * 0.05 = 7,884,000,000 CASH (~7.9B CASH/year)
```

### Spending Categories

| Category | Max % | Purpose | Example |
|----------|-------|---------|---------|
| **Development** | 40% | Protocol upgrades, audits | Security audit: 50M CASH |
| **Marketing** | 30% | Adoption campaigns, events | Conference sponsorship: 20M CASH |
| **Grants** | 20% | Community projects, tools | Block explorer: 10M CASH |
| **Operations** | 10% | Infrastructure, legal | CEX listing fees: 100M CASH |

### Spending Approval

**Thresholds**:

| Amount | Approval Process |
|--------|------------------|
| < 10M CASH | Treasury Committee (3-of-5 multisig) |
| 10M - 100M CASH | Community vote (7-day voting period) |
| > 100M CASH | Community vote + 14-day timelock |

**Transparency**:

All treasury transactions are **public on-chain**:

```bash
# Query treasury balance
curl http://localhost:7070/balance/treasury

# List recent treasury spends
curl http://localhost:7070/governance/treasury/history?limit=100
```

---

## 🛡️ Security Mechanisms

### 1. **Timelock** (Delay Execution)

Prevents instant malicious changes:

```rust
pub const STANDARD_TIMELOCK: u64 = 86_400; // 2 days
pub const EMERGENCY_TIMELOCK: u64 = 10_800; // 6 hours
```

**Attack Scenario**:
- Attacker acquires 51% voting power
- Submits proposal to drain treasury
- **Mitigation**: 48-hour delay allows community to:
  - Detect malicious proposal
  - Coordinate counter-proposal
  - Trigger emergency guardian veto

### 2. **Guardian Multi-Sig** (Phase 2)

Temporary safety net during transition:

**Guardians** (3-of-5 required to veto):
- 2x Core developers
- 2x Community-elected members
- 1x Security auditor

**Powers**:
- Veto proposals during timelock
- Cannot submit proposals themselves
- Automatically removed in Phase 3

### 3. **Quorum Requirements**

Prevents low-turnout attacks:

```rust
pub fn quorum_threshold(state: &State) -> u128 {
    let total_supply = state.get_total_voting_power();
    total_supply / 10 // 10% of circulating supply
}
```

**Example**:
- Total voting power: 10B
- Quorum: 1B votes required
- Proposal with 500M for, 200M against: **FAILS** (700M < 1B)

### 4. **Proposal Deposit**

Discourages spam proposals:

- **Deposit**: 10,000 CASH per proposal
- **Refund**: If quorum met (even if vote fails)
- **Burn**: If quorum not met (spam penalty)

---

## 📋 Governance Categories

### 1. **Protocol Upgrades** (High Risk)

**Examples**:
- Increase block gas limit
- Change consensus rules
- Modify emission schedule

**Requirements**:
- 60% approval threshold (higher than standard 51%)
- 7-day voting period
- Security audit required
- 14-day timelock

### 2. **Economic Parameters** (Medium Risk)

**Examples**:
- Adjust base fee algorithm
- Change LAND transfer fees
- Modify burn rate

**Requirements**:
- 51% approval threshold
- 5-day voting period
- Economic impact analysis
- 7-day timelock

### 3. **Treasury Spending** (Low Risk)

**Examples**:
- Fund marketing campaign
- Grant to developer
- CEX listing payment

**Requirements**:
- 51% approval threshold
- 3-day voting period
- Budget proposal document
- 48-hour timelock

### 4. **Game Features** (Low Risk)

**Examples**:
- Add new property types
- Adjust in-game prices
- Enable new gameplay modes

**Requirements**:
- 51% approval threshold
- 3-day voting period
- Community testing phase
- 48-hour timelock

### 5. **Emergency Actions** (Critical)

**Examples**:
- Pause chain due to exploit
- Emergency fund recovery
- Critical bug fix

**Requirements**:
- 75% approval threshold (super-majority)
- 1-day voting period
- Guardian override available
- 6-hour timelock (expedited)

---

## 🤝 Delegated Voting

### Why Delegate?

Not all token holders have time to vote on every proposal. Delegation allows:

- **Passive holders** to participate indirectly
- **Expert voters** to represent aligned stakeholders
- **Higher quorum** from engaged representatives

### How to Delegate

```json
POST /submit_tx
{
  "module": "governance",
  "method": "delegate_voting_power",
  "args": {
    "delegate_address": "0xExpertVoter...",
    "duration_blocks": 1000000 // ~23 days
  }
}
```

**Key Rules**:
- Delegated votes count toward delegate's voting power
- Original token holder retains token ownership (can transfer/sell)
- Delegation can be revoked anytime
- Self-delegation is default (vote with your own tokens)

### Top Delegates (Example)

| Delegate | Voting Power | Proposals Voted | Win Rate |
|----------|--------------|-----------------|----------|
| @CoreDev1 | 5M | 42 | 85% |
| @CommunityLead | 3M | 38 | 72% |
| @SecurityAuditor | 2M | 25 | 90% |

Track delegates at: `http://localhost:7070/governance/delegates`

---

## 📈 Governance Roadmap

### Phase 1: Foundation (Q1 2024) ✅

- [x] Treasury accumulation starts
- [x] Community feedback channels (Discord/GitHub)
- [x] Proposal template standardization
- [x] Voting power model design

### Phase 2: Transition (Q2-Q3 2024)

- [ ] On-chain proposal submission
- [ ] Snapshot-style voting (off-chain signaling)
- [ ] Guardian multi-sig deployment
- [ ] Treasury grant program launch

### Phase 3: Full DAO (Q4 2024)

- [ ] Binding on-chain governance
- [ ] Timelock contract deployment
- [ ] Delegated voting system
- [ ] Guardian removal (decentralization complete)

### Phase 4: Advanced Governance (2025+)

- [ ] Optimistic governance (execute first, challenge later)
- [ ] Futarchy (prediction market-based decisions)
- [ ] Cross-chain governance (Polygon, BSC)
- [ ] AI-assisted proposal analysis

---

## 📚 Governance Proposals Index

### Historical Proposals (Examples)

| GIP # | Title | Status | Votes | Result |
|-------|-------|--------|-------|--------|
| GIP-001 | Deploy governance system | Draft | - | Pending Phase 2 |
| GIP-002 | Fund block explorer development | Draft | - | Pending Phase 2 |
| GIP-003 | Adjust LAND transfer fees | Discussion | - | Needs feedback |

**View All Proposals**:
```bash
curl http://localhost:7070/governance/proposals
```

---

## 🔍 Querying Governance State

### REST API Examples

```bash
# Get all active proposals
curl http://localhost:7070/governance/proposals?status=active

# Get specific proposal details
curl http://localhost:7070/governance/proposals/42

# Check your voting power
curl http://localhost:7070/governance/voting-power/0xYourAddress

# View treasury balance
curl http://localhost:7070/balance/treasury
```

### PowerShell Examples

```powershell
# Check if you can submit a proposal (need 10,000 CASH)
$address = "0xYourAddress"
$balance = (Invoke-RestMethod "http://localhost:7070/balance/$address").balance
if ([decimal]$balance -ge 10000) {
    Write-Host "✅ You have enough CASH to submit a proposal" -ForegroundColor Green
} else {
    Write-Host "❌ Need $([10000 - [decimal]$balance]) more CASH to submit proposal" -ForegroundColor Red
}

# List all active votes
$proposals = Invoke-RestMethod "http://localhost:7070/governance/proposals?status=voting"
$proposals | Format-Table -Property id, title, for_votes, against_votes, end_height
```

---

## 🤝 Get Involved

### For Token Holders

1. **Join Discussions**: Discord #governance, GitHub Discussions
2. **Vote on Proposals**: Every vote counts (even 1 CASH)
3. **Delegate Wisely**: Choose representatives aligned with your values
4. **Monitor Treasury**: Ensure funds are spent responsibly

### For Developers

1. **Submit GIPs**: Propose protocol improvements
2. **Apply for Grants**: Build tools, integrations, analytics
3. **Audit Code**: Security bounties for bug discoveries
4. **Run Validators**: Stake CASH to secure the network (future)

### For Content Creators

1. **Educate**: Write tutorials, create videos
2. **Community Building**: Organize events, AMAs
3. **Marketing**: Promote Vision in your channels
4. **Translation**: Localize docs for global reach

---

## 📞 Support & Resources

- **Governance Forum**: [forum.vision-blockchain.io](https://forum.vision-blockchain.io)
- **Discord**: [discord.gg/vision-governance](https://discord.gg/vision-governance)
- **GitHub**: [github.com/vision-node/GIPs](https://github.com/vision-node/GIPs)
- **Snapshot**: [snapshot.org/#/vision.eth](https://snapshot.org/#/vision.eth) (Phase 2)

---

## 📝 Related Documentation

- [TOKENOMICS.md](TOKENOMICS.md) - Emission and supply mechanics
- [CASH_SYSTEM.md](CASH_SYSTEM.md) - CASH token details
- [LAND_DEEDS.md](LAND_DEEDS.md) - LAND token system
- [GENESIS.md](GENESIS.md) - Genesis block and launch

---

**Vision Governance: By the community, for the community.**
