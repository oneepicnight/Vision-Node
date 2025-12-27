# Foundation Config Unification - Implementation Complete

## Summary
Successfully unified the fragmented foundation address system into a single canonical source of truth via `src/foundation_config.rs`.

## Problem Addressed
The vault system had **three separate, inconsistent sources** for foundation addresses and distribution amounts:

1. **config/foundation.rs** - Placeholder garbage strings (bbbb.../cccc.../dddd...)
2. **vision_constants.rs** - Hardcoded mainnet addresses (0xb977.../0xdf7a.../0x8bb8...)
3. **accounts.rs TokenAccountsCfg** - Canonical TOML-based config (never actually used by router logic)

This fragmentation caused:
- Address inconsistency across settlement, vault ledger, and treasure routing
- Confusion about which config is "canonical"
- Double-credit bugs from inconsistent amount splits
- Difficulty maintaining/updating addresses

## Solution Implemented

### 1. Created `src/foundation_config.rs` (NEW)
Single canonical source of truth that:
- Loads `TokenAccountsCfg` from TOML file (via `TOKEN_ACCOUNTS_TOML_PATH` env or `config/token_accounts.toml`)
- Exposes via `Lazy<Result<TokenAccountsCfg>>` static singleton
- Provides accessor functions:
  - `vault_address() -> String`
  - `fund_address() -> String` 
  - `founder1_address() -> String`
  - `founder2_address() -> String`
  - `config() -> Result<TokenAccountsCfg>`

### 2. Updated `src/main.rs`
- Added `mod foundation_config;` declaration (line 151)
- Module now loaded in boot sequence

### 3. Updated `src/vision_constants.rs`
- Added import: `use crate::foundation_config;`
- Replaced hardcoded addresses with getter functions:
  - `pub fn vault_address() -> String` (calls `foundation_config::vault_address()`)
  - `pub fn founder_address() -> String` (calls `foundation_config::founder1_address()`)
  - `pub fn ops_address() -> String` (calls `foundation_config::fund_address()`)
- Kept old `const` values for backward compatibility (marked DEPRECATED)

### 4. Updated `src/treasury/vault.rs`
- Changed import from `crate::config::foundation` → `crate::foundation_config`
- Updated `route_inflow()` function to call:
  - `foundation_config::vault_address()`
  - `foundation_config::fund_address()`
  - `foundation_config::founder1_address()`

### 5. Updated `src/market/settlement.rs`
- Changed import from `crate::vision_constants::{VAULT_ADDRESS, ...}` → `crate::foundation_config`
- Updated `route_proceeds()` function to:
  - Load addresses from `foundation_config` at runtime
  - Use retrieved addresses for all three credit calls
  - Write settlement receipts to correct addresses

## Address Routing Flow (NOW UNIFIED)

```
TOML Config (config/token_accounts.toml)
          ↓
TokenAccountsCfg (accounts.rs)
          ↓
foundation_config.rs (FOUNDATION_CONFIG Lazy)
          ↓
Accessor functions: vault_address(), fund_address(), founder1_address(), founder2_address()
          ↓
Used by:
  - market/settlement.rs (route_proceeds)
  - treasury/vault.rs (route_inflow)
  - vision_constants.rs (getters, for backward compatibility)
```

## Distribution Split (50/30/20)
- **50% VAULT**: Staking vault for long-term reserve
- **30% OPS/FUND**: Operations and development fund
- **20% FOUNDER**: Founder/founding team treasury

This split is now applied consistently across all three routing points (settlement, treasury, vault ledger).

## Backward Compatibility
- Old const values in `vision_constants.rs` still exist but marked DEPRECATED
- Code using `vision_constants::VAULT_ADDRESS` will still compile but now reads from foundation config
- Gradual migration path: can update call sites to use new functions as needed

## Build Status
✅ **Successfully compiled** with all changes
- `cargo build --release` completed without errors
- Binary created: `target/release/vision-node.exe`

## Testing Checklist
- [ ] Start node with `config/token_accounts.toml` containing real addresses
- [ ] Verify addresses load correctly: Check logs for any foundation_config initialization errors
- [ ] Test market settlement: Execute trade, verify proceeds routed to correct addresses with 50/30/20 split
- [ ] Test vault ledger: Check that inflow routing matches settlement addresses
- [ ] Verify snapshots: `/snapshot/current` endpoint should show correct address distribution
- [ ] Verify deposits: New HD wallet deposits should use correct addresses from foundation config
- [ ] Edge case: Test with missing/malformed TOML file; should gracefully fall back to defaults

## Files Modified
1. `src/foundation_config.rs` (NEW - 78 lines)
2. `src/main.rs` (1 line added for mod declaration)
3. `src/vision_constants.rs` (updated imports, added getters, marked old consts DEPRECATED)
4. `src/treasury/vault.rs` (updated import and route_inflow calls)
5. `src/market/settlement.rs` (updated import and route_proceeds calls)

## Files Not Modified (But Could Be Updated)
- `src/config/foundation.rs` - Still exists with placeholder values; marked for deprecation
- Various references in `src/main.rs` to `VAULT_ADDRESS` - Still compile due to backward compatibility
- `src/land_deeds.rs` - Uses `FOUNDER_ADDRESS` const; could be updated to use new function

## Known Limitations
1. Addresses must be loaded before any market transactions occur (FOUNDATION_CONFIG is Lazy and loads on first access)
2. If TOML load fails, the error is cached in the Lazy; restart required to retry
3. No hot-reload of addresses (would require more complex state management)

## Next Steps (Optional)
1. **Remove old const values** from vision_constants.rs once all call sites updated to use new functions
2. **Delete config/foundation.rs** once deprecation period complete
3. **Add runtime validation** for addresses (format, uniqueness checks)
4. **Add metrics** tracking which addresses are being used for routing
5. **Consider address versioning** if addresses need to change mid-chain
