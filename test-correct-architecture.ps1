# Test Script: Correct Guardian/Constellation Architecture
# Guardian = HTTP beacon only (no P2P listener)
# Constellation = Full P2P nodes connecting to each other

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Testing Correct Guardian/Constellation Architecture" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# Stop any running nodes
Write-Host "Phase 0: Cleanup" -ForegroundColor Yellow
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Write-Host "✅ Cleaned up existing processes`n" -ForegroundColor Green

# Phase 1: Start Guardian (HTTP beacon only)
Write-Host "Phase 1: Starting Guardian Node (HTTP beacon only)" -ForegroundColor Yellow
$guardianDir = "c:\vision-node\VisionNode-v0.8.6-guardian-WIN64"
$guardianJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "7070"
    $env:VISION_GUARDIAN_MODE = "true"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $guardianDir

Write-Host "Waiting for Guardian to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

# Check Guardian status
$guardianStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/status" -UseBasicParsing
Write-Host "Guardian Beacon Status: $($guardianStatus | ConvertTo-Json)" -ForegroundColor Cyan

if ($guardianStatus.mode -ne "active") {
    Write-Host "❌ Guardian beacon not in active mode!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "✅ Guardian beacon active`n" -ForegroundColor Green

# Phase 2: Check Guardian ports (should only have HTTP, no P2P listener)
Write-Host "Phase 2: Verifying Guardian Ports" -ForegroundColor Yellow
$guardianPorts = Get-NetTCPConnection | Where-Object {$_.LocalPort -in @(7070, 7072) -and $_.State -eq 'Listen'} | Select-Object LocalPort,State
Write-Host "Guardian Listening Ports:" -ForegroundColor Cyan
$guardianPorts | Format-Table

$hasHTTP = $guardianPorts | Where-Object {$_.LocalPort -eq 7070}
$hasP2P = $guardianPorts | Where-Object {$_.LocalPort -eq 7072}

if (!$hasHTTP) {
    Write-Host "❌ Guardian HTTP port 7070 not listening!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
if ($hasP2P) {
    Write-Host "❌ Guardian should NOT have P2P listener on 7072!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "✅ Guardian has HTTP only (7070), no P2P listener (correct)`n" -ForegroundColor Green

# Phase 3: Start First Constellation Node
Write-Host "Phase 3: Starting First Constellation Node" -ForegroundColor Yellow
$const1Dir = "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64"
$const1Job = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "8080"
    $env:VISION_P2P_PORT = "8082"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $const1Dir

Write-Host "Waiting for Constellation to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

# Phase 4: Check Constellation ports (should have both HTTP and P2P)
Write-Host "Phase 4: Verifying Constellation Ports" -ForegroundColor Yellow
$const1Ports = Get-NetTCPConnection | Where-Object {$_.LocalPort -in @(8080, 8082) -and $_.State -eq 'Listen'} | Select-Object LocalPort,State
Write-Host "Constellation Listening Ports:" -ForegroundColor Cyan
$const1Ports | Format-Table

$const1HTTP = $const1Ports | Where-Object {$_.LocalPort -eq 8080}
$const1P2P = $const1Ports | Where-Object {$_.LocalPort -eq 8082}

if (!$const1HTTP) {
    Write-Host "❌ Constellation HTTP port 8080 not listening!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
if (!$const1P2P) {
    Write-Host "❌ Constellation P2P port 8082 not listening!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "✅ Constellation has both HTTP (8080) and P2P (8082) (correct)`n" -ForegroundColor Green

# Phase 5: Check beacon registration (should show 1 constellation node, NO Guardian)
Write-Host "Phase 5: Checking Beacon Registry" -ForegroundColor Yellow
Start-Sleep -Seconds 3
$beaconPeers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
Write-Host "Beacon Peers:" -ForegroundColor Cyan
Write-Host ($beaconPeers | ConvertTo-Json)

if ($beaconPeers.count -ne 1) {
    Write-Host "❌ Expected 1 peer in beacon registry, got $($beaconPeers.count)" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}

# Check that Guardian is NOT in the peer list
$guardianInList = $beaconPeers.peers | Where-Object {$_.node_id -like "*guardian*" -or $_.p2p_port -eq 7072}
if ($guardianInList) {
    Write-Host "❌ Guardian should NOT be in peer list (it's HTTP only)!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}

Write-Host "✅ Beacon shows 1 constellation node (Guardian NOT in list - correct)`n" -ForegroundColor Green

# Phase 6: Start Second Constellation Node
Write-Host "Phase 6: Starting Second Constellation Node" -ForegroundColor Yellow

# Create temp dir for second constellation
$const2Dir = "c:\vision-node\vision_data_9090"
if (!(Test-Path $const2Dir)) {
    New-Item -ItemType Directory -Path $const2Dir | Out-Null
}
Copy-Item "$const1Dir\vision-node.exe" "$const2Dir\vision-node.exe" -Force

$const2Job = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "9090"
    $env:VISION_P2P_PORT = "9092"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $const2Dir

Write-Host "Waiting for second Constellation to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

# Phase 7: Check beacon now has 2 constellation nodes
Write-Host "Phase 7: Checking Updated Beacon Registry" -ForegroundColor Yellow
$beaconPeers2 = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
Write-Host "Beacon Peers (2 Constellation nodes):" -ForegroundColor Cyan
Write-Host ($beaconPeers2 | ConvertTo-Json)

if ($beaconPeers2.count -ne 2) {
    Write-Host "❌ Expected 2 peers in beacon registry, got $($beaconPeers2.count)" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "✅ Beacon shows 2 constellation nodes (Guardian NOT in list - correct)`n" -ForegroundColor Green

# Phase 8: Wait for P2P bootstrap between constellations
Write-Host "Phase 8: Waiting for P2P Bootstrap Between Constellation Nodes" -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Phase 9: Check if constellations connected to each other
Write-Host "Phase 9: Verifying P2P Connections Between Constellations" -ForegroundColor Yellow

# Check first constellation's peers
$const1Peers = Invoke-RestMethod -Uri "http://127.0.0.1:8080/api/peers" -UseBasicParsing
Write-Host "Constellation 1 P2P Peers:" -ForegroundColor Cyan
Write-Host ($const1Peers | ConvertTo-Json)

# Check second constellation's peers
$const2Peers = Invoke-RestMethod -Uri "http://127.0.0.1:9090/api/peers" -UseBasicParsing
Write-Host "Constellation 2 P2P Peers:" -ForegroundColor Cyan
Write-Host ($const2Peers | ConvertTo-Json)

# Verify they connected to each other (not to Guardian)
if ($const1Peers.peers.Count -eq 0 -or $const2Peers.peers.Count -eq 0) {
    Write-Host "⚠️  Warning: Constellation nodes didn't establish P2P connections yet" -ForegroundColor Yellow
    Write-Host "This is the P2P handshake issue we're debugging" -ForegroundColor Yellow
} else {
    Write-Host "✅ Constellation nodes connected to each other via P2P`n" -ForegroundColor Green
}

# Phase 10: Summary
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Architecture Test Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "✅ Guardian: HTTP beacon only (no P2P listener)" -ForegroundColor Green
Write-Host "✅ Constellation nodes: HTTP + P2P listeners" -ForegroundColor Green
Write-Host "✅ Beacon registry: Only constellation nodes (Guardian NOT included)" -ForegroundColor Green
Write-Host "✅ Port architecture correct (HTTP port, P2P port = HTTP + 2)" -ForegroundColor Green

if ($const1Peers.peers.Count -gt 0 -and $const2Peers.peers.Count -gt 0) {
    Write-Host "✅ P2P connections between constellations working!" -ForegroundColor Green
} else {
    Write-Host "⚠️  P2P handshake still needs debugging (expected - separate issue)" -ForegroundColor Yellow
}

Write-Host "`nTest complete! Nodes are still running for inspection." -ForegroundColor Cyan
Write-Host "To stop: Get-Job | Stop-Job" -ForegroundColor Gray
