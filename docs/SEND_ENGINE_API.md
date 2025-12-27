# Send Engine API Documentation

## Overview
The Vision Node Send Engine provides a complete system for sending cryptocurrency (BTC, BCH, DOGE) from Vision wallets to external blockchain addresses. It includes balance management, transaction building, signing, broadcasting, and history tracking.

---

## Endpoints

### POST /wallet/send

Send cryptocurrency to an external blockchain address with optional simulation mode.

#### Request

**Headers:**
```
Content-Type: application/json
```

**Body:**
```json
{
  "user_id": "string",
  "chain": "btc" | "bch" | "doge",
  "to_address": "string",
  "amount": "string",
  "simulate": boolean (optional, default: false)
}
```

**Field Descriptions:**
- `user_id` (required): Unique identifier for the user
- `chain` (required): Target blockchain - must be one of: "btc", "bch", or "doge"
- `to_address` (required): Destination wallet address on the target blockchain
- `amount` (required): Amount to send in smallest units (satoshis for BTC/BCH, koinus for DOGE)
- `simulate` (optional): When `true`, validates the transaction without broadcasting. Defaults to `false`.

#### Response (Success)

**Status:** `200 OK`

```json
{
  "success": true,
  "txid": "abc123def456...",
  "status": "broadcast",
  "message": "Transaction broadcast successfully: abc123...",
  "estimated_fee": 1000,
  "total_spent": 11000,
  "error_code": null
}
```

**Response Fields:**
- `success`: Boolean indicating if the operation succeeded
- `txid`: Transaction ID from the blockchain (or "simulation-only" in simulate mode)
- `status`: Transaction status - "broadcast", "simulated", or "error"
- `message`: Human-readable status message
- `estimated_fee`: Fee amount in smallest units
- `total_spent`: Total amount deducted (amount + fee)
- `error_code`: Error code if failed, null on success

#### Response (Simulation Mode)

**Status:** `200 OK`

```json
{
  "success": true,
  "txid": "simulation-only",
  "status": "simulated",
  "message": "Simulation successful. Would send 10000 + 1000 fee = 11000 total",
  "estimated_fee": 1000,
  "total_spent": 11000,
  "error_code": null
}
```

#### Response (Error)

**Status:** `200 OK` or `400 Bad Request`

```json
{
  "success": false,
  "txid": null,
  "status": "error",
  "message": "Insufficient balance. Need 11000 (amount: 10000, fee: 1000), have 5000",
  "estimated_fee": 1000,
  "total_spent": 11000,
  "error_code": "insufficient_funds"
}
```

**Error Codes:**
- `invalid_chain`: Chain parameter is not supported
- `unsupported_chain`: Chain not enabled for external sends
- `invalid_address`: Destination address format is invalid
- `invalid_amount`: Amount is not a valid number or is zero
- `amount_too_small`: Amount is below dust threshold
- `insufficient_funds`: User balance too low to cover amount + fee
- `rpc_unavailable`: External RPC node not configured or unreachable
- `transaction_failed`: Transaction building or broadcast failed
- `internal_error`: Server-side error

---

### GET /wallet/sends

Retrieve transaction history for outbound sends.

#### Request

**Query Parameters:**
- `user_id` (required): User ID to query transactions for
- `limit` (optional): Maximum number of records to return (default: 20, max: 100)

**Example:**
```
GET /wallet/sends?user_id=user-123&limit=10
```

#### Response (Success)

**Status:** `200 OK`

```json
{
  "success": true,
  "count": 3,
  "sends": [
    {
      "id": 42,
      "user_id": "user-123",
      "chain": "btc",
      "to_address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
      "amount": "10000",
      "txid": "abc123def456...",
      "status": "broadcast",
      "created_at": "2025-11-20T15:30:00Z"
    },
    {
      "id": 41,
      "user_id": "user-123",
      "chain": "doge",
      "to_address": "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L",
      "amount": "1000000",
      "txid": "def789ghi012...",
      "status": "broadcast",
      "created_at": "2025-11-20T14:15:00Z"
    }
  ]
}
```

**Response Fields:**
- `success`: Boolean indicating if query succeeded
- `count`: Number of records returned
- `sends`: Array of transaction records

**Transaction Record Fields:**
- `id`: Unique record ID
- `user_id`: User who initiated the send
- `chain`: Blockchain used (btc, bch, doge)
- `to_address`: Destination address
- `amount`: Amount sent in smallest units
- `txid`: Blockchain transaction ID
- `status`: Transaction status (broadcast, simulated, failed)
- `created_at`: ISO 8601 timestamp

