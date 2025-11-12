# 🚀 VISION WALLET - Installation Guide

## Quick Start (3 Steps)

1. **Extract** this ZIP to a folder (e.g., `C:\VisionWallet\`)
2. **Double-click** `START-WALLET.bat`
3. **Wait** for automatic setup, then wallet opens in browser

---

## First Time Setup

### Required Software

Install these before running the wallet:

1. **Node.js 16+**  
   Download: https://nodejs.org/  
   Choose "LTS" version (recommended)

2. **Rust + Cargo**  
   Download: https://rustup.rs/  
   Follow the installer instructions

3. **Internet Connection**  
   Required for first run to download dependencies

### Installation Steps

1. Extract all files from `VISION-WALLET-STANDALONE.zip`
2. Open the extracted folder
3. Double-click `START-WALLET.bat`

The script will automatically:
- ✅ Check for Node.js and Rust
- ✅ Install npm dependencies (~2-3 minutes)
- ✅ Build the Rust backend (~5 minutes)
- ✅ Start both servers
- ✅ Open wallet in your browser

---

## What Gets Installed

On first run, these folders will be created:

- `node_modules/` - JavaScript dependencies (~200 MB)
- `target/` - Rust compiled code (~500 MB)
- `wallet_data/` - Your wallet database

**Total disk space needed: ~1 GB**

---

## Accessing Your Wallet

Once started, the wallet runs at:

- **Wallet UI**: http://localhost:4173
- **Market Backend**: http://127.0.0.1:8080

The browser should open automatically. If not, manually visit:
```
http://localhost:4173
```

### Full Functionality

**Vision Node Required:**
For exchange trading and blockchain features, Vision Node must be running on port 7070:

```powershell
# Terminal 1: Start Vision Node
cd c:\vision-node
cargo run --release

# Terminal 2: Start Wallet
cd wallet-marketplace-source
.\START-WALLET.bat
```

**What works without Vision Node:**
- ✅ Land marketplace (buy/sell LAND parcels)
- ✅ Cash orders system
- ✅ Electrum payment monitoring

**Requires Vision Node (port 7070):**
- 🔗 Exchange trading (BTC/BCH/DOGE/LAND pairs)
- 🔗 Wallet operations (send/receive LAND)
- 🔗 Vault system (deposits/rewards)
- 🔗 Blockchain queries (balances, receipts)

---

## Startup Options

### Option 1: START-WALLET.bat (Easiest)
```cmd
Double-click START-WALLET.bat
```
This automatically uses the best method for your system.

### Option 2: PowerShell Script
```powershell
powershell -ExecutionPolicy Bypass -File start-wallet-full.ps1
```
Better logging and error messages.

### Option 3: Batch Script
```cmd
start-wallet-full.bat
```
Fallback if PowerShell is restricted.

### Option 4: Manual (Advanced)
```powershell
# Terminal 1 - Backend
$env:VISION_PORT = "8080"
cargo run --release

# Terminal 2 - Frontend
npm install
npm run dev
```

---

## Troubleshooting

### "Port 8080 already in use"
Another program is using port 8080. Find and close it:
```powershell
Get-Process | Where-Object { $_.ProcessName -like "*vision*" } | Stop-Process
```

### "Exchange trading not working"
Vision Node must be running on port 7070:
```powershell
cd c:\vision-node
cargo run --release
```
Without Vision Node, only land marketplace and cash orders work.

### "Node.js not found"
Install Node.js from https://nodejs.org/  
Restart your terminal after installing.

### "Cargo not found"
Install Rust from https://rustup.rs/  
Restart your terminal after installing.

### "npm install failed"
Check internet connection, then:
```cmd
npm cache clean --force
npm install
```

### Firewall blocking connections
Allow these programs through Windows Firewall:
- `node.exe`
- `cargo.exe`
- `vision-node.exe`

### Build takes too long
First build can take 5-10 minutes. This is normal.  
Subsequent starts are much faster (under 30 seconds).

### Browser doesn't open
Manually visit: http://localhost:4173

---

## Stopping the Wallet

Press `Ctrl+C` in the terminal window to stop all servers.

The PowerShell script automatically cleans up background processes.

---

## System Requirements

- **OS**: Windows 10/11
- **RAM**: 4 GB minimum, 8 GB recommended
- **Disk**: 1 GB free space
- **Internet**: Required for first-time setup only

---

## Features

- ✅ Cryptocurrency Exchange (BTC, BCH, DOGE)
- ✅ Real-time Order Book
- ✅ Electrum Balance Watching
- ✅ Cash Order Management
- ✅ Standalone - No external servers needed

---

## Security Notes

- All data stored locally in `wallet_data/`
- No telemetry or external tracking
- Runs completely offline after setup
- Backup `wallet_data/` folder regularly

---

## Getting Help

1. Check this README first
2. Review `STANDALONE-README.md` for detailed docs
3. Check the terminal for error messages
4. Ensure all requirements are installed

---

## File Structure

```
VISION-WALLET-STANDALONE/
├── START-WALLET.bat         ← Double-click this!
├── start-wallet-full.ps1    ← PowerShell version
├── start-wallet-full.bat    ← Batch version
├── package.json             ← Frontend config
├── Cargo.toml               ← Backend config
├── src/                     ← TypeScript source
├── public/                  ← Static assets
├── dist/                    ← Built files
├── README.md                ← This file
└── STANDALONE-README.md     ← Detailed docs
```

---

**Version**: Standalone Edition  
**Build Date**: November 5, 2025  
**Package**: VISION-WALLET-STANDALONE.zip

---

Need more help? See `STANDALONE-README.md` for advanced configuration.
