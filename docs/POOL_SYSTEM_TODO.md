# 🔧 Vision Mining Pool System Implementation

## Goal
Turn the Vision Node into a flexible miner that supports:
- Solo mining
- Hosting a pool (HostPool)
- Joining someone else's pool (JoinPool)

All logic lives inside the existing node + miner manager + React UI.

## Implementation Status

### Phase 1: Core Module Structure ✅
- [x] Create pool module directory
- [x] Define core types and enums (MiningMode, PoolConfig, PoolState)
- [x] Setup state management with Arc<Mutex<>>
- [x] Define protocol types (PoolJob, ShareSubmission, etc.)

### Phase 2: Pool Routes & Endpoints ✅
- [x] POST /pool/register - Worker registration
- [x] GET /pool/job - Fetch mining jobs
- [x] POST /pool/share - Submit shares
- [x] GET /pool/stats - Pool statistics
- [x] POST /pool/configure - Configure pool settings
- [x] POST /pool/start - Start hosting pool
- [x] POST /pool/stop - Stop hosting pool
- [x] GET /pool/mode - Get current mode
- [x] POST /pool/mode - Set mining mode

### Phase 3: Payout Logic ✅
- [x] Compute pool payouts with foundation fee (1%)
- [x] Proportional share distribution
- [x] Handle rounding dust
- [x] Distribute rewards via direct balance updates (TODO: migrate to transactions)
- [x] Reset shares after block found
- [x] Unit tests for payout calculations

### Phase 4: Mining Integration ⚠️ PARTIAL
- [x] Pool state management in routes
- [x] Share validation and tracking
- [x] Block found detection
- [x] Automatic payout distribution
- [ ] Worker mining loop for JoinPool mode
- [ ] Integration with existing miner threads
- [ ] Job generation from block templates

### Phase 5: UI Components ✅
- [x] Mode toggle (Solo/Host/Join)
- [x] Host pool configuration interface
- [x] Join pool interface
- [x] Worker statistics display
- [x] Pool URL generation and sharing
- [x] Foundation fee notice
- [x] Real-time hashrate display

### Phase 6: Testing ⚠️ IN PROGRESS
- [x] Unit tests for payouts in payouts.rs
- [ ] Integration tests for full pool flow
- [ ] Manual testing with multiple nodes
- [ ] Performance testing under load
- [ ] Stress testing with many workers

## Architecture Notes

### Three Mining Modes
1. **Solo**: Current behavior, unchanged
2. **HostPool**: Node becomes pool host, serves workers via HTTP
3. **JoinPool**: Node mines as worker for remote pool

### Foundation Fee
- 1.00% (100 bps) of every block reward goes to Vision Foundation
- Mandatory for ecosystem sustainability
- Deducted before pool fee and worker payouts

### Share-Based Rewards
- Workers submit shares (easier difficulty than network target)
- Shares track contribution
- On block found, rewards distributed proportionally
- Shares reset for next block

### Transaction-Based Payouts
- Use existing transaction system for auditability
- Multi-output transactions for single-block payouts
- Foundation + Pool Fee + Workers in one transaction

---

## 🎉 Current Implementation Summary

### ✅ Completed Features

#### Core Pool System
- **Mining Modes**: Solo, HostPool, JoinPool fully defined
- **State Management**: Thread-safe with Arc<Mutex<>> wrappers
- **Worker Tracking**: Registration, shares, hashrate, timeout detection
- **Configuration**: Pool fees, foundation fees, ports, naming

#### HTTP API Endpoints (9 total)
All pool endpoints are live and wired into main.rs router:

1. `POST /pool/register` - Workers register with host
2. `GET /pool/job` - Workers fetch mining jobs
3. `POST /pool/share` - Submit shares or block solutions
4. `GET /pool/stats` - Pool statistics and worker list
5. `POST /pool/configure` - Update pool settings
6. `POST /pool/start` - Start hosting a pool
7. `POST /pool/stop` - Stop hosting
8. `GET /pool/mode` - Query current mining mode
9. `POST /pool/mode` - Change mining mode

#### Payout Logic
- **Foundation Fee**: Mandatory 1% (100 bps) to Vision Foundation
- **Pool Fee**: Configurable 0-10%, default 1.5%
- **Proportional Distribution**: Based on shares contributed
- **Dust Handling**: Rounding errors go to foundation
- **Unit Tests**: Verified payout calculations

