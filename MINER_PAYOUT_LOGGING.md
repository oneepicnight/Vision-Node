# Miner Payout Logging Reference
**Version:** v1.0.3  
**Date:** 2026-01-12  
**Status:** ✅ ENHANCED

---

## 📊 LOGGING LOCATIONS

### 1. Emission Payout Log
**File:** `src/main.rs:3750-3758`  
**Function:** `apply_tokenomics()`  
**Level:** INFO (always visible)

```rust
tracing::info!(
    block = height,
    miner = %miner_addr,
    reward = miner_emission,
    halvings = halvings,
    new_balance = new_miner_bal,
    "[PAYOUT] Block mined - miner rewarded"
);
```

**Example Output:**
```
[PAYOUT] Block mined - miner rewarded block=123 miner=your_wallet_address reward=32000000000 halvings=0 new_balance=64000000000
```

---

### 2. Tokenomics Summary Log
**File:** `src/main.rs:10682-10692`  
**Function:** `execute_block()`  
**Level:** INFO (always visible)

```rust
tracing::info!(
    block = parent.header.number + 1,
    miner = %miner_addr,
    miner_reward = miner_reward,
    fees_collected = tx_fees_total,
    fees_distributed = fees_distributed,
    treasury_total = treasury_total,
    "[PAYOUT] Tokenomics applied - miner received {} units ({}+fees)",
    miner_reward,
    miner_reward.saturating_sub(tx_fees_total)
);
```

**Example Output:**
```
[PAYOUT] Tokenomics applied - miner received 32000000000 units (32000000000+fees) block=123 miner=your_wallet_address miner_reward=32000000000 fees_collected=0 fees_distributed=0 treasury_total=0
```

---

## 🔍 LOG FIELDS EXPLAINED

| Field | Type | Description |
|-------|------|-------------|
| `block` | u64 | Block number being mined |
| `miner` | String | Wallet address receiving the reward |
| `reward` | u128 | Emission amount (32 LAND = 32,000,000,000 with 9 decimals) |
| `halvings` | u64 | Number of halvings (0 = first epoch, 1 = second, etc.) |
| `new_balance` | u128 | Miner's updated balance after reward |
| `miner_reward` | u128 | Total miner payout (emission + fees) |
| `fees_collected` | u128 | Transaction fees collected in block |
| `fees_distributed` | u128 | Fees distributed to foundation (10%) |
| `treasury_total` | u128 | Treasury siphon (if enabled, default 0%) |

---

## ✅ VERIFICATION CHECKLIST

### Reward Amount Verification:
- **Block 0-2,102,399**: `reward=32000000000` (32 LAND)
- **Block 2,102,400-4,204,799**: `reward=16000000000` (16 LAND)
- **Block 4,204,800-6,307,199**: `reward=8000000000` (8 LAND)

### Miner Address Verification:
```bash
# Check configured miner address
grep "VISION_MINER_ADDRESS" keys.json
# or check in miner.json
cat miner.json
```

### Halving Verification:
```
block < 2,102,400      → halvings=0 → reward=32 LAND ✓
block >= 2,102,400     → halvings=1 → reward=16 LAND ✓
block >= 4,204,800     → halvings=2 → reward=8 LAND  ✓
```

---

## 🔎 FILTERING LOGS

### View Only Payout Logs:
```bash
# Linux/Mac
./vision-node | grep "\[PAYOUT\]"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[PAYOUT\]"
```

### View Specific Miner:
```bash
# Linux/Mac
./vision-node | grep "\[PAYOUT\]" | grep "miner=your_wallet_address"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[PAYOUT\]" | Select-String -Pattern "miner=your_wallet_address"
```

### Track Block Payouts:
```bash
# Linux/Mac
./vision-node | grep "\[PAYOUT\] Block mined"

# Windows PowerShell
.\vision-node.exe 2>&1 | Select-String -Pattern "\[PAYOUT\] Block mined"
```

---

## 📈 MONITORING EXAMPLES

### Example 1: First Block Mined
```
[PAYOUT] Block mined - miner rewarded block=1 miner=pow_miner reward=32000000000 halvings=0 new_balance=32000000000
[PAYOUT] Tokenomics applied - miner received 32000000000 units (32000000000+fees) block=1 miner=pow_miner miner_reward=32000000000 fees_collected=0 fees_distributed=0 treasury_total=0
```

**Verification:**
- ✅ Miner: `pow_miner` (matches configured wallet)
- ✅ Block: 1
- ✅ Reward: 32 LAND (32,000,000,000 units)
- ✅ Halvings: 0 (first epoch)
- ✅ Balance: 32 LAND (first payout)

