# Test Guardian → Website → Constellation Architecture
# Guardian trusts the website as its upstream source

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Guardian → Website → Constellation Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Stop any running nodes
Write-Host "Phase 0: Cleanup" -ForegroundColor Yellow
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Write-Host "Done - cleaned up existing processes" -ForegroundColor Green
Write-Host ""

# Phase 1: Start Constellation Node (Real P2P Chain)
Write-Host "Phase 1: Starting Constellation Node (Real Chain)" -ForegroundColor Yellow
Write-Host "  Role: Run actual P2P consensus, maintain blockchain" -ForegroundColor Gray

$constDir = "c:\vision-node\vision_data_constA"
if (!(Test-Path $constDir)) {
    New-Item -ItemType Directory -Path $constDir | Out-Null
}
Copy-Item "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64\vision-node.exe" "$constDir\vision-node.exe" -Force

$constJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "8181"
    $env:VISION_P2P_PORT = "8081"
    .\vision-node.exe
} -ArgumentList $constDir

Write-Host "  Waiting for Constellation to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

try {
    $constStatus = Invoke-RestMethod -Uri "http://127.0.0.1:8181/api/status" -UseBasicParsing -TimeoutSec 5
    Write-Host "SUCCESS: Constellation online - Height: $($constStatus.chain_height)" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Constellation didn't start properly!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host ""

# Phase 2: Simulate Website (For this test, we'll just verify Guardian can call it)
Write-Host "Phase 2: Website Upstream Endpoints" -ForegroundColor Yellow
Write-Host "  In production, the website at visionworld.tech would:" -ForegroundColor Gray
Write-Host "    1. Accept requests at /api/upstream/*" -ForegroundColor Gray
Write-Host "    2. Forward them to constellation nodes" -ForegroundColor Gray
Write-Host "    3. Return aggregated data to Guardian" -ForegroundColor Gray
Write-Host ""
Write-Host "  For this test:" -ForegroundColor Cyan
Write-Host "    - We'll verify Guardian's configuration" -ForegroundColor Gray
Write-Host "    - Guardian will fall back to local state (no website running)" -ForegroundColor Gray
Write-Host ""

# Phase 3: Start Guardian in Website-Upstream Mode
Write-Host "Phase 3: Starting Guardian (Website Follower Mode)" -ForegroundColor Yellow
Write-Host "  Role: Trust website as upstream, serve Guardian AI" -ForegroundColor Gray

$guardianDir = "c:\vision-node\VisionNode-v0.8.6-guardian-WIN64"
$guardianJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "7070"
    $env:VISION_GUARDIAN_MODE = "true"
    # Point to website (in production this would be https://visionworld.tech)
    $env:VISION_UPSTREAM_HTTP_BASE = "http://127.0.0.1:5173"
    .\vision-node.exe
} -ArgumentList $guardianDir

Write-Host "  Waiting for Guardian to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

try {
    $guardianStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/status" -UseBasicParsing -TimeoutSec 5
    Write-Host "SUCCESS: Guardian online - Mode: Guardian" -ForegroundColor Green
} catch {
    Write-Host "ERROR: Guardian didn't start properly!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host ""

# Phase 4: Verify Architecture
Write-Host "Phase 4: Architecture Verification" -ForegroundColor Yellow
Write-Host ""

Write-Host "Constellation Node Status:" -ForegroundColor Cyan
$constStatus = Invoke-RestMethod -Uri "http://127.0.0.1:8181/api/status" -UseBasicParsing
Write-Host "  Height: $($constStatus.chain_height)" -ForegroundColor White
Write-Host "  Peers: $($constStatus.peer_count)" -ForegroundColor White
Write-Host "  Guardian Mode: $($constStatus.guardian_mode)" -ForegroundColor White

Write-Host ""
Write-Host "Guardian Node Status:" -ForegroundColor Cyan
$guardianStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/status" -UseBasicParsing
Write-Host "  Height: $($guardianStatus.chain_height)" -ForegroundColor White
Write-Host "  Peers: $($guardianStatus.peer_count)" -ForegroundColor White
Write-Host "  Guardian Mode: $($guardianStatus.guardian_mode)" -ForegroundColor White

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Test Results" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if ($guardianStatus.guardian_mode -eq $true) {
    Write-Host "✅ Guardian running in Guardian mode" -ForegroundColor Green
} else {
    Write-Host "❌ Guardian mode not enabled" -ForegroundColor Red
}

Write-Host ""
Write-Host "Architecture Summary:" -ForegroundColor Cyan
Write-Host "  1. Constellation Node: http://127.0.0.1:8181" -ForegroundColor Gray
Write-Host "     - Real P2P chain with Genesis Door" -ForegroundColor Gray
Write-Host "     - Maintains actual blockchain state" -ForegroundColor Gray
Write-Host ""
Write-Host "  2. Website (Production): https://visionworld.tech" -ForegroundColor Gray
Write-Host "     - Receives /api/upstream/* requests from Guardian" -ForegroundColor Gray
Write-Host "     - Forwards to constellation node(s)" -ForegroundColor Gray
Write-Host "     - Returns aggregated view to Guardian" -ForegroundColor Gray
Write-Host ""
Write-Host "  3. Guardian Node: http://127.0.0.1:7070" -ForegroundColor Gray
Write-Host "     - Configured with VISION_UPSTREAM_HTTP_BASE" -ForegroundColor Gray
Write-Host "     - Trusts website as authoritative source" -ForegroundColor Gray
Write-Host "     - Falls back to local state if website unavailable" -ForegroundColor Gray
Write-Host ""

Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Host "  1. Website needs /api/upstream/* endpoints" -ForegroundColor Gray
Write-Host "  2. Website should proxy to constellation node(s)" -ForegroundColor Gray
Write-Host "  3. Guardian will automatically consume website data" -ForegroundColor Gray
Write-Host ""

Write-Host "Check Logs:" -ForegroundColor Cyan
Write-Host "  Get-Job | Receive-Job | Select-String -Pattern 'GUARDIAN|upstream'" -ForegroundColor Gray
Write-Host ""
Write-Host "Stop Nodes:" -ForegroundColor Cyan
Write-Host "  Get-Job | Stop-Job; Get-Process vision-node | Stop-Process -Force" -ForegroundColor Gray
Write-Host ""
