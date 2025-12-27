# Send Endpoint Implementation - COMPLETE ✅

## Overview
Successfully implemented a clean, Axum 0.7 compatible `/wallet/send` endpoint for sending BTC, BCH, and DOGE from Vision wallets to external addresses.

## Key Achievement
**Fixed the Handler Trait Issue:** The problem was `parking_lot::MutexGuard` not being `Send`, which was causing type inference to fail. Solution: Clone the RPC client before any `.await` calls to drop the lock guard immediately.

## Architecture

### Module: `src/send.rs`
Clean HTTP interface that uses existing Phase 2 engines:
- **UtxoManager**: Select and lock UTXOs
- **TransactionBuilder**: Build raw transactions via RPC
- **KeyManager**: Sign transactions (dev-signing feature)
- **External RPC**: Broadcast to blockchain

### Request/Response Types
```rust
#[derive(Deserialize)]
pub struct SendRequest {
    pub user_id: String,
    pub chain: String,      // "btc" | "bch" | "doge"
    pub to_address: String,
    pub amount: String,     // Amount as string (satoshis)
}

#[derive(Serialize)]
pub struct SendResponse {
    pub success: bool,
    pub txid: Option<String>,
    pub status: String,     // "broadcast" | "error"
    pub message: Option<String>,
}
```

## Handler Flow

### 1. HTTP Entry Point
```rust
pub async fn wallet_send_external(
    Json(req): Json<SendRequest>,
) -> impl IntoResponse
```
- Thin wrapper around business logic
- Uses `tokio::spawn` to isolate async types
- Returns appropriate HTTP status codes

### 2. Business Logic (`process_send`)
1. **Validate chain** (BTC/BCH/DOGE only)
2. **Validate address format**
3. **Parse amount** to smallest unit (satoshis)
4. **Check dust threshold** (546 sats for BTC/BCH, 0.01 DOGE)
5. **Estimate fee** (fixed: 1000 sats BTC, 500 BCH, 100k koinus DOGE)
6. **Check user balance** including fee
7. **Reserve balance** (lock funds during transaction)
8. **Build and broadcast** transaction
9. **Finalize** (commit) or **Release** (rollback) on error

### 3. Transaction Building (`build_and_broadcast`)
1. **Sync UTXOs** from blockchain via `listunspent` RPC
2. **Build raw transaction** using `createrawtransaction` RPC
3. **Sign transaction** using `signrawtransactionwithwallet` RPC
4. **Broadcast** using `sendrawtransaction` RPC
5. **Return txid** on success

## Critical Fixes

### Issue: MutexGuard Not Send
**Problem:** `parking_lot::MutexGuard` cannot be held across `.await` points in tasks spawned with `tokio::spawn`.

**Solution:** Clone the RPC client immediately and drop the lock before any awaits:
```rust
let client = {
    let clients = crate::EXTERNAL_RPC_CLIENTS.lock();
    clients.get(chain)?.clone()
}; // Lock dropped here

// Now safe to await
client.call(...).await?;
```

**Applied to:**
- `src/send.rs` - broadcast function
- `src/utxo_manager.rs` - sync_user_utxos function
- `src/tx_builder.rs` - build_send_transaction function

### Issue: StatusCode Type Inference
**Problem:** Rust was inferring `reqwest::StatusCode` instead of `axum::http::StatusCode` when returning from async functions.

**Solution:** Use `tokio::spawn` to isolate the async work, then construct the response with explicit `axum::http::StatusCode` types.

## Balance Management

### UserWallet Structure
Balances are stored as individual fields (not HashMap):
```rust
pub struct UserWallet {
    pub btc_available: f64,
    pub btc_locked: f64,
    pub bch_available: f64,
    pub bch_locked: f64,
    pub doge_available: f64,
    pub doge_locked: f64,
    // ...
}
```

### Reserve/Release/Finalize Pattern
1. **Reserve:** Deduct balance immediately when transaction starts
2. **Release:** Add balance back if transaction fails (rollback)
3. **Finalize:** Confirm deduction on successful broadcast (commit)

This prevents double-spend by locking funds during the transaction.

## Testing

### Test Request
```bash
curl -X POST http://127.0.0.1:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test-user-123",
    "chain": "btc",
    "to_address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
    "amount": "10000"
  }'
```

### Expected Response (Success)
```json
{
  "success": true,
  "txid": "abc123...",
  "status": "broadcast",
  "message": "Transaction broadcast successfully: abc123..."
}
```

### Expected Response (Insufficient Balance)
```json
{
  "success": false,
  "txid": null,
  "status": "error",
  "message": "Insufficient balance. Need 11000 sats (amount: 10000, fee: 1000), have 5000 sats"
}
```

## Security Notes

### dev-signing Feature
The `dev-signing` feature enables server-side transaction signing for development:
- **DO NOT** use in production
- Private keys stored server-side (encrypted with feature flag)
- For production: Use client-side signing with hardware wallets

### RPC Wallet Requirements
- Bitcoin Core wallet must have imported private keys
- Uses `signrawtransactionwithwallet` RPC call
- Wallet must be unlocked for signing

## Future Enhancements

### Dynamic Fee Estimation
Replace fixed fees with `estimatesmartfee` RPC calls:
```rust
let fee_rate = client.call("estimatesmartfee", json!([6])).await?;
```

### UTXO Background Sync
Add periodic task to sync UTXOs for active users:
```rust
tokio::spawn(async {
    loop {
        for user in active_users {
            UtxoManager::sync_user_utxos(&user, chain, addresses).await;
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
});
```

### Transaction History
Store sent transactions in database:
```rust
#[derive(Serialize)]
struct Transaction {
    txid: String,
    user_id: String,
    chain: String,
    amount: u64,
    fee: u64,
    to_address: String,
    status: String, // "pending" | "confirmed" | "failed"
    confirmations: u32,
    timestamp: u64,
}
```

### Multi-Signature Support
Add support for multi-sig wallets:
```rust
pub struct SendRequest {
    // ...existing fields...
    pub multisig_config: Option<MultiSigConfig>,
}
```

## Troubleshooting

### "RPC client not configured"
**Cause:** External RPC not initialized for that chain
**Solution:** Check `config/external_rpc.toml` and ensure RPC clients are configured

### "Transaction signing incomplete"
**Cause:** RPC wallet doesn't have the private key
**Solution:** Import private key into Bitcoin Core wallet with `importprivkey`

### "RPC sendrawtransaction failed"
**Possible causes:**
- Insufficient fee
- Double-spend (inputs already spent)
- Invalid transaction format
- Network connectivity issues

**Solution:** Check RPC logs and blockchain mempool status

## Summary
The `/wallet/send` endpoint is now fully operational with:
- ✅ Clean Axum 0.7 compatible handler
- ✅ Complete transaction building pipeline
- ✅ UTXO management and selection
- ✅ Balance reserve/release/finalize pattern
- ✅ Multi-chain support (BTC/BCH/DOGE)
- ✅ Proper error handling and rollback
- ✅ Thread-safe async operations

**Status:** Production-ready for testnet (with dev-signing feature)
