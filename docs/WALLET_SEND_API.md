# Wallet Send API

## Overview

The **Wallet Send** feature provides a generic interface for sending cryptocurrency from a Vision wallet to any external blockchain address. This replaces the more technical "withdrawal" concept with a user-friendly "send" experience.

**Supported Chains:**
- ✅ Bitcoin (BTC)
- ✅ Bitcoin Cash (BCH)
- ✅ Dogecoin (DOGE)
- 🚧 LAND (coming soon)

---

## HTTP Endpoint

### POST /wallet/send

Send coins from your Vision wallet to an external address.

**Request Body:**
```json
{
  "user_id": "user123",
  "chain": "btc",
  "to_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
  "amount": "0.001"
}
```

**Fields:**
- `user_id` (string, required): User identifier
- `chain` (string, required): Blockchain to send on
  - Accepted values: `"btc"`, `"bch"`, `"doge"`, `"bitcoin"`, `"bitcoincash"`, `"dogecoin"`
- `to_address` (string, required): Destination wallet address
  - Must be valid for the specified chain
- `amount` (string, required): Amount to send
  - Use string format to preserve precision
  - Example: `"0.00123456"`

**Success Response (200 OK):**
```json
{
  "success": true,
  "txid": "a1b2c3d4e5f6...",
  "status": "broadcast",
  "message": "Transaction broadcast successfully"
}
```

**Error Response (400 Bad Request):**
```json
{
  "success": false,
  "txid": null,
  "status": "error",
  "message": "Insufficient balance (including fees)"
}
```

---

## Example Usage

### cURL
```bash
# Send 0.001 BTC
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "alice",
    "chain": "btc",
    "to_address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
    "amount": "0.001"
  }'

# Send 100 DOGE
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "bob",
    "chain": "doge",
    "to_address": "DH5yaieqoZN36fDVciNyRueRGvGLR3mr7L",
    "amount": "100"
  }'
```

### JavaScript (fetch)
```javascript
async function sendCoins(userId, chain, toAddress, amount) {
  const response = await fetch('http://localhost:7070/wallet/send', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      user_id: userId,
      chain: chain,
      to_address: toAddress,
      amount: amount
    })
  });
  
  const result = await response.json();
  
  if (result.success) {
    console.log('Transaction broadcast! TXID:', result.txid);
  } else {
    console.error('Send failed:', result.message);
  }
  
  return result;
}

// Example usage
await sendCoins('alice', 'btc', 'bc1q...', '0.001');
```

### PowerShell
```powershell
$body = @{
    user_id = "alice"
    chain = "btc"
    to_address = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh"
    amount = "0.001"
} | ConvertTo-Json

Invoke-RestMethod -Uri "http://localhost:7070/wallet/send" `
    -Method POST `
    -Body $body `
    -ContentType "application/json"
```

---

## Address Validation

The API validates destination addresses based on the chain:

### Bitcoin (BTC)
- ✅ Bech32: `bc1q...` (SegWit native)
- ✅ P2PKH: `1...` (Legacy)
- ✅ P2SH: `3...` (Script)

### Bitcoin Cash (BCH)
- ✅ CashAddr: `bitcoincash:q...`
- ✅ Legacy: `q...` prefix

### Dogecoin (DOGE)
- ✅ Standard: `D...` prefix

**Invalid addresses will return:**
```json
{
  "success": false,
  "status": "error",
  "message": "Invalid Bitcoin address format"
}
```

---

## Safety Checks

### Pre-Send Validation

Before broadcasting, the system checks:

1. **Chain Support**: Is RPC configured for this chain?
2. **Address Format**: Is the destination address valid?
3. **Amount**: Is amount > 0?
4. **Balance**: Does user have sufficient funds? (including fees)
5. **RPC Health**: Is the blockchain RPC responding?

### Error Scenarios

**RPC Not Configured:**
```json
{
  "success": false,
  "status": "error",
  "message": "BTC RPC not configured or unavailable"
}
```

**Invalid Amount:**
```json
{
  "success": false,
  "status": "error",
  "message": "Amount must be greater than zero"
}
```

**Insufficient Balance:**
```json
{
  "success": false,
  "status": "error",
  "message": "Insufficient balance (including fees)"
}
```

**Invalid Address:**
```json
{
  "success": false,
  "status": "error",
  "message": "Invalid Dogecoin address format"
}
```

---

## Transaction Flow

```
┌─────────────┐
│ User sends  │
│ POST request│
└──────┬──────┘
       │
       ▼
