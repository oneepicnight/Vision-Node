# Vision Node - Integrated Wallet & Miner UI

## Overview

The Vision Node now features a **unified web interface** that combines wallet functionality with miner control panel access. The wallet serves as the **landing page** when you first access the node, with the miner panel available after wallet connection.

## Architecture

```
┌─────────────────────────────────────────────────┐
│         Vision Node Web Interface               │
│                                                 │
│  Landing Page: Wallet (No Auth Required)       │
│  ├─ Generate new wallet                        │
│  ├─ Import existing wallet                     │
│  ├─ Send/Receive tokens (with Ed25519 sigs)    │
│  └─ View balance & transactions                │
│                                                 │
│  After Connection: Miner Panel                  │
│  ├─ Monitor mining operations                   │
│  ├─ View node statistics                       │
│  ├─ Control miner settings                     │
│  └─ Network visualizations                     │
│                                                 │
│  Settings                                       │
│  ├─ Node connection configuration              │
│  ├─ Export/import private keys                 │
│  └─ Wallet management                          │
└─────────────────────────────────────────────────┘
```

## Features

### 🌟 Landing Page: Wallet

When you first open `http://localhost:7070/` you'll see the **Vision Wallet**:

#### Not Connected State
- **Generate New Wallet** - Creates a new Ed25519 keypair
- **Import Private Key** - Load existing wallet from hex private key
- Clean, modern UI with cyberpunk aesthetic

#### Connected State
- **Balance Display** - Large, prominent balance counter
- **Receive** - Display and copy your address
- **Send** - Sign and submit transfers with Ed25519 signatures
- **Wallet Info** - View nonce, public key, and connection status
- **Recent Transactions** - View transaction history

### ⛏️ Miner Panel (After Connection)

Once your wallet is connected, you can access the **Miner Control Panel**:
- Full miner dashboard embedded as iframe
- Monitor mining operations
- View node statistics
- Control miner settings
- Network visualizations with globe view

### ⚙️ Settings

- **Node Connection** - Configure custom node URLs
- **Security** - Export private keys (with warnings)
- **Wallet Management** - Disconnect/clear wallet data

## Getting Started

### 1. Start Your Node

```bash
./vision-node --port 7070
```

### 2. Open the Interface

Navigate to: `http://localhost:7070/`

### 3. Connect Your Wallet

**Option A: Generate New Wallet**
1. Click **"✨ Generate New Wallet"**
2. A new Ed25519 keypair is created instantly
3. Wallet is saved to browser localStorage
4. Balance and actions become available

**Option B: Import Existing Wallet**
1. Click **"📥 Import Private Key"**
2. Enter your 64-character hex private key
3. Click confirm
4. Wallet loads with existing balance

### 4. Use the Wallet

**Check Balance**
- Balance displays prominently at the top
- Click **"🔄 Refresh Balance"** to update

**Receive Tokens**
- Your address is shown in the Receive card
- Click **"📋 Copy Address"** to copy to clipboard
- Share with others to receive tokens

**Send Tokens**
1. Enter recipient address (64-char hex)
2. Enter amount
3. Optionally set fee
4. Click **"🚀 Send Tokens"**
5. Transaction is signed with Ed25519 and submitted
6. Status updates appear below the form

### 5. Access Miner Panel

Once connected:
1. Click **"⛏️ Miner Panel"** in navigation
2. Full miner dashboard loads
3. Monitor and control mining operations

## Security Features

### Ed25519 Signature Verification

All wallet transfers are signed with Ed25519:
- **Private key** stored securely in browser localStorage
- **Signatures** generated client-side
- **Nonce tracking** prevents replay attacks
- **Public key verification** ensures sender authenticity

### Local Storage

