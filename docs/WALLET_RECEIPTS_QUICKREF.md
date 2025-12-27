# Wallet & Receipts API - Quick Reference

## Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/wallet/:addr/balance` | Query token balance |
| POST | `/wallet/transfer` | Transfer tokens between addresses |
| GET | `/receipts/latest?limit=N` | Get recent transaction receipts |

---

## Get Balance

```bash
curl http://127.0.0.1:7070/wallet/alice12345678/balance
```

Response:
```json
{"address": "alice12345678", "balance": "1000000"}
```

---

## Transfer Tokens

⚠️ **SECURITY**: All transfers require Ed25519 signature verification (mainnet-ready).

```bash
# Generate signature with your client library
# Message = from(32B) || to(32B) || amount(16B LE) || fee(16B LE) || nonce(8B LE) || memo(UTF-8)

curl -X POST http://127.0.0.1:7070/wallet/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "from": "alice12345678901234567890123456789012345678901234567890123456",
    "to": "bob98765432109876543210987654321098765432109876543210987654321",
    "amount": "5000",
    "fee": "50",
    "memo": "Payment",
    "signature": "a1b2c3d4...64-byte-hex-signature...",
    "nonce": 1,
    "public_key": "alice12345678901234567890123456789012345678901234567890123456"
  }'
```

Response:
```json
{"status": "ok", "receipt_id": "latest"}
```

### Signature Requirements

- **Algorithm**: Ed25519
- **Public Key**: 32-byte hex string (must derive to `from` address)
- **Signature**: 64-byte hex string
- **Nonce**: Sequential counter (starts at 1, incremented after each transfer)
- **Message Format** (canonical):
  1. `from` address (32 bytes raw)
  2. `to` address (32 bytes raw)
  3. `amount` (16 bytes, little-endian u128)
  4. `fee` (16 bytes, little-endian u128)
  5. `nonce` (8 bytes, little-endian u64)
  6. `memo` (optional UTF-8 bytes)

### Client-Side Signing Example (Pseudocode)

```javascript
// 1. Construct message
const message = Buffer.concat([
  Buffer.from(from_address, 'hex'),      // 32 bytes
  Buffer.from(to_address, 'hex'),        // 32 bytes
  uint128_to_le_bytes(amount),           // 16 bytes
  uint128_to_le_bytes(fee),              // 16 bytes
  uint64_to_le_bytes(nonce),             // 8 bytes
  Buffer.from(memo, 'utf8')              // optional
]);

// 2. Sign with Ed25519 private key
const signature = ed25519.sign(message, privateKey);

// 3. Submit transfer
const response = await fetch('/wallet/transfer', {
  method: 'POST',
  body: JSON.stringify({
    from: from_address,
    to: to_address,
    amount: amount.toString(),
    fee: fee.toString(),
    memo: memo,
    signature: signature.toString('hex'),
    nonce: nonce,
    public_key: publicKey.toString('hex')
  })
});
```

---

## Get Receipts

```bash
curl "http://127.0.0.1:7070/receipts/latest?limit=10"
```

Response:
```json
[{
  "id": "1735689600123456789-000001",
  "ts_ms": 1735689600000,
  "kind": "transfer",
  "from": "alice12345678",
  "to": "bob987654321",
  "amount": "5000",
  "fee": "50",
  "memo": "Payment",
  "ok": true
}]
```

---

## PowerShell Examples

### Get Balance
```powershell
$bal = Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/alice12345678901234567890123456789012345678901234567890123456/balance"
Write-Host "Balance: $($bal.balance)"
```

### Transfer (Signed)
```powershell
# NOTE: Client must implement Ed25519 signing in production
# This example shows the API structure only

$body = @{
    from = "alice12345678901234567890123456789012345678901234567890123456"
    to = "bob98765432109876543210987654321098765432109876543210987654321"
    amount = "5000"
    fee = "50"
    memo = "Payment"
    signature = "a1b2c3...64-byte-hex-ed25519-signature..."
    nonce = 1
    public_key = "alice12345678901234567890123456789012345678901234567890123456"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/transfer" `
    -Method Post -ContentType "application/json" -Body $body
```

### Get Receipts
```powershell
$receipts = Invoke-RestMethod -Uri "http://127.0.0.1:7070/receipts/latest?limit=10"
$receipts | Format-Table kind, from, to, amount, fee
```

---

## Error Codes

| Code | Meaning |
|------|---------|
| 400 | Invalid address, amount, fee, nonce, zero amount, or nonce mismatch |
| 401 | Signature verification failed (invalid signature, public key mismatch) |
| 402 | Insufficient funds |
| 500 | Database or internal error |

### Common Error Messages

- `signature_verification_failed`: Ed25519 signature is invalid
- `public_key_mismatch`: Public key does not derive to `from` address
- `invalid_nonce`: Nonce does not match expected value (current + 1)
- `insufficient_funds`: Balance too low for transfer + fees

---

## Receipt Kinds

- `transfer` - P2P token transfer
- `mint` - New tokens created
- `burn` - Tokens destroyed
- `market_settle` - Market proceeds routing
- `airdrop` - Admin token distribution

---

## Database Trees

| Tree | Purpose | Key | Value |
|------|---------|-----|-------|
| `balances` | Token balances | Address (bytes) | u128 LE (16 bytes) |
| `wallet_nonces` | Replay protection | Address (bytes) | u64 LE (8 bytes) |
| `receipts` | Transaction log | Timestamp-Counter | bincode(Receipt) |
| `__fees__` | Fee accumulation | Special address | u128 LE (16 bytes) |

---

## Test Script

```powershell
.\test-wallet-receipts.ps1 -BaseUrl "http://127.0.0.1:7070"
```

---

## Integration Example (Rust)

```rust
use crate::receipts::{Receipt, write_receipt};
use crate::wallet::{read_u128_le, write_u128_le};

// Log a receipt
let rec = Receipt {
    id: String::new(),
    ts_ms: 0,
    kind: "custom".into(),
    from: "addr1".into(),
    to: "addr2".into(),
    amount: "1000".into(),
    fee: "10".into(),
    memo: None,
    txid: None,
    ok: true,
    note: None,
};
write_receipt(&db, rec)?;

// Read/write balance
let balances = db.open_tree("balances")?;
let bal = read_u128_le(&balances, b"addr1")?;
write_u128_le(&balances, b"addr1", bal + 1000)?;
```

---

**See**: `docs/WALLET_RECEIPTS.md` for full documentation
