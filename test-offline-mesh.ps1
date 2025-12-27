# OFFLINE-FIRST MESH PROOF OF CONCEPT
# Proves that the Vision Node P2P mesh works WITHOUT a guardian.
# Test: Run 2 local nodes, guardian OFFLINE, manually connect them.
# Expected: Handshake succeeds, trust=Untrusted, full P2P connectivity.

Write-Host "`n================================================================" -ForegroundColor Cyan
Write-Host "  OFFLINE-FIRST MESH TEST - Guardian Independence Proof  " -ForegroundColor Cyan
Write-Host "================================================================`n" -ForegroundColor Cyan

# Step 0: Kill any existing nodes
Write-Host "[Step 0] Stopping any running vision-node processes..." -ForegroundColor Yellow
Get-Process -Name vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Step 1: Verify guardian is OFF
Write-Host "`n[Step 1] Verifying Guardian is OFFLINE..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "http://localhost:7070/api/health" -TimeoutSec 2 -ErrorAction Stop
    Write-Host "  FAIL: Guardian is still running on port 7070!" -ForegroundColor Red
    Write-Host "  Stop it manually and re-run this test." -ForegroundColor Red
    exit 1
} catch {
    Write-Host "  Guardian is OFF (as expected)" -ForegroundColor Green
}

# Step 2: Setup node directories
Write-Host "`n[Step 2] Setting up test directories..." -ForegroundColor Yellow
$nodeADir = "c:\vision-node\test-node-a"
$nodeBDir = "c:\vision-node\test-node-b"

if (Test-Path $nodeADir) { Remove-Item -Path $nodeADir -Recurse -Force }
if (Test-Path $nodeBDir) { Remove-Item -Path $nodeBDir -Recurse -Force }

New-Item -Path $nodeADir -ItemType Directory | Out-Null
New-Item -Path $nodeBDir -ItemType Directory | Out-Null

# Copy necessary files
Copy-Item "keys.json" "$nodeADir\keys.json" -ErrorAction SilentlyContinue
Copy-Item "keys.json" "$nodeBDir\keys.json" -ErrorAction SilentlyContinue

# Create config directories and copy token_accounts.toml
New-Item -Path "$nodeADir\config" -ItemType Directory -Force | Out-Null
New-Item -Path "$nodeBDir\config" -ItemType Directory -Force | Out-Null
Copy-Item "config\token_accounts.toml" "$nodeADir\config\token_accounts.toml" -ErrorAction Stop
Copy-Item "config\token_accounts.toml" "$nodeBDir\config\token_accounts.toml" -ErrorAction Stop

Write-Host "  Created: $nodeADir" -ForegroundColor Green
Write-Host "  Created: $nodeBDir" -ForegroundColor Green

# Step 3: Create minimal constellation.json
Write-Host "`n[Step 3] Creating offline constellation configs..." -ForegroundColor Yellow

$constellationConfigA = @'
{
  "guardian_base_url": "http://localhost:7070",
  "bootstrap_mode": "manual"
}
'@

$constellationConfigB = @'
{
  "guardian_base_url": "http://localhost:7070",
  "bootstrap_mode": "manual"
}
'@

Set-Content -Path "$nodeADir\constellation.json" -Value $constellationConfigA
Set-Content -Path "$nodeBDir\constellation.json" -Value $constellationConfigB

Write-Host "  Configs created (guardian URL present but won't be used)" -ForegroundColor Green

# Step 4: Start Node A (port 7071)
Write-Host "`n[Step 4] Starting Node A on port 7071..." -ForegroundColor Yellow

$nodeAEnv = @{
    VISION_PORT = "7071"
    VISION_HOST = "0.0.0.0"
    RUST_LOG = "info,vision_node::p2p=debug"
    VISION_DATA_DIR = $nodeADir
    VISION_GUARDIAN_MODE = "false"
}

$nodeACmd = "cd '$nodeADir'; " + ($nodeAEnv.GetEnumerator() | ForEach-Object { "`$env:$($_.Key)='$($_.Value)'; " }) + "..\target\release\vision-node.exe"

Start-Process powershell -ArgumentList "-NoExit", "-Command", $nodeACmd -WindowStyle Normal
Write-Host "  Node A starting in external window (port 7071)" -ForegroundColor Green

# Step 5: Start Node B (port 7072)
Write-Host "`n[Step 5] Starting Node B on port 7072..." -ForegroundColor Yellow

$nodeBEnv = @{
    VISION_PORT = "7072"
    VISION_HOST = "0.0.0.0"
    RUST_LOG = "info,vision_node::p2p=debug"
    VISION_DATA_DIR = $nodeBDir
    VISION_GUARDIAN_MODE = "false"
}

$nodeBCmd = "cd '$nodeBDir'; " + ($nodeBEnv.GetEnumerator() | ForEach-Object { "`$env:$($_.Key)='$($_.Value)'; " }) + "..\target\release\vision-node.exe"