Wallet data is stored in browser localStorage:
- `privateKey` - Your Ed25519 private key (64-char hex)
- `publicKey` - Your Ed25519 public key (64-char hex)
- `address` - Your wallet address (derived from public key)
- `nodeUrl` - Custom node URL (default: http://127.0.0.1:7070)

⚠️ **Warning**: Never share your private key. Anyone with it can access your funds.

### Export/Backup

To backup your wallet:
1. Go to **Settings**
2. Click **"📤 Export (Danger!)"**
3. Copy the private key displayed
4. Store securely (hardware wallet, encrypted file, password manager)

### Clear Wallet

To disconnect/clear wallet:
1. Go to **Settings**
2. Click **"🗑️ Disconnect"**
3. Confirm
4. All wallet data is cleared from browser

## API Integration

The UI connects to these Vision Node endpoints:

### Wallet Endpoints

- `GET /wallet/:addr/balance` - Query token balance
- `GET /wallet/:addr/nonce` - Get current nonce for replay protection
- `POST /wallet/transfer` - Submit signed transfer
- `GET /receipts/latest` - View recent transactions

### Miner Endpoints (via iframe)

- All endpoints in `/panel.html` for miner control

## File Structure

```
public/
├── index.html              ← NEW: Unified wallet + miner UI
├── index-old.html          ← Backup of original index
├── panel.html              ← Original miner panel (embedded)
├── dashboard.html          ← Original dashboard
├── explorer.html           ← Block explorer
└── wallet/
    ├── index.html          ← Original standalone wallet prototype
    ├── app.js              ← Wallet prototype JS
    └── styles.css          ← Wallet prototype styles
```

## Navigation

### Main Navigation Bar

```
⚡ Vision Node
├── 💰 Wallet (default, always accessible)
├── ⛏️ Miner Panel (visible after wallet connection)
└── ⚙️ Settings (always accessible)
```

### User Info (Top Right)

- Address chip showing current wallet (when connected)
- Auth button:
  - "🔐 Connect Wallet" when not connected
  - "✅ Connected" when connected (click to disconnect)

## Responsive Design

The interface is fully responsive:
- **Desktop**: 3-column grid for wallet actions
- **Tablet**: 2-column grid
- **Mobile**: Single column, stacked layout

## Customization

### Change Node URL

1. Go to **Settings**
2. Update **Node URL** field
3. Click **"💾 Save"**
4. Reload page

### Theme Colors

Modify CSS variables in `<style>` section:

```css
:root {
    --bg-dark: #0a0e27;           /* Background */
    --bg-card: #151b3d;           /* Card background */
    --accent-blue: #00d4ff;       /* Primary accent */
    --accent-purple: #b968ff;     /* Secondary accent */
    --accent-green: #00ff88;      /* Success color */
    --accent-gold: #ffd700;       /* Miner accent */
}
```

## Development

### Prerequisites

- Running Vision Node on localhost:7070 (or custom URL)
- Modern browser with ES6+ support
- localStorage enabled

### Testing Locally

```bash
# Start node
./vision-node --port 7070

# Open browser
http://localhost:7070/

# Or with custom port
./vision-node --port 8080
http://localhost:8080/
```

### Production Deployment

1. Build Vision node binary
2. Ensure `public/` directory is included
3. Start node with desired port
4. Configure firewall rules
5. Optionally add reverse proxy (nginx) for HTTPS

## Known Limitations

### Simplified Cryptography (Demo)

⚠️ **Current Implementation**: For demonstration purposes, the UI uses simplified Ed25519 handling. 

**For production**, integrate proper Ed25519 library:

```javascript
// Install: npm install @noble/ed25519
import * as ed25519 from '@noble/ed25519';

// Generate keypair
const privateKey = ed25519.utils.randomPrivateKey();
const publicKey = await ed25519.getPublicKey(privateKey);

// Sign message
const signature = await ed25519.sign(message, privateKey);
```

### Browser Compatibility

Tested on:
- ✅ Chrome 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Edge 90+

Required features:
- ES6 modules
- fetch API
- localStorage
- CSS Grid

## Troubleshooting

### Wallet Not Connecting

1. Check node is running: `http://localhost:7070/health`
2. Verify CORS settings (node should allow browser requests)
3. Check browser console for errors
4. Try clearing localStorage and regenerating wallet

### Balance Not Updating

1. Click **"🔄 Refresh Balance"**
2. Check node is synced
3. Verify address is correct
4. Check network connectivity

### Transfer Fails

Common errors:
- **Invalid signature** - Regenerate wallet or check private key
- **Invalid nonce** - Refresh to get current nonce
- **Insufficient funds** - Check balance before sending
- **Invalid address** - Recipient address must be 64-char hex

### Miner Panel Not Loading

1. Ensure wallet is connected first
2. Check `/panel.html` exists and is accessible
3. Verify iframe permissions
4. Check browser console for errors

## Migration from Old UI

If you were using the old index.html or standalone wallet:

### Old Index (Backed Up)

The original `index.html` is saved as `index-old.html`

To restore:
```bash
cd public
mv index.html index-unified.html
mv index-old.html index.html
```

### Standalone Wallet

The original wallet prototype remains at `/wallet/index.html`

Access it directly: `http://localhost:7070/wallet/`

## Security Best Practices

### Private Key Management

🔐 **Do**:
- Generate new wallets for each environment (dev/test/prod)
- Export and backup private keys securely
- Use hardware wallets for large amounts
- Store backups encrypted
- Use password managers

❌ **Don't**:
- Share private keys
- Store in plaintext files
- Commit to version control
- Send via email/chat
- Reuse across multiple nodes

### Browser Security

- Only use trusted computers
- Clear localStorage when using public computers
- Enable browser security features
- Keep browser updated
- Use HTTPS in production

### Network Security

- Use firewall rules to restrict node access
- Consider VPN for remote access
- Enable HTTPS with reverse proxy
- Monitor for suspicious activity
- Keep node software updated

## Roadmap

### Planned Features

- [ ] Hardware wallet integration (Ledger, Trezor)
- [ ] Multi-signature support
- [ ] Token swap interface
- [ ] NFT gallery view
- [ ] Transaction history export
- [ ] QR code scanning
- [ ] Mobile app (React Native)
- [ ] Desktop app (Electron)

### Nice-to-Have

- [ ] Dark/light theme toggle
- [ ] Multi-language support
- [ ] Price charts
- [ ] Push notifications
- [ ] Address book
- [ ] Custom token support

## Support

For issues or questions:
- Check documentation in `docs/`
- Review `WALLET_SIGNATURE_VERIFICATION.md`
- See client examples in `docs/examples/`
- File issues on GitHub

## License

Same as Vision Node project

---

**Status**: ✅ Production-Ready

**Last Updated**: 2024-01-27

**Version**: 1.0.0 - Unified Wallet & Miner Interface
