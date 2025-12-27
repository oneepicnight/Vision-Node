Write-Host "=== VISION NETWORK FULL CHECK ===`n" -ForegroundColor Cyan

# 1. Guardian API Check
Write-Host "[1/5] Checking Guardian Local API (7070)..." -ForegroundColor Yellow
try {
    $guardian = Invoke-RestMethod -Uri "http://localhost:7070/api/status" -TimeoutSec 3
    Write-Host "   Guardian online!" -ForegroundColor Green
} catch {
    Write-Host "   Guardian UNREACHABLE on 7070" -ForegroundColor Red
    exit
}

# 2. Upstream Website Check
$upstream = $env:VISION_UPSTREAM_HTTP_BASE
if (-not $upstream) {
    Write-Host "   VISION_UPSTREAM_HTTP_BASE NOT SET!" -ForegroundColor Red
    exit
}

Write-Host "[2/5] Checking Website Upstream ($upstream)..." -ForegroundColor Yellow
try {
    $web = Invoke-RestMethod -Uri "$upstream/api/upstream/status" -TimeoutSec 5
    Write-Host "   Website Upstream online!" -ForegroundColor Green
} catch {
    Write-Host "   Website Upstream UNREACHABLE" -ForegroundColor Red
    exit
}

# 3. Beacon Ping
Write-Host "[3/5] Checking Beacon Service..." -ForegroundColor Yellow
try {
    $ping = Invoke-RestMethod -Uri "$upstream/api/beacon/ping" -TimeoutSec 5
    Write-Host "   Beacon online - Active nodes:" $ping.active -ForegroundColor Green
} catch {
    Write-Host "   Beacon NOT responding" -ForegroundColor Red
    exit
}

# 4. Beacon Peers
Write-Host "[4/5] Fetching Beacon Peers..." -ForegroundColor Yellow
try {
    $peers = Invoke-RestMethod -Uri "$upstream/api/beacon/peers" -TimeoutSec 5
    if ($peers.peers.Count -gt 0) {
        Write-Host "   Peers detected:" $peers.peers.Count -ForegroundColor Green
    } else {
        Write-Host "   No peers in Beacon list!" -ForegroundColor Red
    }
} catch {
    Write-Host "   Failed to fetch peers!" -ForegroundColor Red
    exit
}

# 5. Handshake / Identity Check
Write-Host "[5/5] Checking Node Identity..." -ForegroundColor Yellow
try {
    $id = Invoke-RestMethod "http://localhost:7070/api/identity" -TimeoutSec 3
    Write-Host "   Identity loaded:" $id.node_tag -ForegroundColor Green
    Write-Host "   Vision Address:" $id.vision_address -ForegroundColor Green
} catch {
    Write-Host "   Could not fetch identity!" -ForegroundColor Red
    exit
}

Write-Host "`n=== ALL SYSTEMS GO - CONSTELLATION CAN FORM ===" -ForegroundColor Cyan