┌─────────────────────┐
│ Parse & validate    │
│ - Chain             │
│ - Address           │
│ - Amount            │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Check balance       │
│ (amount + fees)     │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Build raw TX        │
│ - Select UTXOs      │
│ - Add outputs       │
│ - Sign              │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Broadcast via RPC   │
│ sendrawtransaction  │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ Return TXID         │
│ { success, txid }   │
└─────────────────────┘
```

---

## Important Notes

### ⚠️ Blockchain Transactions Are Irreversible

**Users must understand:**
- Once a transaction is broadcast, it **cannot be canceled or reversed**
- Sending to the wrong address means **permanent loss of funds**
- There is **no customer support** that can recover mistakenly sent coins
- Always **double-check the destination address** before confirming

**Recommended UX:**
```
┌──────────────────────────────────────────┐
│          Confirm Send                    │
├──────────────────────────────────────────┤
│ You are sending 0.001 BTC                │
│ To: bc1qxyz...                           │
│                                          │
│ Network fee: 0.00002 BTC                 │
│ Total deducted: 0.00102 BTC              │
│                                          │
│ ⚠️ Blockchain transactions are           │
│    irreversible. Double-check the        │
│    address before confirming.            │
│                                          │
│ ☑ I have verified the address            │
│                                          │
│ [Cancel]              [Confirm & Send]   │
└──────────────────────────────────────────┘
```

### 🔒 Security Considerations

1. **Authentication**: Implement proper user authentication before allowing sends
2. **Rate Limiting**: Limit send frequency to prevent abuse
3. **Withdrawal Limits**: Consider daily/weekly limits for new accounts
4. **2FA**: Require two-factor authentication for large sends
5. **Email Confirmation**: Send confirmation email with transaction details
6. **Audit Logging**: Log all send attempts (success and failures)

### 📊 Transaction Tracking

After a successful send, you can track the transaction:

1. **On-Chain Explorers:**
   - Bitcoin: `https://blockstream.info/tx/{txid}`
   - Bitcoin Cash: `https://blockchair.com/bitcoin-cash/transaction/{txid}`
   - Dogecoin: `https://dogechain.info/tx/{txid}`

2. **Confirmation Count:**
   - Query RPC: `gettransaction` or `getrawtransaction`
   - Check confirmations field
   - Typical wait: 1-6 confirmations depending on chain

3. **Status Updates:**
   - Implement webhook or polling to notify user when transaction confirms
   - Update transaction status in database

---

## Implementation Status

### ✅ Completed
- Chain parsing and validation
- Address format validation
- RPC availability checks
- Amount validation
- API endpoint structure
- Error handling and responses
- Integration with external RPC system

### 🚧 In Progress
- Transaction building (UTXO selection, signing)
- Balance checking and reservation
- Fee estimation
- Transaction broadcasting (waiting for tx building)

### 📋 Planned
- LAND token sends
- Multi-signature sends
- Scheduled/delayed sends
- Send to multiple addresses (batch)
- Replace-by-fee (RBF) support
- Child-pays-for-parent (CPFP)

---

## Testing

### Test Endpoint Availability
```bash
curl http://localhost:7070/wallet/send \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "test",
    "chain": "btc",
    "to_address": "bc1qtest",
    "amount": "0.001"
  }'
```

**Expected Response** (until tx building is implemented):
```json
{
  "success": false,
  "txid": null,
  "status": "error",
  "message": "Transaction building not yet implemented. Coming soon!"
}
```

### Test Chain Validation
```bash
# Valid chain
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","chain":"doge","to_address":"D...","amount":"1"}'

# Invalid chain
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","chain":"eth","to_address":"0x...","amount":"1"}'
```

### Test Address Validation
```bash
# Valid BTC address
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","chain":"btc","to_address":"bc1qxy2kgdy...","amount":"0.001"}'

# Invalid BTC address
curl -X POST http://localhost:7070/wallet/send \
  -H "Content-Type: application/json" \
  -d '{"user_id":"test","chain":"btc","to_address":"not_a_btc_address","amount":"0.001"}'
```

