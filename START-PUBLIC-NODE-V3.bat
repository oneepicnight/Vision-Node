@echo off
echo ========================================
echo   VISION NODE v3.0.0 - CONSTELLATION
echo   Pure Mining System - No PoW Required!
echo ========================================
echo.

REM Check if vision_data exists, if not show first-time setup message
if not exist "vision_data_7070" (
    echo [FIRST TIME SETUP]
    echo Creating vision_data_7070 directory...
    echo This testnet runs for 11 days (475,200 blocks)
    echo.
)

echo Starting Vision Node...
echo.
echo Web Interface:  http://127.0.0.1:7070
echo Miner Panel:    http://127.0.0.1:7070/panel.html
echo Wallet:         http://127.0.0.1:7070/app
echo P2P Status:     http://127.0.0.1:7070/api/p2p/status
echo.
echo [PURE MINING SYSTEM]
echo âœ… No PoW required for rewards!
echo ðŸŽ° Block rewards randomly distributed to synced nodes
echo ðŸ”— Link your wallet via panel to participate
echo ðŸŒŠ Stay synced within 2 blocks to be eligible
echo.
echo [TESTNET DURATION]
echo â±ï¸  11 days (475,200 blocks at 2 sec/block)
echo ðŸŒ… Expires at block 475,210
echo.
echo [AUTO-DETECTION]
echo Your node will automatically detect its role:
echo - ANCHOR (backbone) if publicly reachable with 3+ peers
echo - EDGE (regular) otherwise - no port forwarding needed!
echo.
echo Optional "Make Fans Go Brr" mining = satisfaction only!
echo.
echo Press Ctrl+C to stop the node
echo ========================================
echo.

"%~dp0vision-node.exe"

REM Keep window open if node crashes
if errorlevel 1 pause

