@echo off
setlocal enabledelayedexpansion

echo ========================================
echo   VISION WALLET v1.0 - INSTALLER
echo ========================================
echo.
echo This will install Vision Wallet and create
echo desktop shortcuts for easy access.
echo.
echo ========================================
echo.

REM Get the directory where this script is located
set "INSTALL_DIR=%~dp0"
set "INSTALL_DIR=%INSTALL_DIR:~0,-1%"

REM Check if executables exist
if not exist "%INSTALL_DIR%\vision-market.exe" (
    echo ERROR: vision-market.exe not found!
    echo.
    echo Please make sure all files are extracted together.
    echo.
    pause
    exit /b 1
)

if not exist "%INSTALL_DIR%\frontend_server.exe" (
    echo ERROR: frontend_server.exe not found!
    echo.
    echo Please make sure all files are extracted together.
    echo.
    pause
    exit /b 1
)

echo [1/3] Checking installation directory...
echo Location: %INSTALL_DIR%
echo.

REM Create desktop shortcut for Wallet
echo [2/3] Creating desktop shortcut for Vision Wallet...

set "DESKTOP=%USERPROFILE%\Desktop"
set "SHORTCUT=%DESKTOP%\Vision Wallet.lnk"

REM Create VBScript to make the shortcut
set "VBSCRIPT=%TEMP%\CreateWalletShortcut.vbs"

echo Set oWS = WScript.CreateObject("WScript.Shell") > "%VBSCRIPT%"
echo sLinkFile = "%SHORTCUT%" >> "%VBSCRIPT%"
echo Set oLink = oWS.CreateShortcut(sLinkFile) >> "%VBSCRIPT%"
echo oLink.TargetPath = "%INSTALL_DIR%\START-WALLET.bat" >> "%VBSCRIPT%"
echo oLink.WorkingDirectory = "%INSTALL_DIR%" >> "%VBSCRIPT%"
echo oLink.Description = "Vision Wallet - Marketplace & Exchange" >> "%VBSCRIPT%"
echo oLink.IconLocation = "%SystemRoot%\System32\SHELL32.dll,45" >> "%VBSCRIPT%"
echo oLink.Save >> "%VBSCRIPT%"

cscript //nologo "%VBSCRIPT%"
del "%VBSCRIPT%"

if exist "%SHORTCUT%" (
    echo SUCCESS: "Vision Wallet" shortcut created!
) else (
    echo WARNING: Could not create desktop shortcut
    echo You can manually run START-WALLET.bat
)
echo.

REM Optional: Create Start Menu shortcut
echo [3/3] Creating Start Menu shortcut...

set "STARTMENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs"
set "STARTSHORTCUT=%STARTMENU%\Vision Wallet.lnk"

echo Set oWS = WScript.CreateObject("WScript.Shell") > "%VBSCRIPT%"
echo sLinkFile = "%STARTSHORTCUT%" >> "%VBSCRIPT%"
echo Set oLink = oWS.CreateShortcut(sLinkFile) >> "%VBSCRIPT%"
echo oLink.TargetPath = "%INSTALL_DIR%\START-WALLET.bat" >> "%VBSCRIPT%"
echo oLink.WorkingDirectory = "%INSTALL_DIR%" >> "%VBSCRIPT%"
echo oLink.Description = "Vision Wallet - Marketplace & Exchange" >> "%VBSCRIPT%"
echo oLink.IconLocation = "%SystemRoot%\System32\SHELL32.dll,45" >> "%VBSCRIPT%"
echo oLink.Save >> "%VBSCRIPT%"

cscript //nologo "%VBSCRIPT%"
del "%VBSCRIPT%"

if exist "%STARTSHORTCUT%" (
    echo SUCCESS: Start Menu shortcut created!
) else (
    echo WARNING: Could not create Start Menu shortcut
)
echo.

echo ========================================
echo   INSTALLATION COMPLETE!
echo ========================================
echo.
echo Desktop shortcut: "Vision Wallet" created
echo Start Menu: Vision Wallet added
echo.
echo IMPORTANT: Make sure Vision Node is running first!
echo   Vision Node must be on port 7070 for full features.
echo.
echo To start Vision Wallet:
echo   1. Double-click "Vision Wallet" on desktop
echo   OR
echo   2. Search "Vision Wallet" in Start Menu
echo   OR
echo   3. Run START-WALLET.bat directly
echo.
echo After starting, open browser to:
echo   http://localhost:4173
echo.
echo ========================================
echo.
pause
