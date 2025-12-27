@echo off@echo off

REM Vision Node - Production LauncherREM Vision Node - Quick Start Launcher

REM Double-click to start the nodeREM Double-click this file to start Vision Node



echo.echo.

echo ================================echo ================================

echo   Vision Node - Starting...echo   Vision Node Launcher

echo ================================echo ================================

echo.echo.



REM Check if executable existsREM Try PowerShell first

if not exist "%~dp0vision-node.exe" (where powershell >nul 2>nul

    echo ERROR: vision-node.exe not found!if %ERRORLEVEL% EQU 0 (

    echo Please run cargo build --release first    echo Starting with PowerShell...

    pause    powershell -ExecutionPolicy Bypass -File "%~dp0START-VISION-NODE.ps1"

    exit /b 1    goto :end

))



REM Create config directory if neededREM Fallback: Try running executable directly

if not exist "%~dp0config" mkdir "%~dp0config"echo PowerShell not found, running directly...



REM Create default config if missingREM Check for config directory

if not exist "%~dp0config\token_accounts.toml" (if not exist "%~dp0config" (

    echo Creating default config/token_accounts.toml...    echo Creating config directory...

    (    mkdir "%~dp0config"

        echo # System accounts)

        echo vault_address = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

        echo fund_address  = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"REM Check for config file and create if missing

        echo.if not exist "%~dp0config\token_accounts.toml" (

        echo # Founders    echo Creating default config/token_accounts.toml...

        echo founder1_address = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"    (

        echo founder2_address = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"        echo # System accounts

        echo.        echo vault_address = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"   # staking vault

        echo # Split ratios        echo fund_address  = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"   # ecosystem/fund

        echo vault_pct = 50        echo.

        echo fund_pct  = 30        echo # Founders ^(Treasury split^)

        echo treasury_pct = 20        echo founder1_address = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" # Donnie

        echo founder1_pct = 50        echo founder2_address = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee" # Travis

        echo founder2_pct = 50        echo.

    ) > "%~dp0config\token_accounts.toml"        echo # Split ratios ^(integers summing to 100^)

)        echo vault_pct = 50

        echo fund_pct  = 30

REM Start the node        echo treasury_pct = 20

echo Starting Vision Node on port 8080...        echo.

"%~dp0vision-node.exe"        echo # Treasury sub-split between founders ^(sum to 100^)

        echo founder1_pct = 50

REM Pause if there was an error        echo founder2_pct = 50

if %ERRORLEVEL% NEQ 0 (    ) > "%~dp0config\token_accounts.toml"

    echo.    echo Config file created successfully!

    echo Node stopped with errors.)

    pause

)if exist "%~dp0vision-node.exe" (

    "%~dp0vision-node.exe"
) else (
    echo ERROR: vision-node.exe not found!
    pause
    exit /b 1
)

:end
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Vision Node stopped with errors.
    pause
)
