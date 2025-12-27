# Vision Node: Wallet & Receipts System

## Overview

The Vision Node now includes a complete wallet and receipts tracking system that provides:

- **Balance Management**: Query and track token balances for any address
- **Token Transfers**: Execute peer-to-peer token transfers with optional fees
- **Receipt Tracking**: Maintain an immutable audit trail of all operations
- **REST API**: Clean, well-documented HTTP endpoints

## Architecture

### Module Structure

```
src/
├── wallet.rs      # Balance queries and transfer logic
├── receipts.rs    # Receipt storage and retrieval
└── metrics.rs     # Database context wrapper (DbCtx)
```

### Data Storage (sled trees)

1. **`balances`** - Token balance ledger
   - Key: Address (string/bytes)
   - Value: Balance as u128 (16 bytes, little-endian)

2. **`receipts`** - Transaction receipt log
   - Key: Monotonic timestamp + counter (`{ts_ns:020}-{counter:06}`)
   - Value: bincode-serialized `Receipt` struct

3. **`__fees__`** - Fee accumulation account
   - Special address that collects all transfer fees

## API Reference

### 1. Get Balance

**Endpoint**: `GET /wallet/:addr/balance`

**Description**: Query the token balance for any address.

**Parameters**:
- `addr` (path): The address to query (min 8 characters)

**Response**:
```json
{
  "address": "test_addr_12345678",
  "balance": "1000000"
}
```

**Example (PowerShell)**:
```powershell
$response = Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/alice12345678/balance"
Write-Host "Balance: $($response.balance)"
```

**Example (curl)**:
```bash
curl http://127.0.0.1:7070/wallet/alice12345678/balance
```

---

### 2. Transfer Tokens

**Endpoint**: `POST /wallet/transfer`

**Description**: Transfer tokens from one address to another with optional fee and memo.

**Request Body**:
```json
{
  "from": "sender_address_here",
  "to": "recipient_address_here",
  "amount": "1000",
  "fee": "10",
  "memo": "Payment for services"
}
```

**Fields**:
- `from` (required): Sender address (min 8 chars)
- `to` (required): Recipient address (min 8 chars)
- `amount` (required): Transfer amount as decimal string (must be > 0)
- `fee` (optional): Transaction fee as decimal string (default: 0)
- `memo` (optional): Human-readable note (max recommended: 256 chars)

**Response (Success)**:
```json
{
  "status": "ok",
  "receipt_id": "latest"
}
```

**Response (Error)**:
```json
{
  "status": "rejected",
  "code": 402,
  "error": "insufficient_funds"
}
```

**Example (PowerShell)**:
```powershell
$body = @{
    from = "alice12345678"
    to = "bob987654321"
    amount = "5000"
    fee = "50"
    memo = "Test transfer"
} | ConvertTo-Json

$response = Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/transfer" `
    -Method Post `
    -ContentType "application/json" `
    -Body $body
```

**Example (curl)**:
```bash
curl -X POST http://127.0.0.1:7070/wallet/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "from": "alice12345678",
    "to": "bob987654321",
    "amount": "5000",
    "fee": "50",
    "memo": "Test transfer"
  }'
