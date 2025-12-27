# Tokenomics + 2-LAND Block Tithe Implementation

**Date:** November 11, 2025  
**Status:** ✅ Complete - Ready for Testing

---

## 🎯 Overview

Successfully purged custom emission code and wired the **official Tokenomics module** with a **2-LAND block tithe** system that ensures Vault growth from block 1.

### What Changed

1. ✅ **Removed** `src/emissions.rs` (custom halvings, reward splits)
2. ✅ **Created** `src/tokenomics/tithe.rs` (2-LAND block tithe)
3. ✅ **Rewrote** `apply_tokenomics()` to use official Tokenomics emission
4. ✅ **Added** `/foundation/addresses` debug endpoint
5. ✅ **Updated** `.env` with complete Tokenomics configuration

---

## 📊 Emission System Architecture

### Official Tokenomics Emission
- **Source:** Built-in Tokenomics module (Bitcoin-style halving)
- **Default:** 1000 tokens per block (configurable via `VISION_TOK_EMISSION_PER_BLOCK`)
- **Halving:** Every 2,102,400 blocks (~30 days at 1.25s blocks)
- **Recipient:** 100% to miner
- **Supply Impact:** Increases total supply

### 2-LAND Block Tithe
- **Amount:** 2 LAND per block (200,000,000 units with 8 decimals)
- **Frequency:** Every block (applied after Tokenomics emission)
- **Purpose:** Ensure Vault grows from block 1 to fund future mining incentives
- **Supply Impact:** Increases total supply (minted)

---

## 💰 Foundation Address Mapping

| Role | Address | Tithe Share |
|------|---------|-------------|
| **Vault** (Cold Storage) | `0xb977c16e539670ddfecc0ac902fcb916ec4b944e` | 50% (5000 bps) |
| **Fund** (Ops/Dev) | `0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd` | 30% (3000 bps) |
| **Treasury** (Founders) | `0xdf7a79291bb96e9dd1c77da089933767999eabf0` | 20% (2000 bps) |

**Note:** These match the Tokenomics guide naming convention (Vault/Fund/Treasury)

---

## ⚙️ Configuration (.env)

```properties
# === Tokenomics Core ===
VISION_TOK_ENABLE_EMISSION=true
VISION_TOK_EMISSION_PER_BLOCK=1000000000000
VISION_TOK_HALVING_INTERVAL_BLOCKS=2102400
VISION_TOK_FEE_BURN_BPS=0
VISION_TOK_TREASURY_BPS=0
VISION_TOK_STAKING_EPOCH_BLOCKS=720

# === Foundation Addresses ===
VISION_TOK_VAULT_ADDR=0xb977c16e539670ddfecc0ac902fcb916ec4b944e
VISION_TOK_FUND_ADDR=0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd
VISION_TOK_TREASURY_ADDR=0xdf7a79291bb96e9dd1c77da089933767999eabf0

# === 2-LAND Block Tithe ===
VISION_TOK_TITHE_AMOUNT=200000000
VISION_TOK_TITHE_MINER_BPS=0
VISION_TOK_TITHE_VAULT_BPS=5000
VISION_TOK_TITHE_FUND_BPS=3000
VISION_TOK_TITHE_TREASURY_BPS=2000
```

### Customization Examples

**Give miners 20% of tithe:**
```properties
VISION_TOK_TITHE_MINER_BPS=2000
VISION_TOK_TITHE_VAULT_BPS=4000
VISION_TOK_TITHE_FUND_BPS=2500
VISION_TOK_TITHE_TREASURY_BPS=1500
```

**Increase tithe to 5 LAND:**
```properties
VISION_TOK_TITHE_AMOUNT=500000000
```

---

## 🔌 API Endpoints

### 1. Check Foundation Addresses
```bash
curl -s http://127.0.0.1:7070/foundation/addresses | jq
```

**Response:**
```json
{
  "ok": true,
  "addresses": {
    "vault": "0xb977c16e539670ddfecc0ac902fcb916ec4b944e",
    "fund": "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd",
    "treasury": "0xdf7a79291bb96e9dd1c77da089933767999eabf0"
  },
  "tithe": {
    "amount": "200000000",
    "split_bps": {
      "miner": 0,
      "vault": 5000,
      "fund": 3000,
      "treasury": 2000
    }
  },
  "note": "Tithe is applied every block..."
}
```

### 2. Check Tokenomics Stats
```bash
curl -s http://127.0.0.1:7070/tokenomics/stats | jq
```

