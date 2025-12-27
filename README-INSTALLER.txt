========================================
  VISION NODE v1.0
  Production Installer Package
========================================

INSTALLATION INSTRUCTIONS:

1. Extract ALL files to a permanent folder
   (e.g., C:\VisionNode or Documents\VisionNode)
   
2. Double-click: INSTALL-VISION-NODE.bat

3. Follow the on-screen instructions

4. Done! Use the desktop shortcut to start

========================================

What Gets Installed:
--------------------
- Desktop shortcut: "Vision Node"
- Start Menu shortcut: "Vision Node"
- Server runs on: http://127.0.0.1:7070

========================================

After Installation:
-------------------
To Start:
  - Double-click "Vision Node" on desktop
  - Or search "Vision Node" in Start Menu
  
To Stop:
  - Press Ctrl+C in the terminal window
  - Or close the terminal window

========================================

What Vision Node Does:
-----------------------
1. Exchange Trading Engine
   - BTC/LAND trading pair
   - BCH/LAND trading pair
   - DOGE/LAND trading pair
   - LAND/LAND trading pair
   
2. Blockchain API
   - Block management
   - Transaction processing
   - Network synchronization
   
3. Wallet & Vault Services
   - Wallet operations
   - Vault management
   - Epoch rewards

========================================

Required By:
------------
Vision Wallet Marketplace needs this running!
  - Start Vision Node FIRST
  - Then start Vision Wallet
  - Wallet connects to port 7070

========================================

Testing:
--------
After starting, test with:

PowerShell:
  Invoke-RestMethod "http://127.0.0.1:7070/status"

Browser:
  http://127.0.0.1:7070

Should show server info and status.

========================================

Troubleshooting:
----------------
"Port 7070 already in use"
  - Another instance is running
  - Stop it first:
    Get-Process *vision* | Stop-Process
    
"Can't create shortcut"
  - Run INSTALL-VISION-NODE.bat as Administrator
  - Or manually run START-VISION-NODE.bat
  
"vision-node.exe not found"
  - Extract ALL files to the same folder
  - Don't move files separately
  
"Windows Defender blocks it"
  - Click "More info" → "Run anyway"
  - Or add folder to Windows Defender exclusions

========================================

Package Contents:
-----------------
- vision-node.exe           Main server (17 MB)
- INSTALL-VISION-NODE.bat   Installer (creates shortcuts)
- START-VISION-NODE.bat     Launcher (auto-creates config)
- START-VISION-NODE.ps1     PowerShell launcher (auto-creates config)
- README-INSTALLER.txt      This file
- VERSION                   Version info
- CHANGELOG-v1.0-LAND.md    Update notes

Note: config/token_accounts.toml is created automatically
on first run if not present!

========================================

Uninstalling:
-------------
1. Delete desktop shortcut "Vision Node"
2. Delete Start Menu shortcut (if created)
3. Delete the Vision Node folder
4. No registry changes - clean removal!

========================================

Technical Details:
------------------
Server:     Rust + Axum framework
Port:       7070 (HTTP)
Database:   RocksDB (auto-created)
Size:       ~6.6 MB compressed, 17+ MB installed

Features:
- Exchange order matching engine
- Blockchain consensus & validation
- Wallet transaction management
- Vault epoch-based rewards
- P2P network synchronization
- Compact block relay

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

For Vision Wallet integration, see:
  VisionWallet-v1.0-Production.zip

========================================

NEED HELP?
----------
1. Read this file completely
2. Check CHANGELOG-v1.0-LAND.md
3. Ensure Vision Node starts on port 7070
4. Test with browser: http://127.0.0.1:7070

========================================
