# Node Identity & Approval System - Quick Reference

## 📋 Quick Start

### For Node Operators

1. **First Run** - Node automatically generates Ed25519 identity
   ```
   🆔 Generated new node identity
      Node ID: 8f2a9c...aa9e
      Derived from Ed25519 public key
   ```

2. **Check Identity**
   ```bash
   GET http://localhost:7070/api/status
   ```
   Look for:
   - `node_id` - 40 hex chars
   - `node_pubkey_fingerprint` - XXXX-XXXX-XXXX-XXXX
   - `approved` - true/false

3. **Approve Node with Wallet**
   - Open miner panel: `http://localhost:7070/panel.html`
   - Go to "Node Identity" section
   - Click "🔐 Approve Node with Wallet"
   - Sign the displayed message with your LAND wallet
   - Paste signature and submit

### For Developers

#### API Endpoints

**Get Approval Status**
```bash
curl http://localhost:7070/api/node/approval/status
```

**Submit Approval**
```bash
curl -X POST http://localhost:7070/api/node/approval/submit \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_address": "LAND1...",
    "ts_unix": 1765460000,
    "nonce_hex": "a1b2c3d4...",
    "signature_b64": "..."
  }'
```

## 🔑 Identity Components

### Node ID
- **Format:** 40 hex characters (20 bytes)
- **Source:** SHA-256(Ed25519 public key)[0..20]
- **Example:** `8f2a9c3e7d1b4a5f9c2e8d7a6b5c4d3e2f1a0b9c`

### Pubkey Fingerprint
- **Format:** XXXX-XXXX-XXXX-XXXX (4 groups of 4 hex chars)
- **Source:** SHA-256(Ed25519 public key)[0..8]
- **Example:** `3A7C-91D2-0B44-FF10`
- **Use:** Easy visual verification

### Public Key
- **Format:** Base64-encoded 32 bytes
- **Source:** Ed25519 public key
- **Use:** Cryptographic verification

## 📝 Canonical Message Format

```
VISION_NODE_APPROVAL_V1
wallet=LAND1abc123
node_id=8f2a9c...aa9e
node_pubkey=base64pubkey...
ts=1765460000
nonce=a1b2c3d4e5f6g7h8
```

**Rules:**
- Must be exact format (no extra spaces/newlines)
- Timestamp within ±10 minutes
- Nonce must be unique (16 bytes hex)
- Sign with wallet's private key

## 🔐 Signature Verification

**Algorithm:** ECDSA secp256k1 (same as LAND wallet)

**Process:**
1. Hash message with SHA-256
2. Sign with wallet private key
3. Encode signature as hex (65 bytes: r + s + v)
4. Convert to base64 for API submission

**Verification:**
1. Recover public key from signature
2. Derive address from public key
3. Compare with claimed wallet address

## 🚀 Integration Examples

### JavaScript (Browser)
```javascript
// Get node info
const status = await fetch('/api/status').then(r => r.json());

// Build canonical message
const message = `VISION_NODE_APPROVAL_V1
wallet=${walletAddress}
node_id=${status.node_id}
node_pubkey=${status.node_pubkey}
ts=${Math.floor(Date.now() / 1000)}
nonce=${generateNonce()}`;

// Sign with wallet (implementation depends on wallet type)
const signature = await wallet.sign(message);

// Submit approval
await fetch('/api/node/approval/submit', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    wallet_address: walletAddress,
    ts_unix: Math.floor(Date.now() / 1000),
    nonce_hex: nonceHex,
    signature_b64: signature
  })
});
```

### Rust (Backend)
```rust
use crate::identity::{local_node_id, local_pubkey_b64, local_fingerprint};
use crate::node_approval::NodeApproval;

// Check if node is approved
let approved = match NodeApproval::load()? {
    Some(approval) => {
        let node_id = local_node_id();
        let pubkey = local_pubkey_b64();
        approval.verify(&node_id, &pubkey).is_ok()
    }
    None => false,
};

// Get fingerprint for display
let fingerprint = local_fingerprint();
println!("Pubkey FPR: {}", fingerprint);
```

## 🔄 Migration from Legacy Identity

**Automatic Migration:**
- Detects old `vision_data/node_id.txt`
- Generates new Ed25519 keypair
- Creates `identity_migration.json` mapping

**Migration Record:**
```json
{
  "legacy_node_id": "temp-9917d14f-...",
  "new_node_id": "8f2a9c...aa9e",
  "new_pubkey_b64": "...",
  "migrated_at_unix": 1765460000
}
```

**Legacy Peer Support:**
- Peers without pubkey marked as `legacy=true`
- Still can connect and sync
- Upgrade to verified identity when pubkey provided

## 🛡️ Security Features

### Anti-Replay Protection
- Nonce cache with DashMap
- Timestamp window (±10 minutes)
- Per-node nonce tracking
- Automatic cleanup of old nonces

### Identity Verification
- Node ID must match pubkey derivation
- Signature must be valid for claimed wallet
- Timestamp must be recent
- No private keys in code

## 📊 Monitoring

### Check Node Identity
```bash
# Full status
curl http://localhost:7070/api/status | jq '.node_id, .node_pubkey_fingerprint, .approved'

# Approval status only
curl http://localhost:7070/api/node/approval/status
```

### View Identity Files
```bash
# Node keypair (binary)
ls vision_data/node_ed25519.key

# Approval (if exists)
cat vision_data/node_approval.json

# Migration (if migrated)
cat vision_data/identity_migration.json
```

### Logs to Watch
```
🆔 Generated new node identity
🔄 Migrated from legacy node ID
✅ Node approved by wallet: LAND1...
⚠️  Nonce replay detected
❌ Timestamp outside acceptable window
```

## 🎨 UI Elements

### Miner Panel Identity Section
```
┌─────────────────────────────────────────────┐
│ 🔑 Node Identity                            │
├─────────────────────────────────────────────┤
│ Node ID:    8f2a9c...aa9e                   │
│ Pubkey FPR: 3A7C-91D2-0B44-FF10            │
│ Approval:   ✅ Approved / ⚠ Not approved   │
└─────────────────────────────────────────────┘
```

### Approve Button (when not approved)
```
┌─────────────────────────────────────────────┐
│ 🔐 Approve Node with Wallet                 │
│                                             │
│ Sign a message with your wallet to approve │
│ this node's identity                        │
└─────────────────────────────────────────────┘
```

## ⚠️ Troubleshooting

### "Signature invalid"
- Check message format exactly matches canonical format
- Verify wallet address is correct
- Ensure signature is in correct format (65 bytes hex)

### "Timestamp outside window"
- Clock sync issue - check system time
- Message too old - regenerate with current timestamp

### "Nonce replay detected"
- Using same nonce twice - generate new random nonce

### "Node ID mismatch"
- Node identity changed - approval invalid
- Delete `node_approval.json` and re-approve

## 📞 Support

**Files to check:**
- `src/identity/` - Identity system
- `src/node_approval.rs` - Approval logic
- `src/api/node_approval_api.rs` - API endpoints
- `public/panel.html` - UI implementation

**Diagnostic Commands:**
```bash
# Check identity
curl http://localhost:7070/api/status | jq

# Check approval
curl http://localhost:7070/api/node/approval/status | jq

# View logs
tail -f logs/vision-node.log | grep -E "identity|approval|nonce"
```

---

**Version:** v2.7.0
**Last Updated:** December 12, 2025
