# ATOMIC SWAP HARDENING - PRODUCTION READY

## Overview

This document describes the three critical hardening features implemented for Vision Node's atomic swap system:

1. **Confirmation Depth Enforcement** - Per-coin finality profiles
2. **Swap Timeouts + Refund Paths** - Trustless recovery mechanism
3. **Watch-Only Mode Indicators** - Honest UX with zero ambiguity

All three features work together to create a production-grade non-custodial exchange infrastructure that **fails safely**.

---

## 🔐 Feature 1: Confirmation Depth Enforcement (Per Coin)

### Design

Each coin has a different finality profile. Vision enforces chain-specific confirmation thresholds before allowing swap state progression.

### Default Confirmation Requirements

| Coin | Confirmations | Approximate Time | Rationale |
|------|---------------|------------------|-----------|
| **BTC** | 3 | ~30 minutes | Deep reorgs rare after 3 blocks |
| **BCH** | 6 | ~60 minutes | Lower hashrate requires more blocks |
| **DOGE** | 12 | ~12 minutes | Fast blocks compensate for lower security |

### Environment Variable Overrides

Optional per-coin configuration:

```bash
export VISION_BTC_CONFIRMATIONS=6   # Custom BTC requirement
export VISION_BCH_CONFIRMATIONS=10  # Custom BCH requirement
export VISION_DOGE_CONFIRMATIONS=20 # Custom DOGE requirement
```

### Implementation

**Module**: `src/swap/confirmations.rs`

**Key Functions**:
- `required_confirmations(coin: &str) -> u32` - Get threshold for coin
- `confirmations_met(coin: &str, observed: u32) -> bool` - Check if requirement met
- `confirmation_status_message(coin: &str, observed: u32) -> String` - Human-readable status

**HTLC Integration**:

The `Htlc` struct now tracks confirmations:

```rust
struct Htlc {
    htlc_id: String,
    coin: String,              // Coin type (BTC/BCH/DOGE)
    confirmations: u32,        // Current confirmations count
    initiated_height: u64,     // Block height when swap created
    refund_height: u64,        // Block height when refund unlocks
    // ... other fields
}
```

**Enforcement Points**:

1. **Swap Claim** (`claim_htlc`):
   ```rust
   swap::verify_confirmations(&htlc.coin, htlc.confirmations)
       .map_err(|e| format!("{}", e))?;
   ```
   
   - Blocks claim if confirmations < required
   - Logs: `[SWAP] Claim blocked: Insufficient confirmations for BTC: 2/3 (waiting)`

2. **User Visibility**:
   - API returns confirmation status with each HTLC query
   - Status messages: `⏳ 2/3 confirmations (waiting)` or `✓ 3/3 confirmations (ready)`

### Testing

```bash
# Create swap (defaults to BTC)
curl -X POST http://localhost:7070/htlc/create \
  -H "Content-Type: application/json" \
  -d '{
    "sender": "alice",
    "recipient": "bob",
    "amount": 100000000,
    "coin": "BTC",
    "hash_lock": "abc123...",
    "time_lock_seconds": 86400
  }'

# Response includes confirmation requirements:
# {
#   "ok": true,
#   "htlc_id": "htlc_...",
#   "coin": "BTC",
#   "required_confirmations": 3,
#   "initiated_height": 12345,
#   "refund_height": 12489,  // +144 blocks for BTC
#   "timeout_blocks": 144
# }

# Try to claim before confirmations met (will fail)
curl -X POST http://localhost:7070/htlc/htlc_abc123/claim \
  -H "Content-Type: application/json" \
  -d '{"preimage": "secret"}'

# Response:
# {
#   "ok": false,
#   "error": "Insufficient confirmations for BTC: 2/3 (waiting)"
# }
```

---

## ⏳ Feature 2: Swap Timeout + Refund Paths (Non-Custodial Safety Net)

### Design

If the counterparty disappears, funds are **recoverable by design**, not by admin intervention. Uses height-based HTLC timeouts with coin-specific durations.

### Default Timeout Periods

| Coin | Timeout Blocks | Approximate Time | Calculation |
|------|----------------|------------------|-------------|
| **BTC** | 144 | ~24 hours | 144 blocks × 10 min/block |
| **BCH** | 72 | ~12 hours | 72 blocks × 10 min/block |
| **DOGE** | 720 | ~12 hours | 720 blocks × 1 min/block |

### Implementation

**Module**: `src/swap/timeouts.rs`

