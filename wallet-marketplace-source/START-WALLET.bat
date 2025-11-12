@echo off
echo ========================================
echo   VISION WALLET MARKETPLACE v1.0
echo ========================================
echo.
echo Starting wallet marketplace...
echo.
echo Frontend: http://localhost:4173
echo Backend:  http://localhost:8080
echo.
echo IMPORTANT: Vision Node must be running!
echo   (Vision Node on port 7070 for exchange/vault)
echo.
echo ========================================

REM Check if vision-market.exe exists
if not exist "%~dp0vision-market.exe" (
    echo ERROR: vision-market.exe not found!
    echo Make sure all files are extracted together.
    echo.
    pause
    exit /b 1
)

REM Check if dist folder exists
if not exist "%~dp0dist" (
    echo ERROR: dist folder not found!
    echo Make sure all files are extracted together.
    echo.
    pause
    exit /b 1
)

REM Try PowerShell first for better output
where powershell >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    powershell -ExecutionPolicy Bypass -File "%~dp0START-WALLET.ps1"
) else (
    echo PowerShell not found, starting directly...
    echo.
    start "Vision Market Backend" "%~dp0vision-market.exe"
    timeout /t 3 /nobreak >nul
    echo Backend started! Opening frontend...
    echo.
    echo Press Ctrl+C to stop when done.
    pause
)
