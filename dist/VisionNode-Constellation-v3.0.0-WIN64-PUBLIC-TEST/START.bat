@echo off
REM VisionNode v3.0.0 Quick Start
REM Double-click this file to start your node

echo.
echo ========================================
echo   VisionNode v3.0.0 - Starting...
echo ========================================
echo.

if not exist "%~dp0vision-node.exe" (
    echo ERROR: vision-node.exe not found!
    echo Make sure this batch file is in the same folder as vision-node.exe
    pause
    exit /b 1
)

echo Starting VisionNode...
echo.
echo Wallet and Dashboard will be available at:
echo   http://localhost:7070
echo.
echo Press Ctrl+C to stop the node
echo.

"%~dp0vision-node.exe"

pause