**Response:**
```json
{
  "ok": true,
  "config": {
    "enable_emission": true,
    "emission_per_block": "1000000000000",
    "halving_interval_blocks": 2102400,
    "vault_addr": "0xb977c16e539670ddfecc0ac902fcb916ec4b944e",
    "fund_addr": "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd",
    "treasury_addr": "0xdf7a79291bb96e9dd1c77da089933767999eabf0"
  },
  "state": {
    "current_height": 42,
    "total_supply": "...",
    "vault_total": "...",
    "fund_total": "...",
    "treasury_total": "..."
  }
}
```

### 3. Check Emission at Height
```bash
curl -s http://127.0.0.1:7070/tokenomics/emission/0 | jq
curl -s http://127.0.0.1:7070/tokenomics/emission/2102400 | jq  # After first halving
```

**Response:**
```json
{
  "ok": true,
  "height": 0,
  "halvings": 0,
  "halving_divisor": 1,
  "block_emission": "1000000000000",
  "tithe": {
    "amount": "200000000",
    "vault_share": "100000000",
    "fund_share": "60000000",
    "treasury_share": "40000000",
    "split_bps": {
      "miner": 0,
      "vault": 5000,
      "fund": 3000,
      "treasury": 2000
    }
  }
}
```

### 4. Check Vault Epoch Status
```bash
curl -s http://127.0.0.1:7070/vault/epoch | jq
```

**Response:**
```json
{
  "ok": true,
  "current_epoch": 5,
  "vault_balance": "...",
  "total_staked": "...",
  "next_payout_height": 3600
}
```

---

## 🧪 Testing Steps

### 1. One-Time Migration
```bash
curl -X POST "http://127.0.0.1:7070/admin/migrations/tokenomics_v1?admin_token=YOUR_TOKEN" | jq
```

### 2. Start Node
```bash
.\target\release\vision-node.exe
```

### 3. Mine First Block
```bash
curl -X POST http://127.0.0.1:7070/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_addr":"YOUR_MINER_ADDR"}' | jq
```

### 4. Verify Balances

**Check Vault:**
```bash
curl -s "http://127.0.0.1:7070/api/balance/0xb977c16e539670ddfecc0ac902fcb916ec4b944e" | jq
# Expected: 100000000 (1 LAND = 50% of 2 LAND tithe)
```

**Check Fund (Ops):**
```bash
curl -s "http://127.0.0.1:7070/api/balance/0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd" | jq
# Expected: 60000000 (0.6 LAND = 30% of 2 LAND tithe)
```

**Check Treasury (Founders):**
```bash
curl -s "http://127.0.0.1:7070/api/balance/0xdf7a79291bb96e9dd1c77da089933767999eabf0" | jq
# Expected: 40000000 (0.4 LAND = 20% of 2 LAND tithe)
```

**Check Miner:**
```bash
curl -s "http://127.0.0.1:7070/api/balance/YOUR_MINER_ADDR" | jq
# Expected: 1000000000000 (1000 tokens = Tokenomics emission)
# Miner gets 0 from tithe (unless you configured TITHE_MINER_BPS > 0)
```

### 5. Verify Supply Growth
```bash
curl -s http://127.0.0.1:7070/api/supply | jq
```

**Expected after 1 block:**
- Total Supply = Emission (1000 tokens) + Tithe (2 LAND)
- Emission goes to miner
- Tithe split: 1 LAND (Vault) + 0.6 LAND (Fund) + 0.4 LAND (Treasury)

### 6. Mine 10 Blocks and Re-check
```bash
for i in {1..10}; do
  curl -X POST http://127.0.0.1:7070/mine \
    -H "Content-Type: application/json" \
    -d '{"miner_addr":"YOUR_MINER_ADDR"}' -s | jq '.ok'
  sleep 2
done

# Check vault growth
curl -s "http://127.0.0.1:7070/api/balance/0xb977c16e539670ddfecc0ac902fcb916ec4b944e" | jq
# Expected: 1000000000 (10 LAND = 10 blocks × 1 LAND per block)
```

---

## 📈 Expected Results Per Block

| Component | Amount | Recipient | Impact |
|-----------|--------|-----------|--------|
| **Tokenomics Emission** | 1000 tokens | Miner | +Supply |
| **Tithe (Vault)** | 1 LAND (50%) | Vault | +Supply, +Vault |
| **Tithe (Fund)** | 0.6 LAND (30%) | Fund/Ops | +Supply, +Fund |
| **Tithe (Treasury)** | 0.4 LAND (20%) | Treasury/Founders | +Supply, +Treasury |
| **Total Supply Growth** | 1000 tokens + 2 LAND | - | - |

