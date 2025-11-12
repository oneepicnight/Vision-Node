# Vision Wallet Marketplace - Quick Reference

## 🚀 Quick Start

### Start Both Servers (Full Functionality)
```powershell
# Terminal 1: Vision Node (REQUIRED for exchange trading)
cd c:\vision-node
cargo run --release

# Terminal 2: Wallet (market backend + frontend)
cd wallet-marketplace-source
.\START-WALLET.bat
```

**Access:** http://127.0.0.1:4173

---

## 🔌 Port Reference

| Port | Service | Purpose |
|------|---------|---------|
| **4173** | Frontend | React + Vite UI |
| **7070** | Vision Node | Exchange + Blockchain + Wallet |
| **8080** | Market Backend | Land + Cash + Electrum |

---

## 📊 Feature Map

### Exchange Trading (Vision Node - 7070)
- **Trading Pairs:** BTC/LAND, BCH/LAND, DOGE/LAND, LAND/LAND
- **Order Types:** Limit (GTC/IOC/FOK/GTT), Market, Post-only
- **Features:** Order book, trades, ticker, WebSocket

### Land Marketplace (Market Backend - 8080)
- **Create Listings:** POST /market/land/list
- **View Parcels:** GET /market/land/listings
- **Payment:** BTC/BCH/DOGE via Electrum monitoring

### Cash Orders (Market Backend - 8080)
- **Rate:** GET /cash/rate
- **Admin:** GET /admin/cash/orders (requires X-Admin-Token)

---

## 🔧 Configuration Files

### vite.config.ts
Proxy routing configuration:
- `/api/market/exchange/*` → Vision Node (7070)
- `/electrum/*`, `/cash_order/*`, `/admin/cash/*` → Market Backend (8080)
- `/wallet/*`, `/vault/*`, `/balance/*` → Vision Node (7070)

### vision.toml (Market Backend)
Electrum server configuration:
```toml
btc_host = "your.electrum.server"
btc_port = 50001
btc_conf = 1
```

### Environment Variables
```powershell
$env:VISION_PORT = "8080"         # Market backend port
$env:ADMIN_TOKEN = "secret123"    # Admin endpoints auth
```

---

## 🧪 Testing Endpoints

### Vision Node (7070)
```powershell
# Order book
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/book?chain=BTC&depth=10"

# Ticker
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/ticker?chain=BTC"

# Trades
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/trades?chain=BTC&limit=10"
```

### Market Backend (8080)
```powershell
# Exchange rate
Invoke-RestMethod "http://127.0.0.1:8080/cash/rate"

# Land listings
Invoke-RestMethod "http://127.0.0.1:8080/market/land/listings"
```

---

## ⚠️ Troubleshooting

### Exchange not working
**Problem:** Order book empty, trades fail  
**Solution:** Start Vision Node on port 7070
```powershell
cd c:\vision-node
cargo run --release
```

### Port already in use
**Problem:** "Address already in use"  
**Solution:** Kill existing process
```powershell
Get-Process | Where-Object { $_.ProcessName -like "*vision*" } | Stop-Process
```

### Build errors
**Problem:** Cargo build fails  
**Solution:** Clean and rebuild
```powershell
cargo clean
cargo build --release
```

---

## 📁 Important Directories

```
wallet-marketplace-source/
├── src/                    # TypeScript frontend source
│   ├── modules/exchange/   # Exchange UI components
│   └── market/             # Rust backend (land, cash, watchers)
├── target/                 # Rust compiled binaries (~500MB)
├── node_modules/           # Node.js dependencies (~200MB)
├── wallet_data/market/     # Sled database (persistent storage)
├── dist/                   # Production build output
└── vite.config.ts          # Proxy configuration
```

---

## 🔐 Security Notes

- **Admin Token:** Set `ADMIN_TOKEN` env var to protect admin endpoints
- **No Auth:** Frontend uses hardcoded demo user ("demo-user-1")
- **DEV Mode:** Warning shown if ADMIN_TOKEN not set

---

## 📚 Documentation Files

- **MARKETPLACE_ASSESSMENT.md** - Complete feature inventory and architecture
- **STANDALONE-README.md** - Quick start guide
- **INSTALL.md** - Installation and setup instructions
- **QUICK_REFERENCE.md** - This file (quick lookup)

---

## 🎯 Quick Commands

```powershell
# Build production frontend
npm run build

# Run tests
npm run test:run

# Format TypeScript
npm run format

# Build Rust backend
cargo build --release

# Run backend only
cargo run --release

# Clean builds
cargo clean
rm -rf node_modules dist
```

---

## 📞 Support

For issues:
1. Check Vision Node is running (port 7070)
2. Check Market Backend is running (port 8080)
3. Check browser console for errors (F12)
4. Review logs in terminal windows
5. Verify vite.config.ts proxy settings

---

**Token Name:** LAND (not VISION)  
**Trading Pairs:** BTC/LAND, BCH/LAND, DOGE/LAND, LAND/LAND  
**Architecture:** Dual-backend (Vision Node + Market Backend)