**Key Functions**:
- `swap_timeout_blocks(coin: &str) -> u64` - Get timeout duration for coin
- `calculate_refund_height(coin, current_height) -> u64` - Calculate when refund unlocks
- `can_refund(current_height, refund_height, completed) -> bool` - Check refund eligibility
- `RefundStatus` struct - Comprehensive refund status tracking

**HTLC Refund Tracking**:

The `Htlc` struct tracks refund eligibility:

```rust
struct Htlc {
    initiated_height: u64,     // Block height when swap was created
    refund_height: u64,        // Block height when refund becomes available
    coin: String,              // Determines timeout duration
    status: HTLCStatus,        // Pending/Claimed/Refunded/Expired
    // ... other fields
}
```

**Refund Flow**:

1. **Swap Creation**: `refund_height = current_height + timeout_blocks(coin)`
2. **During Timeout**: Refund attempts return `SwapNotExpired` error
3. **After Timeout**: Refund unlocks automatically
4. **Refund Execution**: Returns funds to original sender

**Enforcement**:

```rust
// In refund_htlc():
swap::verify_refund_allowed(current_height, htlc.refund_height, completed)
    .map_err(|e| format!("{}", e))?;
```

**Logging**:

```
[SWAP] Refund unlocked: swap_id=htlc_abc123 coin=BTC height=12489 (expired 5 blocks ago)
```

### Refund Status API

Query refund eligibility:

```bash
curl http://localhost:7070/htlc/htlc_abc123
```

Response includes refund status:

```json
{
  "htlc_id": "htlc_abc123",
  "coin": "BTC",
  "initiated_height": 12345,
  "refund_height": 12489,
  "current_height": 12400,
  "blocks_remaining": 89,
  "time_remaining_seconds": 53400,
  "can_refund": false,
  "refund_status": "⏳ Refund in 89 blocks (~14h 50m)"
}
```

After timeout:

```json
{
  "can_refund": true,
  "blocks_remaining": -5,
  "refund_status": "✓ Refund available (expired 5 blocks ago)"
}
```

### Testing

```bash
# Create swap with short timeout (for testing)
curl -X POST http://localhost:7070/htlc/create \
  -H "Content-Type: application/json" \
  -d '{
    "sender": "alice",
    "coin": "BTC",
    "time_lock_seconds": 600
  }'

# Try to refund before timeout (will fail)
curl -X POST http://localhost:7070/htlc/htlc_abc123/refund

# Response:
# {
#   "ok": false,
#   "error": "Swap has not expired yet: 89 blocks remaining"
# }

# Wait for timeout...

# Refund now succeeds
curl -X POST http://localhost:7070/htlc/htlc_abc123/refund

# Response:
# {
#   "ok": true,
#   "refunded_amount": 100000000,
#   "refunded_to": "alice"
# }
```

### Safety Properties

✅ **No admin intervention required** - Timeouts are deterministic  
✅ **Never locks funds permanently** - Always refundable after timeout  
✅ **Never sweeps to vault** - Refunds go back to original sender  
✅ **No multisig required** - User controls their own refund  
✅ **Network-enforced** - Height-based logic cannot be bypassed  

---

## 👀 Feature 3: Watch-Only Mode Indicators (UX Truthfulness)

### Design

Make it **impossible** for users to misunderstand their node's role. A watch-only node can observe but cannot sign.

### Watch-Only Detection

A node is watch-only if:
- `data/external_master_seed.bin` does not exist
- No private keys available for signing

### Implementation

**Module**: `src/swap/watch_only.rs`

**Key Functions**:
- `is_watch_only() -> bool` - Check if seed file exists
- `can_sign() -> bool` - Inverse of is_watch_only
- `require_signing_capability(operation) -> Result<(), WatchOnlyError>` - Guard function
- `WalletMode` enum - Full or WatchOnly
- `WalletModeStatus` struct - Comprehensive status

### Capability Matrix

| Capability | Watch-Only | Full |
|-----------|------------|------|
| **Balance Viewing** | ✅ Yes | ✅ Yes |
| **Swap Monitoring** | ✅ Yes | ✅ Yes |
| **Confirmation Tracking** | ✅ Yes | ✅ Yes |
| **Swap Initiation** | ❌ No | ✅ Yes |
| **Swap Claim** | ❌ No | ✅ Yes |
| **Refund Signing** | ❌ No | ✅ Yes |
| **Key Export** | ❌ No | ✅ Yes |

### API Endpoints

#### GET /api/status

Now includes wallet_mode fields:

