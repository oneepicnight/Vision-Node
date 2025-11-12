# Vision Wallet - Standalone Quick Start

The Vision Wallet is a **self-contained application** with its own market backend and Vite dev server.

## 🚀 Quick Start (Recommended)

Run the full stack with one command:

```powershell
.\start-wallet-full.ps1
```

This script will:
1. ✅ Install npm dependencies
2. ✅ Build the Rust market backend (port 8080)
3. ✅ Start the market backend in background
4. ✅ Start the Vite dev server (port 4173)
5. ✅ Clean up both servers when you press Ctrl+C

**Access the wallet at:** http://localhost:4173

## 📊 Architecture

```
┌─────────────────────┐
│   Browser (4173)    │
│  Vision Wallet UI   │
└──────────┬──────────┘
           │
           │ HTTP Proxy
           ├──────────────┐
           ▼              ▼
┌─────────────────┐  ┌─────────────────┐
│ Market Backend  │  │  Vision Node    │
│ Rust/Axum(8080) │  │  (REQUIRED)     │
│ - Land Market   │  │  Port 7070      │
│ - Cash Orders   │  │  - Exchange API │
│ - Electrum      │  │  - Blockchain   │
└─────────────────┘  │  - Wallet API   │
                     └─────────────────┘

Land & Cash Orders → Market Backend (8080) ✅
Exchange Trading → Vision Node (7070) 🔗
Wallet/Vault → Vision Node (7070) 🔗
```

## 🔧 Manual Setup (Advanced)

If you want to run servers separately:

### Terminal 1: Market Backend
```powershell
cd wallet-marketplace-source
$env:VISION_PORT = "8080"
cargo run --release
```

### Terminal 2: Vite Dev Server
```powershell
cd wallet-marketplace-source
npm install
npm run dev
```

## 🧪 Testing

Run tests:
```powershell
npm run test:run
```

## 🏗️ Production Build

Build wallet for deployment:
```powershell
npm run build
```

The `dist/` folder will contain the production-ready static files.

## 🔌 API Endpoints

### Market Backend (Port 8080)
- `POST /market/land/list` - Create land listing
- `GET /market/land/listings` - View land parcels
- `GET /cash/rate` - Get exchange rate
- `GET /admin/cash/orders` - Manage cash orders
- `GET /electrum/*` - Crypto payment monitoring

### Vision Node (Port 7070) - Required for Trading
- `GET /api/market/exchange/book?chain=BTC&depth=200` - Order book
- `GET /api/market/exchange/ticker?chain=BTC` - Price ticker
- `POST /api/market/exchange/order` - Place limit order
- `POST /api/market/exchange/buy` - Market buy
- `GET /api/market/exchange/my/orders` - Your orders
- `GET /api/market/exchange/trades` - Recent trades

## 🐛 Troubleshooting

**Port 8080 already in use?**
- Stop any running Vision nodes: `Get-Process | Where-Object { $_.Name -like "*vision*" } | Stop-Process`

**Vite proxy errors?**
- Make sure market backend is running first
- Check logs for "Server listening on http://0.0.0.0:8080"

**Market data not loading?**
- Verify market backend is responding: `Invoke-WebRequest http://127.0.0.1:8080/exchange/ticker`

## 📚 Documentation

- Full API docs: See `docs/` folder
- Market endpoints: See `src/market/` in Rust code
- Frontend components: See `src/` folder

---

## ⚠️ Important: Vision Node Required

**Exchange Trading Requires Vision Node:**
- Order matching engine runs in Vision Node (port 7070)
- Without it, exchange trading will **not work**
- Land marketplace and cash orders work without Vision Node
- For full functionality, run both servers:

```powershell
# Terminal 1: Vision Node
cd c:\vision-node
cargo run --release

# Terminal 2: Wallet
cd wallet-marketplace-source
.\start-wallet-full.ps1
```

**Trading Pairs:** BTC/LAND, BCH/LAND, DOGE/LAND, LAND/LAND
