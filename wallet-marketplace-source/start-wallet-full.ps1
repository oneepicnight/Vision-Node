# Vision Wallet - Full Stack Startup
# Starts both the Rust market backend AND the Vite dev server

Write-Host "====================================" -ForegroundColor Cyan
Write-Host "  Vision Wallet Marketplace" -ForegroundColor White
Write-Host "  Market Backend + Frontend" -ForegroundColor Gray
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "NOTE: Vision Node (port 7070) must be running separately" -ForegroundColor Yellow
Write-Host "      for exchange trading and blockchain features." -ForegroundColor Yellow
Write-Host ""

# Change to wallet directory
Set-Location $PSScriptRoot

# Check if Node.js is installed
try {
    $nodeVersion = node --version
    Write-Host "[OK] Node.js found: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Node.js not found. Please install Node.js from https://nodejs.org/" -ForegroundColor Red
    exit 1
}

# Check if npm is available
try {
    $npmVersion = npm --version
    Write-Host "[OK] npm found: v$npmVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] npm not found. Please reinstall Node.js" -ForegroundColor Red
    exit 1
}

# Check if Rust is installed
try {
    $cargoVersion = cargo --version
    Write-Host "[OK] Cargo found: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Rust/Cargo not found. Please install from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Installing npm dependencies..." -ForegroundColor Yellow
npm install
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Failed to install dependencies" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Dependencies installed" -ForegroundColor Green

Write-Host ""
Write-Host "Building Rust market backend..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Failed to build market backend" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Market backend built" -ForegroundColor Green

Write-Host ""
Write-Host "Starting servers..." -ForegroundColor Green
Write-Host ""

# Set environment variables for market backend
$env:VISION_PORT = "8080"
$env:RUST_LOG = "info"

# Start Rust market backend in background
Write-Host "   Starting Market Backend on http://127.0.0.1:8080" -ForegroundColor Cyan
$marketJob = Start-Job -ScriptBlock {
    Set-Location $using:PSScriptRoot
    $env:VISION_PORT = "8080"
    $env:RUST_LOG = "info"
    cargo run --release 2>&1 | ForEach-Object { Write-Host "[Market] $_" -ForegroundColor DarkCyan }
}

# Wait for market backend to start
Write-Host "   Waiting for market backend to initialize..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check if market backend is running
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:8080/exchange/ticker" -TimeoutSec 2 -ErrorAction Stop
    Write-Host "   [OK] Market backend is ready!" -ForegroundColor Green
} catch {
    Write-Host "   [WARN] Market backend may still be starting..." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "   Starting Vite Dev Server on http://127.0.0.1:4173" -ForegroundColor Cyan
Write-Host ""
Write-Host "================================================" -ForegroundColor Gray
Write-Host "   Your wallet is now running at:" -ForegroundColor Magenta
Write-Host "      http://localhost:4173" -ForegroundColor White
Write-Host ""
Write-Host "   Backend Services:" -ForegroundColor Magenta
Write-Host "      Market API: http://127.0.0.1:8080" -ForegroundColor White
Write-Host ""
Write-Host "   REQUIRED - For exchange trading:" -ForegroundColor Yellow
Write-Host "      Vision Node must run on port 7070" -ForegroundColor White
Write-Host "      cd c:\vision-node && cargo run --release" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   Trading Pairs: BTC/LAND, BCH/LAND, DOGE/LAND" -ForegroundColor Cyan
Write-Host ""
Write-Host "   Press Ctrl+C to stop all servers" -ForegroundColor Yellow
Write-Host "================================================" -ForegroundColor Gray
Write-Host ""

# Start Vite dev server (this will block)
try {
    npm run dev
} catch {
    Write-Host ""
    Write-Host "[ERROR] Vite dev server stopped" -ForegroundColor Red
} finally {
    # Cleanup: Stop background job when Vite stops
    Write-Host ""
    Write-Host "Stopping market backend..." -ForegroundColor Yellow
    Stop-Job -Job $marketJob -ErrorAction SilentlyContinue
    Remove-Job -Job $marketJob -ErrorAction SilentlyContinue
    Write-Host "[OK] All servers stopped" -ForegroundColor Green
}
