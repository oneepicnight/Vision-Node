# Vision Node Wallet - Client Signing Examples

This directory contains complete client implementation examples for signing wallet transfers with Ed25519 signatures.

## Files

- **wallet-signing.js** - JavaScript/Node.js implementation
- **wallet-signing.py** - Python implementation
- **README.md** - This file

## Prerequisites

### JavaScript/Node.js

```bash
npm install @noble/ed25519 node-fetch
```

### Python

```bash
pip install cryptography requests
```

## Quick Start

### JavaScript

```bash
node wallet-signing.js
```

### Python

```bash
python wallet-signing.py
```

## Examples Included

### 1. Generate Keypair
- Generate new Ed25519 keypair
- Display private key, public key, and address

### 2. Query Balance
- Query token balance from Vision node
- Requires running Vision node

### 3. Query Nonce
- Query current nonce for an address
- Required for replay protection

### 4. Signed Transfer
- Construct canonical message
- Sign with Ed25519
- Submit to Vision node

### 5. Message Construction
- Demonstrate canonical message format
- Show hex encoding and byte layout

### 6. Multiple Sequential Transfers
- Execute multiple transfers with proper nonce handling
- Demonstrates nonce increment logic

### 7. Load Existing Wallet (Python only)
- Load wallet from existing private key
- Useful for persistent wallets

## Usage as Library

### JavaScript

```javascript
const wallet = require('./wallet-signing.js');

// Generate keypair
const keypair = await wallet.generateKeypair();

// Query nonce
const nonce = await wallet.getNonce(keypair.address);

// Sign and submit transfer
const result = await wallet.signAndSubmitTransfer(
    keypair.privateKey,
    keypair.publicKey,
    {
        from: keypair.address,
        to: recipientAddress,
        amount: 5000,
        fee: 50,
        memo: 'Payment',
        nonce: nonce + 1
    }
);
```

### Python

```python
from wallet_signing import VisionWallet

# Generate new wallet
wallet = VisionWallet()
print(f'Address: {wallet.address}')

# Query nonce
nonce = wallet.get_nonce()
print(f'Nonce: {nonce}')

# Execute transfer
result = wallet.transfer(
    to=recipient_address,
    amount=5000,
    fee=50,
    memo='Payment'
)
print(f'Result: {result}')

# Load existing wallet
wallet = VisionWallet.from_private_key_hex(private_key_hex)
```

## Message Format

All signed transfers use the following canonical message format:

```
┌─────────────────┬──────────────────┬──────────┐
│ Field           │ Size (bytes)     │ Encoding │
├─────────────────┼──────────────────┼──────────┤
│ from address    │ 32               │ Raw      │
│ to address      │ 32               │ Raw      │
│ amount          │ 16               │ u128 LE  │
│ fee             │ 16               │ u128 LE  │
│ nonce           │ 8                │ u64 LE   │
│ memo (optional) │ variable         │ UTF-8    │
└─────────────────┴──────────────────┴──────────┘

Total: 104 + memo_len bytes
```

### Important Notes

1. **Addresses are raw bytes**: Hex-decode addresses before including in message
2. **Little-endian encoding**: All integers use LE encoding
3. **UTF-8 for memo**: Use UTF-8 encoding, not UTF-16
4. **Deterministic**: Same inputs must produce identical message

## API Endpoints

### GET /wallet/:addr/balance
Query token balance for an address.

**Response**:
```json
{
  "address": "abc123...",
  "balance": "1000000"
}
```

### GET /wallet/:addr/nonce
Query current nonce for an address (for replay protection).

**Response**:
```json
{
  "address": "abc123...",
  "nonce": 5
}
```

### POST /wallet/transfer
Submit a signed token transfer.

**Request**:
```json
{
  "from": "abc123...",
  "to": "def456...",
  "amount": "5000",
  "fee": "50",
  "memo": "Payment",
  "signature": "a1b2c3...",
  "nonce": 6,
  "public_key": "abc123..."
}
```

