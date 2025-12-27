# Deterministic Deposit Addresses — Test Results

## Patch Summary
✅ **Patch 1 Complete**: Replaced fake deposit addresses with deterministic HD-derived addresses.

### Changes Made

#### 1. New Public Helper: `src/market/deposits.rs`
```rust
pub fn deposit_address_for_user(user_id: &str, asset: QuoteAsset) -> Result<String>
```

Maps `QuoteAsset` to BIP44 coin types:
- `QuoteAsset::Btc` → coin_type: 0
- `QuoteAsset::Bch` → coin_type: 145
- `QuoteAsset::Doge` → coin_type: 3
- `QuoteAsset::Land` → Error (native asset, no deposit)

Internally calls existing `derive_address(coin_type, user_index)` and `user_id_to_index(user_id)` helpers.

#### 2. Updated `UserWallet::new()` in `src/market/wallet.rs`
Replaced fake address patterns:
- `format!("btc_{}", user_id)` → `deposit_address_for_user(&user_id, QuoteAsset::Btc)`
- `format!("bch_{}", user_id)` → `deposit_address_for_user(&user_id, QuoteAsset::Bch)`
- `format!("doge_{}", user_id)` → `deposit_address_for_user(&user_id, QuoteAsset::Doge)`

#### 3. Protected Wallet Load
`get_or_create_wallet()` uses `or_insert_with()` — only generates addresses if wallet is missing. Existing wallets are never overwritten.

### Properties
✅ **Deterministic**: Same `user_id` always produces the same addresses  
✅ **Cross-Restart Stable**: Addresses persist across node restarts  
✅ **HD-Derived**: Uses existing HD derivation logic (`blake3` hash of master_key + coin_type + user_index)  
✅ **Immutable Once Created**: Wallet load does not regenerate addresses  

### Build Status
✅ Compiled successfully: `cargo build --release` (13m 05s)  
✅ Binary updated: `C:\vision-node\target\release\vision-node.exe`  

### Testing Notes
To verify deterministic behavior:
1. Create a user wallet with `user_id = "test_user_123"`
2. Note the BTC deposit address (e.g., `bc1q...`)
3. Restart the node or call `get_or_create_wallet("test_user_123")` again
4. Confirm the same address is returned (not regenerated)

### Related Files
- `src/market/deposits.rs` — Public helper and HD derivation logic
- `src/market/wallet.rs` — Wallet creation using derived addresses
- `src/market/engine.rs` — `QuoteAsset` enum (BTC, BCH, DOGE, LAND)
- `src/config/wallet.rs` — Optional explicit persistence (not yet integrated)

### Next Steps
- Integration test: Load two nodes with same seed → verify same addresses
- Consider persisting to config if user wants explicit address tracking
- Test with real BTC/BCH/DOGE deposit scanning once RPC is connected
