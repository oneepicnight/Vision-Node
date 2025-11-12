#!/usr/bin/env pwsh
# Test the wallet's market backend only

Write-Host "🧪 Testing Vision Wallet Market Backend" -ForegroundColor Cyan
Write-Host "=======================================" -ForegroundColor Cyan
Write-Host ""

Set-Location $PSScriptRoot

$env:VISION_PORT = "8080"
$env:RUST_LOG = "info"

Write-Host "Starting market backend on port 8080..." -ForegroundColor Yellow
Write-Host "Press Ctrl+C to stop" -ForegroundColor Yellow
Write-Host ""

cargo run --release