```bash
curl http://localhost:7070/api/status
```

Response:

```json
{
  "height": 12345,
  "peers": [...],
  "wallet_mode": "watch-only",
  "can_sign": false,
  ...
}
```

#### GET /api/wallet/mode

Dedicated endpoint for wallet mode:

```bash
curl http://localhost:7070/api/wallet/mode
```

Response for watch-only node:

```json
{
  "mode": "watch-only",
  "can_sign": false,
  "message": "This node can observe swaps but cannot sign transactions. Import a seed to enable signing.",
  "capabilities": {
    "swap_initiation": false,
    "refund_signing": false,
    "key_export": false,
    "balance_viewing": true,
    "swap_monitoring": true,
    "confirmation_tracking": true
  },
  "instructions": "To enable signing: POST /api/wallet/external/import with a valid seed_hex"
}
```

Response for full node:

```json
{
  "mode": "full",
  "can_sign": true,
  "message": "This node has full signing capability.",
  "capabilities": {
    "swap_initiation": true,
    "refund_signing": true,
    "key_export": true,
    "balance_viewing": true,
    "swap_monitoring": true,
    "confirmation_tracking": true
  },
  "instructions": "This node has full signing capability."
}
```

### Enforcement

**Signing operations fail fast with clear errors**:

```bash
# Watch-only node tries to initiate swap
curl -X POST http://localhost:7070/htlc/create \
  -H "Content-Type: application/json" \
  -d '{"sender": "alice", ...}'

# Response:
# {
#   "ok": false,
#   "error": "WATCH_ONLY_NODE: swap initiation disabled - import seed to enable signing"
# }
```

**Error Message Format**:
```
WATCH_ONLY_NODE: <operation> disabled - import seed to enable signing
```

### Testing

```bash
# 1. Check current mode
curl http://localhost:7070/api/wallet/mode | jq .

# 2. Test with full node (seed exists)
# - All capabilities enabled
# - Swap initiation works
# - Export works

# 3. Delete seed file to simulate watch-only
rm data/external_master_seed.bin

# 4. Restart node

# 5. Verify watch-only mode
curl http://localhost:7070/api/wallet/mode | jq .
# Should show: "mode": "watch-only", "can_sign": false

# 6. Try to initiate swap (should fail)
curl -X POST http://localhost:7070/htlc/create \
  -d '{"sender": "alice", ...}'
# Should return: "error": "WATCH_ONLY_NODE: swap initiation disabled"

# 7. Verify monitoring still works
curl http://localhost:7070/htlc/htlc_abc123  # Should succeed
curl http://localhost:7070/api/status        # Should succeed

# 8. Import seed to restore full mode
curl -X POST http://localhost:7070/api/wallet/external/import \
  -H "Content-Type: application/json" \
  -d '{"seed_hex": "abc123..."}'

# 9. Verify full mode restored
curl http://localhost:7070/api/wallet/mode | jq .
# Should show: "mode": "full", "can_sign": true
```

---

## 🧪 Complete Test Matrix (10-minute verification)

### Test 1: Create Wallet → Verify Real Addresses

```bash
# Generate new seed (if needed)
curl -X POST http://localhost:7070/api/wallet/external/import \
  -d '{"seed_hex": "0000000000000000000000000000000000000000000000000000000000000001"}'

# Get BTC deposit address
curl "http://localhost:7070/api/market/exchange/deposit?user=testuser&asset=BTC"
# Should return: bc1... (Bech32 P2WPKH)

# Get BCH deposit address  
curl "http://localhost:7070/api/market/exchange/deposit?user=testuser&asset=BCH"
# Should return: bitcoincash:q... (CashAddr P2PKH)

# Get DOGE deposit address
curl "http://localhost:7070/api/market/exchange/deposit?user=testuser&asset=DOGE"
# Should return: D... (Base58Check P2PKH)
```

### Test 2: Start Swap → Observe Confirmation Counter

```bash
# Create HTLC swap
SWAP_ID=$(curl -s -X POST http://localhost:7070/htlc/create \
  -H "Content-Type: application/json" \
  -d '{
    "sender": "alice",
    "recipient": "bob",
    "amount": 100000000,
    "coin": "BTC",
    "hash_lock": "abc123",
    "time_lock_seconds": 86400
  }' | jq -r .htlc_id)

echo "Created swap: $SWAP_ID"

# Check status (confirmations = 0 initially)
curl http://localhost:7070/htlc/$SWAP_ID | jq .

# Observe confirmation field:
# {
#   "confirmations": 0,
#   "required_confirmations": 3,
#   "confirmation_status": "⏳ 0/3 confirmations (waiting)"
# }

# External chain scanner would update confirmations field
# (In production, this happens automatically as blocks arrive)
```

