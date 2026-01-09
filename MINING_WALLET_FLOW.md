# Mining Wallet Address Flow - How Node Knows Where to Pay

## TL;DR - Do Miners Need to Enter a Wallet Address?

**YES** - Miners must configure their wallet address to receive LAND token rewards. Here's the flow:

1. **Create wallet** in the wallet UI at `http://127.0.0.1:7070/app#/wallet`
2. **Copy your wallet address** (starts with `0x...`, 42 characters)
3. **Link wallet to node** via panel at `http://127.0.0.1:7070/panel.html`
4. **Start mining** - all rewards automatically go to your linked wallet

---

## The Complete Flow

### Step 1: Wallet Creation (One-Time)
**Location:** `http://127.0.0.1:7070/app#/wallet`

Users create a wallet in the web-based wallet interface. This generates:
- **Ethereum-compatible address** (0x... format, 42 chars)
- **Private key** (stored in browser LocalStorage)
- **Ed25519 signing key** (for node approval)

### Step 2: Wallet Linking (One-Time)
**Location:** `http://127.0.0.1:7070/panel.html` → "🔐 Link Wallet to Node" section

**UI Elements:**
```html
<input id="wallet-address-input" placeholder="0x..." />
<button id="link-node-btn">🔗 Link Node</button>
<span id="current-wallet">None</span>  <!-- Shows linked wallet -->
```

**What Happens When User Clicks "Link Node":**

#### Step 2.1: Register Wallet with Node
```javascript
// JavaScript in panel.html
POST /wallet/register
Body: { "wallet_address": "0x1234..." }
```

