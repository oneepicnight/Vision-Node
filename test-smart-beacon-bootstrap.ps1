#!/usr/bin/env pwsh
# Smart Beacon → P2P Bootstrap Test
# Tests Guardian sentinel injection and smart peer discovery

Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  SMART BEACON → P2P BOOTSTRAP TEST v2" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Step 1: Start Guardian node with P2P address configured
Write-Host "[1/7] Starting Guardian node (HTTP: 7070, P2P: 7072)..." -ForegroundColor Yellow
Push-Location "VisionNode-v0.8.6-guardian-WIN64"
$env:VISION_GUARDIAN_MODE = "true"
$env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
$env:VISION_GUARDIAN_P2P_ADDR = "127.0.0.1:7072"  # Synthetic sentinel P2P address
$env:VISION_PORT = "7070"        # HTTP API port
$env:VISION_P2P_PORT = "7072"    # P2P TCP port
$env:VISION_LOG = "info,vision_node=debug"
Start-Process -FilePath ".\vision-node.exe" -WindowStyle Minimized
Pop-Location
Start-Sleep -Seconds 8
Write-Host "✅ Guardian started (HTTP: 7070, P2P: 7072)" -ForegroundColor Green
Write-Host ""

# Step 2: Check Guardian beacon status
Write-Host "[2/7] Checking Guardian beacon status..." -ForegroundColor Yellow
try {
    $beaconStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/status" -UseBasicParsing
    Write-Host "✅ Beacon: mode=$($beaconStatus.mode), running=$($beaconStatus.running)" -ForegroundColor Green
} catch {
    Write-Host "❌ Guardian beacon not responding" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Step 3: Check beacon peers (should show Guardian sentinel)
Write-Host "[3/7] Checking beacon peers (Guardian sentinel should appear)..." -ForegroundColor Yellow
try {
    $peers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
    Write-Host "✅ Beacon registry: $($peers.count) peers" -ForegroundColor Green
    $peers.peers | ForEach-Object {
        $type = if ($_.node_id -eq "guardian-sentinel") { "🛡️  GUARDIAN" } else { "🌌 Constellation" }
        Write-Host "   $type - $($_.node_id) at $($_.ip):$($_.p2p_port)" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ Failed to query beacon registry" -ForegroundColor Red
}
Write-Host ""

# Step 4: Start Constellation node
Write-Host "[4/7] Starting Constellation node (HTTP: 8080, P2P: 8082)..." -ForegroundColor Yellow
Push-Location "VisionNode-v0.8.6-constellation-testnet-WIN64"
$env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
$env:VISION_PORT = "8080"        # HTTP API port
$env:VISION_P2P_PORT = "8082"    # P2P TCP port
$env:VISION_LOG = "info,vision_node=debug"
Remove-Item Env:\VISION_GUARDIAN_MODE -ErrorAction SilentlyContinue
Remove-Item Env:\VISION_GUARDIAN_P2P_ADDR -ErrorAction SilentlyContinue
Start-Process -FilePath ".\vision-node.exe" -WindowStyle Minimized
Pop-Location
Start-Sleep -Seconds 8
Write-Host "✅ Constellation started (HTTP: 8080, P2P: 8082)" -ForegroundColor Green
Write-Host ""

# Step 5: Check beacon peer registry (should show Guardian + Constellation)
Write-Host "[5/7] Checking beacon registry after Constellation registration..." -ForegroundColor Yellow
Start-Sleep -Seconds 3
try {
    $peers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
    Write-Host "✅ Beacon registry: $($peers.count) peers" -ForegroundColor Green
    $peers.peers | ForEach-Object {
        $type = if ($_.node_id -eq "guardian-sentinel") { "🛡️  GUARDIAN" } else { "🌌 Constellation" }
        Write-Host "   $type - $($_.node_id) at $($_.ip):$($_.p2p_port)" -ForegroundColor Gray
    }
} catch {
    Write-Host "❌ Failed to query beacon registry" -ForegroundColor Red
}
Write-Host ""

# Step 6: Wait for P2P bootstrap to complete
Write-Host "[6/7] Waiting for P2P bootstrap (connects after 5s delay)..." -ForegroundColor Yellow
Start-Sleep -Seconds 10
Write-Host ""

# Step 7: Check P2P peer connections
Write-Host "[7/7] Checking P2P peer connections..." -ForegroundColor Yellow
Write-Host ""
Write-Host "  Guardian P2P peers:" -ForegroundColor Cyan
try {
    $guardianPeers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/peers" -UseBasicParsing
    Write-Host "  ✅ Connected to $($guardianPeers.peers.Count) peers" -ForegroundColor Green
    $guardianPeers.peers | ForEach-Object {
        Write-Host "     - $_" -ForegroundColor Gray
    }
} catch {
    Write-Host "  ⚠️  Endpoint error: $_" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "  Constellation P2P peers:" -ForegroundColor Cyan
try {
    $constellationPeers = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/peers" -UseBasicParsing
    Write-Host "  ✅ Connected to $($constellationPeers.peers.Count) peers" -ForegroundColor Green
    $constellationPeers.peers | ForEach-Object {
        Write-Host "     - $_" -ForegroundColor Gray
    }
} catch {
    Write-Host "  ⚠️  Endpoint error: $_" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  TEST COMPLETE" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "✅ Expected Results:" -ForegroundColor Green
Write-Host "  1. Beacon registry shows Guardian sentinel + Constellation node" -ForegroundColor Gray
Write-Host "  2. Constellation fetched peer list (Guardian + others)" -ForegroundColor Gray
Write-Host "  3. Constellation connected to Guardian first" -ForegroundColor Gray
Write-Host "  4. Guardian shows Constellation in /api/peers" -ForegroundColor Gray
Write-Host "  5. Constellation shows Guardian in /api/peers" -ForegroundColor Gray
Write-Host ""
Write-Host "To stop nodes: Get-Process vision-node | Stop-Process -Force" -ForegroundColor Yellow
