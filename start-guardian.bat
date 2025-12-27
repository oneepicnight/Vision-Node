@echo off
title Vision Node - Guardian Development
color 0C

echo.
echo ========================================
echo   VISION NODE - GUARDIAN MODE
echo ========================================
echo.
echo Starting guardian node with BEACON ACTIVE...
echo HTTP API: http://localhost:7070
echo P2P Port: 7072
echo.
echo Beacon endpoints:
echo   - /api/beacon/peers
echo   - /api/passport
echo.

REM Set environment variables for Guardian mode
set VISION_GUARDIAN_MODE=true
set BEACON_MODE=active
set VISION_PORT=7070
set VISION_HOST=0.0.0.0
set RUST_LOG=info
set VISION_PUBLIC_DIR=%~dp0public
set VISION_WALLET_DIR=%~dp0wallet\dist

REM Start the node from target/release
"%~dp0target\release\vision-node.exe"

echo.
echo Guardian node stopped.
pause
