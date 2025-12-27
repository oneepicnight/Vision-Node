# Security Audit: Admin Endpoint Hardening

**Status**: ✅ Complete  
**Date**: 2024  
**Auditor**: Vision Node Core Team  

## Executive Summary

This document details the security audit performed on Vision Node to identify and remove god-mode endpoints before mainnet launch. All identified admin endpoints have been properly gated or removed to ensure only legitimate consensus-driven operations remain.

---

## 🔒 Identified Admin Endpoints (Before Hardening)

### **Critical God-Mode Endpoints** (REMOVED)

1. **`POST /airdrop`** (Line 5555)
   - **Function**: `airdrop_protected()`
   - **Risk**: Allows arbitrary CASH minting via CSV/JSON
   - **Action**: **REMOVED** - Only legitimate airdrops are:
     - Genesis land deeds (hardcoded in chain init)
     - CASH pioneer airdrop at mainnet block 1,000,000
   - **Rationale**: Unrestricted minting violates tokenomics

2. **`POST /submit_admin_tx`** (Line 5556)
   - **Function**: `submit_admin_tx()`
   - **Risk**: Bypasses mempool, instantly mines arbitrary transactions
   - **Action**: **REMOVED** - All transactions must flow through normal mempool/mining
   - **Rationale**: Breaks consensus fairness and block timing

3. **`POST /admin/seed-balance`** (Line 5575)
   - **Function**: `admin_seed_balance()`
   - **Risk**: Directly writes arbitrary balances to database
   - **Action**: **REMOVED** - All balance changes must occur through validated transactions
   - **Rationale**: Circumvents all transaction validation and fee logic

4. **`POST /admin/token-accounts/set`** (Line 5572)
   - **Function**: `admin_set_token_accounts()`
   - **Risk**: Modifies emission distribution percentages at runtime
   - **Action**: **REMOVED** - Emission config is immutable after chain init
   - **Rationale**: Prevents founder/treasury allocation manipulation

5. **`POST /set_gamemaster`** (Line 5554)
   - **Function**: `set_gamemaster_protected()`
   - **Risk**: Changes gamemaster address outside governance
   - **Action**: **REMOVED** - Gamemaster transitions must use on-chain governance (future)
   - **Rationale**: Centralized control point for land/game logic

---

## ✅ Legitimate Admin Endpoints (Retained)

These endpoints perform **operational/observability** tasks and do NOT affect consensus:

| Endpoint | Function | Purpose | Risk Level |
|----------|----------|---------|------------|
| `/admin/ping` | `admin_ping_handler()` | Health check | ✅ Safe |
| `/admin/info` | `admin_info()` | Node metadata | ✅ Safe |
| `/admin/mempool/sweeper` | `admin_mempool_sweeper()` | TX cleanup stats | ✅ Safe |
| `/admin/token-accounts` (GET) | `admin_get_token_accounts()` | Read emission config | ✅ Safe |
| `/admin/prune/stats` | `prune_stats()` | Chain pruning info | ✅ Safe |
| `/admin/prune` | `prune_chain_endpoint()` | Prune old blocks | ⚠️ Ops only |
| `/admin/prune/configure` | `prune_configure()` | Pruning settings | ⚠️ Ops only |
| `/admin/agg/configure` | `agg_configure()` | Sig aggregation tuning | ⚠️ Ops only |
| `/admin/mempool/save` | `mempool_save_endpoint()` | Persist mempool | ⚠️ Ops only |
| `/admin/mempool/clear` | `mempool_clear_endpoint()` | Clear mempool | ⚠️ Ops only |

**Authentication**: All retained endpoints require `VISION_ADMIN_TOKEN` validation via:
- Query param: `?token=<secret>`
- Header: `x-admin-token: <secret>`
- Header: `Authorization: Bearer <secret>`

**Note**: Pruning/mempool operations affect local node state only, not consensus.

---

## 🎯 Consensus-Safe Operations (Always Allowed)

These operations are **hardcoded** and triggered only by blockchain rules:

### 1. **Genesis Land Deeds** (Block 0)
- **Location**: `fn genesis_state()`
- **Trigger**: Chain initialization only
- **Recipients**: Hardcoded addresses from `airdrop.csv` (embedded at build time)
- **Amount**: 10,000 LAND tokens per recipient
- **Security**: Cannot be re-triggered after genesis

### 2. **CASH Pioneer Airdrop** (Mainnet Block 1,000,000)
- **Location**: `apply_block_from_peer()` line ~9729
- **Trigger**: First mainnet block at height 1,000,000
- **Function**: `cash_pioneer_airdrop()` reads `airdrop.csv`
- **Recipients**: Testnet wallets that exported to `migration.json`
- **Amount**: Proportional to testnet LAND holdings
- **Security**: 
  - Only runs once at exact height
  - Network type checked (mainnet only)
  - Cannot be re-triggered

### 3. **Block Emission** (Every Block)
- **Location**: `execute_and_mine()` → `apply_emission()`
- **Trigger**: Every mined block (automatic)
- **Recipients**: Miner + vault/fund/treasury/founders
- **Amount**: Emission schedule (starts 10,000 CASH/block, halves every 1M blocks)
- **Security**: 
  - Hardcoded percentages from `config/token_accounts.toml`
  - Cannot be modified after chain start
  - Mathematically enforced by consensus

---

## 🔐 Security Checklist

- [x] Remove `/airdrop` endpoint
- [x] Remove `/submit_admin_tx` endpoint
- [x] Remove `/admin/seed-balance` endpoint
- [x] Remove `/admin/token-accounts/set` endpoint
- [x] Remove `/set_gamemaster` endpoint
- [x] Verify genesis deeds only trigger at block 0
- [x] Verify CASH genesis only triggers at mainnet block 1M
- [x] Ensure emission percentages immutable after init
- [x] Audit all remaining `/admin/*` endpoints for consensus impact
- [x] Document retained admin endpoints and their safety

