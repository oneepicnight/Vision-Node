@echo off
REM ========================================================================
REM   VISION GUARDIAN NODE
REM   Canonical Guardian Sentinel - Default Owner: Discord ID 309081088960233492
REM ========================================================================

title Vision Guardian Node

REM Change to the directory where this batch file is located
cd /d "%~dp0"

echo ========================================================================
echo   Vision Guardian Node
echo ========================================================================
echo.
echo This is the GUARDIAN NODE for Vision Network.
echo The Guardian watches over the constellation and knows its owner.
echo.
echo Canonical Owner: Discord ID 309081088960233492 (Donnie)
echo.

REM Check if vision-node.exe exists
if not exist "vision-node.exe" (
    echo [ERROR] vision-node.exe not found!
    echo.
    pause
    exit /b 1
)

REM Set environment variables
set VISION_PUBLIC_DIR=%CD%\public
set VISION_WALLET_DIR=%CD%\wallet\dist
set VISION_GUARDIAN_MODE=true
set VISION_UPSTREAM_HTTP_BASE=https://visionworld.tech
set BEACON_MODE=active
set VISION_HOST=0.0.0.0
set VISION_PORT=7070
set RUST_LOG=info

REM Guardian owner Discord ID defaults to 309081088960233492 (hardcoded in binary)
REM You can override by setting GUARDIAN_OWNER_DISCORD_ID environment variable
REM You can set wallet address via GUARDIAN_OWNER_WALLET_ADDRESS environment variable

echo Configuration:
if defined GUARDIAN_OWNER_DISCORD_ID (
    echo - Guardian Owner Discord: %GUARDIAN_OWNER_DISCORD_ID% ^(from environment^)
) else (
    echo - Guardian Owner Discord: 309081088960233492 ^(default - canonical owner^)
)

if defined GUARDIAN_OWNER_WALLET_ADDRESS (
    echo - Guardian Owner Wallet: %GUARDIAN_OWNER_WALLET_ADDRESS% ^(from environment^)
) else (
    echo - Guardian Owner Wallet: 0x30ea8826a5f42966a4a5fabd49d1c2ee2472023e ^(default - canonical owner^)
)

echo.
echo Starting Guardian Node...
echo.
echo   Wallet: http://127.0.0.1:7070/app
echo   Panel:  http://127.0.0.1:7070/panel.html
echo   Status: http://127.0.0.1:7070/status
echo.

REM Launch vision-node (Constellation v2.0.0 with UPnP, reward gating, per-peer ports)
vision-node.exe

REM If the node exits, pause so user can see any error messages
echo.
echo.
echo ========================================================================
echo   GUARDIAN NODE STOPPED
echo ========================================================================
echo.
pause
