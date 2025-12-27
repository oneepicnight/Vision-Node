# Miners Vault Addresses - Quick Reference

## What Was Implemented

### ✅ Address Validators
Strict format checking for BTC, BCH, and DOGE to prevent accidental cross-chain transfers.

```rust
// In your code
use crate::market::address_validate::{validate_address, Asset};

// Validate BTC address
validate_address(Asset::BTC, "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")?;

// Validate BCH address  
validate_address(Asset::BCH, "bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p")?;

// Validate DOGE address
validate_address(Asset::DOGE, "D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e")?;
```

### ✅ Configuration Loading
Three environment variables control miners vault addresses:

```bash
# Set before starting node
export VISION_MINERS_BTC_ADDRESS="bc1q..."
export VISION_MINERS_BCH_ADDRESS="bitcoincash:q..."
export VISION_MINERS_DOGE_ADDRESS="D..."

cargo run
```

### ✅ Startup Banner
Node displays miners addresses on startup for transparency:

```
🔐 Miners Vault Deposit Addresses (read-only):
   BTC: bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4
   BCH: bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p
   DOGE: D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e
   ✅ All configured miners addresses are valid
```

### ✅ API Endpoint
Query addresses via HTTP:

```bash
curl http://localhost:7070/api/vault/miners/addresses
```

Response:
```json
{
  "ok": true,
  "purpose": "Read-only display of where mining fees are deposited",
  "addresses": {
    "btc": "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4",
    "bch": "bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p",
    "doge": "D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e"
  }
}
```

## Runtime Integration Points

### In settlement.rs (When fees arrive)
```rust
use crate::market::address_validate::{validate_address, Asset};

// When processing a BTC deposit
if let Err(e) = validate_address(Asset::BTC, &deposit_from_address) {
    tracing::warn!("Invalid BTC address: {}", e);
    // Reject deposit or route to review queue
}
```

### In main.rs (Future sweep logic)
```rust
// When sweeping vault to miners
if let Some(miners_btc_addr) = crate::foundation_config::miners_btc_address() {
    // Use miners_btc_addr to send accumulated fees
}
```

## Address Formats Reference

| Asset | Format | Example |
|-------|--------|---------|
| BTC | Bech32 | `bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4` |
| BTC | P2PKH | `1A1z7agoat5wbwrZCch3Z1PePPjRsrSne9` |
| BTC | P2SH | `3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy` |
| BCH | CashAddr | `bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p` |
| BCH | CashAddr (no prefix) | `qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p` |
| DOGE | P2PKH | `D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e` |
| DOGE | P2SH | `AUrxN1234...` (starts with A) |

## Important Design Constraints

### 1. Addresses Are Read-Only Tonight
- Validators check format only
- Configuration loads at startup
- No spending/withdrawal logic
- VaultStore handles all internal accounting

### 2. Cross-Chain Prevention
```
✗ REJECTED: Sending BTC to BCH address
✗ REJECTED: Sending BCH with wrong prefix
✗ REJECTED: Sending DOGE with D to miners BTC address
✓ ALLOWED: Each asset routed to correctly formatted address
```

### 3. Configuration Priority
1. TOML file: `config/token_accounts.toml`
2. Environment variables: `VISION_MINERS_*_ADDRESS`
3. Default: `[not configured]`

Environment variables override TOML values.

## Files Changed

```
NEW:
  src/market/address_validate.rs - All validators
  MINERS_VAULT_ADDRESSES_IMPLEMENTATION.md - Full docs

MODIFIED:
  src/accounts.rs - Added Option<String> address fields
  src/foundation_config.rs - Getters & env loading
  src/main.rs - Startup banner
  src/api/vault_routes.rs - GET /vault/miners/addresses endpoint
  src/market/mod.rs - Module declaration
```

## Testing

### Compile Check
```bash
cargo check  # 0 errors, 30 warnings (no new)
```

### Build
```bash
cargo build  # ~1 minute
```

### Start Node
```bash
export VISION_MINERS_BTC_ADDRESS="bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
export VISION_MINERS_BCH_ADDRESS="bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p"
export VISION_MINERS_DOGE_ADDRESS="D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e"

cargo run
# Check logs for: "🔐 Miners Vault Deposit Addresses (read-only):"
```

### Test Endpoint
```bash
curl http://localhost:7070/api/vault/miners/addresses | jq .
```

### Verify All Three Addresses Show
```
BTC: bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4
BCH: bitcoincash:qph2v4mkxjgkydg4w2l4r7nrw3xysxxcu659nzs28p
DOGE: D5q6iKgGU91y4x6EdGYvVvGqA9hXXvWy6e
✅ All configured miners addresses are valid
```

---
**Status**: Production Ready ✅
**Tested**: 2025-12-25
**Build**: 0 errors, 30 warnings