Start-Process powershell -ArgumentList "-NoExit", "-Command", $nodeBCmd -WindowStyle Normal
Write-Host "  Node B starting in external window (port 7072)" -ForegroundColor Green

# Step 6: Wait for nodes to boot
Write-Host "`n[Step 6] Waiting for nodes to boot (15 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Step 7: Connect nodes manually
Write-Host "`n[Step 7] Manually connecting nodes (peer addition)..." -ForegroundColor Yellow

Write-Host "  Node A: Adding Node B as peer..." -ForegroundColor Gray
try {
    $addPeerA = @{
        url = "http://127.0.0.1:7072"
    } | ConvertTo-Json
    
    $respA = Invoke-RestMethod -Uri "http://localhost:7071/api/peers/add" -Method Post -Body $addPeerA -ContentType "application/json" -TimeoutSec 5
    Write-Host "    Node A added Node B: $($respA.message)" -ForegroundColor Green
} catch {
    Write-Host "    Node A peer add failed: $_" -ForegroundColor Red
}

Write-Host "  Node B: Adding Node A as peer..." -ForegroundColor Gray
try {
    $addPeerB = @{
        url = "http://127.0.0.1:7071"
    } | ConvertTo-Json
    
    $respB = Invoke-RestMethod -Uri "http://localhost:7072/api/peers/add" -Method Post -Body $addPeerB -ContentType "application/json" -TimeoutSec 5
    Write-Host "    Node B added Node A: $($respB.message)" -ForegroundColor Green
} catch {
    Write-Host "    Node B peer add failed: $_" -ForegroundColor Red
}

# Step 8: Wait for handshake
Write-Host "`n[Step 8] Waiting for P2P handshake (5 seconds)..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Step 9: Check peer status
Write-Host "`n[Step 9] Checking peer status..." -ForegroundColor Yellow

Write-Host "`n  Node A peers:" -ForegroundColor Cyan
try {
    $peersA = Invoke-RestMethod -Uri "http://localhost:7071/api/peers" -TimeoutSec 3
    if ($peersA.peers -and $peersA.peers.Count -gt 0) {
        Write-Host "    Connected peers: $($peersA.peers.Count)" -ForegroundColor Green
        $peersA.peers | ForEach-Object { Write-Host "       - $_" -ForegroundColor White }
    } else {
        Write-Host "    No peers connected yet" -ForegroundColor Yellow
    }
} catch {
    Write-Host "    Failed to get peers: $_" -ForegroundColor Red
}

Write-Host "`n  Node B peers:" -ForegroundColor Cyan
try {
    $peersB = Invoke-RestMethod -Uri "http://localhost:7072/api/peers" -TimeoutSec 3
    if ($peersB.peers -and $peersB.peers.Count -gt 0) {
        Write-Host "    Connected peers: $($peersB.peers.Count)" -ForegroundColor Green
        $peersB.peers | ForEach-Object { Write-Host "       - $_" -ForegroundColor White }
    } else {
        Write-Host "    No peers connected yet" -ForegroundColor Yellow
    }
} catch {
    Write-Host "    Failed to get peers: $_" -ForegroundColor Red
}

# Step 10: Instructions
Write-Host "`n================================================================" -ForegroundColor Green
Write-Host "                TEST SETUP COMPLETE                       " -ForegroundColor Green
Write-Host "================================================================`n" -ForegroundColor Green

Write-Host "What to look for in the node windows:" -ForegroundColor Yellow
Write-Host ""
Write-Host "  SUCCESS INDICATORS:" -ForegroundColor Green
Write-Host "     - [P2P] Outbound connection established to 127.0.0.1:707X" -ForegroundColor White
Write-Host "     - [P2P] Handshake WireHandshakeV1 accepted" -ForegroundColor White
Write-Host "     - [PEER_MANAGER] Registered new peer ... trust=Untrusted" -ForegroundColor White
Write-Host "     - A new star joins the constellation" -ForegroundColor White
Write-Host ""
Write-Host "  FAILURE INDICATORS:" -ForegroundColor Red
Write-Host "     - Guardian call failed or ticket required" -ForegroundColor White
Write-Host "     - Network isolated or bootstrap failed" -ForegroundColor White
Write-Host "     - Handshake rejected" -ForegroundColor White
Write-Host ""
Write-Host "  NOTE:" -ForegroundColor Cyan
Write-Host "     Both nodes should show trust=Untrusted (no passports yet)" -ForegroundColor White
Write-Host "     This is CORRECT and EXPECTED for offline-first operation." -ForegroundColor White
Write-Host ""
Write-Host "If you see trust=Untrusted + successful handshake:" -ForegroundColor Magenta
Write-Host "  PROOF COMPLETE - Mesh works without guardian!" -ForegroundColor Green
Write-Host ""
Write-Host "Press Ctrl+C in node windows to stop them when done." -ForegroundColor Gray
Write-Host ""
