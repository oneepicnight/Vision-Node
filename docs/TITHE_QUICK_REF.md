# Tokenomics + Tithe Quick Reference

## 🚀 Start Node
```bash
.\target\release\vision-node.exe
```

## 📊 Key Endpoints

### Foundation Addresses
```bash
curl -s http://127.0.0.1:7070/foundation/addresses | jq
```

### Tokenomics Stats
```bash
curl -s http://127.0.0.1:7070/tokenomics/stats | jq
```

### Emission Calculator
```bash
curl -s http://127.0.0.1:7070/tokenomics/emission/0 | jq
curl -s http://127.0.0.1:7070/tokenomics/emission/2102400 | jq  # After halving
```

### Balance Checks
```bash
# Vault (50% of tithe)
curl -s http://127.0.0.1:7070/api/balance/0xb977c16e539670ddfecc0ac902fcb916ec4b944e | jq

# Fund/Ops (30% of tithe)
curl -s http://127.0.0.1:7070/api/balance/0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd | jq

# Treasury/Founders (20% of tithe)
curl -s http://127.0.0.1:7070/api/balance/0xdf7a79291bb96e9dd1c77da089933767999eabf0 | jq
```

### Total Supply
```bash
curl -s http://127.0.0.1:7070/api/supply | jq
```

## 💰 Expected Tithe Distribution (Per Block)

| Component | Amount (units) | Amount (LAND) | Recipient |
|-----------|----------------|---------------|-----------|
| **Vault** | 100,000,000 | 1.0 | Cold Storage |
| **Fund** | 60,000,000 | 0.6 | Ops/Dev |
| **Treasury** | 40,000,000 | 0.4 | Founders |
| **Total** | 200,000,000 | 2.0 | All |

## ⚙️ Configuration (.env)

### Core Settings
```properties
VISION_TOK_ENABLE_EMISSION=true
VISION_TOK_EMISSION_PER_BLOCK=1000000000000
VISION_TOK_HALVING_INTERVAL_BLOCKS=2102400
```

### Foundation Addresses
```properties
VISION_TOK_VAULT_ADDR=0xb977c16e539670ddfecc0ac902fcb916ec4b944e
VISION_TOK_FUND_ADDR=0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd
VISION_TOK_TREASURY_ADDR=0xdf7a79291bb96e9dd1c77da089933767999eabf0
```

### Tithe Settings
```properties
VISION_TOK_TITHE_AMOUNT=200000000       # 2 LAND
VISION_TOK_TITHE_MINER_BPS=0            # 0%
VISION_TOK_TITHE_VAULT_BPS=5000         # 50%
VISION_TOK_TITHE_FUND_BPS=3000          # 30%
VISION_TOK_TITHE_TREASURY_BPS=2000      # 20%
```

## 🧪 Test Commands

### Mine Block
```bash
curl -X POST http://127.0.0.1:7070/mine \
  -H "Content-Type: application/json" \
  -d '{"miner_addr":"YOUR_ADDRESS"}' | jq
```

### Run Full Test Suite
```powershell
.\test-tokenomics-tithe.ps1
```

## 📈 Supply Growth Math

**Per Block:**
- Emission: 1000 tokens (to miner)
- Tithe: 2 LAND (split across foundation)
- **Total Supply Growth:** 1000 tokens + 2 LAND

**After 1,000,000 blocks:**
- Miner earned: 1,000,000,000 tokens (before halvings)
- Vault accumulated: 1,000,000 LAND
- Fund accumulated: 600,000 LAND
- Treasury accumulated: 400,000 LAND

## 🔍 Logs to Watch

```
tokenomics emission: height=1, halvings=0, emission=1000000000000, miner_bal=...
block tithe: height=1, amount=200000000, splits(miner/vault/fund/tres)=0/100000000/60000000/40000000
```

## 📞 Documentation

- **Full Guide:** `TOKENOMICS_TITHE_IMPLEMENTATION.md`
- **Tokenomics API:** `TOKENOMICS_QUICKSTART.md`
- **Vault Epochs:** `VAULT_EPOCH_IMPLEMENTATION.md`
