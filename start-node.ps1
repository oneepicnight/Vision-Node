# Vision Node Startup Script
# Ensures environment variables are set correctly

Write-Host "🛡️  Starting Vision Node..." -ForegroundColor Cyan
Write-Host ""

# Set environment variables
$env:VISION_PUBLIC_DIR = "c:\vision-node\public"
$env:VISION_WALLET_DIR = "c:\vision-node\wallet\dist"

# Verify directories exist
if (-not (Test-Path $env:VISION_PUBLIC_DIR)) {
    Write-Host "❌ ERROR: Public directory not found: $env:VISION_PUBLIC_DIR" -ForegroundColor Red
    pause
    exit 1
}

if (-not (Test-Path $env:VISION_WALLET_DIR)) {
    Write-Host "❌ ERROR: Wallet directory not found: $env:VISION_WALLET_DIR" -ForegroundColor Red
    pause
    exit 1
}

Write-Host "✅ Public dir: $env:VISION_PUBLIC_DIR" -ForegroundColor Green
Write-Host "✅ Wallet dir: $env:VISION_WALLET_DIR" -ForegroundColor Green
Write-Host ""

# Change to node directory
Set-Location "c:\vision-node"

# Start the node
Write-Host "Starting node executable..." -ForegroundColor Yellow
Write-Host ""
.\target\release\vision-node.exe
