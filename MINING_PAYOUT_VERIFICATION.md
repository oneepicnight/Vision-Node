# Mining Payout Verification Report
**Date:** 2026-01-12  
**Version:** v1.0.3  
**Status:** ✅ VERIFIED CORRECT

---

## 📊 BLOCK REWARD FORMULA

### Components:
1. **Miner Emission** (Bitcoin-style halving)
2. **Block Tithe** (Fixed 2 LAND, split to foundation)
3. **Transaction Fees** (Optional, distributed 50/30/20)

---

## 💰 MINER PAYOUT CALCULATION

### Source Code: `src/main.rs:3720-3760`

```rust
// 1. Calculate emission with halving
let halvings = height / cfg.halving_interval_blocks;
let halving_divisor = 2u128.saturating_pow(halvings as u32);
miner_emission = cfg.emission_per_block / halving_divisor;

// 2. Credit miner with emission
let miner_key = acct_key(miner_addr);
let miner_bal = chain.balances.entry(miner_key.clone()).or_insert(0);
*miner_bal = miner_bal.saturating_add(miner_emission);

// 3. Block tithe (miner gets 0% of tithe)
let tithe_miner = tithe_amt.saturating_mul(bp_miner as u128) / 10_000; // = 0
```

---

## 🔢 DEFAULT PARAMETERS

| Parameter | Value | Notes |
|-----------|-------|-------|
| `emission_per_block` | 32,000,000,000 | 32 LAND (9 decimals) |
| `halving_interval_blocks` | 2,102,400 | ~4 years @ 1.25s blocks |
| `decimals` | 9 | 1 LAND = 10^9 units |
| `tithe_amount` | 2,000,000,000 | 2 LAND per block |
| `tithe_miner_bps` | 0 | Miner gets 0% of tithe |
| `tithe_vault_bps` | 5,000 | Vault gets 50% (1 LAND) |
| `tithe_fund_bps` | 3,000 | Fund gets 30% (0.6 LAND) |
| `tithe_treasury_bps` | 2,000 | Treasury gets 20% (0.4 LAND) |

**Source:** `src/main.rs:2554-2576` and `src/tokenomics/tithe.rs:9-18`

---

## 📈 HALVING SCHEDULE

| Block Range | Halvings | Emission | Miner Gets |
|-------------|----------|----------|------------|
| **0 - 2,102,399** | 0 | 32 LAND | **32 LAND** ✅ |
| **2,102,400 - 4,204,799** | 1 | 16 LAND | **16 LAND** |
| **4,204,800 - 6,307,199** | 2 | 8 LAND | **8 LAND** |
| **6,307,200 - 8,409,599** | 3 | 4 LAND | **4 LAND** |
| **8,409,600 - 10,511,999** | 4 | 2 LAND | **2 LAND** |

**Formula:** `emission = 32 / 2^halvings`

---

## 🎯 DETAILED BREAKDOWN (Block 0-2,102,399)

### Per-Block Allocations:

| Recipient | Source | Amount | Percentage | Calculation |
|-----------|--------|--------|------------|-------------|
| **MINER** | Emission | **32.000 LAND** | **100%** | `emission_per_block` ✅ |
| ⎿ Subtotal | | **32.000 LAND** | | |
| **Vault** | Tithe | 1.000 LAND | 50% | `2 × 0.50` |
| **Fund** | Tithe | 0.600 LAND | 30% | `2 × 0.30` |
| **Treasury** | Tithe | 0.400 LAND | 20% | `2 × 0.20` |
| ⎿ Subtotal (Foundation) | | 2.000 LAND | | |
| **TOTAL MINTED** | | **34.000 LAND** | | Emission + Tithe |

### Transaction Fees (Variable):
- **Fee distribution:** 10% of fees (configurable, default `fee_burn_bps=1000`)
- **Split:** 50% Vault, 30% Fund, 20% Treasury
- **Miner gets:** Transaction fees collected (separate from emission)

---

## ✅ VERIFICATION CHECKLIST

### Code Locations Verified:

1. **Emission Calculation** ✅
   - File: `src/main.rs:3736-3739`
   - Logic: `emission = base / 2^halvings`
   - Correct: Bitcoin-style halving

2. **Miner Credit** ✅
   - File: `src/main.rs:3741-3756`
   - Logic: `miner_bal += miner_emission`
   - Correct: Full emission to miner

