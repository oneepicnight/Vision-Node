# Foundation Config Unification - Integration Test Plan

## Objective
Verify that the unified foundation config system correctly routes all payments (market settlement, vault inflow, deposits) through a single canonical address source.

## Pre-Test Setup

### 1. Prepare Test Configuration File
Create/update `config/token_accounts.toml`:
```toml
vault_address = "0xtest_vault_123456789abcdef123456789abc1234567"
fund_address = "0xtest_fund_abcdef123456789abcdef123456789abcde"
founder1_address = "0xtest_founder1_fedcba9876543210fedcba9876543210fe"
founder2_address = "0xtest_founder2_0123456789abcdef0123456789abcdef0"
vault_pct = 50
fund_pct = 30
treasury_pct = 20
```

### 2. Start Vision Node
```powershell
cd c:\vision-node
./target/release/vision-node.exe
```

### 3. Verify Startup Logs
Look for:
- ✓ `foundation_config::FOUNDATION_CONFIG loaded from token_accounts.toml`
- ✓ No panic or initialization errors
- ✓ All addresses loaded successfully

## Test Cases

### Test 1: Foundation Config Loads Correctly
**Purpose**: Verify that FOUNDATION_CONFIG initializes on first access

**Steps**:
1. Start node
2. Call any endpoint that triggers settlement (e.g., execute a trade)
3. Check logs for foundation_config initialization

**Expected Result**:
```
[INFO] Loading foundation config from: config/token_accounts.toml
[INFO] Vault address: 0xtest_vault_123456789abcdef123456789abc1234567
[INFO] Fund address: 0xtest_fund_abcdef123456789abcdef123456789abcde
```

---

### Test 2: Market Settlement Routes to Correct Addresses
**Purpose**: Verify that trade settlement uses foundation_config addresses

**Steps**:
1. Execute a test trade: Buy 100 LAND for 500 CASH (quote=CASH)
2. Total fee = 500 (just for this test)
3. Check node logs for settlement routing
4. Query database balances:
   ```powershell
   # Check vault address balance
   curl "http://localhost:7070/account/0xtest_vault_123456789abcdef123456789abc1234567"
   
   # Check fund address balance
   curl "http://localhost:7070/account/0xtest_fund_abcdef123456789abcdef123456789abcde"
   
   # Check founder address balance
   curl "http://localhost:7070/account/0xtest_founder1_fedcba9876543210fedcba9876543210fe"
   ```

**Expected Result**:
- Vault receives: 500 × 0.50 = 250 CASH
- Fund receives: 500 × 0.30 = 150 CASH
- Founder receives: 500 × 0.20 = 100 CASH
- **Logs show**:
  ```
  [INFO] Routing 500 total: vault=250 (50%), ops=150 (30%), founder=100 (20%)
  [INFO] Credited 0xtest_vault_... with 250 (new balance: 250)
  [INFO] Credited 0xtest_fund_... with 150 (new balance: 150)
  [INFO] Credited 0xtest_founder1_... with 100 (new balance: 100)
  ```

---

### Test 3: Vault Ledger Routes to Correct Addresses
**Purpose**: Verify that treasury::vault::route_inflow uses foundation_config addresses

**Steps**:
1. Trigger a vault inflow (e.g., through admin endpoint or market settlement)
2. Check vault ledger logs
3. Verify addresses in ledger match foundation_config

**Expected Result**:
- Ledger events show correct vault/fund/founder1 addresses
- No references to old placeholder addresses (bbbb.../cccc...)

---

### Test 4: Deterministic Deposits Use Correct Base Address
**Purpose**: Verify that HD wallet deposit addresses are derived correctly

**Steps**:
1. Create a new user: `POST /api/user` → returns `user_id=42`
2. Query wallet: `GET /api/wallet/user/42`
3. Check deposit addresses for BTC, BCH, DOGE
4. Verify addresses are deterministic (same user_id → same address)