### Test 3: Try Early Redeem → Blocked

```bash
# Try to claim before confirmations met
curl -X POST http://localhost:7070/htlc/$SWAP_ID/claim \
  -H "Content-Type: application/json" \
  -d '{"preimage": "secret"}'

# Should fail with:
# {
#   "ok": false,
#   "error": "Insufficient confirmations for BTC: 0/3 (waiting)"
# }
```

### Test 4: Hit Confirmation Threshold → Allowed

```bash
# Simulate confirmations being updated (production does this automatically)
# Update HTLC confirmations field to 3

# Now claim succeeds
curl -X POST http://localhost:7070/htlc/$SWAP_ID/claim \
  -H "Content-Type: application/json" \
  -d '{"preimage": "secret"}'

# Should succeed:
# {
#   "ok": true,
#   "claimed_amount": 100000000
# }
```

### Test 5: Let Swap Timeout → Refund Unlocks

```bash
# Create swap with short timeout
SWAP_ID=$(curl -s -X POST http://localhost:7070/htlc/create \
  -d '{
    "sender": "alice",
    "coin": "BTC",
    "time_lock_seconds": 60
  }' | jq -r .htlc_id)

# Check refund status
curl http://localhost:7070/htlc/$SWAP_ID | jq .refund_status
# Shows: "⏳ Refund in 144 blocks (~24h 0m)"

# Try to refund before timeout
curl -X POST http://localhost:7070/htlc/$SWAP_ID/refund
# Should fail: "Swap has not expired yet"

# Wait for timeout (or simulate height increase)
# ... wait 144 blocks ...

# Now refund succeeds
curl -X POST http://localhost:7070/htlc/$SWAP_ID/refund
# Should succeed:
# {
#   "ok": true,
#   "refunded_amount": 100000000,
#   "refunded_to": "alice"
# }
```

### Test 6: Delete Seed File → Node Enters Watch-Only

```bash
# Export seed for backup
SEED=$(curl -s http://localhost:7070/api/wallet/external/export | jq -r .seed_hex)
echo "Backed up seed: $SEED"

# Delete seed file
rm data/external_master_seed.bin

# Restart node (or wait for next operation)

# Check wallet mode
curl http://localhost:7070/api/wallet/mode | jq .
# Should show: "mode": "watch-only"

# Verify /api/status also shows watch-only
curl http://localhost:7070/api/status | jq '{wallet_mode, can_sign}'
# Should show: {"wallet_mode": "watch-only", "can_sign": false}
```

### Test 7: UI/API Reflects Watch-Only Mode

```bash
# Try to initiate swap (should fail)
curl -X POST http://localhost:7070/htlc/create \
  -d '{"sender": "alice", ...}'
# Response: "WATCH_ONLY_NODE: swap initiation disabled"

# Try to export seed (should fail)
curl http://localhost:7070/api/wallet/external/export
# Response: "WATCH_ONLY_NODE: key export disabled"

# Try to claim swap (should fail)
curl -X POST http://localhost:7070/htlc/htlc_abc123/claim \
  -d '{"preimage": "secret"}'
# Response: "WATCH_ONLY_NODE: swap claim disabled"
```

### Test 8: Swap Signing Blocked

```bash
# All signing operations return clear errors
curl -X POST http://localhost:7070/htlc/create -d '{...}'
# "WATCH_ONLY_NODE: swap initiation disabled"

curl -X POST http://localhost:7070/htlc/abc123/refund
# "WATCH_ONLY_NODE: swap refund disabled"
```

### Test 9: Monitoring Still Works

```bash
# Balance viewing works
curl http://localhost:7070/api/vault | jq .
# Returns current balances

# Swap monitoring works
curl http://localhost:7070/htlc/htlc_abc123 | jq .
# Returns swap details

# Confirmation tracking works
curl http://localhost:7070/htlc/htlc_abc123 | jq .confirmations
# Returns current confirmation count

# Status endpoint works
curl http://localhost:7070/api/status | jq .
# Returns full node status
```

### Test 10: Vault Balances Unchanged