```

**Error Codes**:
- `400` - Invalid address, amount, or fee format
- `400` - Zero amount or same sender/recipient
- `402` - Insufficient funds
- `500` - Database or internal error

---

### 3. Get Latest Receipts

**Endpoint**: `GET /receipts/latest?limit={n}`

**Description**: Retrieve the most recent transaction receipts (newest first).

**Query Parameters**:
- `limit` (optional): Number of receipts to return (default: 100, max: 500)

**Response**:
```json
[
  {
    "id": "1735689600000000000-000001",
    "ts_ms": 1735689600000,
    "kind": "transfer",
    "from": "alice12345678",
    "to": "bob987654321",
    "amount": "5000",
    "fee": "50",
    "memo": "Test transfer",
    "txid": null,
    "ok": true,
    "note": null
  },
  ...
]
```

**Receipt Fields**:
- `id`: Unique monotonic identifier (timestamp-based)
- `ts_ms`: Unix timestamp in milliseconds
- `kind`: Operation type (`"transfer"`, `"mint"`, `"burn"`, `"market_settle"`, etc.)
- `from`: Source address
- `to`: Destination address
- `amount`: Amount transferred (decimal string)
- `fee`: Fee paid (decimal string)
- `memo`: Optional user note
- `txid`: Optional L1 transaction ID (for cross-chain operations)
- `ok`: Success status (true/false)
- `note`: Optional system note (error details if `ok=false`)

**Example (PowerShell)**:
```powershell
$receipts = Invoke-RestMethod -Uri "http://127.0.0.1:7070/receipts/latest?limit=50"
foreach ($r in $receipts) {
    Write-Host "[$($r.kind)] $($r.from) -> $($r.to): $($r.amount)"
}
```

**Example (curl)**:
```bash
curl "http://127.0.0.1:7070/receipts/latest?limit=50"
```

---

## Receipt System Details

### Monotonic ID Generation

Receipt IDs are designed for time-ordered scanning:
```
{nanosecond_timestamp:020}-{counter:06}
```

Example: `1735689600123456789-000042`

This ensures:
- Chronological ordering in reverse iteration
- Collision resistance (1M operations/second before counter reuse)
- Natural sorting in database

### Receipt Lifecycle

1. **Operation Execution** (e.g., transfer)
2. **Receipt Creation** with auto-generated ID
3. **bincode Serialization** for compact storage
4. **sled Tree Insert** to `receipts` tree
5. **Best-effort Guarantee**: Receipt write failure doesn't fail the operation

### Receipt Kinds

- `"transfer"` - Peer-to-peer token transfer
- `"mint"` - New tokens created (genesis, rewards, etc.)
- `"burn"` - Tokens destroyed
- `"market_settle"` - Market proceeds routing to vault/fund/founders
- `"airdrop"` - Admin token distribution
- *Custom kinds can be added per your use case*

---

## Database Schema

### Balances Tree

```
Key (bytes):  "alice12345678"
Value (16 bytes): [0x00, 0xe8, 0x03, 0x00, 0x00, ...] (u128 LE = 1000)
```

Helper functions:
```rust
fn read_u128_le(tree: &sled::Tree, key: &[u8]) -> anyhow::Result<u128>
fn write_u128_le(tree: &sled::Tree, key: &[u8], v: u128) -> anyhow::Result<()>
```

### Receipts Tree

```
Key (bytes):   "1735689600123456789-000042"
Value (bincode): Receipt struct serialized
```

Iteration order: **Newest first** (using `tree.iter().rev()`)

---

## Integration Guide

### Adding Receipt Logging to Custom Operations

```rust
use crate::receipts::{Receipt, write_receipt};

// After your operation succeeds:
let rec = Receipt {
    id: String::new(),        // Auto-generated
    ts_ms: 0,                 // Auto-set to now
    kind: "custom_op".into(),
    from: source_addr.clone(),
    to: dest_addr.clone(),
    amount: value.to_string(),
    fee: fee_amount.to_string(),
    memo: Some("Custom operation".into()),
    txid: None,
    ok: true,
    note: None,
};

// Best-effort write (don't fail operation if receipt fails)
if let Err(e) = write_receipt(&db, rec) {
    eprintln!("[custom_op] Receipt write failed: {e}");
}
```

### Querying Balances in Code

```rust
use crate::wallet::read_u128_le;

let db = &state.dbctx.db;
let balances = db.open_tree("balances")?;
let balance = read_u128_le(&balances, address.as_bytes()).unwrap_or(0);
```

### Seeding Initial Balances (Testing)

**Option 1: Direct sled write (Rust)**
```rust
use crate::wallet::write_u128_le;

let balances = db.open_tree("balances")?;
write_u128_le(&balances, b"test_addr_123", 1_000_000)?;
```

**Option 2: Use admin airdrop endpoint**
```powershell
curl -X POST http://127.0.0.1:7070/airdrop `
  -H "Authorization: Bearer $VISION_ADMIN_TOKEN" `
  -H "Content-Type: application/json" `
  -d '{"recipient": "test_addr_123", "amount": 1000000}'
```

---

## Testing

### Test Script

Run the included test suite:
```powershell
.\test-wallet-receipts.ps1 -BaseUrl "http://127.0.0.1:7070"
```

The script tests:
1. Balance queries
2. Token transfers (with fees and memos)
3. Receipt retrieval
4. Address validation
5. Zero amount rejection
6. Same sender/recipient rejection

### Manual Testing

1. **Start the node**:
   ```powershell
   cargo run --release
   ```

2. **Seed a test address** (via admin airdrop or direct DB write)

3. **Query balance**:
   ```powershell
   curl http://127.0.0.1:7070/wallet/test_addr_123/balance
   ```

4. **Transfer tokens**:
   ```powershell
   curl -X POST http://127.0.0.1:7070/wallet/transfer `
     -H "Content-Type: application/json" `
     -d '{"from":"test_addr_123","to":"test_addr_456","amount":"500","fee":"5"}'
   ```