3. **Tithe Allocation** ✅
   - File: `src/main.rs:3758-3833`
   - Logic: Miner gets 0% of tithe (line 3767: `tithe_miner = tithe × 0 / 10000`)
   - Correct: Miner doesn't share tithe

4. **Supply Increase** ✅
   - File: `src/main.rs:3748` (emission)
   - File: `src/main.rs:3827` (tithe)
   - Correct: Both increase total supply

5. **Fee Distribution** ✅
   - File: `src/main.rs:3835-3886`
   - Logic: 10% distributed (50/30/20 split)
   - Correct: Separate from block reward

---

## 🔍 EDGE CASES VERIFIED

### ✅ Halving Boundary (Block 2,102,400)
```rust
// Block 2,102,399: halvings = 2102399 / 2102400 = 0 → 32 LAND
// Block 2,102,400: halvings = 2102400 / 2102400 = 1 → 16 LAND
```
**Status:** Correct - uses integer division

### ✅ Overflow Protection
```rust
miner_bal.saturating_add(miner_emission)  // Line 3745
```
**Status:** Correct - saturating arithmetic prevents overflow

### ✅ Supply Tracking
```rust
chain.add_supply(miner_emission);  // Line 3748
chain.add_supply(tithe_amt);       // Line 3827
```
**Status:** Correct - both emissions tracked

---

## 📊 EXAMPLE CALCULATIONS

### Block 60 (Current Height):
```
Halvings: 60 / 2,102,400 = 0
Halving Divisor: 2^0 = 1
Miner Emission: 32,000,000,000 / 1 = 32,000,000,000 (32 LAND)

Tithe: 2,000,000,000 (2 LAND)
  - Miner: 0% = 0 LAND
  - Vault: 50% = 1,000,000,000 (1 LAND)
  - Fund: 30% = 600,000,000 (0.6 LAND)
  - Treasury: 20% = 400,000,000 (0.4 LAND)

MINER RECEIVES: 32 LAND ✅
```

### Block 2,102,400 (First Halving):
```
Halvings: 2,102,400 / 2,102,400 = 1
Halving Divisor: 2^1 = 2
Miner Emission: 32,000,000,000 / 2 = 16,000,000,000 (16 LAND)

Tithe: Same (2 LAND to foundation)

MINER RECEIVES: 16 LAND ✅
```

### Block 10,000,000 (4 Halvings):
```
Halvings: 10,000,000 / 2,102,400 = 4
Halving Divisor: 2^4 = 16
Miner Emission: 32,000,000,000 / 16 = 2,000,000,000 (2 LAND)

Tithe: Same (2 LAND to foundation)

MINER RECEIVES: 2 LAND ✅
```

---

## 🎯 SUMMARY

### ✅ MINER PAYOUT IS CORRECT

**Current Era (Block 0-2,102,399):**
- **Miner receives:** 32 LAND per block (emission only)
- **Foundation receives:** 2 LAND per block (tithe, split 50/30/20)
- **Total minted:** 34 LAND per block

**Key Findings:**
1. ✅ Emission formula matches Bitcoin-style halving
2. ✅ Miner gets 100% of emission (32 LAND)
3. ✅ Miner gets 0% of tithe (as designed)
4. ✅ Halving occurs exactly at interval boundaries
5. ✅ Supply tracking is accurate
6. ✅ Overflow protection in place

**No Issues Found** - Mining payouts are calculated correctly according to the tokenomics design.

---

## 📝 NOTES

1. **Tithe Design:** The miner gets 0% of the 2 LAND tithe because they already receive the full 32 LAND emission. The tithe funds the foundation (Vault/Fund/Treasury) to ensure ecosystem sustainability.

2. **Transaction Fees:** Separate from block reward. Miner collects fees, and 10% is distributed to foundation (configurable via `fee_burn_bps`).

3. **Supply Cap:** System continues until total supply reaches MAX_SUPPLY, then transitions to Staking Era (emissions end).

4. **Halving Precision:** Integer division ensures clean halving boundaries with no fractional edge cases.

---

## 🔒 SECURITY NOTES

- ✅ Saturating arithmetic prevents overflow attacks
- ✅ Balance updates happen atomically
- ✅ Supply tracking matches balance increases
- ✅ No double-spend in reward distribution
- ✅ Miner address validated before payout (chain/accept.rs:32-38)

**Status:** Production-ready, no payout vulnerabilities found.
