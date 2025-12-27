========================================
  VISION NODE v0.8.9 - CONSTELLATION
========================================

EASY START: Just double-click "START-PUBLIC-NODE.bat"

========================================

What's Included:
----------------
- vision-node.exe       Main server (~23 MB)
- START-PUBLIC-NODE.bat Easy launcher
- .env                  Configuration file
- README.txt            This file
- Cargo.toml            Package info
- VERSION               Version info (0.8.9)

========================================

Quick Start:
------------
1. Extract ALL files to a folder
2. Double-click: START-PUBLIC-NODE.bat
3. Server starts on: http://127.0.0.1:7070

That's it!

========================================

OPTIONAL: BEACON PEER DISCOVERY
================================

By default, the node runs in standalone mode using manually configured peers.

To enable auto-discovery of constellation peers from a Guardian beacon:

Option 1: Environment variable
   Edit .env file and set:
   BEACON_ENDPOINT=http://<guardian-ip>:7070

   Examples:
   BEACON_ENDPOINT=http://localhost:7070
   BEACON_ENDPOINT=http://192.168.1.100:7070
   BEACON_ENDPOINT=http://visionworld.tech:7070

Option 2: CLI flag
   vision-node.exe --beacon-endpoint http://192.168.1.100:7070

If BEACON_ENDPOINT is not set, the node will run normally in standalone mode
without any warnings. The beacon is optional and only used for peer discovery.

How Beacon Works:
1. Your node makes HTTP requests to the Guardian beacon
2. Guardian responds with a list of active constellation peers
3. Your node connects to discovered peers automatically
4. Peers sync blocks and form the constellation network

No port forwarding needed! The Guardian does NOT need incoming connections
from your node - your node makes outbound HTTP requests to fetch the peer list.

========================================

Test if Running:
----------------
Open PowerShell and run:
  Invoke-RestMethod "http://127.0.0.1:7070/api/status"

Or open browser to:
  http://127.0.0.1:7070

========================================

Web Interface:
--------------
Dashboard: http://127.0.0.1:7070/panel.html
Wallet:    http://127.0.0.1:7070/app

========================================

Trading Pairs:
--------------
- BTC/LAND  (Bitcoin to LAND)
- BCH/LAND  (Bitcoin Cash to LAND)
- DOGE/LAND (Dogecoin to LAND)
- LAND/LAND (LAND to LAND)

========================================

Stopping:
---------
Press Ctrl+C in the terminal window

========================================

Troubleshooting:
----------------
"Port already in use"
  - Another instance is running
  - Close it first with:
    Get-Process *vision* | Stop-Process

"Can't start"
  - Run as Administrator
  - Check Windows Firewall

"File not found"
  - Extract ALL files, not just .exe
  - Keep all files together in one folder

========================================

For Vision Wallet:
------------------
Vision Wallet needs this running on port 7070 for:
- Exchange trading
- Wallet operations
- Vault features

========================================

Version: 0.8.9 - Constellation Testnet
Build: Windows 64-bit
Release Date: November 28, 2025
Size: ~14.85 MB (compressed)

========================================

Network Mode: Constellation
- Full blockchain consensus
- P2P networking & sync
- Optional Guardian beacon integration
- Multi-currency wallet
- REST API
- Web dashboard

========================================