#### UI Integration (panel.html)
- **Mode Selector**: 3-button toggle (Solo/Host/Join)
- **Host Configuration**: Pool name, port (7072/8082), fee settings
- **Join Interface**: Pool URL input, worker credentials
- **Real-time Stats**: Worker count, hashrate, shares
- **Foundation Notice**: Prominent fee disclosure
- **Pool URL Sharing**: Auto-generated URL with copy button

### ⚠️ Remaining Work

#### 1. Worker Mining Loop (JoinPool Mode)
**What's Missing**: When a node is in JoinPool mode, it needs to:
- Call `/pool/register` on startup
- Periodically fetch `/pool/job`
- Run local hashing with provided job parameters
- Submit shares via `/pool/share` when found

**Implementation Location**: Extend existing miner threads in `src/main.rs` miner logic

**Pseudocode**:
```rust
// In miner thread loop
if mining_mode == JoinPool {
    let job = fetch_pool_job(pool_url, worker_id);
    let result = mine_with_job(job.target, job.nonce_range);
    if result.valid_share {
        submit_share(pool_url, worker_id, result);
    }
}
```

#### 2. Transaction-Based Payouts
**Current**: Direct balance updates via sled DB
**TODO**: Replace with proper multi-output transactions for auditability

**Benefits**:
- Full blockchain audit trail
- Verifiable on-chain
- Consistent with Vision's transaction model

**Implementation**:
- Create multi-transfer function
- Build transaction with multiple outputs
- Submit through normal tx pipeline

#### 3. Integration Testing
- **Two-Node Test**: One host, one worker
- **Share Submission**: Verify shares increment correctly
- **Block Found**: Confirm payouts execute
- **Fee Distribution**: Verify foundation + pool + workers get correct amounts

### 📊 Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Vision Node (Any Mode)                    │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Solo Mode   │  │  HostPool    │  │  JoinPool    │      │
│  │              │  │              │  │              │      │
│  │ • Mine solo  │  │ • Serve jobs │  │ • Fetch jobs │      │
│  │ • Full block │  │ • Track      │  │ • Submit     │      │
│  │   rewards    │  │   workers    │  │   shares     │      │
│  │              │  │ • Distribute │  │ • Receive    │      │
│  │              │  │   payouts    │  │   payouts    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                            │                                  │
│                            │ Block Found                     │
│                            ▼                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │          Payout Distribution (payouts.rs)             │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ 1. Foundation Fee: 1.00% → Foundation Vault           │  │
│  │ 2. Pool Fee: 1.50% → Pool Host                        │  │
│  │ 3. Workers: 97.50% → Proportional by shares           │  │
│  │    • Worker A: 60 shares → 60% of 97.5%               │  │
│  │    • Worker B: 40 shares → 40% of 97.5%               │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### 🔧 Quick Start Guide

#### Host a Pool
1. Open panel.html in Vision Node
2. Select "🏊 Host Pool" mode
3. Configure: pool name, port, fee
4. Click "Start Pool"
5. Share pool URL with miners

#### Join a Pool
1. Open panel.html in Vision Node
2. Select "🤝 Join Pool" mode
3. Enter host's pool URL
4. Set worker name
5. Click "Connect to Pool"
6. Start mining threads

#### Test Locally
```powershell
# Terminal 1: Start host node
.\START-VISION-NODE.bat

# Terminal 2: Start worker node (different data dir)
$env:VISION_DATA_DIR="vision_data_worker1"
cargo run --release

# Use panel.html to configure each node
```

### 📝 Code Quality Notes

#### ✅ Good Patterns
- Clean separation: state.rs, worker.rs, protocol.rs, payouts.rs, routes.rs
- Thread-safe state with Arc<Mutex<>>
- Type-safe protocol messages with serde
- Unit tests for core logic
- Comprehensive error handling

#### ⚠️ Technical Debt
- **Direct Balance Updates**: Should use transactions
- **Global Statics**: POOL_STATE, MINING_MODE could be in AppState
- **Job Generation**: Currently simplified, needs full merkle root
- **Worker Mining Loop**: Not yet implemented for JoinPool

#### 🎯 Next Priority
1. Implement JoinPool worker mining loop
2. Add integration tests
3. Replace balance updates with transactions
4. Performance testing with 10+ workers