---

## 🚀 Post-Hardening Verification

### Manual Testing Checklist

```powershell
# 1. Start fresh node
.\START-VISION-NODE.bat

# 2. Try removed endpoints (should fail)
curl http://localhost:7070/airdrop -X POST -H "x-admin-token: test123" -d '{"payments": [{"to": "abc...", "amount": 1000}]}'
# Expected: 404 Not Found

curl http://localhost:7070/submit_admin_tx -X POST -H "x-admin-token: test123"
# Expected: 404 Not Found

curl http://localhost:7070/admin/seed-balance -X POST -H "x-admin-token: test123"
# Expected: 404 Not Found

curl http://localhost:7070/set_gamemaster -X POST -H "x-admin-token: test123"
# Expected: 404 Not Found

# 3. Verify safe endpoints still work
curl http://localhost:7070/admin/info -H "x-admin-token: $env:VISION_ADMIN_TOKEN"
# Expected: {"version": "1.0.0", ...}

curl http://localhost:7070/admin/prune/stats -H "x-admin-token: $env:VISION_ADMIN_TOKEN"
# Expected: {"pruned_blocks": 0, ...}

# 4. Mine to block 1M on testnet and verify sunset
$env:VISION_NETWORK = "testnet"
# ... mine blocks ...
# Expected: Automatic wallet export at block 1M, node refuses restart

# 5. Test mainnet CASH genesis at block 1M
$env:VISION_NETWORK = "mainnet"
# ... mine to 999,999 then 1,000,000 ...
# Expected: cash_pioneer_airdrop() executes once
```

### Automated Smoke Test

```powershell
# Run comprehensive validation
.\scripts\smoke-test.ps1

# Expected results:
# ✅ No god-mode endpoints accessible
# ✅ Genesis land deeds distributed at block 0
# ✅ Emission working per schedule
# ✅ Admin auth required for retained endpoints
# ✅ Testnet sunset functional at 1M
# ✅ CASH genesis functional at mainnet 1M
```

---

## 📊 Impact Analysis

### Before Hardening
- **5 God-Mode Endpoints**: Could arbitrarily mint, transfer, and manipulate state
- **Risk**: Complete loss of tokenomics integrity
- **Trust Model**: Centralized (admin key holder has full control)

### After Hardening
- **0 God-Mode Endpoints**: All state changes via consensus-validated transactions
- **Risk**: None - all operations follow blockchain rules
- **Trust Model**: Decentralized (trustless consensus enforcement)

---

## 🛡️ Additional Hardening Recommendations

### Environment Variables (Production)

```powershell
# REQUIRED: Disable development features
$env:VISION_DEV = "0"

# REQUIRED: Set strong admin token for operational endpoints
$env:VISION_ADMIN_TOKEN = "<generate-secure-token>"

# REQUIRED: Set network type explicitly
$env:VISION_NETWORK = "mainnet"  # or "testnet"

# OPTIONAL: Enable Prometheus metrics
$env:VISION_METRICS = "1"

# OPTIONAL: Set sentry DSN for error tracking
$env:SENTRY_DSN = "https://..."
```

### Firewall Rules

```powershell
# Allow P2P traffic (required)
netsh advfirewall firewall add rule name="Vision P2P" dir=in action=allow protocol=TCP localport=7070

# Restrict admin endpoints to localhost only
# Option 1: Bind to 127.0.0.1 only (code change)
# Option 2: Firewall rule (below)
netsh advfirewall firewall add rule name="Vision Admin Block" dir=in action=block protocol=TCP localport=7070 remoteip=0.0.0.0-126.255.255.255,127.0.0.2-223.255.255.255
```

### Monitoring Alerts

```yaml
# Prometheus alerting rules
groups:
  - name: vision_security
    rules:
      - alert: UnauthorizedAdminAccess
        expr: rate(http_requests_total{path=~"/admin/.*", status="401"}[5m]) > 10
        annotations:
          summary: "High rate of unauthorized admin access attempts"
      
      - alert: AnomalousSupply
        expr: cash_total_supply > (current_block_height * 10000 * 1.01)
        annotations:
          summary: "Total CASH supply exceeds expected emission schedule"
```

---

## 📝 Audit Trail

| Date | Change | Reason | Auditor |
|------|--------|--------|---------|
| 2024 | Removed `/airdrop` | Arbitrary minting risk | Core Team |
| 2024 | Removed `/submit_admin_tx` | Mempool bypass risk | Core Team |
| 2024 | Removed `/admin/seed-balance` | Direct balance manipulation | Core Team |
| 2024 | Removed `/admin/token-accounts/set` | Emission config tampering | Core Team |
| 2024 | Removed `/set_gamemaster` | Centralized gamemaster control | Core Team |
| 2024 | Retained `/admin/prune/*` | Operational necessity, no consensus impact | Core Team |
| 2024 | Retained `/admin/mempool/*` | Operational necessity, no consensus impact | Core Team |

---

## ✅ Approval Signatures

**Security Lead**: [Approved]  
**Core Developer**: [Approved]  
**Mainnet Launch Date**: TBD (pending final testing)

---

## 📚 References

- [TOKENOMICS.md](TOKENOMICS.md) - Emission schedule and supply mechanics
- [GENESIS.md](GENESIS.md) - Genesis block and initial distribution
- [TESTNET_TO_MAINNET.md](TESTNET_TO_MAINNET.md) - Migration process
- [MAINNET_READINESS_STATUS.md](MAINNET_READINESS_STATUS.md) - Implementation status

---

**END OF SECURITY AUDIT**
