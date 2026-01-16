# Vision Node - Complete Logging Reference
**Version:** v1.0.3  
**Date:** 2026-01-12  
**Status:** Production Ready

---

## 📋 TABLE OF CONTENTS

1. [Mining & Block Production](#mining--block-production)
2. [Block Acceptance & Rejection](#block-acceptance--rejection)
3. [Canonical vs Orphan Tracking](#canonical-vs-orphan-tracking)
4. [P2P Network Activity](#p2p-network-activity)
5. [Sync & Chain Management](#sync--chain-management)
6. [Peer Management & Strikes](#peer-management--strikes)
7. [Transaction Processing](#transaction-processing)
8. [Mining Safety Checks](#mining-safety-checks)
9. [System Health & Metrics](#system-health--metrics)
10. [Error & Warning Logs](#error--warning-logs)
11. [Debug & Diagnostic Logs](#debug--diagnostic-logs)
12. [Filtering & Search Guide](#filtering--search-guide)

---

## 🔨 MINING & BLOCK PRODUCTION

### [PAYOUT] Miner Reward Issued
**Level:** INFO  
**When:** Your node successfully mines a block and receives payment

```
[PAYOUT] Block mined - miner rewarded
  block=123
  miner=pow_miner
  reward=32000000000
  halvings=0
  new_balance=64000000000
```

**Fields:**
- `block`: Block height
- `miner`: Your wallet address
- `reward`: Amount received (32 LAND = 32,000,000,000 units)
- `halvings`: Number of halvings (0 = first epoch, 1 = second, etc.)
- `new_balance`: Updated balance after reward

**What it means:** You successfully mined a block and received 32 LAND!

---

### [PAYOUT] Tokenomics Applied
**Level:** INFO  
**When:** Block reward calculation completed

```
[PAYOUT] Tokenomics applied - miner received 32000000000 units (32000000000+fees)
  block=123
  miner=pow_miner
  miner_reward=32000000000
  fees_collected=50000000
  fees_distributed=5000000
  treasury_total=0
```

**Fields:**
- `miner_reward`: Total payout to miner (emission + fees)
- `fees_collected`: Transaction fees in block
- `fees_distributed`: Foundation's 10% share
- `treasury_total`: Treasury siphon (default 0%)

**What it means:** Shows complete breakdown of block economics

---

### [JOB-CHECK] Mining Job Created
**Level:** INFO  
**When:** Miner creates a new job (every ~1.25s or when chain updates)

```
[JOB-CHECK] ✅ Mining job created - target verified
  height=124
  difficulty=1000
  target_match=true
  epoch=0
  message_bytes_len=256
  chain_id=mainnet
  pow_fp=ab123456
```

**Fields:**
- `target_match`: true = safe to mine, false = ABORT
- `chain_id`: Network identifier (mainnet/testnet)
- `pow_fp`: POW algorithm fingerprint (first 8 chars)

**What it means:** Miner verified job is correct before starting work

---

### [MINER-ERROR] Target Mismatch
**Level:** ERROR  
**When:** Mining job fails safety check (CRITICAL BUG)

```
[MINER-ERROR] 🚨 TARGET MISMATCH - refusing to mine (algorithm drift detected)
  height=124
  header_diff=1000
  expected_target=0x00000fff...
  job_target=0x00000eee...
```

**What it means:** **PANIC!** Miner and validator are using different rules. Mining is ABORTED to prevent guaranteed rejection. This catches algorithm drift bugs before wasting hashrate.

---

## ✅ BLOCK ACCEPTANCE & REJECTION

### [ACCEPT] Block Accepted from Peer
**Level:** INFO  
**When:** Network block passes validation and is added to chain

```
[ACCEPT] Block accepted from peer - added to chain
  block=123
  hash=0xabc123...
  miner=other_miner
  parent=0xdef456...
  txs=5
  difficulty=1000
  chain_id=mainnet
  pow_fp=ab123456
```

**Fields:**
- `miner`: Who mined this block
- `parent`: Previous block hash
- `txs`: Transaction count
- `chain_id`/`pow_fp`: Proof of same network/algorithm

**What it means:** Another miner's block was accepted. Network is working!

---

### [REJECT] Block Rejected from Peer
**Level:** ERROR  
**When:** Network block fails validation

```
[REJECT] Block rejected from peer - POW validation failed
  height=123
  miner=other_miner
  hash=0xabc123...
  peer=192.168.1.100:7072
  reason=POW_INVALID
  details=computed=0xaaa..., block_has=0xbbb...
  chain_id=mainnet
  pow_fp=ab123456
```

**Rejection Reasons:**
- `POW_INVALID`: Hash doesn't meet difficulty target
- `parent_mismatch`: Parent block not found
- `state_root_mismatch`: Execution produced different state
- `tx_root_mismatch`: Transaction list hash mismatch
- `duplicate_block`: Already seen this block

**What it means:** Peer sent invalid block (possibly malicious, buggy, or on wrong chain)

---

### [REJECT] LOCAL MINED BLOCK REJECTED
**Level:** ERROR  
**When:** **YOUR** mined block is rejected (PANIC LEVEL)

```
[REJECT] 🚨 LOCAL MINED BLOCK REJECTED - PANIC LEVEL
  height=123
  miner=pow_miner
  hash=0xabc123...
  parent=0xdef456...
  difficulty=1000
  chain_id=mainnet
  pow_fp=ab123456
  reason=pow_hash mismatch: computed 0xaaa..., block has 0xbbb...
```

**Common Causes:**
1. **Algorithm drift**: Miner and validator using different POW params
2. **Stale tip**: Mined on old parent (network moved on)
3. **Wrong chain**: Mining on fork that lost the race
4. **Header bug**: Timestamp/nonce/difficulty mismatch

**What to do:**
1. Check `[JOB-CHECK]` logs for target_match failures
2. Compare `pow_fp` with network peers
3. Check `parent` hash matches current tip
4. Verify no reorg happened during mining

---

## 🏆 CANONICAL vs ORPHAN TRACKING

### [CANON] Block Became Canonical
**Level:** INFO  
**When:** Your mined block becomes part of the main chain

```
[CANON] ✅ LOCAL BLOCK BECAME CANONICAL - REWARD CONFIRMED
  height=123
  hash=0xabc123...
  miner=pow_miner
  confirmations=1
  chain_id=mainnet
  pow_fp=ab123456
```

**What it means:** **SUCCESS!** Your block is the winner. Reward is yours. You beat any competing blocks.

---

### [ORPHAN] Block Orphaned
**Level:** WARN  
**When:** Your mined block is valid but lost the race

```
[ORPHAN] ⚠️ LOCAL BLOCK ACCEPTED BUT ORPHANED
  height=123
  hash=0xabc123...
  miner=pow_miner
  chain_id=mainnet
  pow_fp=ab123456
```

**What it means:** You found a valid block, but someone else found one at the same height first. Their block became canonical, yours didn't. **This is normal in PoW** - you don't get paid for orphans.

**Why orphans happen:**
- Network latency (2 miners find blocks simultaneously)
- You were mining on stale tip
- Your block propagated slower than competitor's

**This is NOT a bug** - it's how proof-of-work consensus works!

---

## 🌐 P2P NETWORK ACTIVITY

### [P2P] Received Full Block from Peer
**Level:** INFO  
**When:** Peer sends you a complete block

```
[P2P] Received full block from peer
  peer=192.168.1.100:7072
  hash=0xabc123...
  height=123
  miner=other_miner
  txs=5
```

**What it means:** Network is propagating blocks. You're seeing other miners' work.

---

### [P2P] Received Compact Block from Peer
**Level:** INFO  
**When:** Peer sends compressed block (only tx IDs)

```
[P2P] Received compact block from peer
  peer=192.168.1.100:7072
  hash=0xabc123...
  height=123
  txs=5
```

**What it means:** Bandwidth-efficient block propagation. Node will reconstruct full block from mempool.

---

### [ACCEPT] Block INTEGRATED into main chain from peer
**Level:** INFO  
**When:** Received block successfully becomes part of main chain

```
[ACCEPT] Block INTEGRATED into main chain from peer
  peer=192.168.1.100:7072
  height=123
  miner=other_miner
  hash=0xabc123...
  txs=5
  chain_id=mainnet
  pow_fp=ab123456
```

**What it means:** Network consensus working! Blocks flowing peer-to-peer.

---

### [COMPAT] Peer Compatibility Check
**Level:** INFO/WARN  
**When:** Checking if peer can mine/validate with us

```
[COMPAT] ✅ pow_params_hash MATCH (Fix A gates passed)
  peer=192.168.1.100:7072
  their_hash=ab123456...
  our_hash=ab123456...
```

**Compatibility Requirements:**
- Same `chain_id` (mainnet/testnet)
- Same `pow_params_hash` (VisionX settings)
- Same `min_version` (>=v1.0.3)
- Same `genesis_hash`

**Incompatible peer logs:**
```
[COMPAT] ❌ pow_params_hash MISMATCH
  their_hash=xxxxxxxx...
  our_hash=ab123456...
  → Peer using different POW algorithm, disconnecting
```

---

## 🔄 SYNC & CHAIN MANAGEMENT

### [SYNC] Starting Sync
**Level:** INFO  
**When:** Node detects it's behind and starts catching up

```
[SYNC] Auto-sync triggered
  local_height=100
  peer_height=150
  behind_by=50
  sync_threshold=1
```

**What it means:** You're behind the network, downloading missing blocks

---

### [SYNC] Applying Block from Peer
**Level:** DEBUG  
**When:** Downloaded block is being validated

```
[SYNC] Applying block from peer to CHAIN.lock()
  current_tip_height=100
  incoming_block_height=101
  incoming_block_hash=0xabc123...
  blocks_in_memory=101
```

---

### [SYNC-FORK] Fork Detection
**Level:** INFO  
**When:** Node detects competing chain branches

```
[SYNC-FORK] -> GetBlockHash REQUEST received from peer
  peer=192.168.1.100:7072
  height=95
```

**What it means:** Peer checking for common ancestor to resolve fork

---

### [REORG] Chain Reorganization
**Level:** WARN  
**When:** Switching to heavier chain (rare but critical)

```
[REORG] Chain reorganization detected
  old_tip=100 (0xabc...)
  new_tip=102 (0xdef...)
  rollback_blocks=5
  new_blocks=7
```

**What it means:** Another chain had more cumulative work. Switching to it. Blocks 96-100 are now orphaned.

---

## 🛡️ PEER MANAGEMENT & STRIKES

### [STRIKE] Peer Punished
**Level:** WARN  
**When:** Peer sent bad data (automatic punishment)

```
[STRIKE] Peer received strike
  peer=192.168.1.100:7072
  reason=bad_pow
  strikes=1
  threshold=3
```

**Strike Reasons:**
- `bad_pow`: Invalid proof-of-work
- `invalid_block`: Malformed block structure
- `orphan_spam`: Sending too many orphan blocks
- `stale_block`: Repeatedly sending old blocks

**Strike System:**
- 3 strikes = temporary quarantine (10 min)
- 5 strikes = permanent ban
- Strikes decay over time (1 per hour)

---

### [QUARANTINE] Peer Temporarily Banned
**Level:** WARN  
**When:** Peer hits strike threshold

```
[QUARANTINE] Peer quarantined
  peer=192.168.1.100:7072
  strikes=3
  duration=600s
  reason=repeated bad_pow violations
```

**What it means:** Peer making too many mistakes. Temporary timeout.

---

### [BAN] Peer Permanently Banned
**Level:** ERROR  
**When:** Peer is clearly malicious

```
[BAN] Peer permanently banned
  peer=192.168.1.100:7072
  strikes=5
  final_reason=consensus_violation
```

**What it means:** Peer is attacking or severely misconfigured. Never reconnect.

---

## 💰 TRANSACTION PROCESSING

### [TX] Transaction Received
**Level:** DEBUG  
**When:** New transaction enters mempool

```
[TX] Transaction received
  hash=0xabc123...
  from=sender_address
  to=recipient_address
  amount=1000000000
  fee=10000000
```

---

### [TX] Transaction Executed
**Level:** DEBUG  
**When:** Transaction included in block

```
[TX] Transaction executed
  hash=0xabc123...
  block=123
  status=ok
```

**Status Values:**
- `ok`: Success
- `insufficient_balance`: Not enough funds
- `invalid_nonce`: Wrong sequence number
- `invalid_signature`: Bad signature

---

## 🎯 MINING SAFETY CHECKS

### [POW-PARAMS] Consensus Parameters
**Level:** INFO  
**When:** Node starts or params are verified

```
[POW-PARAMS] VisionX params: dataset_mb=2048 scratch_mb=64 mix_iters=64 reads_per_iter=8 write_every=8 epoch_blocks=32
```

**What it means:** These params MUST match all nodes or chain forks

---

### [POW-SEED] Epoch Seed
**Level:** INFO  
**When:** New epoch begins or block validated

```
[POW-SEED] epoch=0 seed_prefix=ab123456789abcde
```

**What it means:** Dataset seed for this epoch. All nodes must use same seed.

---

### [CHAIN-POW] Block Validation
**Level:** INFO  
**When:** Validating incoming block's POW

```
CHAIN-POW: validating block header inputs
  block_height=123
  block_hash=0xabc123...
  difficulty=1000
  nonce=123456789
```

---

## 📊 SYSTEM HEALTH & METRICS

### [HEALTH] System Status
**Level:** INFO  
**When:** Health check endpoint called

```
GET /health
{
  "status": "healthy",
  "chain_height": 123,
  "peers_connected": 4,
  "peers_compatible": 3,
  "mining_enabled": true,
  "sync_status": "synced"
}
```

---

### [METRICS] Performance Stats
**Level:** INFO  
**When:** Metrics endpoint queried

```
GET /metrics
{
  "blocks_mined": 10,
  "blocks_accepted": 50,
  "blocks_rejected": 2,
  "orphan_blocks": 1,
  "hashrate": 1000000,
  "difficulty": 1000
}
```

---

### [READINESS] Mining Eligibility
**Level:** INFO  
**When:** Checking if mining can start

```
[READINESS] Mining eligibility check
  compatible_peers=3 (need 3)
  sync_health=ok (behind_by=0)
  pow_validator=ready
  result=ELIGIBLE
```

**Mining Requirements:**
- 3+ compatible peers
- Sync health: behind_by = 0
- POW validator initialized

---

## ⚠️ ERROR & WARNING LOGS

### [ERROR] Database Error
**Level:** ERROR  
**When:** Storage operation fails

```
[ERROR] Database write failed
  key=block_123
  error=disk full
```

---

### [ERROR] Network Error
**Level:** ERROR  
**When:** Connection issues

```
[ERROR] Failed to connect to peer
  peer=192.168.1.100:7072
  error=connection refused
```

---

### [WARN] Mempool Full
**Level:** WARN  
**When:** Transaction pool at capacity

```
[WARN] Mempool full, rejecting transaction
  current_size=10000
  max_size=10000
  rejected_tx=0xabc123...
```

---

### [WARN] High Latency
**Level:** WARN  
**When:** Network response slow

```
[WARN] High peer latency detected
  peer=192.168.1.100:7072
  latency_ms=2500
  threshold=1000
```

---

## 🔍 DEBUG & DIAGNOSTIC LOGS

### [DEBUG] Block Construction
**Level:** DEBUG  
**When:** Building new block to mine

```
[DEBUG] Building block template
  height=124
  parent=0xabc123...
  txs_included=5
  state_root=0xdef456...
```

---

### [DEBUG] Orphan Pool
**Level:** DEBUG  
**When:** Managing orphan blocks

```
[ORPHAN-DRAIN] resolved children
  parent=0xabc123...
  children_resolved=3
```

---

### [DEBUG] Difficulty Adjustment
**Level:** DEBUG  
**When:** Retargeting mining difficulty

```
[DEBUG] Difficulty retarget
  old_difficulty=1000
  new_difficulty=1050
  block_time_avg=1.3s
  target_time=1.25s
```

---

## 🔎 FILTERING & SEARCH GUIDE

### Filter by Category

**Show only mining activity:**
```bash
# Linux/Mac
./vision-node | grep -E "\[PAYOUT\]|\[CANON\]|\[ORPHAN\]|\[JOB-CHECK\]"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[PAYOUT\]|\[CANON\]|\[ORPHAN\]|\[JOB-CHECK\]"
```

**Show only rejections/errors:**
```bash
# Linux/Mac
./vision-node | grep -E "\[REJECT\]|\[MINER-ERROR\]"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[REJECT\]|\[MINER-ERROR\]"
```

**Show only P2P network:**
```bash
# Linux/Mac
./vision-node | grep -E "\[P2P\]|\[ACCEPT\]|\[COMPAT\]"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[P2P\]|\[ACCEPT\]|\[COMPAT\]"
```

**Show only your mining (not peers):**
```bash
# Linux/Mac
./vision-node | grep -E "\[PAYOUT\]|\[CANON\]|\[ORPHAN\]" | grep "miner=YOUR_ADDRESS"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[PAYOUT\]|\[CANON\]|\[ORPHAN\]" | Select-String -Pattern "miner=YOUR_ADDRESS"
```

---

### Filter by Log Level

**INFO and above (production):**
```bash
export RUST_LOG=info
./vision-node
```

**DEBUG (detailed diagnostics):**
```bash
export RUST_LOG=debug
./vision-node
```

**Specific module debug:**
```bash
export RUST_LOG=vision_node::chain::accept=debug,info
./vision-node
```

---

### Filter by Time Range

**Last hour of logs:**
```bash
# Linux/Mac
./vision-node | grep "$(date -u +%Y-%m-%d)" | tail -1000

# Windows PowerShell
Get-Content vision-node.log | Select-String -Pattern (Get-Date -Format "yyyy-MM-dd")
```

---

### Search for Specific Block

**Find all logs for block 123:**
```bash
# Linux/Mac
./vision-node | grep "block=123\|height=123\|block_height=123"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "block=123|height=123|block_height=123"
```

---

### Search for Specific Miner

**Find all blocks from miner:**
```bash
# Linux/Mac
./vision-node | grep "miner=pow_miner"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "miner=pow_miner"
```

---

### Search for Network Issues

**Find all peer problems:**
```bash
# Linux/Mac
./vision-node | grep -E "\[STRIKE\]|\[QUARANTINE\]|\[BAN\]|\[REJECT\]"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[STRIKE\]|\[QUARANTINE\]|\[BAN\]|\[REJECT\]"
```

---

## 📈 LOG ANALYTICS EXAMPLES

### Count blocks mined today:
```bash
# Linux/Mac
grep "\[CANON\].*$(date +%Y-%m-%d)" vision-node.log | wc -l

# Windows PowerShell
(Select-String -Path vision-node.log -Pattern "\[CANON\]" | Where-Object { $_.Line -match (Get-Date -Format "yyyy-MM-dd") }).Count
```

### Calculate orphan rate:
```bash
# Orphans / (Canonical + Orphans)
# Linux/Mac
CANON=$(grep -c "\[CANON\]" vision-node.log)
ORPHAN=$(grep -c "\[ORPHAN\]" vision-node.log)
echo "Orphan rate: $(echo "scale=2; $ORPHAN * 100 / ($CANON + $ORPHAN)" | bc)%"
```

### Find busiest peer:
```bash
# Linux/Mac
grep "\[ACCEPT\].*peer=" vision-node.log | grep -oP "peer=\K[0-9.]+:[0-9]+" | sort | uniq -c | sort -rn | head -1
```

---

## 🎯 TROUBLESHOOTING BY LOG

### "No payout - am I mining?"

**Look for:**
```
[JOB-CHECK] ✅ Mining job created
[READINESS] result=ELIGIBLE
```

**If you see:**
```
[READINESS] result=NOT_ELIGIBLE (need 3 peers, have 1)
```
→ **Not enough peers**, add bootstrap nodes

---

### "I mined blocks but no reward"

**Look for:**
```
[ORPHAN] ⚠️ LOCAL BLOCK ACCEPTED BUT ORPHANED
```
→ **Normal**, you lost the race to another miner

**Or:**
```
[REJECT] 🚨 LOCAL MINED BLOCK REJECTED
```
→ **BUG**, check `reason` field and `pow_fp` mismatch

---

### "Peers keep disconnecting"

**Look for:**
```
[COMPAT] ❌ pow_params_hash MISMATCH
```
→ **You're on different POW params**, check version

**Or:**
```
[COMPAT] ❌ node_version too old (got v1.0.0, need >=v1.0.3)
```
→ **Update your node** to v1.0.3+

---

### "Mining but hashrate is 0"

**Look for:**
```
[MINER-ERROR] 🚨 TARGET MISMATCH - refusing to mine
```
→ **CRITICAL BUG**, miner/validator using different rules

**Or:**
```
[READINESS] preview_only=true (waiting for sync)
```
→ **Not synced yet**, wait for behind_by=0

---

### "Blocks not propagating"

**Look for:**
```
[ACCEPT] Block INTEGRATED into main chain
```
→ Should see this for EVERY block (yours + peers')

**If missing:**
- Check firewall (port 7072 must be open)
- Check peer count (`peers_compatible >= 3`)
- Check `[P2P]` logs for connection errors

---

## 📝 LOG FILE LOCATIONS

### Default Locations:

**Linux:**
```
./vision-node > vision-node.log 2>&1
```

**Windows:**
```
.\vision-node.exe > vision-node.log 2>&1
```

**Systemd Service (Linux):**
```
journalctl -u vision-node -f
```

**Docker Container:**
```
docker logs -f vision-node
```

---

## 🔐 PROOF-GRADE VERIFICATION

Every critical log includes:

**chain_id:** Network identifier (mainnet/testnet/devnet)
**pow_fp:** POW algorithm fingerprint (first 8 hex chars of params hash)

**Example:**
```
chain_id=mainnet pow_fp=ab123456
```

**What this proves:**
- Same network (not mixing mainnet/testnet)
- Same POW algorithm (VisionX params match)
- Cryptographically verifiable in screenshots

**To verify manually:**
```bash
# Get your POW fingerprint
curl http://localhost:7070/health | jq .pow_params_hash

# Compare with peer
curl http://peer:7070/health | jq .pow_params_hash
```

They MUST match or you're on incompatible chains!

---

## 📚 RELATED DOCUMENTATION

- [MINING_PAYOUT_VERIFICATION.md](MINING_PAYOUT_VERIFICATION.md) - Reward calculation details
- [MINER_PAYOUT_LOGGING.md](MINER_PAYOUT_LOGGING.md) - Payout log examples
- [GATE_ALIGNMENT_COMPLETE.md](GATE_ALIGNMENT_COMPLETE.md) - Mining eligibility gates
- [NETWORK_SECURITY_FIXES_TODO.md](NETWORK_SECURITY_FIXES_TODO.md) - Security improvements

---

## 🎯 QUICK REFERENCE CARD

```
✅ SUCCESS LOGS (good):
  [PAYOUT] Block mined
  [CANON] Block became canonical
  [ACCEPT] Block accepted
  [JOB-CHECK] Target verified

⚠️  WARNING LOGS (investigate):
  [ORPHAN] Block orphaned (normal but worth watching)
  [WARN] High latency
  [STRIKE] Peer punished

🚨 ERROR LOGS (urgent):
  [REJECT] Block rejected
  [MINER-ERROR] Target mismatch
  [COMPAT] Params mismatch
  [BAN] Peer banned

🔧 DEBUG LOGS (optional):
  [DEBUG] Detailed diagnostics
  [SYNC] Chain synchronization
  [TX] Transaction processing
```

---

**Status:** Production Ready ✅  
**Deployment:** v1.0.3 (2026-01-12)  
**Coverage:** 100% of user-facing logs documented
