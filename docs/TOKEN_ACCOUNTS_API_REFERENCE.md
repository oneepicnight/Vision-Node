# Token Accounts API Quick Reference

## Admin Endpoints (Require VISION_ADMIN_TOKEN)

### Get Current Configuration
```http
GET /admin/token-accounts?token={ADMIN_TOKEN}
```

**Response:**
```json
{
  "ok": true,
  "config": {
    "vault_address": "bbbb...",
    "fund_address": "cccc...",
    "founder1_address": "dddd...",
    "founder2_address": "eeee...",
    "vault_pct": 50,
    "fund_pct": 30,
    "treasury_pct": 20,
    "founder1_pct": 50,
    "founder2_pct": 50
  }
}
```

### Update Configuration
```http
POST /admin/token-accounts/set?token={ADMIN_TOKEN}
Content-Type: application/json

{
  "vault_pct": 45,
  "fund_pct": 35,
  "treasury_pct": 20
}
```

**Response:**
```json
{
  "ok": true,
  "message": "config updated (restart node to apply)",
  "config": { ... }
}
```

## Market Endpoints

### Test Sale (Development)
```http
POST /market/test-sale
Content-Type: application/json

{
  "amount": 1000
}
```

**Response:**
```json
{
  "ok": true,
  "total": 1000,
  "vault_amount": 500,
  "fund_amount": 300,
  "founder1_amount": 100,
  "founder2_amount": 100
}
```

## PowerShell Examples

### Get Config
```powershell
$token = $env:VISION_ADMIN_TOKEN
Invoke-RestMethod -Uri "http://127.0.0.1:7070/admin/token-accounts?token=$token"
```

### Update Config
```powershell
$token = $env:VISION_ADMIN_TOKEN
$body = @{
    vault_pct = 45
    fund_pct = 35
    treasury_pct = 20
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:7070/admin/token-accounts/set?token=$token" `
    -Method Post `
    -Body $body `
    -ContentType "application/json"
```

### Test Sale
```powershell
$body = @{ amount = 1000 } | ConvertTo-Json
Invoke-RestMethod -Uri "http://127.0.0.1:7070/market/test-sale" `
    -Method Post `
    -Body $body `
    -ContentType "application/json"
```

## cURL Examples

### Get Config
```bash
curl "http://127.0.0.1:7070/admin/token-accounts?token=YOUR_ADMIN_TOKEN"
```

### Update Config
```bash
curl -X POST "http://127.0.0.1:7070/admin/token-accounts/set?token=YOUR_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"vault_pct": 45, "fund_pct": 35, "treasury_pct": 20}'
```

### Test Sale
```bash
curl -X POST "http://127.0.0.1:7070/market/test-sale" \
  -H "Content-Type: application/json" \
  -d '{"amount": 1000}'
```

## Configuration File Format

**File:** `config/token_accounts.toml`

```toml
# System accounts
vault_address = "your-vault-address-64-chars"
fund_address  = "your-fund-address-64-chars"

# Founders
founder1_address = "donnie-address-64-chars"
founder2_address = "travis-address-64-chars"

# Primary split (must sum to 100)
vault_pct = 50
fund_pct  = 30
treasury_pct = 20

# Treasury sub-split (must sum to 100)
founder1_pct = 50
founder2_pct = 50
```

## Default Split Example

For a sale of **1000 tokens**:
- Vault: 500 (50%)
- Fund: 300 (30%)
- Treasury: 200 (20%)
  - Founder1 (Donnie): 100 (50% of treasury = 10% total)
  - Founder2 (Travis): 100 (50% of treasury = 10% total)

## Error Responses

### Unauthorized
```json
{
  "error": {
    "code": "unauthorized",
    "message": "invalid or missing admin token"
  }
}
```

### Validation Failed
```json
{
  "error": "validation failed: vault_pct + fund_pct + treasury_pct must equal 100"
}
```

### Config Not Found
```json
{
  "error": "failed to load: No such file or directory (os error 2)"
}
```

## Integration Code Example

```rust
use crate::market::settlement::route_proceeds;
use crate::accounts::TokenAccountsCfg;

// After successful market sale
let sale_amount = 1000u128;
let tok_accounts = &state.tok_accounts;
let db = &state.db;

// Route proceeds automatically
route_proceeds(tok_accounts, db, sale_amount)?;

// Balances are now updated:
// - vault: +500
// - fund: +300
// - founder1: +100
// - founder2: +100
```

## Monitoring

Check vault ledger for routing history:
```http
GET /vault/ledger
```

Look for entries like:
```
"market_sale_vault total=1000"
"market_sale_fund total=1000"
"market_sale_founder1 total=1000"
"market_sale_founder2 total=1000"
```
