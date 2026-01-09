# dev-5node.ps1
# Start 5 local Vision nodes for multi-node testing
# Nodes run on ports 7070, 8080, 9090, 10100, 11110
# P2P ports are HTTP + 2 (7072, 8082, 9092, 10102, 11112)

param(
    [switch]$Testnet,
    [switch]$Mainnet,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node 5-Node Dev Environment ===" -ForegroundColor Cyan

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
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue vision_data_7070, vision_data_8080, vision_data_9090, vision_data_10100, vision_data_11110
    Write-Host "Data directories cleaned" -ForegroundColor Green
}

# Build release binary (disabled - uses existing binary)
# Write-Host "Building release binary..." -ForegroundColor Cyan
# cargo build --release
# if ($LASTEXITCODE -ne 0) {
#     Write-Error "Build failed"
#     exit 1
# }

# Check if binary exists
if (-not (Test-Path "target\release\vision-node.exe")) {
    Write-Host "Binary not found at target\release\vision-node.exe - please build first" -ForegroundColor Red
    exit 1
}
Write-Host "Using existing binary at target\release\vision-node.exe" -ForegroundColor Green

# Node configurations (mesh topology: A->B,C | B->A | C->A,D | D->A | E->A)
$nodes = @(
    @{ Port = 7070; P2PPort = 7072; Seeds = "127.0.0.1:8082,127.0.0.1:9092" },
    @{ Port = 8080; P2PPort = 8082; Seeds = "127.0.0.1:7072" },
    @{ Port = 9090; P2PPort = 9092; Seeds = "127.0.0.1:7072" },
    @{ Port = 10100; P2PPort = 10102; Seeds = "127.0.0.1:7072" },
    @{ Port = 11110; P2PPort = 11112; Seeds = "127.0.0.1:7072" }
)

Write-Host "Starting 5 nodes..." -ForegroundColor Cyan

