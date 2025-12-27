@echo off
title Vision Node - Public Internet Node

echo.
echo ================================================
echo   Vision Node - Public Internet Node
echo ================================================
echo.
echo Network Configuration:
echo   Local IP:  192.168.1.123
echo   Public IP: 12.74.244.112
echo.
echo Ports:
echo   HTTP API: 7070 (local access)
echo   TCP P2P:  7071 (internet access)
echo.
echo ================================================
echo.

REM Check if vision-node.exe exists
if not exist "%~dp0vision-node.exe" (
    echo ERROR: vision-node.exe not found!
    echo Please ensure vision-node.exe is in the same directory.
    pause
    exit /b 1
)

echo IMPORTANT: Before starting, ensure:
echo   1. Windows Firewall configured (run setup-firewall.ps1 as Admin)
echo   2. Router port forwarding: 7071 -^> 192.168.1.123:7071
echo.
echo Press any key to start the public node...
pause >nul

echo.
echo Starting Vision Node for internet access...
echo.
echo The node will listen on:
echo   - 0.0.0.0:7070 (HTTP API - local only)
echo   - 0.0.0.0:7071 (TCP P2P - internet accessible)
echo.
echo Miners connect to: 12.74.244.112:7071
echo.
echo Check connected peers:
echo   http://localhost:7070/api/tcp_peers
echo.
echo Check logs:
echo   logs\vision-node-*.log
echo.
echo Press Ctrl+C to stop the node
echo ================================================
echo.

REM Start the node (it automatically binds to 0.0.0.0)
vision-node.exe

pause
