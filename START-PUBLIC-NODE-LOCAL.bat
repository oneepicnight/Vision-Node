@echo off
title Vision Node - Local Network Public Seed (TCP P2P)

echo ====================================================
echo Starting Vision Node - Local Network Public Seed
echo ====================================================
echo.
echo IMPORTANT: Update the IP address below!
echo.
echo Current configuration:
echo   HTTP API: 192.168.1.123:7070
echo   TCP P2P:  192.168.1.123:7071
echo.
echo To change IP, edit this .bat file and update:
echo   SET LOCAL_IP=192.168.1.123
echo.
echo ====================================================
echo.

REM Configure your local IP address here
SET LOCAL_IP=192.168.1.123

echo Starting Vision Node...
echo   HTTP API Port: 7070
echo   TCP P2P Port:  7071 (auto-configured as port+1)
echo.
echo Other nodes on your network can connect using configure-peer.ps1:
echo   .\configure-peer.ps1 -PeerIP "%LOCAL_IP%" -PeerPort 7070
echo.
echo Or manually set in config\node_peer_config.toml:
echo   p2p_peer = "%LOCAL_IP%:7071"
echo.
echo Check connected peers:
echo   http://localhost:7070/api/tcp_peers
echo.
echo Press Ctrl+C to stop the node
echo ====================================================
echo.

REM Set environment for HTTP port (P2P will auto-configure as 7071)
SET VISION_PORT=7070

REM Start the public node
vision-node.exe

pause