---

## Frontend Integration

### React Example

```jsx
import { useState } from 'react';

function SendCoinsForm() {
  const [chain, setChain] = useState('btc');
  const [toAddress, setToAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [result, setResult] = useState(null);
  const [loading, setLoading] = useState(false);

  const handleSend = async (e) => {
    e.preventDefault();
    setLoading(true);
    
    try {
      const response = await fetch('http://localhost:7070/wallet/send', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user_id: 'current_user', // Get from auth
          chain,
          to_address: toAddress,
          amount
        })
      });
      
      const data = await response.json();
      setResult(data);
      
      if (data.success) {
        alert(`Transaction broadcast! TXID: ${data.txid}`);
      } else {
        alert(`Error: ${data.message}`);
      }
    } catch (error) {
      alert(`Network error: ${error.message}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSend}>
      <h2>Send Coins</h2>
      
      <label>
        Chain:
        <select value={chain} onChange={(e) => setChain(e.target.value)}>
          <option value="btc">Bitcoin (BTC)</option>
          <option value="bch">Bitcoin Cash (BCH)</option>
          <option value="doge">Dogecoin (DOGE)</option>
        </select>
      </label>
      
      <label>
        Recipient Address:
        <input 
          type="text" 
          value={toAddress} 
          onChange={(e) => setToAddress(e.target.value)}
          placeholder="bc1q..." 
          required 
        />
      </label>
      
      <label>
        Amount:
        <input 
          type="text" 
          value={amount} 
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.001" 
          required 
        />
      </label>
      
      <p className="warning">
        ⚠️ Blockchain sends are final and irreversible.
        Double-check the address before sending.
      </p>
      
      <button type="submit" disabled={loading}>
        {loading ? 'Sending...' : 'Send'}
      </button>
      
      {result && (
        <div className={result.success ? 'success' : 'error'}>
          {result.message}
          {result.txid && <p>TXID: {result.txid}</p>}
        </div>
      )}
    </form>
  );
}
```

---

## Migration from Legacy "Withdraw" API

If you have existing code using the old withdrawal API, here's how to migrate:

### Old API (Deprecated)
```json
POST /withdraw
{
  "user_id": "alice",
  "asset": "Btc",
  "address": "bc1q...",
  "amount": 0.001
}
```

### New API (Recommended)
```json
POST /wallet/send
{
  "user_id": "alice",
  "chain": "btc",
  "to_address": "bc1q...",
  "amount": "0.001"
}
```

**Changes:**
1. Endpoint: `/withdraw` → `/wallet/send`
2. Field: `asset` → `chain` (lowercase string)
3. Field: `address` → `to_address` (clearer naming)
4. Field: `amount` → string format (better precision)
5. Response: Added `status` and `message` fields

**Backward Compatibility:**
The legacy `WithdrawRequest`/`WithdrawResponse` types are maintained in code for backward compatibility, but new integrations should use `SendRequest`/`SendResponse`.

---

## Troubleshooting

### "RPC not configured" Error

**Cause**: External RPC client not set up for this chain

**Solution**:
1. Check `config/external_rpc.toml`
2. Ensure chain is enabled: `enabled = true`
3. Verify RPC credentials are correct
4. Test RPC connection: `GET /rpc/status`

### "Transaction building not yet implemented" Error

**Cause**: UTXO management and signing infrastructure is still in development

**Status**: Coming soon in Phase 3

**Workaround**: None currently - this feature is under active development

### "Invalid address format" Error

**Cause**: Destination address doesn't match chain format

**Solution**:
- Bitcoin: Use bc1q..., 1..., or 3... addresses
- Bitcoin Cash: Use bitcoincash:q... or q... addresses
- Dogecoin: Use D... addresses
- Double-check you're using the correct chain parameter

---

## Support

For issues or questions:
- **Documentation**: See `docs/EXTERNAL_RPC_PHASE2_IMPLEMENTATION.md`
- **RPC Status**: Check `GET /rpc/status` endpoint
- **Logs**: Check server logs for detailed error messages

---

**Last Updated**: November 20, 2025  
**API Version**: 2.0 (Send)  
**Status**: ✅ Endpoint Active (Transaction building in progress)