#### Response (Error)

**Status:** `400 Bad Request` or `500 Internal Server Error`

```json
{
  "error": "missing_user_id",
  "message": "Query parameter 'user_id' is required"
}
```

---

## Transaction Lifecycle

### Normal Send Flow

1. **Validation**
   - Validate chain is supported (BTC, BCH, DOGE)
   - Validate address format (basic length check)
   - Parse and validate amount
   - Check dust threshold

2. **Balance Check**
   - Calculate total needed: `amount + estimated_fee`
   - Verify user has sufficient balance
   - Reserve balance (lock funds)

3. **Transaction Building**
   - Sync UTXOs from blockchain via RPC `listunspent`
   - Select UTXOs using largest-first algorithm
   - Build raw transaction via RPC `createrawtransaction`
   - Sign transaction via RPC `signrawtransactionwithwallet`

4. **Broadcasting**
   - Broadcast signed transaction via RPC `sendrawtransaction`
   - Store transaction record in history
   - Finalize balance deduction

5. **Error Handling**
   - On any error, release reserved balance (rollback)
   - Log detailed error for operators
   - Return user-friendly error message

### Simulation Mode Flow

When `simulate: true`:

1. **Validation** (same as normal)
2. **Balance Check** (same as normal)
3. **Stop Here** - Do NOT build, sign, or broadcast
4. **Return Success** with estimated fees and totals

No funds are moved, no UTXOs are locked, no RPC calls to blockchain.

---

## Fee Structure

Current fees are fixed per chain:

| Chain | Fee (smallest units) | Approximate USD* |
|-------|---------------------|------------------|
| BTC   | 1,000 satoshis     | ~$0.40           |
| BCH   | 500 satoshis       | ~$0.20           |
| DOGE  | 100,000 koinus     | ~$0.01           |

*At $40k BTC, $400 BCH, $0.10 DOGE

### Dust Thresholds

Minimum sendable amounts:

| Chain | Dust Threshold      |
|-------|---------------------|
| BTC   | 546 satoshis       |
| BCH   | 546 satoshis       |
| DOGE  | 1,000,000 koinus   |

Transactions below these amounts will be rejected.

---

## Example Usage

### Example 1: Normal Send (BTC)

**Request:**
```bash
curl -X POST http://127.0.0.1:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "chain": "btc",
    "to_address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
    "amount": "10000"
  }'
```

**Response:**
```json
{
  "success": true,
  "txid": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "status": "broadcast",
  "message": "Transaction broadcast successfully: e3b0...",
  "estimated_fee": 1000,
  "total_spent": 11000
}
```

### Example 2: Simulation (DOGE)

**Request:**
```bash
curl -X POST http://127.0.0.1:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "bob",
    "chain": "doge",
    "to_address": "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L",
    "amount": "5000000",
    "simulate": true
  }'
```

**Response:**
```json
{
  "success": true,
  "txid": "simulation-only",
  "status": "simulated",
  "message": "Simulation successful. Would send 5000000 + 100000 fee = 5100000 total",
  "estimated_fee": 100000,
  "total_spent": 5100000
}
```

### Example 3: Insufficient Funds

**Request:**
```bash
curl -X POST http://127.0.0.1:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "charlie",
    "chain": "bch",
    "to_address": "bitcoincash:qpm2qsznhks23z7629mms6s4cwef74vcwvy22gdx6a",
    "amount": "1000000"
  }'
```

**Response:**
```json
{
  "success": false,
  "txid": null,
  "status": "error",
  "message": "Insufficient balance. Need 1000500 (amount: 1000000, fee: 500), have 50000",
  "estimated_fee": 500,
  "total_spent": 1000500,
  "error_code": "insufficient_funds"
}
```

### Example 4: Query Transaction History

**Request:**
```bash
curl "http://127.0.0.1:7070/wallet/sends?user_id=alice&limit=5"
```

**Response:**
```json
{
  "success": true,
  "count": 2,
  "sends": [
    {
      "id": 103,
      "user_id": "alice",
      "chain": "btc",
      "to_address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
      "amount": "10000",
      "txid": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "status": "broadcast",
      "created_at": "2025-11-20T16:45:22Z"
    },
    {
      "id": 95,
      "user_id": "alice",
      "chain": "doge",
      "to_address": "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L",
      "amount": "2000000",
      "txid": "abc123def456...",
      "status": "broadcast",
      "created_at": "2025-11-19T10:30:00Z"
    }
  ]
}
```