5. **View receipts**:
   ```powershell
   curl "http://127.0.0.1:7070/receipts/latest?limit=10"
   ```

---

## Security & Best Practices

### Address Validation

Current validation (placeholder):
- Minimum 8 characters
- Non-empty

**TODO**: Integrate with your address module for cryptographic validation:
```rust
if !address::is_valid(&addr) {
    return api_err(400, "invalid_address");
}
```

### Balance Checks

Transfers enforce:
1. Sufficient balance (`balance >= amount + fee`)
2. Non-zero amount
3. Different sender/recipient
4. Valid u128 amount parsing

### Atomicity

**Current implementation**: Non-transactional (separate read/write operations)

**Future enhancement**: Wrap in sled transaction for true ACID guarantees:
```rust
db.transaction(|txn| {
    let balances = txn.open_tree("balances")?;
    // atomic read-modify-write
    Ok(())
})?;
```

**Note**: For high-concurrency scenarios, consider optimistic locking or compare-and-swap patterns.

### Fee Collection

Fees accumulate in the special address `__fees__`. To retrieve or redistribute:
```rust
let fees = read_u128_le(&balances, b"__fees__")?;
```

---

## Performance Characteristics

### Balance Queries
- **O(1)** sled tree lookup
- ~1-5ms typical latency (SSD)

### Transfers
- **3 writes**: sender, recipient, fees
- ~5-15ms typical latency (non-transactional)

### Receipt Retrieval
- **O(limit)** reverse iteration
- ~10-50ms for 100 receipts (depending on disk I/O)

### Database Size
- **Balance**: 16 bytes/address
- **Receipt**: ~200-500 bytes/receipt (bincode-compressed)

---

## Monitoring & Metrics

### Key Metrics (Future)

Add to `src/metrics.rs`:
```rust
pub transfers_total: IntCounter,
pub transfer_failures: IntCounter,
pub receipts_written: IntCounter,
pub avg_transfer_amount: Gauge,
```

### Logging

Transfers log at `info` level (optional):
```rust
tracing::info!(from = %req.from, to = %req.to, amount = %amount, "Transfer executed");
```

---

## Troubleshooting

### Error: "insufficient_funds"
**Cause**: Sender balance < (amount + fee)

**Solution**: Seed balance via airdrop or verify amount/fee calculations

### Error: "invalid_address"
**Cause**: Address < 8 characters or invalid format

**Solution**: Use proper address format (check your address encoding module)

### Error: "db_open: ..."
**Cause**: sled database corruption or permission issues

**Solution**: Check file permissions, verify `VISION_DATA_DIR` env var

### Receipts not appearing
**Cause**: Receipt write failed (non-critical, operation still succeeded)

**Solution**: Check logs for write errors, verify sled tree integrity

---

## Roadmap

### Planned Enhancements

1. **Transaction IDs**: Link receipts to on-chain transaction hashes
2. **Batch Transfers**: Single API call for multiple recipients
3. **Receipt Pagination**: Cursor-based pagination for large result sets
4. **Balance History**: Time-series balance tracking
5. **Fee Market**: Dynamic fee calculation based on mempool congestion
6. **Multi-sig Transfers**: Require multiple signatures for high-value transfers
7. **Receipt Filtering**: Query by address, kind, time range

### Compatibility

- **Rust Version**: 1.70+ (edition 2021)
- **axum**: 0.7.x
- **sled**: 0.34.x
- **bincode**: 1.3.x

---

## FAQ

**Q: Are transfers atomic?**  
A: Current implementation uses separate reads/writes. For production, wrap in `db.transaction()` for ACID guarantees.

**Q: Can I delete receipts?**  
A: Not via API (immutable audit trail). Manually prune old receipts via sled tree operations if needed.

**Q: What happens if receipt write fails?**  
A: Transfer succeeds anyway (best-effort logging). Check logs for errors.

**Q: How do I backup balances?**  
A: Use sled's snapshot feature or copy the `balances` tree to a backup DB.

**Q: Can I use non-string addresses?**  
A: Yes, addresses are stored as bytes. Adapt serialization in helper functions.

---

## References

- **API Error Schema**: See `api_error_schema.md` for uniform error responses
- **Token Accounts**: See `TOKEN_ACCOUNTS_SETTLEMENT.md` for market proceeds routing
- **Prometheus Metrics**: See `PROMETHEUS_METRICS.md` for monitoring integration

---

**Last Updated**: 2025-01-31  
**Version**: 1.0.0