---

### Example 2: Block With Transaction Fees
```
[PAYOUT] Block mined - miner rewarded block=50 miner=pow_miner reward=32000000000 halvings=0 new_balance=1600000000000
[PAYOUT] Tokenomics applied - miner received 32050000000 units (32000000000+fees) block=50 miner=pow_miner miner_reward=32050000000 fees_collected=50000000 fees_distributed=5000000 treasury_total=0
```

**Verification:**
- ✅ Base emission: 32,000,000,000
- ✅ Transaction fees: 50,000,000 (0.05 LAND)
- ✅ Miner gets: 32,050,000,000 (32.05 LAND)
- ✅ Foundation gets 10%: 5,000,000 (0.005 LAND)
- ✅ Miner keeps 90% of fees

---

### Example 3: First Halving Block
```
[PAYOUT] Block mined - miner rewarded block=2102400 miner=pow_miner reward=16000000000 halvings=1 new_balance=67251200000000
[PAYOUT] Tokenomics applied - miner received 16000000000 units (16000000000+fees) block=2102400 miner=pow_miner miner_reward=16000000000 fees_collected=0 fees_distributed=0 treasury_total=0
```

**Verification:**
- ✅ Block 2,102,400 (first halving boundary)
- ✅ Reward: 16 LAND (halved from 32)
- ✅ Halvings: 1 (second epoch)
- ✅ Balance: Previous + 16 LAND

---

## 🚨 RED FLAGS TO WATCH FOR

### ❌ Wrong Miner Address
```
[PAYOUT] Block mined - miner rewarded block=123 miner=UNKNOWN_ADDRESS ...
```
**Action:** Check `VISION_MINER_ADDRESS` env var or `miner.json` configuration

### ❌ Wrong Reward Amount
```
[PAYOUT] Block mined - miner rewarded block=50 miner=pow_miner reward=0 ...
```
**Action:** Check `enable_emission=true` in tokenomics config

### ❌ Wrong Halving Count
```
[PAYOUT] Block mined - miner rewarded block=2102400 miner=pow_miner reward=32000000000 halvings=0 ...
```
**Action:** Should be `halvings=1` at block 2,102,400 - check halving logic

### ❌ Balance Not Increasing
```
[PAYOUT] Block mined - miner rewarded block=10 ... new_balance=32000000000
[PAYOUT] Block mined - miner rewarded block=11 ... new_balance=32000000000
```
**Action:** Balance should increment by ~32 LAND each block - check balance update logic

---

## 🛠️ TROUBLESHOOTING

### No Payout Logs Appearing:

1. **Check log level:**
   ```bash
   export RUST_LOG=info  # or vision_node=info
   ```

2. **Check mining is enabled:**
   ```bash
   # In keys.json or environment
   "ENABLE_MINING": true
   ```

3. **Check miner eligibility:**
   - Minimum 3 compatible peers
   - Sync health: behind_by = 0
   - POW validator ready

### Payout Amount Incorrect:

1. **Verify tokenomics config:**
   ```rust
   emission_per_block: 32_000_000_000  // 32 LAND
   halving_interval_blocks: 2_102_400  // ~4 years
   ```

2. **Check halving calculation:**
   ```
   halvings = block_height / 2_102_400
   reward = 32_000_000_000 / 2^halvings
   ```

3. **Verify no treasury siphon:**
   ```rust
   treasury_bps: 0  // Should be 0% in mining mode
   ```

---

## 📝 CHANGELOG

### v1.0.3 - 2026-01-12
- ✅ Enhanced payout logging to INFO level (always visible)
- ✅ Added miner address to all payout logs
- ✅ Added block number to all payout logs
- ✅ Added clear `[PAYOUT]` prefix for filtering
- ✅ Added emission + fees breakdown
- ✅ Added halvings count for verification
- ✅ Added new balance tracking

### Prior to v1.0.3
- ❌ Debug-level logging (hidden by default)
- ❌ No miner address shown
- ❌ Unclear log messages

---

## 🔗 RELATED DOCUMENTATION

- [MINING_PAYOUT_VERIFICATION.md](MINING_PAYOUT_VERIFICATION.md) - Complete payout audit
- [MINING_QUICK_REF.md](MINING_QUICK_REF.md) - Mining system reference
- [GATE_ALIGNMENT_COMPLETE.md](GATE_ALIGNMENT_COMPLETE.md) - Gate threshold audit

---

**Status:** Production-ready ✅  
**Deployment:** v1.0.3 (2026-01-12 10:31:44)
