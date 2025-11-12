@echo off
REM Vision Wallet - Full Stack Startup (Batch Version)
REM For systems where PowerShell execution is restricted

echo =============================================
echo  Vision Wallet - Full Stack Startup
echo =============================================
echo.

cd /d "%~dp0"

REM Check Node.js
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Node.js not found!
    echo Please install from: https://nodejs.org/
    pause
    exit /b 1
)

REM Check npm
where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] npm not found!
    pause
    exit /b 1
)

REM Check cargo
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust/Cargo not found!
    echo Please install from: https://rustup.rs/
    pause
    exit /b 1
)

echo [OK] All dependencies found
echo.

REM Install npm dependencies
echo Installing npm dependencies...
call npm install
if %errorlevel% neq 0 (
    echo [ERROR] Failed to install npm dependencies
    pause
    exit /b 1
)
echo [OK] Dependencies installed
echo.

REM Build Rust backend
echo Building Rust market backend...
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Failed to build market backend
    pause
    exit /b 1
)
echo [OK] Market backend built
echo.

echo =============================================
echo  Starting servers...
echo =============================================
echo.
echo Wallet UI: http://localhost:4173
echo Market API: http://127.0.0.1:8080
echo.
echo Press Ctrl+C to stop all servers
echo =============================================
echo.

REM Set environment variables
set VISION_PORT=8080
set RUST_LOG=info

REM Start backend in background
echo Starting market backend...
start "Vision Market Backend" /B cargo run --release

REM Wait for backend to initialize
timeout /t 5 /nobreak >nul

REM Start Vite dev server (foreground)
echo Starting Vite dev server...
call npm run dev

REM Cleanup happens automatically when this window closes
