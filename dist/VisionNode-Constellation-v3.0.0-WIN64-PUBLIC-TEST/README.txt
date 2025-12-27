=========================================
  VisionNode v3.0.0 - Public Test Build
=========================================

QUICK START
-----------
1. Double-click:  START.bat
2. Open: http://localhost:7070

That's it! The node will:
  - Auto-detect your external IP
  - Connect to genesis seed peers
  - Start syncing the blockchain
  - Serve wallet UI on port 7070

Alternative: .\run-wan.ps1 (PowerShell with more options)


WHAT'S INCLUDED
---------------
vision-node.exe      - Main node executable
run-wan.ps1          - Quick launcher script
env-public.ps1       - Optional config template
seed_peers.json      - Genesis P2P seeds (port 7072)
public/              - Web UI (wallet, dashboard, panel)
Cargo.toml/lock      - Build manifest (for reference)


WEB INTERFACES
--------------
After starting the node:

🔐 Wallet:      http://localhost:7070
   Create wallet, send transactions, check balance

📊 Dashboard:   http://localhost:7070/dashboard.html
   Network stats, block height, peer count

⚙️  Panel:       http://localhost:7070/panel.html
   Node control, mining setup, approval status


ADVANCED USAGE
--------------
Custom ports:
  .\run-wan.ps1 -HttpPort 8080 -P2pPort 8082

Set public IP manually (if auto-detect fails):
  .\run-wan.ps1 -PublicIp "203.0.113.50" -PublicPort 7072

Override anchor seeds:
  .\run-wan.ps1 -AnchorSeeds "35.151.236.81,16.163.123.221"

Custom seed peers file:
  .\run-wan.ps1 -SeedPeersPath "C:\path\to\seed_peers.json"


FIREWALL / NAT
--------------
For incoming P2P connections:
  - Open port 7072 (TCP) in firewall
  - Forward port 7072 to this machine if behind NAT
  - Set VISION_PUBLIC_IP if external IP differs from detected

HTTP API (port 7070) does NOT need external access unless
you want remote wallet/dashboard access.


PORTS EXPLAINED
---------------
7070 - HTTP API + Web UI (wallet, dashboard)
7072 - P2P mesh networking (blockchain sync, peer discovery)


ENVIRONMENT VARIABLES
---------------------
Edit env-public.ps1 or set before running:

VISION_PORT              - HTTP port (default: 7070)
VISION_P2P_PORT          - P2P port (default: 7072)
VISION_PUBLIC_IP         - External IP for P2P advertisement
VISION_PUBLIC_PORT       - External port for P2P advertisement
VISION_ANCHOR_SEEDS      - HTTP anchor seeds (comma-separated)


SUPPORT & DOCS
--------------
Web:     https://vision-node.network
Discord: https://discord.gg/vision
Docs:    https://docs.vision-node.network


TROUBLESHOOTING
---------------
Q: "Could not detect external IP"
A: Set manually: .\run-wan.ps1 -PublicIp "YOUR_IP"

Q: "No peers connecting"
A: Check firewall allows port 7072, verify NAT forwarding

Q: "Can't access wallet"
A: Ensure node is running, try http://127.0.0.1:7070

Q: "Sync is slow"
A: Normal for first run. Check connected peers in dashboard.


LICENSE
-------
See Cargo.toml for license information.
Build date: December 22, 2025
