# dev-3node.ps1
# Start 3 local Vision nodes for multi-node testing
# Nodes run on ports 7070, 8080, 9090
# P2P ports are HTTP + 2 (7072, 8082, 9092)

param(
    [switch]$Testnet,
    [switch]$Mainnet,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node 3-Node Dev Environment ===" -ForegroundColor Cyan

# Determine network
$network = "testnet"
if ($Mainnet) {
    $network = "mainnet"
    Write-Host "Starting nodes on MAINNET" -ForegroundColor Yellow
} else {
    Write-Host "Starting nodes on TESTNET (default)" -ForegroundColor Green
}

# Clean data directories if requested
if ($Clean) {
    Write-Host "Cleaning data directories..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue vision_data_7070, vision_data_7071, vision_data_7072
    Write-Host "Data directories cleaned" -ForegroundColor Green
}

# Build release binary
Write-Host "Building release binary..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

# Node configurations
$nodes = @(
    @{ Port = 7070; P2PPort = 7072; Seeds = "127.0.0.1:8082" },
    @{ Port = 8080; P2PPort = 8082; Seeds = "127.0.0.1:9092" },
    @{ Port = 9090; P2PPort = 9092; Seeds = "127.0.0.1:7072" }
)

Write-Host "Starting 3 nodes..." -ForegroundColor Cyan

# Start each node in a new PowerShell window
foreach ($node in $nodes) {
    $port = $node.Port
    $p2pPort = $node.P2PPort
    $seeds = $node.Seeds
    
    $cmd = @"
`$env:VISION_PORT='$port';
`$env:VISION_HTTP_PORT='$port';
`$env:VISION_P2P_BIND='127.0.0.1:$p2pPort';
`$env:VISION_P2P_PORT='$p2pPort';
`$env:VISION_P2P_SEEDS='$seeds';
`$env:VISION_ALLOW_PRIVATE_PEERS='true';
`$env:VISION_MINER_ADDRESS='node_$port';
`$env:VISION_MIN_PEERS_FOR_MINING='3';
`$env:RUST_LOG='info';
Set-Location '$PSScriptRoot';
./target/release/vision-node.exe
"@

    Start-Process powershell -ArgumentList "-NoExit", "-Command", $cmd
    Write-Host "  Node $port started (P2P: $p2pPort) Seeds: $seeds" -ForegroundColor Green
    Start-Sleep -Seconds 2
}

Write-Host "`n[WAIT] Waiting for health endpoints (20s)..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

# Check all nodes are healthy
$allHealthy = $true
foreach ($node in $nodes) {
    try {
        $health = Invoke-RestMethod "http://localhost:$($node.Port)/health" -TimeoutSec 5
        if ($health.status -eq "alive") {
            Write-Host "  Node $($node.Port): OK" -ForegroundColor Green
        } else {
            Write-Host "  Node $($node.Port) health check failed: status=$($health.status)" -ForegroundColor Red
            $allHealthy = $false
        }
    } catch {
        Write-Host "  Node $($node.Port): Not responding - $_" -ForegroundColor Red
        $allHealthy = $false
    }
}

if (!$allHealthy) {
    Write-Host "[ERROR] Not all nodes are healthy - check logs" -ForegroundColor Red
}

# Wait for mesh formation
Write-Host "`n[MESH] Waiting for mesh to form (up to 60 seconds)..." -ForegroundColor Yellow
$meshFormed = $false
$waitStart = Get-Date
while (((Get-Date) - $waitStart).TotalSeconds -lt 60) {
    Start-Sleep -Seconds 5
    
    $peerCounts = @()
    foreach ($node in $nodes) {
        try {
            $status = Invoke-RestMethod "http://localhost:$($node.Port)/api/status" -TimeoutSec 3
            $peerCounts += $status.peer_count
        } catch {
            $peerCounts += 0
        }
    }
    
    Write-Host "  Peers: Node1=$($peerCounts[0]) Node2=$($peerCounts[1]) Node3=$($peerCounts[2])"
    
    # Mesh is "formed" when all nodes have at least 2 peers
    if (($peerCounts[0] -ge 2) -and ($peerCounts[1] -ge 2) -and ($peerCounts[2] -ge 2)) {
        $meshFormed = $true
        Write-Host "[OK] Mesh formed! All nodes have 2+ peers." -ForegroundColor Green
        break
    }
}

if (!$meshFormed) {
    Write-Host "[WARN] Mesh didn't fully form, but continuing..." -ForegroundColor Yellow
}

# Show detailed status
Write-Host "`n[STATUS] Node details:" -ForegroundColor Cyan
foreach ($node in $nodes) {
    try {
        $status = Invoke-RestMethod "http://localhost:$($node.Port)/api/status" -TimeoutSec 3
        Write-Host "  Node $($node.Port): Height=$($status.chain_height) Peers=$($status.peer_count) P2P=$($status.p2p_health)"
    } catch {
        Write-Host "  Node $($node.Port): Not responding" -ForegroundColor Red
    }
}

# Set miner wallet and check mining readiness
Write-Host "`n[MINER] Checking mining readiness..." -ForegroundColor Cyan
foreach ($node in $nodes) {
    try {
        $null = Invoke-RestMethod -Method Post `
            -Uri "http://localhost:$($node.Port)/api/miner/wallet" `
            -ContentType "application/json" `
            -Body '{"wallet":"VISION_MINER_3NODE_TEST"}' `
            -TimeoutSec 3
        
        $mining = Invoke-RestMethod "http://localhost:$($node.Port)/api/mining/status" -TimeoutSec 3
        Write-Host "  Node $($node.Port): Ready=$($mining.mining_ready) VPeers=$($mining.validated_peer_count)"
    } catch {
        Write-Host "  Node $($node.Port): Mining status unavailable" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "=== 3-Node Environment Running ===" -ForegroundColor Cyan
Write-Host "Node 1: http://localhost:7070 (P2P: 7072)" -ForegroundColor White
Write-Host "Node 2: http://localhost:8080 (P2P: 8082)" -ForegroundColor White
Write-Host "Node 3: http://localhost:9090 (P2P: 9092)" -ForegroundColor White
Write-Host ""
Write-Host "Check status: curl http://localhost:7070/api/status" -ForegroundColor Yellow
Write-Host "Check mining: curl http://localhost:7070/api/mining/status" -ForegroundColor Yellow
Write-Host ""
Write-Host "Press any key to stop all nodes..." -ForegroundColor Red
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

# Stop all vision-node processes
Write-Host "Stopping all nodes..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host "All nodes stopped" -ForegroundColor Green