---

## 🔍 Monitoring

### Prometheus Metrics
```bash
curl -s http://127.0.0.1:7070/metrics | grep vision_tok
```

**Key Metrics:**
- `vision_tok_supply` - Total supply (should grow by emission + tithe per block)
- `vision_tok_vault_total` - Vault cumulative balance
- `vision_tok_fund_total` - Fund cumulative balance
- `vision_tok_treasury_total` - Treasury cumulative balance

### Logs
Look for these log entries:
```
tokenomics emission: height=1, halvings=0, emission=1000000000000, miner_bal=...
block tithe: height=1, amount=200000000, splits(miner/vault/fund/tres)=0/100000000/60000000/40000000
```

---

## 🛠️ Code Architecture

### Files Changed

1. **`src/main.rs`**
   - Removed `mod emissions;`
   - Added `mod tokenomics;`
   - Rewrote `apply_tokenomics()` to use official Tokenomics + tithe
   - Updated `tokenomics_emission_handler()` to show tithe info
   - Added `foundation_addresses_handler()` debug endpoint

2. **`src/tokenomics/mod.rs`** (NEW)
   - Module declaration

3. **`src/tokenomics/tithe.rs`** (NEW)
   - `tithe_amount()` - Returns 2 LAND (200M units)
   - `tithe_split_bps()` - Returns (miner, vault, fund, treasury) basis points
   - `vault_addr()`, `fund_addr()`, `treasury_addr()` - Read from env
   - Unit tests for defaults and customization

4. **`src/emissions.rs`** (DISABLED)
   - Renamed to `emissions.rs.DISABLED` (no longer used)

5. **`.env`** (UPDATED)
   - Complete Tokenomics configuration
   - Foundation addresses
   - Tithe amount and splits

---

## ✅ Benefits

1. **Vault Growth from Block 1**
   - Vault receives 1 LAND per block immediately
   - After 1,000,000 blocks: Vault has 1M LAND
   - Ensures long-term sustainability for mining incentives

2. **Configurable Without Code Changes**
   - Adjust tithe amount via `VISION_TOK_TITHE_AMOUNT`
   - Change splits via `VISION_TOK_TITHE_*_BPS` vars
   - No recompilation needed

3. **Official Tokenomics Integration**
   - Uses proven Bitcoin-style halving
   - All endpoints already documented in TOKENOMICS_QUICKSTART.md
   - Migration path via `/admin/migrations/tokenomics_v1`

4. **Vault Epoch System Works Automatically**
   - Existing `vault_epoch.rs` reads Vault balance
   - Distributes growth to landholders pro-rata
   - No changes needed to epoch system

---

## 🚨 Important Notes

### Supply Accounting
- **Emission:** Increases supply, goes to miner
- **Tithe:** Increases supply, split across foundation addresses
- **Total Supply Growth:** Emission + Tithe per block

### Fee Distribution
- Currently set to 0% (`VISION_TOK_FEE_BURN_BPS=0`)
- Can be enabled later if you want fees to also split 50/30/20

### Treasury Siphon
- Currently set to 0% (`VISION_TOK_TREASURY_BPS=0`)
- Can be enabled later if you want treasury to get % of emission

### Basis Points
- 10000 bps = 100%
- 5000 bps = 50%
- 1000 bps = 10%
- Tithe splits must sum to 10000

---

## 📞 Support

**API Reference:** See `TOKENOMICS_QUICKSTART.md`  
**Vault Epochs:** See `VAULT_EPOCH_IMPLEMENTATION.md`  
**Configuration:** See `.env` comments

**Debug Endpoint:** `GET /foundation/addresses`  
**Stats Endpoint:** `GET /tokenomics/stats`  
**Emission Calculator:** `GET /tokenomics/emission/:height`

---

## 🎉 Summary

✅ Custom emission code purged  
✅ Official Tokenomics module wired  
✅ 2-LAND block tithe implemented  
✅ Foundation addresses configured (Vault/Fund/Treasury)  
✅ Vault grows from block 1  
✅ Configurable via environment variables  
✅ Debug endpoints added  
✅ Ready for testing  

**Next Step:** Build, migrate, mine blocks, verify balances! 🚀