---

## Operator Notes

### Configuration Requirements

1. **External RPC Endpoints**
   
   Must configure RPC connections in `config/external_rpc.toml`:
   
   ```toml
   [btc]
   url = "http://127.0.0.1:8332"
   username = "rpcuser"
   password = "rpcpassword"
   
   [bch]
   url = "http://127.0.0.1:8432"
   username = "rpcuser"
   password = "rpcpassword"
   
   [doge]
   url = "http://127.0.0.1:22555"
   username = "rpcuser"
   password = "rpcpassword"
   ```

2. **RPC Wallet Setup**
   
   - Bitcoin Core wallet must be created and unlocked
   - Private keys must be imported with `importprivkey` or `importdescriptors`
   - Wallet must have sufficient UTXOs for sends

3. **Feature Flags**
   
   - `dev-signing`: Enables server-side signing (DEVELOPMENT ONLY)
   - For production: Use client-side signing or hardware wallets

### Logging

All send operations are logged with structured fields:

**Success:**
```
INFO send ok: user_id="alice", chain=Btc, to=1A1zP..., amount=10000, txid=e3b0c44...
```

**Failure:**
```
ERROR send failed: user_id="bob", chain=Btc, to=1A1zP..., amount=10000, error="RPC unavailable"
```

**Key Logged Fields:**
- `user_id`: User who initiated the send
- `chain`: Target blockchain
- `to_address`: Destination address
- `amount`: Amount in smallest units
- `txid`: Transaction ID (on success)
- `error`: Error details (on failure)

### Monitoring

**Key Metrics to Monitor:**
- Send success rate by chain
- Average transaction time
- RPC availability and latency
- Failed transactions by error code
- Balance reserve/release operations

**Health Checks:**
- RPC endpoint connectivity
- Wallet unlock status
- UTXO availability
- Balance synchronization

### Common Issues

**"RPC client not configured"**
- Cause: External RPC endpoint not set up in config
- Fix: Add RPC configuration to `config/external_rpc.toml`

**"Transaction signing incomplete"**
- Cause: RPC wallet doesn't have the private key
- Fix: Import key with `bitcoin-cli importprivkey <WIF>`

**"Insufficient UTXOs"**
- Cause: Wallet has balance but no spendable UTXOs
- Fix: Send funds to the wallet or consolidate UTXOs

**"RPC sendrawtransaction failed"**
- Causes: Invalid tx, double-spend, low fee, network issues
- Fix: Check RPC logs, verify UTXO selection, increase fee

### Security Considerations

1. **Private Keys**: Never log or expose private keys in responses
2. **Rate Limiting**: Consider implementing per-user rate limits
3. **Amount Limits**: Consider max send amounts for risk management
4. **Address Validation**: Enhance with checksum validation
5. **Audit Trail**: All transactions are logged and stored in history

---

## Integration Examples

### JavaScript/TypeScript (Frontend)

```typescript
async function sendCrypto(
  userId: string,
  chain: 'btc' | 'bch' | 'doge',
  toAddress: string,
  amount: string,
  simulate: boolean = false
) {
  const response = await fetch('/wallet/send', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      user_id: userId,
      chain,
      to_address: toAddress,
      amount,
      simulate
    })
  });
  
  return await response.json();
}

// Usage
const result = await sendCrypto('alice', 'btc', '1A1zP...', '10000');
if (result.success) {
  console.log('TXID:', result.txid);
} else {
  console.error('Error:', result.message);
}
```

### Python

```python
import requests

def send_crypto(user_id, chain, to_address, amount, simulate=False):
    response = requests.post('http://127.0.0.1:7070/wallet/send', json={
        'user_id': user_id,
        'chain': chain,
        'to_address': to_address,
        'amount': amount,
        'simulate': simulate
    })
    return response.json()

# Usage
result = send_crypto('alice', 'btc', '1A1zP...', '10000')
if result['success']:
    print(f"TXID: {result['txid']}")
else:
    print(f"Error: {result['message']}")
```

---

## Version History

**v0.7.9 (Current)**
- Initial release of Send Engine
- Support for BTC, BCH, DOGE
- Simulation mode
- Transaction history tracking
- Comprehensive error handling
- Structured logging

---

## Support

For issues or questions:
- Check logs for detailed error messages
- Verify RPC configuration and connectivity
- Review transaction history for patterns
- Consult operator notes above

For feature requests or bugs, please file an issue in the repository.
