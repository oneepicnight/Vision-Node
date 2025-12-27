@echo off
title Vision Node - Public with Ngrok

echo.
echo ================================================
echo   Vision Node - Public Mode (Ngrok)
echo ================================================
echo.
echo This will expose your node to the internet using Ngrok
echo No router configuration needed!
echo.
echo Starting Vision Node...
start "VISION NODE" cmd /k "%~dp0vision-node.exe"

echo.
echo Waiting 5 seconds for node to start...
timeout /t 5 >nul

echo.
echo Starting Ngrok tunnel for P2P port 7071...
echo.
start "NGROK P2P TUNNEL" cmd /k "%~dp0ngrok.exe tcp 7071"

echo.
echo ================================================
echo   IMPORTANT: Check the NGROK window!
echo ================================================
echo.
echo Look for a line like:
echo   Forwarding: tcp://0.tcp.ngrok.io:12345 -^> localhost:7071
echo.
echo Copy that address (example: 0.tcp.ngrok.io:12345)
echo.
echo Share with miners:
echo   configure-peer.ps1 -PeerIP "0.tcp.ngrok.io" -PeerPort 12345
echo.
echo Check connected peers:
echo   http://localhost:7070/api/tcp_peers
echo.
echo Both windows will stay open. Close them to stop.
echo.
pause