**Backend Action** ([src/main.rs](src/main.rs#L2942)):
- Generates Ed25519 keypair for wallet
- Binds wallet address to node's database
- Stores relationship: `wallet_address` → `node_id`

#### Step 2.2: Challenge-Response Authentication
```javascript
// Get challenge from node
POST /node/approval/challenge
Body: { "wallet_address": "0x1234..." }
Response: { 
  "message": "challenge_text",
  "ts_unix": 1234567890,
  "nonce_hex": "abc123..."
}
```

```javascript
// Sign challenge with wallet's Ed25519 key
POST /wallet/sign_message
Body: { 
  "wallet_address": "0x1234...",
  "message": "challenge_text"
}
Response: { 
  "signature_b64": "base64_signature",
  "wallet_address": "0x1234..."
}
```

```javascript
// Submit signed approval
POST /node/approval/submit
Body: {
  "wallet_address": "0x1234...",
  "ts_unix": 1234567890,
  "nonce_hex": "abc123...",
  "signature_b64": "base64_signature"
}
Response: { "ok": true }
```

**Backend Action** ([src/api/node_approval_api.rs](src/api/node_approval_api.rs#L112)):
- Verifies Ed25519 signature
- Approves node for mining
- **Calls `bind_wallet_to_node()`** - THIS IS THE KEY!

#### Step 2.3: Wallet Address Stored Globally
**Code Location:** [src/main.rs](src/main.rs#L1429)

```rust
async fn miner_configure(Json(req): Json<MinerConfigureReq>) {
    let wallet = req.wallet.trim().to_string();
    
    // Update global miner address
    *MINER_ADDRESS.lock() = wallet.clone();
    
    // Persist to database (set once, remember forever)
    let db = crate::CHAIN.lock().db.clone();
    db.insert(META_MINER_ADDR.as_bytes(), wallet.as_bytes());
    db.flush();
    
    println!("✅ Mining address configured: {}", wallet);
}
```

**Global State:** [src/main.rs](src/main.rs#L2912)
```rust
static MINER_ADDRESS: Lazy<Arc<Mutex<String>>> = Lazy::new(|| {
    let default_addr = std::env::var("VISION_MINER_ADDRESS")
        .unwrap_or_else(|_| "pow_miner".to_string());
    Arc::new(Mutex::new(default_addr))
});
```

**Persistence:** Database key `meta_miner_address` stores wallet address across restarts.

**Load on Startup:** [src/main.rs](src/main.rs#L3291)
```rust
if let Ok(Some(v)) = db.get(META_MINER_ADDR.as_bytes()) {
    if let Ok(s) = String::from_utf8(v.to_vec()) {
        *crate::MINER_ADDRESS.lock() = s;
        tracing::info!("✅ Loaded mining address from DB");
    }
}
```

### Step 3: Mining Block Creation
**Location:** [src/main.rs](src/main.rs#L5426)

When miner mines a block, the `MINER_ADDRESS` is used as the recipient:

```rust
let miner_addr = MINER_ADDRESS.lock().clone();
let mut cloned_balances = g.balances.clone();
let mut cloned_nonces = g.nonces.clone();
let miner_key = acct_key(&miner_addr);

// Ensure miner account exists
cloned_balances.entry(miner_key.clone()).or_insert(0);
cloned_nonces.entry(miner_key.clone()).or_insert(0);

// Execute transactions (fees go to miner)
for tx in &selected_txs {
    let res = execute_tx_with_nonce_and_fees(
        tx,
        &mut cloned_balances,
        &mut cloned_nonces,
        &miner_key,  // ← Receives transaction fees
        &mut cloned_gm,
    );
}

// Block reward is added to miner's balance
// (happens in block validation/application)
```

### Step 4: Block Rewards
When a block is mined and accepted:
1. **Block reward** (base amount) → `miner_addr`
2. **Transaction fees** (from all txs in block) → `miner_addr`
3. Balance updates are written to state tree
4. State root computed and included in block

---

## API Endpoints

### Configure Miner Wallet
```http
POST /api/miner/wallet
Content-Type: application/json

{
  "wallet": "0x1234567890abcdef1234567890abcdef12345678"
}

Response:
{
  "ok": true,
  "wallet": "0x1234567890abcdef1234567890abcdef12345678",
  "message": "Miner address updated successfully"
}
```

### Get Current Miner Wallet
```http
GET /api/miner/wallet

Response:
{
  "ok": true,
  "wallet": "0x1234567890abcdef1234567890abcdef12345678"
}
```

---

## Command Line Alternative

Users can also set wallet address via environment variable:

```bash
# Windows PowerShell
$env:VISION_MINER_ADDRESS = "0x1234567890abcdef1234567890abcdef12345678"
./vision-node.exe
```

```bash
# Linux/Mac
export VISION_MINER_ADDRESS=0x1234567890abcdef1234567890abcdef12345678
./vision-node
```

**Note:** Environment variable is only used on first startup. After wallet is linked via UI, the database value takes precedence.

---

## Current Wallet Display

### Panel UI
**Location:** `http://127.0.0.1:7070/panel.html`

```html
<span id="current-wallet" style="color: var(--accent-green);">
  0x1234...5678
</span>
```

Updated by JavaScript:
```javascript
const response = await fetch('/api/miner/wallet');
const data = await response.json();
document.getElementById('current-wallet').textContent = 
    data.wallet || 'None';
```

### Command Center
**Location:** `http://127.0.0.1:7070/` (wallet UI)

The Command Center doesn't currently show the mining wallet address, but it could be added to the new MiningControls component.

---

## Security Notes

### Why Challenge-Response?
The node approval flow uses Ed25519 signatures to prove:
1. **User owns the wallet** (can sign with wallet's Ed25519 key)
2. **Challenge is fresh** (timestamp + nonce prevents replay)
3. **Approval is intentional** (signature over specific challenge text)

This prevents:
- ❌ Someone else linking your wallet to their node
- ❌ Replay attacks (reusing old signatures)
- ❌ Mining to wrong address by accident

### Wallet Key Storage
- **Ed25519 keys** stored in node database (for signing)
- **Mining rewards** go to Ethereum-compatible address
- **Private keys** for spending are in browser LocalStorage

---

## Pool Mining Scenarios

### Solo Mining
- Wallet address: Your personal wallet
- Rewards: 100% to you (minus network fees)

### Pool Mining (HostPool)
- Pool operator sets wallet address
- Workers join pool
- Rewards distributed according to shares
- Pool fee taken by operator
- Foundation fee: 1%

### Join Pool Mode
- Worker enters pool URL
- Worker wallet configured separately
- Pool pays worker based on contribution
- Worker gets their share directly

### Farm Mode
- Multiple rigs managed from one dashboard
- Each rig can have its own wallet (optional)
- Or all pay to master wallet address

---

## Troubleshooting

### "Wallet: None" in Panel
**Problem:** Miner hasn't linked wallet yet

**Solution:**
1. Open `http://127.0.0.1:7070/app#/wallet`
2. Create or access existing wallet
3. Copy address (0x...)
4. Go to `http://127.0.0.1:7070/panel.html`
5. Paste address in "Link Wallet to Node" section
6. Click "🔗 Link Node"

### Mining But No Rewards
**Problem:** Wallet address not configured

**Check:**
```bash
curl http://127.0.0.1:7070/api/miner/wallet
```

If returns empty or "pow_miner", follow linking steps above.

### "Invalid wallet address format"
**Problem:** Address is not 0x... format or wrong length

**Valid Format:**
- Starts with `0x`
- Exactly 42 characters total
- Example: `0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb2`

### Wallet Linked But Mining Disabled
**Problem:** Node approval expired or failed

**Check approval status:**
```bash
curl http://127.0.0.1:7070/api/node/approval/status
```

**Re-approve:**
- Repeat wallet linking process in panel
- Signature proves you still control the wallet

---

## Code References

| Feature | File | Line | Description |
|---------|------|------|-------------|
| Global miner address | [src/main.rs](src/main.rs#L2912) | 2912 | `MINER_ADDRESS` static global |
| Configure wallet API | [src/main.rs](src/main.rs#L1413) | 1413 | POST `/miner/wallet` |
| Get wallet API | [src/main.rs](src/main.rs#L1450) | 1450 | GET `/miner/wallet` |
| Load on startup | [src/main.rs](src/main.rs#L3291) | 3291 | Restore from DB |
| Use in mining | [src/main.rs](src/main.rs#L5426) | 5426 | Block creation |
| UI - Link wallet | [public/panel.html](public/panel.html#L1218) | 1218 | "Link Wallet to Node" section |
| UI - Link function | [public/panel.html](public/panel.html#L2916) | 2916 | `linkNodeWithWallet()` |
| Approval flow | [src/api/node_approval_api.rs](src/api/node_approval_api.rs) | - | Challenge/submit |

---

## Summary

**Yes, miners MUST configure their wallet address.** The flow is:

1. ✅ Create wallet in wallet UI
2. ✅ Link wallet to node via panel.html (one-time setup)
3. ✅ Node stores wallet address in global `MINER_ADDRESS`
4. ✅ Wallet address persisted to database
5. ✅ When mining blocks, rewards go to this address
6. ✅ Address survives node restarts

**No manual entry needed per mining session** - once linked, the wallet address is remembered permanently until changed.
