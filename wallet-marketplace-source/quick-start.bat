@echo off
title Vision Wallet - Quick Start
color 0B

echo.
echo  =============================================
echo   🌟 Vision Wallet - Quick Start Setup
echo  =============================================
echo.

REM Check if Node.js is installed
node --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ Node.js not found!
    echo.
    echo Please install Node.js from: https://nodejs.org/
    echo Minimum required version: Node.js 16+
    echo.
    pause
    exit /b 1
)

echo ✓ Node.js found
node --version

REM Check npm
npm --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ npm not found! Please reinstall Node.js
    pause
    exit /b 1
)

echo ✓ npm found
echo.

echo 📦 Installing dependencies...
echo This may take a few minutes on first run...
echo.

npm install
if %errorlevel% neq 0 (
    echo.
    echo ❌ Failed to install dependencies
    echo Please check your internet connection and try again
    pause
    exit /b 1
)

echo.
echo ✓ Dependencies installed successfully
echo.

echo 🧪 Running quick tests...
npm run test:run
if %errorlevel% neq 0 (
    echo ⚠️  Some tests failed, but continuing...
) else (
    echo ✓ All tests passed
)

echo.
echo 🚀 Starting Vision Wallet...
echo.
echo   Local server will start at: http://localhost:5173
echo   Your browser should open automatically
echo   Press Ctrl+C to stop the server
echo.
echo 📖 Quick Guide:
echo   1. Click 'Enter' on the splash screen
echo   2. Choose a handle (username)  
echo   3. Save your 12-word recovery phrase
echo   4. Start using your Vision Wallet!
echo.

REM Start the development server
npm run dev

echo.
echo Server stopped. Press any key to exit...
pause >nul