param(
    [int]$MonitorIterations = 12,
    [int]$MonitorIntervalSeconds = 10
)

$ErrorActionPreference = 'Stop'

Write-Host '=== Vision Node 5-Node (single miner @7070) ===' -ForegroundColor Cyan

# Stop any leftover nodes
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

foreach ($node in $nodes) {
    $port = $node.Port; $p2p = $node.P2PPort; $seeds = $node.Seeds
    # Build command lines explicitly to avoid quoting issues
    $cmdLines = @(
        "`$env:VISION_PORT='$port'",
        "`$env:VISION_HTTP_PORT='$port'",
        "`$env:VISION_P2P_BIND='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_PORT='$p2p'",
        "`$env:VISION_P2P_ADDR='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_SEEDS='$seeds'",
        "`$env:VISION_ALLOW_PRIVATE_PEERS='true'",
        "`$env:VISION_LOCAL_TEST='1'",
        "`$env:VISION_MINER_ADDRESS='node_$port'",
        "`$env:VISION_MIN_PEERS_FOR_MINING='3'",
        "`$env:VISION_MIN_DIFFICULTY='1'",
        "`$env:VISION_INITIAL_DIFFICULTY='1'",
        "`$env:VISION_TARGET_BLOCK_SECS='1'",
        "`$env:VISION_MINER_THREADS='4'",
        "`$env:RUST_LOG='info'",
        "Set-Location '$PSScriptRoot'",
        "./target/release/vision-node.exe"
    )
    $cmd = ($cmdLines -join '; ')

    # Launch in a visible PowerShell window so you can see live logs; no redirection.
    Start-Process powershell -ArgumentList '-NoExit','-Command', $cmd
    Write-Host "Started node $port (P2P $p2p) [logs visible in window]" -ForegroundColor Green
    Start-Sleep -Seconds 2
}

Write-Host 'Waiting 20s for health...' -ForegroundColor Yellow
Start-Sleep -Seconds 20

foreach ($node in $nodes) {
    $port = $node.Port
    $healthRaw = curl.exe -s "http://localhost:$port/api/health"
    $health = $healthRaw
    try { $health = ($healthRaw | ConvertFrom-Json).status } catch {}
    Write-Host "Health ${port}: $health"
}

Write-Host 'Waiting up to 60s for mesh...' -ForegroundColor Yellow
$meshFormed = $false
$waitStart = Get-Date
while (((Get-Date) - $waitStart).TotalSeconds -lt 60) {
    Start-Sleep -Seconds 5
    $peerCounts = @()
    foreach ($node in $nodes) {
        try { $peerCounts += (curl.exe -s "http://localhost:$($node.Port)/panel_status" | ConvertFrom-Json).peers } catch { $peerCounts += 0 }
    }
    Write-Host "Peers: A=$($peerCounts[0]) B=$($peerCounts[1]) C=$($peerCounts[2]) D=$($peerCounts[3]) E=$($peerCounts[4])"
    if (($peerCounts -ge 3).Count -eq 5) { $meshFormed = $true; break }
}
if ($meshFormed) { Write-Host 'Mesh formed (all have >=3 peers)' -ForegroundColor Green } else { Write-Host 'Mesh not fully formed (continuing)' -ForegroundColor Yellow }

Write-Host 'Starting miner only on 7070...' -ForegroundColor Cyan
try {
    Invoke-RestMethod -Method Post -Uri 'http://localhost:7070/api/miner/wallet' -ContentType 'application/json' -Body (@{ wallet = 'VISION_MINER_SINGLE_7070' } | ConvertTo-Json) | Out-Null
    Invoke-RestMethod -Method Post -Uri 'http://localhost:7070/api/miner/start' -ContentType 'application/json' -Body (@{ threads = 8 } | ConvertTo-Json) | Out-Null
    Write-Host 'Miner started on 7070' -ForegroundColor Green
} catch {
    Write-Host 'Failed to start miner on 7070' -ForegroundColor Red
}

Write-Host "Monitoring heights for $MonitorIterations iterations (interval $MonitorIntervalSeconds s)..." -ForegroundColor Cyan
for ($i = 1; $i -le $MonitorIterations; $i++) {
    $timestamp = Get-Date -Format 'HH:mm:ss'
    $line = "[$timestamp]"
    foreach ($node in $nodes) {
        try {
            $status = curl.exe -s "http://localhost:$($node.Port)/panel_status" | ConvertFrom-Json
            $line += " ${node.Port}:H=$($status.height) P=$($status.peers)"
        } catch {
            $line += " ${node.Port}:H=? P=?"
        }
    }
    Write-Host $line
    Start-Sleep -Seconds $MonitorIntervalSeconds
}

Write-Host 'Monitoring done. Nodes are still running.' -ForegroundColor Yellow
Write-Host 'To stop nodes: Get-Process -Name vision-node | Stop-Process -Force' -ForegroundColor Yellow