```bash
# Check vault before swap operations
VAULT_BEFORE=$(curl -s http://localhost:7070/api/vault | jq .total_land)

# Perform swap operations (create, timeout, refund)

# Check vault after
VAULT_AFTER=$(curl -s http://localhost:7070/api/vault | jq .total_land)

# Verify vault unchanged (no sweeping occurred)
test "$VAULT_BEFORE" = "$VAULT_AFTER" && echo "✅ Vault unchanged"
```

---

## ✅ Result After These 3 Additions

### What You Now Have

🔐 **Finality Enforcement Per Chain**
- BTC: 3 confirmations (~30 minutes)
- BCH: 6 confirmations (~60 minutes)
- DOGE: 12 confirmations (~12 minutes)
- Configurable via environment variables
- Clear error messages when insufficient

⏳ **Trustless Recovery on Failure**
- Height-based HTLC timeouts
- Automatic refund unlock after timeout
- No admin intervention required
- No vault sweeping
- Original sender always gets refund

👀 **Honest UX with Zero Ambiguity**
- Watch-only mode clearly indicated in `/api/status`
- Dedicated `/api/wallet/mode` endpoint
- Signing operations fail fast with clear errors
- Monitoring always available
- Import seed to restore full capability

🚫 **No Admin Keys, No Custody Escape Hatches**
- User funds stay in user addresses
- Refunds go back to sender, not vault
- Multisig vault only for fees
- No sweeping mechanism
- Deterministic timeouts

🧱 **A System That Fails Safely**
- Confirmations prevent reorg attacks
- Timeouts prevent permanent lockup
- Watch-only prevents unauthorized signing
- Clear error messages guide users
- Recovery paths always available

---

## Production Deployment Checklist

### Before Launch

- [ ] Generate production multisig keypairs (offline)
- [ ] Set `VISION_MINERS_MULTISIG_PUBKEYS` environment variable
- [ ] Verify multisig addresses match expected values
- [ ] Document key custody procedures
- [ ] Test confirmation depth on testnet
- [ ] Test timeout/refund flow on testnet
- [ ] Test watch-only mode on testnet

### Optional Configuration

- [ ] Set custom confirmation requirements:
  ```bash
  export VISION_BTC_CONFIRMATIONS=6
  export VISION_BCH_CONFIRMATIONS=10
  export VISION_DOGE_CONFIRMATIONS=20
  ```

### Monitoring

- [ ] Monitor `/api/status` for `wallet_mode` field
- [ ] Alert if node enters watch-only unexpectedly
- [ ] Monitor HTLC timeout expiry rates
- [ ] Track confirmation depth compliance
- [ ] Monitor refund execution success rate

### User Documentation

- [ ] Explain confirmation requirements per coin
- [ ] Document timeout periods per coin
- [ ] Provide seed backup instructions
- [ ] Explain watch-only mode limitations
- [ ] Create recovery procedures guide

---

## Architecture Comparison

### Before Hardening

❌ No confirmation depth checks  
❌ Time-based timeouts (clock drift risk)  
❌ No watch-only mode detection  
❌ Unclear error messages  
❌ No refund status tracking  

### After Hardening

✅ Chain-specific confirmation enforcement  
✅ Height-based timeouts (consensus-verified)  
✅ Watch-only mode clearly indicated  
✅ Detailed error messages with guidance  
✅ Comprehensive refund status API  
✅ Fail-safe by design  

---

## File Structure

```
src/swap/
├── mod.rs                 # Module exports and SwapError enum
├── confirmations.rs       # Confirmation depth enforcement (151 lines)
├── timeouts.rs           # Timeout/refund logic (169 lines)
└── watch_only.rs         # Watch-only mode detection (174 lines)

src/main.rs
├── struct Htlc           # Extended with: coin, initiated_height, refund_height, confirmations
├── fn create_htlc()      # Added: watch-only check, coin parameter, height tracking
├── fn claim_htlc()       # Added: confirmation verification, watch-only check
└── fn refund_htlc()      # Added: height-based timeout check, watch-only check

src/api/vault_routes.rs
└── GET /api/wallet/mode  # New endpoint: Wallet mode status

(StatusView in main.rs)
├── wallet_mode: String   # "full" or "watch-only"
└── can_sign: bool        # Signing capability flag
```

---

## This Is No Longer "A Good Crypto Idea"

This is **production-grade non-custodial exchange infrastructure** with:

- **Finality enforcement** that respects chain security profiles
- **Trustless recovery** that works without admin intervention
- **Honest UX** that prevents user confusion
- **Safe failure modes** at every layer
- **Zero custody** except for fee collection
- **Clear documentation** for operators and users

The system now **fails safely** by design, not by accident.
