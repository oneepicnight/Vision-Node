# Vision Wallet v1.0 - Installation Guide

## Package Information
- **Version**: 1.0
- **Size**: 5.69 MB
- **Platform**: Windows 10/11

## What's Included
- Vision Wallet UI (React + TypeScript)
- Backend API Server (vision-market.exe)
- Frontend Server (frontend_server.exe)
- Desktop Shortcut Installer
- Startup Scripts

## Installation Steps

### Quick Install
1. **Extract** the ZIP file to your desired location
2. **Run** `INSTALL-WALLET.bat` (creates desktop shortcut)
3. **Launch** using "Vision Wallet" desktop shortcut
4. **Open** your browser to `http://localhost:4173`

### Manual Start
If you prefer not to install the shortcut:
1. Double-click `START-WALLET.bat`
2. Wait for both servers to start
3. Open `http://localhost:4173` in your browser

## Features

> ⚠️ MANDATORY: Client-side signing MUST be enforced for mainnet. Never use server-side signing in production environments. See `docs/WALLET_SIGNATURE_VERIFICATION.md` for canonical signing format and examples.

### Wallet Management
- ✅ Create new wallets with mnemonic phrases (BIP39)
- ✅ Import existing wallets
- ✅ Secure key storage
- ✅ Handle (username) claiming
- ✅ Multiple wallet support

### Connectivity
- ✅ Works standalone (offline mode)
- ✅ Auto-connects to Vision Node when available
- ✅ Graceful offline/online transitions
- ✅ Real-time status indicator

### Market Features
- ✅ Token marketplace integration
- ✅ Exchange interface
- ✅ Balance tracking
- ✅ Transaction history

## System Requirements
- **OS**: Windows 10 or Windows 11
- **RAM**: 2 GB minimum
- **Disk Space**: 50 MB
- **Browser**: Chrome, Edge, Firefox, or any modern browser
- **Ports**: 4173 (frontend), 8080 (backend) must be available

## Troubleshooting

### Wallet Won't Start
- Check if ports 4173 and 8080 are available
- Run as Administrator if needed
- Check Windows Firewall settings

### "Node Offline" Banner
- This is normal if Vision Node is not running
- Wallet still works in offline mode
- Install Vision Node separately to enable full features

### Can't Access UI
- Ensure both servers started (check PowerShell windows)
- Try `http://127.0.0.1:4173` instead of localhost
- Clear browser cache and refresh
- Check antivirus isn't blocking the servers

### JavaScript Errors
- This version includes all necessary polyfills
- If errors occur, please report with browser console output

## Connecting to Vision Node
To enable full blockchain features:
1. Install Vision Node (separate package)
2. Start Vision Node with CORS enabled
3. Wallet will auto-detect and connect
4. "Node Offline" banner will disappear

## Technical Details

### Architecture
- **Frontend**: React 18 + TypeScript + Vite
- **Backend**: Rust + Axum web framework
- **Crypto**: @noble/secp256k1, bip39
- **Styling**: Tailwind CSS

### API Endpoints
- Frontend UI: `http://localhost:4173`
- Backend API: `http://localhost:8080`
- Default Node: `http://127.0.0.1:7070` (if available)

### Security
- Keys stored locally (never transmitted)
- BIP39 mnemonic generation
- Secp256k1 elliptic curve cryptography
- Local-only by default

## Uninstallation
1. Delete desktop shortcut
2. Close wallet servers (Task Manager if needed)
3. Delete wallet folder

## Support & Updates
- Check for updates regularly
- Report issues with detailed error messages
- Keep backup of your mnemonic phrases

## Version History
**v1.0 (November 8, 2025)**
- Initial release
- Full wallet functionality
- Standalone mode support
- Market/Exchange interface
- BIP39 mnemonic generation
- Secure key management

---

**Important**: Always keep your mnemonic phrase secure and backed up. Loss of the phrase means permanent loss of wallet access.
