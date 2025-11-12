========================================
  VISION WALLET v1.0
  Production Installer Package
========================================

INSTALLATION INSTRUCTIONS:

1. Extract ALL files to a permanent folder
   (e.g., C:\VisionWallet or Documents\VisionWallet)
   
2. Double-click: INSTALL-WALLET.bat

3. Follow the on-screen instructions

4. Done! Use the desktop shortcut to start

========================================

What Gets Installed:
--------------------
- Desktop shortcut: "Vision Wallet" 💰
- Start Menu shortcut: "Vision Wallet"
- Frontend: http://localhost:4173
- Backend: http://localhost:8080

========================================

IMPORTANT - Vision Node Required:
----------------------------------
Vision Wallet NEEDS Vision Node running!

1. Install Vision Node first (VisionNode-v1.0-Installer.zip)
2. Start Vision Node (port 7070)
3. THEN start Vision Wallet

Vision Node provides:
- Exchange trading engine
- Blockchain API
- Vault management

========================================

After Installation:
-------------------
To Start:
  1. Make sure Vision Node is running (port 7070)
  2. Double-click "Vision Wallet" on desktop
  3. Open browser to: http://localhost:4173

To Stop:
  - Press Ctrl+C in the terminal window
  - Or close the terminal window

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

Testing:
--------
After starting, test with:

PowerShell:
  Invoke-RestMethod "http://localhost:8080/health"
  Invoke-RestMethod "http://localhost:4173"

Browser:
  http://localhost:4173 (main wallet UI)
  http://localhost:8080/health (backend status)

========================================

Troubleshooting:
----------------
"Port 8080 already in use"
  - Another instance is running
  - Close it first or change port

"Port 4173 already in use"
  - Frontend already running
  - Close it first

"Can't connect to Vision Node"
  - Vision Node not running on port 7070
  - Start VisionNode-v1.0-Installer.zip FIRST
  - Check http://localhost:7070

"Frontend won't load"
  - Wait 5 seconds after starting
  - Try refreshing browser
  - Check both servers started (2 terminal windows)

"vision-market.exe not found"
  - Extract ALL files, not just some
  - Keep folder structure intact

"Runs but opens Notepad"
  - Don't click .ps1 files directly
  - Use INSTALL-WALLET.bat instead

========================================

Package Contents:
-----------------
- vision-market.exe       Backend server (11.9 MB)
- frontend_server.exe     Frontend server (2.3 MB)
- dist/                   Production UI files
- INSTALL-WALLET.bat      Installer (creates shortcuts)
- START-WALLET.bat        Launcher
- START-WALLET.ps1        PowerShell launcher
- README-WALLET-INSTALL.txt  This file

========================================

Uninstalling:
-------------
1. Delete desktop shortcut "Vision Wallet"
2. Delete Start Menu shortcut (if created)
3. Delete the Vision Wallet folder
4. No registry changes - clean removal!

========================================

Technical Details:
------------------
Backend:  Rust + Axum framework (port 8080)
Frontend: React + TypeScript + Vite (port 4173)
Database: SQLite (auto-created)
Size:     ~5.7 MB compressed, 14+ MB installed

Requires:
- Vision Node on port 7070
- Windows 10 or later
- Modern web browser

Features:
- LAND marketplace with order matching
- Exchange trading (4 pairs)
- Cash order management
- Wallet transaction handling
- Vault epoch rewards
- Electrum blockchain watchers

========================================

Token Update (v1.0):
--------------------
All trading pairs now use LAND token:
  ✓ BTC/LAND  (was BTC/VISION)
  ✓ BCH/LAND  (was BCH/VISION)
  ✓ DOGE/LAND (was DOGE/VISION)
  ✓ LAND/LAND (was LAND/VISION)

========================================

Version: 1.0 (Production Release)
Date: November 7, 2025
License: Proprietary

Works with:
  VisionNode-v1.0-Installer.zip (required)

========================================

QUICK START:
------------
1. Install & Start Vision Node first
2. Run INSTALL-WALLET.bat
3. Double-click "Vision Wallet" desktop icon
4. Open http://localhost:4173 in browser
5. Start trading! 🚀

========================================
