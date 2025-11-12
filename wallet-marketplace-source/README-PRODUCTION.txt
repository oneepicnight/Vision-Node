========================================
  VISION WALLET MARKETPLACE v1.0
  Production Release
========================================

QUICK START: Double-click "START-WALLET.bat"

========================================

What's Included:
----------------
- vision-market.exe      Market backend server
- dist/                  Production frontend (HTML/JS/CSS)
- START-WALLET.bat       Easy launcher (double-click!)
- START-WALLET.ps1       PowerShell version
- README-PRODUCTION.txt  This file

========================================

Installation:
-------------
1. Extract ALL files to a folder
2. Make sure Vision Node is running (port 7070)
3. Double-click: START-WALLET.bat
4. Open browser to: http://localhost:4173

That's it!

========================================

IMPORTANT - Vision Node Required:
----------------------------------
This wallet NEEDS Vision Node running for:
- Exchange trading (BTC/BCH/DOGE to LAND)
- Wallet operations
- Vault features

Download Vision Node separately:
- VisionNode-v1.0.zip (in same location)
- Start it FIRST before starting wallet

========================================

What You Get:
-------------
Frontend: http://localhost:4173
  - React-based wallet interface
  - Trading dashboard
  - Market listings

Backend: http://localhost:8080
  - LAND marketplace API
  - Cash order management
  - Electrum watchers
  
Vision Node: http://localhost:7070 (separate)
  - Exchange engine
  - Blockchain API
  - Vault management

========================================

Features:
---------
1. LAND Marketplace
   - Buy/Sell LAND tokens
   - Create market orders
   - Browse listings

2. Exchange Trading (Requires Vision Node!)
   - BTC/LAND pair
   - BCH/LAND pair
   - DOGE/LAND pair
   - LAND/LAND pair
   - Limit & Market orders

3. Cash Orders
   - Sell LAND for cash
   - Create cash listings
   - Manage orders

4. Wallet Operations (Requires Vision Node!)
   - Send/Receive LAND
   - Transaction history
   - Address management

5. Vault Features (Requires Vision Node!)
   - Epoch-based rewards
   - Stake LAND tokens
   - View vault status

========================================

Testing Backend:
----------------
Open PowerShell and run:
  Invoke-RestMethod "http://localhost:8080/health"

Should return: {"status":"healthy"}

========================================

Testing Frontend:
-----------------
Open browser to:
  http://localhost:4173

You should see the wallet interface

========================================

Stopping:
---------
1. Close the browser
2. Press Ctrl+C in the terminal window
3. Or close the terminal window

========================================

Troubleshooting:
----------------
"Port 8080 already in use"
  - Another instance is running
  - Close it first or change port

"Can't connect to Vision Node"
  - Vision Node not running
  - Start VisionNode-v1.0.zip first
  - Check it's on port 7070

"Frontend won't load"
  - Wait 5 seconds after starting
  - Try refreshing browser
  - Check http://localhost:4173

"vision-market.exe not found"
  - Extract ALL files, not just some
  - Keep folder structure intact

"Runs but opens Notepad"
  - Don't click .ps1 files directly
  - Use START-WALLET.bat instead

========================================

Technical Details:
------------------
Backend:  Rust + Axum framework
Frontend: React + TypeScript + Vite
Database: SQLite (auto-created)
Size:     ~8 MB compressed

Ports Used:
- 4173: Frontend (wallet UI)
- 8080: Backend (market API)
- 7070: Vision Node (exchange/vault) - SEPARATE

========================================

Token Update:
-------------
Version 1.0 corrects all trading pairs to use LAND:
- Old: BTC/VISION, BCH/VISION, DOGE/VISION
- New: BTC/LAND, BCH/LAND, DOGE/LAND ✓

All documentation and code updated.

========================================

Version: 1.0 (Production)
Date: November 7, 2025
Size: ~8 MB (compressed)

For support or issues, check documentation
in the wallet-marketplace-source folder.

========================================
