@echo off
title Ngrok Setup

echo.
echo ================================
echo   Ngrok Setup for Vision Node
echo ================================
echo.

REM Check if ngrok.exe exists
if not exist "%~dp0ngrok.exe" (
    echo ERROR: ngrok.exe not found in current directory!
    echo Please ensure ngrok.exe is in the same folder as this script.
    pause
    exit /b 1
)

echo Ngrok found: %~dp0ngrok.exe
echo.
echo To use ngrok, you need an auth token (free account):
echo.
echo 1. Visit: https://dashboard.ngrok.com/signup
echo 2. Sign up for free
echo 3. Copy your auth token
echo 4. Paste it below when prompted
echo.
echo.

set /p NGROK_TOKEN="Enter your ngrok auth token (or press Ctrl+C to cancel): "

if "%NGROK_TOKEN%"=="" (
    echo No token entered. Exiting...
    pause
    exit /b 1
)

echo.
echo Configuring ngrok with your token...
"%~dp0ngrok.exe" config add-authtoken %NGROK_TOKEN%

if %errorlevel% equ 0 (
    echo.
    echo ================================
    echo   SUCCESS! Ngrok is configured
    echo ================================
    echo.
    echo You can now run START-PUBLIC-NODE.bat
    echo.
) else (
    echo.
    echo ERROR: Failed to configure ngrok
    echo Please check your token and try again
    echo.
)

pause