**Response**:
```json
{
  "status": "ok",
  "receipt_id": "latest"
}
```

## Error Handling

### Signature Verification Failed (401)
```json
{
  "status": "rejected",
  "code": 401,
  "error": "signature_verification_failed: Verification equation was not satisfied"
}
```

**Causes**:
- Wrong private key used
- Message constructed incorrectly
- Signature corrupted

### Public Key Mismatch (401)
```json
{
  "status": "rejected",
  "code": 401,
  "error": "public_key_mismatch: derived abc..., expected def..."
}
```

**Causes**:
- Public key doesn't match sender address
- Wrong keypair provided

### Invalid Nonce (400)
```json
{
  "status": "rejected",
  "code": 400,
  "error": "invalid_nonce: expected 5, got 3"
}
```

**Causes**:
- Transfer submitted out of order
- Nonce not incremented after previous transfer
- Concurrent transfers from same address

### Insufficient Funds (402)
```json
{
  "status": "rejected",
  "code": 402,
  "error": "insufficient_funds"
}
```

**Causes**:
- Balance too low for amount + fee

## Security Best Practices

### Private Key Storage

❌ **Never**:
- Store in version control
- Log or print to console (in production)
- Store unencrypted in browser localStorage
- Transmit over unencrypted connections

✅ **Always**:
- Use hardware wallets for high-value accounts
- Store encrypted with strong passwords
- Use secure enclaves (iOS Keychain, Android Keystore)
- Keep backups in secure locations

### Nonce Management

- **Cache nonces client-side** to avoid excessive API calls
- **Don't increment on failure** - retry with same nonce
- **Avoid concurrent transfers** from same address (causes nonce races)
- **Query nonce on startup** to sync with server

### Message Construction

- **Verify byte order** - Use little-endian for all integers
- **Test with known values** - Validate against reference implementation
- **Handle edge cases** - Test with zero amounts, max values, empty memos

## Testing

### With Mock Data

Both examples run without a Vision node (examples 1, 5, 7) to demonstrate:
- Keypair generation
- Message construction
- Hex encoding/decoding

### With Vision Node

Uncomment examples 2-4 and 6 to test with a running node:

```bash
# Start Vision node
./vision-node --port 7070

# Run examples
node wallet-signing.js
# or
python wallet-signing.py
```

### Unit Tests

JavaScript:
```bash
npm test
```

Python:
```bash
pytest wallet-signing.py
```

## Troubleshooting

### "Connection refused" error
- Ensure Vision node is running on configured URL
- Check firewall settings
- Verify port (default: 7070)

### "invalid_nonce" errors
- Query current nonce before each transfer
- Don't submit concurrent transfers from same address
- Check if previous transfer succeeded

### "signature_verification_failed" errors
- Verify message construction matches specification
- Check byte order (little-endian)
- Ensure addresses are hex-decoded (raw bytes)
- Validate signature encoding (64-byte hex)

### "public_key_mismatch" errors
- Ensure `public_key` field matches `from` address
- Verify using correct keypair
- Check for typos in address/public key

## References

- **Vision Wallet Documentation**: `../WALLET_SIGNATURE_VERIFICATION.md`
- **API Reference**: `../WALLET_RECEIPTS_QUICKREF.md`
- **Ed25519 Specification**: [RFC 8032](https://tools.ietf.org/html/rfc8032)
- **@noble/ed25519**: https://github.com/paulmillr/noble-ed25519
- **cryptography (Python)**: https://cryptography.io/

## Support

For issues or questions:
- Check documentation in `docs/`
- Review implementation summary in `WALLET_SIGNATURE_IMPLEMENTATION.md`
- Test with example scripts first
- Verify Vision node is running and accessible

---

**Status**: Production-ready examples for mainnet deployment

**Last Updated**: 2024-01-27
