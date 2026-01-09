# Run 5-node test with ONLY 7070 mining; others accept blocks
param([int]$MonitorSeconds = 120)

$ErrorActionPreference = 'Stop'

Write-Host '=== 5-Node Test: Only 7070 mines, others sync ===' -ForegroundColor Cyan

# Kill existing nodes
Get-Process -Name 'vision-node' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

$nodes = @(
    @{ Port = 7070; P2PPort = 7072; Seeds = '127.0.0.1:8082,127.0.0.1:9092' },
    @{ Port = 8080; P2PPort = 8082; Seeds = '127.0.0.1:7072' },
    @{ Port = 9090; P2PPort = 9092; Seeds = '127.0.0.1:7072' },
    @{ Port = 10100; P2PPort = 10102; Seeds = '127.0.0.1:7072' },
    @{ Port = 11110; P2PPort = 11112; Seeds = '127.0.0.1:7072' }
)

$logDir = Join-Path $PSScriptRoot 'localhost'
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }

# Launch all 5 nodes
Write-Host 'Launching 5 nodes...' -ForegroundColor Yellow
foreach ($node in $nodes) {
    $port = $node.Port; $p2p = $node.P2PPort; $seeds = $node.Seeds
    
    # Base environment variables for all nodes
    $cmdLines = @(
        "`$env:VISION_PORT='$port'",
        "`$env:VISION_HTTP_PORT='$port'",
        "`$env:VISION_P2P_BIND='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_PORT='$p2p'",
        "`$env:VISION_P2P_ADDR='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_SEEDS='$seeds'",
        "`$env:VISION_ALLOW_PRIVATE_PEERS='true'",
        "`$env:VISION_MIN_DIFFICULTY='1'",
        "`$env:VISION_INITIAL_DIFFICULTY='1'",
        "`$env:VISION_TARGET_BLOCK_SECS='1'",
        "`$env:RUST_LOG='info'"
    )
    
    # Only enable mining on 7070
    if ($port -eq 7070) {
        $cmdLines += @(
            "`$env:VISION_LOCAL_TEST='1'",
            "`$env:VISION_MINER_ADDRESS='VISION_MINER_NODE_7070'",
            "`$env:VISION_MIN_PEERS_FOR_MINING='0'",
            "`$env:VISION_MINER_THREADS='8'"
        )
    } else {
        # Validator nodes: NO LOCAL_TEST, NO MINER_ADDRESS
        # This prevents mining eligibility via is_mining_eligible() gate
    }
    
    $cmdLines += @(
        "Set-Location '$PSScriptRoot'",
        "./target/release/vision-node.exe"
    )
    
    $cmd = ($cmdLines -join '; ')

    $outLog = Join-Path $logDir "node_${port}.out.log"
    $roleStr = if ($port -eq 7070) { "MINER" } else { "VALIDATOR" }
    Start-Process powershell -NoNewWindow -ArgumentList '-NoExit','-Command', $cmd
    Write-Host "  Node $port (P2P $p2p) [$roleStr] -> $outLog" -ForegroundColor $(if ($port -eq 7070) { 'Yellow' } else { 'Green' })
    Start-Sleep -Seconds 2
}

Write-Host 'Waiting 20s for health checks...' -ForegroundColor Yellow
Start-Sleep -Seconds 20

Write-Host 'Health checks:' -ForegroundColor Cyan
foreach ($node in $nodes) {
    try {
        $health = curl.exe -s "http://localhost:$($node.Port)/api/health"
        Write-Host "  Node $($node.Port): $health"
    } catch {
        Write-Host "  Node $($node.Port): not responding"
    }
}

Write-Host 'Waiting 60s for mesh formation...' -ForegroundColor Yellow
$meshStart = Get-Date
while (((Get-Date) - $meshStart).TotalSeconds -lt 60) {
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
    Write-Host "Peers: A=$($peerCounts[0]) B=$($peerCounts[1]) C=$($peerCounts[2]) D=$($peerCounts[3]) E=$($peerCounts[4])"
    if (($peerCounts -ge 2).Count -eq 5) { break }
}

Write-Host "`nStarting miner on node 7070 ONLY..." -ForegroundColor Cyan
try {
    Invoke-RestMethod -Method Post -Uri 'http://localhost:7070/api/miner/wallet' -ContentType 'application/json' -Body (@{ wallet = 'VISION_SINGLE_MINER_7070' } | ConvertTo-Json) | Out-Null
    Invoke-RestMethod -Method Post -Uri 'http://localhost:7070/api/miner/start' -ContentType 'application/json' -Body (@{ threads = 8 } | ConvertTo-Json) | Out-Null
    Write-Host 'Miner started on 7070 ONLY' -ForegroundColor Green
} catch {
    Write-Host 'Failed to start miner on 7070' -ForegroundColor Red
}

Write-Host "`nMonitoring all node heights for ${MonitorSeconds}s...`n" -ForegroundColor Cyan
$startTime = Get-Date
while (((Get-Date) - $startTime).TotalSeconds -lt $MonitorSeconds) {
    $timestamp = Get-Date -Format 'HH:mm:ss'
    Write-Host "[$timestamp]" -NoNewline
    
    $heights = @()
    foreach ($node in $nodes) {
        try {
            $status = curl.exe -s "http://localhost:$($node.Port)/panel_status" | ConvertFrom-Json
            Write-Host " Node $($node.Port): H=$($status.height) P=$($status.peers)" -NoNewline
            $heights += $status.height
        } catch {
            Write-Host " Node $($node.Port): ERROR" -NoNewline
        }
    }
    
    if ($heights.Count -eq 5) {
        $maxHeight = ($heights | Measure-Object -Maximum).Maximum
        $minHeight = ($heights | Measure-Object -Minimum).Minimum
        $spread = $maxHeight - $minHeight
        Write-Host " [SPREAD: $spread]" -ForegroundColor $(if ($spread -eq 0) { 'Green' } else { 'Yellow' })
    } else {
        Write-Host ""
    }
    
    Start-Sleep -Seconds 10
}

Write-Host "`nMonitoring complete. Nodes are still running." -ForegroundColor Yellow
Write-Host 'To stop: Get-Process -Name vision-node | Stop-Process -Force' -ForegroundColor Yellow
