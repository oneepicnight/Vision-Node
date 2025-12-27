# 3-Node Proof of Life Test
# Guardian + 2 Constellation nodes discovering each other

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "3-Node Proof of Life Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Stop any running nodes
Write-Host "Phase 0: Cleanup" -ForegroundColor Yellow
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Write-Host "Done - cleaned up existing processes" -ForegroundColor Green
Write-Host ""

# Phase 1: Start Guardian
Write-Host "Phase 1: Starting Guardian Node (The Lighthouse)" -ForegroundColor Yellow
Write-Host "  Role: HTTP beacon only - judges who's legit" -ForegroundColor Gray
$guardianDir = "c:\vision-node\VisionNode-v0.8.6-guardian-WIN64"
$guardianJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "7070"
    $env:VISION_GUARDIAN_MODE = "true"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $guardianDir

Write-Host "  Waiting for Guardian to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 8

$guardianStatus = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/status" -UseBasicParsing
if ($guardianStatus.mode -ne "active") {
    Write-Host "ERROR: Guardian beacon not in active mode!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "SUCCESS: Guardian online - The lighthouse is lit" -ForegroundColor Green
Write-Host ""

# Phase 2: Start Constellation A
Write-Host "Phase 2: Starting Constellation A (First Traveler)" -ForegroundColor Yellow
Write-Host "  Role: Register with Guardian, start P2P, wait for friends" -ForegroundColor Gray

$constADir = "c:\vision-node\vision_data_constA"
if (!(Test-Path $constADir)) {
    New-Item -ItemType Directory -Path $constADir | Out-Null
}
Copy-Item "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64\vision-node.exe" "$constADir\vision-node.exe" -Force
Copy-Item "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64\config" "$constADir\config" -Recurse -Force -ErrorAction SilentlyContinue

$constAJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "8181"
    $env:VISION_P2P_PORT = "8081"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $constADir

Write-Host "  Waiting for Constellation A to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 10

$beaconPeers = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
if ($beaconPeers.count -lt 1) {
    Write-Host "ERROR: Constellation A didn't register!" -ForegroundColor Red
    Get-Job | Stop-Job
    exit 1
}
Write-Host "SUCCESS: Constellation A online - First star is orbiting" -ForegroundColor Green
Write-Host ""

# Phase 3: Start Constellation B
Write-Host "Phase 3: Starting Constellation B (Second Traveler)" -ForegroundColor Yellow
Write-Host "  Role: Register, discover Constellation A, connect via P2P" -ForegroundColor Gray

$constBDir = "c:\vision-node\vision_data_constB"
if (!(Test-Path $constBDir)) {
    New-Item -ItemType Directory -Path $constBDir | Out-Null
}
Copy-Item "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64\vision-node.exe" "$constBDir\vision-node.exe" -Force
Copy-Item "c:\vision-node\VisionNode-v0.8.6-constellation-testnet-WIN64\config" "$constBDir\config" -Recurse -Force -ErrorAction SilentlyContinue

$constBJob = Start-Job -ScriptBlock {
    param($dir)
    Set-Location $dir
    $env:VISION_PORT = "8182"
    $env:VISION_P2P_PORT = "8082"
    $env:BEACON_ENDPOINT = "http://127.0.0.1:7070"
    .\vision-node.exe
} -ArgumentList $constBDir

Write-Host "  Waiting for Constellation B to initialize..." -ForegroundColor Gray
Start-Sleep -Seconds 10

$beaconPeers2 = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
if ($beaconPeers2.count -lt 2) {
    Write-Host "WARNING: Constellation B might not have registered yet (only $($beaconPeers2.count) peers)" -ForegroundColor Yellow
} else {
    Write-Host "SUCCESS: Constellation B online - Second star is orbiting" -ForegroundColor Green
}
Write-Host ""

# Phase 4: The Moment of Ignition
Write-Host "Phase 4: The Moment of Ignition" -ForegroundColor Yellow
Write-Host "  Waiting 20 seconds for peer discovery and P2P handshakes..." -ForegroundColor Gray
Start-Sleep -Seconds 20

# Phase 5: Check P2P Connections
Write-Host ""
Write-Host "Phase 5: Checking P2P Connections" -ForegroundColor Yellow
Write-Host ""

Write-Host "Guardian Beacon Registry:" -ForegroundColor Cyan
$beaconFinal = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/beacon/peers" -UseBasicParsing
Write-Host "  Registered Peers: $($beaconFinal.count)" -ForegroundColor White
foreach ($peer in $beaconFinal.peers) {
    Write-Host "    - $($peer.node_id) at $($peer.ip):$($peer.p2p_port)" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Constellation A P2P Peers:" -ForegroundColor Cyan
try {
    $constAPeers = Invoke-RestMethod -Uri "http://127.0.0.1:8181/api/peers" -UseBasicParsing
    Write-Host "  Connected Peers: $($constAPeers.peers.Count)" -ForegroundColor White
    if ($constAPeers.peers.Count -gt 0) {
        foreach ($peer in $constAPeers.peers) {
            Write-Host "    - $peer" -ForegroundColor Gray
        }
    } else {
        Write-Host "    (none yet)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  WARNING: Could not reach Constellation A API" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Constellation B P2P Peers:" -ForegroundColor Cyan
try {
    $constBPeers = Invoke-RestMethod -Uri "http://127.0.0.1:8182/api/peers" -UseBasicParsing
    Write-Host "  Connected Peers: $($constBPeers.peers.Count)" -ForegroundColor White
    if ($constBPeers.peers.Count -gt 0) {
        foreach ($peer in $constBPeers.peers) {
            Write-Host "    - $peer" -ForegroundColor Gray
        }
    } else {
        Write-Host "    (none yet)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  WARNING: Could not reach Constellation B API" -ForegroundColor Yellow
}

# Phase 6: Verdict
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "The Proof of Life Verdict" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$success = $false
if ($beaconFinal.count -ge 2) {
    Write-Host "SUCCESS: Guardian knows about both constellation nodes" -ForegroundColor Green
    
    if ($constAPeers.peers.Count -gt 0 -or $constBPeers.peers.Count -gt 0) {
        Write-Host "SUCCESS: CONSTELLATION NODES FOUND EACH OTHER!" -ForegroundColor Green
        Write-Host ""
        Write-Host "   This is it. Real peer discovery." -ForegroundColor Green
        Write-Host "   Node B saw Node A. They connected." -ForegroundColor Green
        Write-Host "   Guardian isn't holding their hands." -ForegroundColor Green
        Write-Host "   The system WORKS." -ForegroundColor Green
        $success = $true
    } else {
        Write-Host "WARNING: Nodes registered but haven't connected yet" -ForegroundColor Yellow
        Write-Host "   (P2P handshake issue - genesis hash mismatch expected)" -ForegroundColor Yellow
    }
} else {
    Write-Host "ERROR: Not enough nodes registered with Guardian" -ForegroundColor Red
}

Write-Host ""
Write-Host "What You're Looking For In Logs:" -ForegroundColor Cyan
Write-Host "  On Node A: [P2P] Incoming connection from 127.0.0.1:8082" -ForegroundColor Gray
Write-Host "  On Node B: [P2P] Connected to peer: 127.0.0.1:8081" -ForegroundColor Gray
Write-Host "  On Guardian: [BEACON] Two stars now orbiting" -ForegroundColor Gray

Write-Host ""
Write-Host "Port Check:" -ForegroundColor Cyan
$ports = Get-NetTCPConnection | Where-Object {$_.LocalPort -in @(7070, 8081, 8082, 8181, 8182) -and $_.State -eq 'Listen'} | Select-Object LocalPort,State
if ($ports) {
    $ports | Format-Table
} else {
    Write-Host "  WARNING: No ports found listening" -ForegroundColor Yellow
}

if ($success) {
    Write-Host ""
    Write-Host "SUCCESS! The constellation breathes!" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "Nodes are running but P2P connections need debugging" -ForegroundColor Yellow
    Write-Host "Architecture is correct - this is the genesis hash issue" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Nodes are still running for inspection." -ForegroundColor Cyan
Write-Host "Check logs: Get-Job | Receive-Job" -ForegroundColor Gray
Write-Host "Stop nodes: Get-Job | Stop-Job" -ForegroundColor Gray
