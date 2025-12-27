@echo off
title Vision Node - Public Mode (localhost.run TCP Tunnel)

echo.
echo ================================================
echo   Vision Node - Public Mode (TCP P2P)
echo   Using localhost.run (Free TCP Tunnel)
echo   HTTP API: localhost:7070
echo   P2P Port: 7071 (exposed via SSH tunnel)
echo ================================================
echo.

REM Check if vision-node.exe exists
if not exist "%~dp0vision-node.exe" (
    echo ERROR: vision-node.exe not found!
    echo Please ensure vision-node.exe is in the same directory.
    pause
    exit /b 1
)

REM Check if SSH is available
where ssh >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: SSH not found.
    echo SSH should be available on Windows 10/11.
    echo If not, enable it in: Settings ^> Apps ^> Optional Features ^> OpenSSH Client
    pause
    exit /b 1
)

echo Starting Vision Node...
echo   HTTP API: localhost:7070
echo   TCP P2P:  0.0.0.0:7071
echo.
start "VISION NODE" cmd /k "%~dp0vision-node.exe"

echo.
echo Waiting 5 seconds for node startup...
timeout /t 5 >nul

echo.
echo Starting FREE TCP tunnel via localhost.run...
echo This will expose P2P port 7071 to the internet for FREE!
echo.

REM Start localhost.run TCP tunnel for P2P port
start "FREE P2P TUNNEL" cmd /k "ssh -o StrictHostKeyChecking=no -R 0:localhost:7071 nokey@localhost.run"

echo.
echo ================================================
echo   PUBLIC NODE IS STARTING
echo ================================================
echo.
echo IMPORTANT: Check the "FREE P2P TUNNEL" window!
echo.
echo You'll see output like:
echo   Connect to serveo.net at following TCP address:
echo   tcp://serveo.net:12345
echo.
echo Or from localhost.run:
echo   Forwarding TCP traffic from serveo.net:12345
echo.
echo Copy that address and share with miners.
echo.
echo Example: If you see "serveo.net:12345", miners configure:
echo   .\configure-peer.ps1 -PeerIP "serveo.net" -PeerPort 12345
echo.
echo Or manually in config\node_peer_config.toml:
echo   p2p_peer = "serveo.net:12345"
echo.
echo Check connected peers:
echo   http://localhost:7070/api/tcp_peers
echo.
echo Check handshake logs in: logs\vision-node-*.log
echo   Look for: "Handshake validation successful"
echo.
echo The tunnel is FREE and unlimited!
echo.
pause

echo.
echo To stop: Close both windows (Vision Node and Free P2P Tunnel)
echo.
pause
