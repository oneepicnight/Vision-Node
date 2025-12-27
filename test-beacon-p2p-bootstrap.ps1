#!/usr/bin/env pwsh
# Test Beacon → P2P Bootstrap Integration
# This script tests the full flow:
# 1. Guardian starts and broadcasts beacon
# 2. Constellation starts, registers with Guardian, fetches peer list
# 3. Constellation auto-connects to Guardian via P2P

Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  BEACON → P2P BOOTSTRAP TEST" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Step 1: Start Guardian node
Write-Host "[1/6] Starting Guardian node on port 7070..." -ForegroundColor Yellow
Push-Location "VisionNode-v0.8.6-guardian-WIN64"
$env:VISION_GUARDIAN_MODE = "true"
$env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
$env:VISION_PORT = "7070"
$env:VISION_LOG = "info"
Start-Process -FilePath ".\vision-node.exe" -WindowStyle Minimized
Pop-Location
Start-Sleep -Seconds 8
Write-Host "✅ Guardian started" -ForegroundColor Green
Write-Host ""

# Step 2: Check Guardian beacon status
Write-Host "[2/6] Checking Guardian beacon status..." -ForegroundColor Yellow
try {
    $beaconStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/status" -UseBasicParsing
    Write-Host "✅ Beacon status: mode=$($beaconStatus.mode), running=$($beaconStatus.running)" -ForegroundColor Green
} catch {
    Write-Host "❌ Guardian beacon not responding" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Step 3: Start Constellation node
Write-Host "[3/6] Starting Constellation node on port 8080..." -ForegroundColor Yellow
Push-Location "VisionNode-v0.8.6-constellation-testnet-WIN64"
$env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
$env:VISION_PORT = "8080"
$env:VISION_LOG = "info,vision_node=debug"
Remove-Item Env:\VISION_GUARDIAN_MODE -ErrorAction SilentlyContinue
Start-Process -FilePath ".\vision-node.exe" -WindowStyle Minimized
Pop-Location
Start-Sleep -Seconds 8
Write-Host "✅ Constellation started" -ForegroundColor Green
Write-Host ""

# Step 4: Check beacon peer registry
Write-Host "[4/6] Checking beacon peer registry..." -ForegroundColor Yellow
Start-Sleep -Seconds 3
try {
    $peers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
    Write-Host "✅ Beacon registry: $($peers.count) peers" -ForegroundColor Green
    $peers.peers | ForEach-Object {
        Write-Host "   - $($_.node_id) at $($_.ip):$($_.p2p_port)" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ Failed to query beacon registry" -ForegroundColor Red
}
Write-Host ""

# Step 5: Check P2P peer connections on Guardian
Write-Host "[5/6] Checking P2P peer connections on Guardian..." -ForegroundColor Yellow
Start-Sleep -Seconds 5
try {
    $p2pPeers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/peers" -UseBasicParsing
    Write-Host "✅ Guardian P2P peers: $($p2pPeers.count)" -ForegroundColor Green
    $p2pPeers.peers | ForEach-Object {
        Write-Host "   - $_" -ForegroundColor Gray
    }
} catch {
    Write-Host "⚠️  P2P peers endpoint not available" -ForegroundColor Yellow
}
Write-Host ""

# Step 6: Check P2P peer connections on Constellation
Write-Host "[6/6] Checking P2P peer connections on Constellation..." -ForegroundColor Yellow
try {
    $constellationPeers = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/peers" -UseBasicParsing
    Write-Host "✅ Constellation P2P peers: $($constellationPeers.count)" -ForegroundColor Green
    $constellationPeers.peers | ForEach-Object {
        Write-Host "   - $_" -ForegroundColor Gray
    }
} catch {
    Write-Host "⚠️  P2P peers endpoint not available" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  TEST COMPLETE" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "Expected Behavior:" -ForegroundColor White
Write-Host "  1. Beacon registry should show 1+ peers (Constellation registered)" -ForegroundColor Gray
Write-Host "  2. Guardian /api/peers should show Constellation as P2P peer" -ForegroundColor Gray
Write-Host "  3. Constellation /api/peers should show Guardian as P2P peer" -ForegroundColor Gray
Write-Host ""
Write-Host "To stop nodes: Get-Process vision-node | Stop-Process -Force" -ForegroundColor Yellow