**Expected Result**:
```json
{
  "btc_deposit": "0x...", // Derived from user_id=42, coin_type=0 (BTC)
  "bch_deposit": "0x...", // Derived from user_id=42, coin_type=145 (BCH)
  "doge_deposit": "0x...", // Derived from user_id=42, coin_type=3 (DOGE)
  "land_balance": "0",
  "cash_balance": "0"
}
```

---

### Test 5: Snapshot Uses Correct Foundation Addresses
**Purpose**: Verify that `/snapshot/current` uses foundation_config addresses

**Steps**:
1. Execute multiple trades to populate vault balances
2. Call: `GET /snapshot/current`
3. Check response addresses

**Expected Result**:
```json
{
  "vault_snapshot": {
    "vault_address": "0xtest_vault_123456789abcdef123456789abc1234567",
    "fund_address": "0xtest_fund_abcdef123456789abcdef123456789abcde",
    "founder1_address": "0xtest_founder1_fedcba9876543210fedcba9876543210fe",
    "vault_balance": "250",
    "fund_balance": "150",
    "founder_balance": "100"
  }
}
```

---

### Test 6: Backward Compatibility
**Purpose**: Verify old const references still work (for gradual migration)

**Steps**:
1. Search codebase for `vision_constants::VAULT_ADDRESS`
2. Verify code still compiles and runs
3. Check that old const values are no longer used (should be replaced by function calls)

**Expected Result**:
- Code compiles without warnings
- Old const values marked as DEPRECATED
- New function calls used everywhere

---

### Test 7: Config Reload on Restart
**Purpose**: Verify that changing config file and restarting loads new addresses

**Steps**:
1. Start node with addresses set A
2. Stop node
3. Update `config/token_accounts.toml` with addresses set B
4. Start node again
5. Execute trade and verify addresses set B is used

**Expected Result**:
- New addresses loaded from updated TOML
- Settlement routes to addresses set B
- Old addresses set A no longer used

---

### Test 8: Graceful Handling of Missing TOML
**Purpose**: Verify node doesn't crash if config file is missing

**Steps**:
1. Delete `config/token_accounts.toml`
2. Delete `TOKEN_ACCOUNTS_TOML_PATH` environment variable
3. Start node
4. Try to execute a trade

**Expected Result**:
- Node starts but logs warning about missing config
- Uses fallback defaults (likely "error_" prefixed strings)
- No panic/crash
- Error messages clear and actionable

---

## Validation Checklist

- [ ] Config loads on startup without errors
- [ ] Settlement routes to vault/fund/founder addresses with 50/30/20 split
- [ ] Vault ledger shows correct addresses for all inflow events
- [ ] Deposits use HD wallet derivation (deterministic per user)
- [ ] Snapshot endpoint returns correct addresses and balances
- [ ] Old const values no longer used in routing logic
- [ ] Code compiles without warnings
- [ ] Node restarts and reloads new addresses from TOML
- [ ] Graceful handling of missing/malformed TOML

## Debug Commands

### Check Address in Database
```powershell
# Query account balance
curl "http://localhost:7070/account/0xtest_vault_123456789abcdef123456789abc1234567"
```

### Check Settlement Receipt
```powershell
# Query receipts for an address
curl "http://localhost:7070/receipts/0xtest_vault_123456789abcdef123456789abc1234567"
```

### Check Vault Ledger
```powershell
# Query vault events (if endpoint exists)
curl "http://localhost:7070/vault/ledger"
```

### Monitor Logs
```powershell
# Filter for foundation_config related logs
Get-Content vision-node.log | Select-String "foundation_config|vault_address|fund_address"
```

## Success Criteria

✅ **ALL of the following must be true**:
1. No panics or compilation errors
2. All addresses route through foundation_config
3. 50/30/20 split applied consistently everywhere
4. Deterministic deposit addresses work correctly
5. Backward compatibility maintained
6. Config file changes take effect on restart
7. Graceful handling of edge cases

## Post-Test Cleanup

1. Restore production addresses in `config/token_accounts.toml`
2. Verify node still operates correctly with real addresses
3. Archive test results and logs
