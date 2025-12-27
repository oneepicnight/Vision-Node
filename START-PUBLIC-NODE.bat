@echo off
title Vision Node - Public Mode

echo.
echo ================================================
echo   Vision Node v2.0.0 - Constellation
echo   HTTP API: localhost:7070
echo   P2P Port: 7072
echo   UPnP: Enabled (auto port-forward)
echo   Panel: http://localhost:7070/panel.html
echo ================================================
echo.

REM Check if vision-node.exe exists
if not exist "%~dp0vision-node.exe" (
    echo ERROR: vision-node.exe not found!
    echo Please ensure vision-node.exe is in the same directory.
    pause
    exit /b 1
)

REM Set environment variables for production
set VISION_GUARDIAN_MODE=false
set VISION_UPSTREAM_HTTP_BASE=https://visionworld.tech

echo Starting Vision Node in Constellation Mode...
echo.
echo Connecting to Guardian beacon for peer discovery...
echo (Beacon endpoint configured in .env file)
echo Upstream: %VISION_UPSTREAM_HTTP_BASE%
echo.

"%~dp0vision-node.exe"

pause
