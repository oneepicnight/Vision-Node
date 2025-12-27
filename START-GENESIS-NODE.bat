@echo off
echo ========================================
echo   VISION NODE v3.0.0 - GENESIS LAUNCHER
echo   ðŸŒŸ FIRST NODE - TESTNET INITIALIZATION
echo ========================================
echo.
echo [GENESIS NODE CONFIGURATION]
echo âœ… Skip bootstrap - YOU are the first node!
echo âœ… Start producing blocks immediately
echo âœ… Accept incoming connections from other nodes
echo âœ… Testnet runs for 11 days (475,200 blocks)
echo.

REM Check if vision_data exists
if not exist "vision_data_7070" (
    echo [FIRST TIME SETUP]
    echo Creating vision_data_7070 directory...
    echo.
)

REM Set genesis configuration as environment variables
echo âœ… Loading genesis configuration...
set VISION_GUARDIAN_MODE=false
set BEACON_MODE=passive
set VISION_PORT=7070
set VISION_HOST=0.0.0.0
set RUST_LOG=info

REM GENESIS MODE - Skip bootstrap!
set VISION_PURE_SWARM_MODE=false
set VISION_BEACON_BOOTSTRAP=false
set VISION_GUARDIAN_RELAY=false
set VISION_MIN_PEERS_FOR_MINING=0
set VISION_MAX_PEERS=50
set VISION_MIN_HEALTHY_CONNECTIONS=0
set VISION_SWARM_INTELLIGENCE=false

echo âœ… Genesis mode enabled - will skip bootstrap
echo.

echo Starting Genesis Node...
echo.
echo Web Interface:  http://127.0.0.1:7070
echo Miner Panel:    http://127.0.0.1:7070/panel.html
echo Wallet:         http://127.0.0.1:7070/app
echo P2P Status:     http://127.0.0.1:7070/api/p2p/status
echo.
echo [GENESIS NODE STATUS]
echo ðŸŒ± This is the FIRST node - no peers required
echo ðŸ”— Other nodes will connect TO YOU
echo ðŸ“¦ Block production starts immediately
echo ðŸŒŠ Pure mining rewards begin when peers sync
echo.
echo [SHARE WITH TESTERS]
echo Your IP:PORT will be the seed peer for others
echo Check your public IP and share: YOUR_IP:7072
echo.
echo Press Ctrl+C to stop the node
echo ========================================
echo.

"%~dp0target\release\vision-node.exe"

REM Keep window open if node crashes
if errorlevel 1 pause

