# Token Accounts Settlement System

## Overview

The Vision node implements automatic routing of market sale proceeds to designated accounts with configurable percentage splits.

## Architecture

### Components

1. **Configuration File**: `config/token_accounts.toml`
   - Defines system addresses (vault, fund, founders)
   - Specifies split percentages

2. **Accounts Module**: `src/accounts.rs`
   - Loads and validates configuration
   - Ensures percentages sum correctly

3. **Settlement Module**: `src/market/settlement.rs`
   - Handles proceeds routing logic
   - Credits accounts based on splits

4. **Admin Endpoints**: Protected by admin token
   - `GET /admin/token-accounts` - View current config
   - `POST /admin/token-accounts/set` - Update config

## Default Split Configuration

```toml
vault_pct = 50        # 50% to staking vault
fund_pct  = 30        # 30% to ecosystem fund
treasury_pct = 20     # 20% to treasury

# Treasury sub-split
founder1_pct = 50     # 10% total (50% of 20%)
founder2_pct = 50     # 10% total (50% of 20%)
```

## Usage

### 1. Configure Addresses

Edit `config/token_accounts.toml`:

```toml
vault_address = "your-vault-address-here"
fund_address  = "your-fund-address-here"
founder1_address = "donnie-address-here"
founder2_address = "travis-address-here"
```

### 2. Restart Node

The configuration is loaded at startup. Restart required after manual config changes.

### 3. Process Sales

When a market sale is processed, `route_proceeds()` is called automatically:

```rust
use crate::market::settlement::route_proceeds;

// After successful sale
let sale_amount = 1000u128;
route_proceeds(&tok_accounts, &db, sale_amount)?;
```

This will:
- Credit vault with 500 (50%)
- Credit fund with 300 (30%)
- Credit founder1 with 100 (10%)
- Credit founder2 with 100 (10%)

### 4. Monitor Distribution

Check the vault ledger to see routing history:

```bash
curl http://127.0.0.1:7070/vault/ledger
```

## Admin API

### Get Current Config

```bash
curl "http://127.0.0.1:7070/admin/token-accounts?token=YOUR_ADMIN_TOKEN"
```

Response:
```json
{
  "ok": true,
  "config": {
    "vault_address": "...",
    "fund_address": "...",
    "founder1_address": "...",
    "founder2_address": "...",
    "vault_pct": 50,
    "fund_pct": 30,
    "treasury_pct": 20,
    "founder1_pct": 50,
    "founder2_pct": 50
  }
}
```

### Update Config

```bash
curl -X POST "http://127.0.0.1:7070/admin/token-accounts/set?token=YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "vault_pct": 45,
    "fund_pct": 35,
    "treasury_pct": 20
  }'
```

**Note**: Node restart required for changes to take effect.

## Testing

Run the test script:

```powershell
# Set admin token
$env:VISION_ADMIN_TOKEN = "your-admin-token"

# Run test
.\test-token-accounts.ps1
```

Or test manually:

```powershell
# Test a sale
Invoke-RestMethod -Uri "http://127.0.0.1:7070/market/test-sale" `
  -Method Post `
  -Body '{"amount": 1000}' `
  -ContentType "application/json"
```

## Validation Rules

The system enforces:

1. **Primary split must sum to 100%**:
   ```
   vault_pct + fund_pct + treasury_pct = 100
   ```

2. **Founder split must sum to 100%**:
   ```
   founder1_pct + founder2_pct = 100
   ```

3. **Addresses must be valid** (at least 12 characters)

## Integration Points

### Market Module

The market routes (`src/market/routes.rs`) call settlement after successful sales:

```rust
// After payment verified and LAND transferred
route_proceeds(&state.tok_accounts, &state.db, sale_price)?;
```

### Balance Updates

Credits are applied via the balance system:

```rust
fn credit_address(db: &Db, address: &str, amount: u128) -> Result<()> {
    // Get existing balance
    let key = format!("balance:{}", address);
    let current = db.get(key.as_bytes())?
        .map(|v| u128::from_be_bytes(...))
        .unwrap_or(0);
    
    // Add amount
    let new_balance = current.saturating_add(amount);
    
    // Store back
    db.insert(key.as_bytes(), &new_balance.to_be_bytes())?;
    Ok(())
}
```

### Vault Ledger

Each routing event is recorded in the vault ledger for audit:

```rust
crate::treasury::vault::route_inflow(
    "CASH",
    amount,
    format!("market_sale_vault total={}", total)
);
```

## Security

- Admin endpoints require `VISION_ADMIN_TOKEN`
- Config validation prevents invalid splits
- Atomic database updates ensure consistency
- Saturating arithmetic prevents overflows

## Troubleshooting

### Config File Not Found

```
Error: No such file or directory (os error 2)
```

**Solution**: Create `config/token_accounts.toml` with default values.

### Validation Failed

```
Error: vault_pct + fund_pct + treasury_pct must equal 100
```

**Solution**: Ensure percentages sum to exactly 100.

### Restart Required

After updating config via admin API, restart the node:

```bash
# Stop node
pkill vision-node

# Start node
./target/release/vision-node
```

## Future Enhancements

- [ ] Hot-reload config without restart
- [ ] Support more than 2 founders
- [ ] Dynamic percentage adjustments based on metrics
- [ ] Multi-currency settlement (LAND, CASH, etc.)
- [ ] Historical settlement analytics

## See Also

- `src/accounts.rs` - Configuration loader
- `src/market/settlement.rs` - Settlement logic
- `src/market/routes.rs` - Market API integration
- `config/token_accounts.toml` - Configuration file