$logDir = Join-Path $PSScriptRoot "localhost"
if (Test-Path $logDir) {
    if (-not (Get-Item $logDir).PSIsContainer) {
        Remove-Item $logDir -Force
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
} else {
    New-Item -ItemType Directory -Path $logDir | Out-Null
}

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
`$env:VISION_P2P_ADDR='127.0.0.1:$p2pPort';
`$env:VISION_P2P_SEEDS='$seeds';
`$env:VISION_ALLOW_PRIVATE_PEERS='true';
`$env:VISION_LOCAL_TEST='1';
`$env:VISION_MINER_ADDRESS='node_$port';
`$env:VISION_MIN_PEERS_FOR_MINING='3';
`$env:VISION_MIN_DIFFICULTY='1';
`$env:VISION_INITIAL_DIFFICULTY='1';
`$env:VISION_TARGET_BLOCK_SECS='1';
`$env:VISION_MINER_THREADS='4';
`$env:RUST_LOG='info';
Set-Location '$PSScriptRoot';
./target/release/vision-node.exe
"@

    $outLog = Join-Path $logDir "node_${port}.out.log"
    $errLog = Join-Path $logDir "node_${port}.err.log"
    Start-Process powershell -ArgumentList "-NoExit", "-Command", $cmd -RedirectStandardOutput $outLog -RedirectStandardError $errLog
    Write-Host "  Node $port started (P2P: $p2pPort) Seeds: $seeds -> $outLog" -ForegroundColor Green
    Start-Sleep -Seconds 2
}

Write-Host "`n[WAIT] Waiting for health endpoints (20s)..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

# Check all nodes are healthy
$allHealthy = $true
foreach ($node in $nodes) {
    try {
        $healthRaw = curl.exe -s "http://localhost:$($node.Port)/api/health"
        $healthJson = $null
        try { $healthJson = $healthRaw | ConvertFrom-Json } catch {}
        $statusVal = if ($healthJson) { $healthJson.status } else { $healthRaw }
        if ($statusVal -match "alive" -or $statusVal -match "ok") {
            Write-Host "  Node $($node.Port): OK" -ForegroundColor Green
        } else {
            Write-Host "  Node $($node.Port) health check failed: status=$statusVal" -ForegroundColor Red
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
            $status = curl.exe -s "http://localhost:$($node.Port)/panel_status" | ConvertFrom-Json
            $peerCounts += $status.peers
        } catch {
            $peerCounts += 0
        }
    }
    
    Write-Host "  Peers: A=$($peerCounts[0]) B=$($peerCounts[1]) C=$($peerCounts[2]) D=$($peerCounts[3]) E=$($peerCounts[4])"
    
    # Mesh is "formed" when all nodes have at least 3 peers (connected to most others)
    if (($peerCounts[0] -ge 3) -and ($peerCounts[1] -ge 3) -and ($peerCounts[2] -ge 3) -and ($peerCounts[3] -ge 3) -and ($peerCounts[4] -ge 3)) {
        $meshFormed = $true
        Write-Host "[OK] Mesh formed! All nodes have 3+ peers." -ForegroundColor Green
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
        $status = curl.exe -s "http://localhost:$($node.Port)/panel_status" | ConvertFrom-Json
        Write-Host "  Node $($node.Port): Height=$($status.height) Peers=$($status.peers)"
    } catch {
        Write-Host "  Node $($node.Port): Not responding" -ForegroundColor Red
    }
}

# Set miner wallet and start mining
Write-Host "`n[MINER] Configuring wallets and starting miners..." -ForegroundColor Cyan
foreach ($node in $nodes) {
    try {
        Invoke-RestMethod -Method Post -Uri "http://localhost:$($node.Port)/api/miner/wallet" -ContentType "application/json" -Body (@{ wallet = "VISION_MINER_3NODE_TEST" } | ConvertTo-Json) | Out-Null
        Invoke-RestMethod -Method Post -Uri "http://localhost:$($node.Port)/api/miner/start" -ContentType "application/json" -Body (@{ threads = 8 } | ConvertTo-Json) | Out-Null

        $mining = Invoke-RestMethod -Uri "http://localhost:$($node.Port)/api/miner/status"
        Write-Host "  Node $($node.Port): Ready=$($mining.enabled) Threads=$($mining.threads) BlocksFound=$($mining.blocks_found) Height=$($mining.last_block_height)"
    } catch {
        Write-Host "  Node $($node.Port): Mining status unavailable" -ForegroundColor Yellow
    }
}

# Probe likely mining endpoints to confirm the correct path
$miningEndpoints = @("/api/mining/status", "/api/mining", "/api/miner/status")
Write-Host "`n[MINER] Endpoint probes (HTTP codes):" -ForegroundColor Yellow
foreach ($ep in $miningEndpoints) {
    $code = (curl.exe -s -o NUL -w "%{http_code}" "http://localhost:7070$ep")
    Write-Host "  $ep -> $code"
}

Write-Host ""
Write-Host "=== 5-Node Environment Running ===" -ForegroundColor Cyan
Write-Host "NodeA: http://localhost:7070 (P2P: 7072)" -ForegroundColor White
Write-Host "NodeB: http://localhost:8080 (P2P: 8082)" -ForegroundColor White
Write-Host "NodeC: http://localhost:9090 (P2P: 9092)" -ForegroundColor White
Write-Host "NodeD: http://localhost:10100 (P2P: 10102)" -ForegroundColor White
Write-Host "NodeE: http://localhost:11110 (P2P: 11112)" -ForegroundColor White
Write-Host ""
Write-Host "Check status: curl.exe -s http://localhost:7070/api/status" -ForegroundColor Yellow
Write-Host "Check mining: curl.exe -s http://localhost:7070/api/mining/status" -ForegroundColor Yellow
Write-Host ""
Write-Host "Press any key to stop all nodes..." -ForegroundColor Red
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

# Stop all vision-node processes
Write-Host "Stopping all nodes..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host "All nodes stopped" -ForegroundColor Green
