# Test Constellation Beacon Implementation
# This script tests the beacon functionality

param(
    [switch]$Guardian,
    [switch]$Constellation
)

Write-Host "=".repeat(70) -ForegroundColor Cyan
Write-Host "   CONSTELLATION BEACON TEST SUITE" -ForegroundColor Green
Write-Host "=".repeat(70) -ForegroundColor Cyan
Write-Host ""

# Stop any running nodes
Write-Host "[1/6] Stopping any running vision-node processes..." -ForegroundColor Yellow
Stop-Process -Name "vision-node" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

if ($Guardian) {
    Write-Host "[2/6] Testing GUARDIAN MODE (Active Beacon)..." -ForegroundColor Yellow
    Write-Host ""
    
    # Set Guardian environment
    $env:VISION_GUARDIAN_MODE="true"
    $env:BEACON_MODE="active"
    $env:BEACON_INTERVAL_SECS="10"  # Faster for testing
    
    Write-Host "   Environment:" -ForegroundColor Cyan
    Write-Host "   • VISION_GUARDIAN_MODE = true" -ForegroundColor White
    Write-Host "   • BEACON_MODE = active" -ForegroundColor White
    Write-Host "   • BEACON_INTERVAL_SECS = 10" -ForegroundColor White
    Write-Host ""
    
    Write-Host "[3/6] Starting Guardian node..." -ForegroundColor Yellow
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd c:\vision-node; `$env:VISION_GUARDIAN_MODE='true'; `$env:BEACON_MODE='active'; `$env:BEACON_INTERVAL_SECS='10'; Write-Host 'Starting Guardian with Active Beacon...' -ForegroundColor Green; .\target\release\vision-node.exe" -WindowStyle Normal
    
    Write-Host "[4/6] Waiting 15 seconds for node to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 15
    
    Write-Host "[5/6] Testing beacon endpoints..." -ForegroundColor Yellow
    Write-Host ""
    
    # Test /beacon/ping
    Write-Host "   Testing /beacon/ping..." -ForegroundColor Cyan
    try {
        $ping = Invoke-RestMethod -Uri "http://127.0.0.1:7070/beacon/ping" -Method Get
        Write-Host "   ✅ Beacon ONLINE: $($ping.message)" -ForegroundColor Green
    } catch {
        Write-Host "   ❌ Beacon ping FAILED: $_" -ForegroundColor Red
    }
    
    Start-Sleep -Seconds 2
    
    # Test /beacon/status
    Write-Host "   Testing /beacon/status..." -ForegroundColor Cyan
    try {
        $status = Invoke-RestMethod -Uri "http://127.0.0.1:7070/beacon/status" -Method Get
        Write-Host "   ✅ Beacon Status:" -ForegroundColor Green
        Write-Host "      • Mode: $($status.mode)" -ForegroundColor White
        Write-Host "      • Running: $($status.running)" -ForegroundColor White
        Write-Host "      • Uptime: $($status.uptime_secs) seconds" -ForegroundColor White
        Write-Host "      • Heartbeats: $($status.heartbeat_count)" -ForegroundColor White
        Write-Host "      • Interval: $($status.interval_secs) seconds" -ForegroundColor White
    } catch {
        Write-Host "   ❌ Beacon status FAILED: $_" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "[6/6] Monitor beacon heartbeats for 30 seconds..." -ForegroundColor Yellow
    Write-Host "   (Check the Guardian node window for [BEACON] Broadcasting active logs)" -ForegroundColor Cyan
    Write-Host ""
    
    $endTime = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $endTime) {
        try {
            $status = Invoke-RestMethod -Uri "http://127.0.0.1:7070/beacon/status" -Method Get -ErrorAction SilentlyContinue
            $remaining = [math]::Round(($endTime - (Get-Date)).TotalSeconds)
            Write-Host "   📡 Heartbeats: $($status.heartbeat_count) | Uptime: $($status.uptime_secs)s | Remaining: ${remaining}s" -ForegroundColor Yellow
        } catch {
            Write-Host "   ⚠️  Could not fetch status" -ForegroundColor Red
        }
        Start-Sleep -Seconds 5
    }
    
    Write-Host ""
    Write-Host "=".repeat(70) -ForegroundColor Green
    Write-Host "   GUARDIAN BEACON TEST COMPLETE" -ForegroundColor Green
    Write-Host "=".repeat(70) -ForegroundColor Green
    Write-Host ""
    Write-Host "✅ Check the Guardian node window for detailed [BEACON] logs" -ForegroundColor Cyan
    Write-Host "✅ Heartbeat count should be increasing every 10 seconds" -ForegroundColor Cyan
    Write-Host ""
    
} elseif ($Constellation) {
    Write-Host "[2/6] Testing CONSTELLATION MODE (Passive Beacon)..." -ForegroundColor Yellow
    Write-Host ""
    
    # Set Constellation environment
    $env:BEACON_ENDPOINT="http://127.0.0.1:7070"
    
    Write-Host "   Environment:" -ForegroundColor Cyan
    Write-Host "   • BEACON_ENDPOINT = http://127.0.0.1:7070" -ForegroundColor White
    Write-Host ""
    
    Write-Host "[3/6] Starting Constellation node..." -ForegroundColor Yellow
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd c:\vision-node; `$env:BEACON_ENDPOINT='http://127.0.0.1:7070'; Write-Host 'Starting Constellation node...' -ForegroundColor Green; .\target\release\vision-node.exe" -WindowStyle Normal
    
    Write-Host "[4/6] Waiting 10 seconds for node to initialize..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10
    
    Write-Host "[5/6] Testing beacon client connection..." -ForegroundColor Yellow
    Write-Host ""
    
    # Test if constellation node can reach beacon
    Write-Host "   Checking if node logged beacon connection..." -ForegroundColor Cyan
    Write-Host "   (Check Constellation node window for [NETWORK] Connecting to beacon logs)" -ForegroundColor White
    Write-Host ""
    
    Write-Host "[6/6] Verifying beacon is accessible..." -ForegroundColor Yellow
    try {
        $ping = Invoke-RestMethod -Uri "http://127.0.0.1:7070/beacon/ping" -Method Get
        Write-Host "   ✅ Beacon is reachable from constellation node" -ForegroundColor Green
    } catch {
        Write-Host "   ❌ Beacon NOT reachable (make sure Guardian node is running first)" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "=".repeat(70) -ForegroundColor Green
    Write-Host "   CONSTELLATION CLIENT TEST COMPLETE" -ForegroundColor Green
    Write-Host "=".repeat(70) -ForegroundColor Green
    Write-Host ""
    Write-Host "✅ Check the Constellation node window for beacon connection logs" -ForegroundColor Cyan
    Write-Host ""
    
} else {
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "   .\test-beacon.ps1 -Guardian        # Test Guardian beacon (active broadcasting)" -ForegroundColor White
    Write-Host "   .\test-beacon.ps1 -Constellation   # Test Constellation client (passive listening)" -ForegroundColor White
    Write-Host ""
    Write-Host "Recommended Test Sequence:" -ForegroundColor Cyan
    Write-Host "   1. Run: .\test-beacon.ps1 -Guardian" -ForegroundColor White
    Write-Host "   2. Wait for beacon to start broadcasting" -ForegroundColor White
    Write-Host "   3. In another terminal: .\test-beacon.ps1 -Constellation" -ForegroundColor White
    Write-Host ""
}
