# test-5node-single-miner-simple.ps1
# Run 5-node test where only node 7070 mines
# Monitor heights of all nodes to verify block sync

$ErrorActionPreference = "Continue"

Write-Host "=== Vision Node 5-Node Single-Miner Test ===" -ForegroundColor Cyan
Write-Host "Node 7070: MINER (will mine blocks)" -ForegroundColor Yellow
Write-Host "Nodes 8080, 9090, 10100, 11110: VALIDATORS (receive/integrate blocks)" -ForegroundColor Green

# Clean old processes
Write-Host "`nStopping any old vision-node processes..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Clean old logs
$logDir = "c:\vision-node\localhost"
if (Test-Path $logDir) {
    Get-ChildItem $logDir -Filter "node_*.log" -ErrorAction SilentlyContinue | Remove-Item -Force
}

# Start 5 nodes directly
Write-Host "`nStarting 5-node harness..." -ForegroundColor Cyan
$binary = "c:\vision-node\target\release\vision-node.exe"
if (-not (Test-Path $binary)) {
    Write-Host "ERROR: Binary not found at $binary" -ForegroundColor Red
    exit 1
}

$nodes = @(
    @{ Port = 7070; P2PPort = 7072; Seeds = "127.0.0.1:8082,127.0.0.1:9092" },
    @{ Port = 8080; P2PPort = 8082; Seeds = "127.0.0.1:7072" },
    @{ Port = 9090; P2PPort = 9092; Seeds = "127.0.0.1:7072" },
    @{ Port = 10100; P2PPort = 10102; Seeds = "127.0.0.1:7072" },
    @{ Port = 11110; P2PPort = 11112; Seeds = "127.0.0.1:7072" }
)

foreach ($node in $nodes) {
    $port = $node.Port
    $p2pPort = $node.P2PPort
    $seeds = $node.Seeds
    
    $cmd = @"
`$env:VISION_PORT='$port'
`$env:VISION_HTTP_PORT='$port'
`$env:VISION_P2P_BIND='127.0.0.1:$p2pPort'
`$env:VISION_P2P_PORT='$p2pPort'
`$env:VISION_P2P_ADDR='127.0.0.1:$p2pPort'
`$env:VISION_P2P_SEEDS='$seeds'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_LOCAL_TEST='1'
`$env:VISION_MINER_ADDRESS='node_$port'
`$env:VISION_MIN_PEERS_FOR_MINING='3'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:VISION_MINER_THREADS='4'
`$env:RUST_LOG='info'
Set-Location 'c:\vision-node'
& '$binary'
"@

    $outLog = Join-Path $logDir "node_${port}.out.log"
    $errLog = Join-Path $logDir "node_${port}.err.log"
    Start-Process powershell -ArgumentList "-NoExit", "-Command", $cmd -RedirectStandardOutput $outLog -RedirectStandardError $errLog -WindowStyle Hidden
    Write-Host "  Node $port started (P2P: $p2pPort)" -ForegroundColor Green
    Start-Sleep -Seconds 1
}

# Function to get node height
function Get-NodeHeight {
    param([int]$port)
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/api/status" -TimeoutSec 2 -ErrorAction SilentlyContinue
        if ($response.StatusCode -eq 200) {
            $data = $response.Content | ConvertFrom-Json
            return $data.chain_height
        }
    } catch {
        return $null
    }
    return $null
}

# Function to start mining on a node
function Start-NodeMining {
    param([int]$port, [int]$threads)
    try {
        $body = @{ threads = $threads } | ConvertTo-Json
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/api/miner/start" -Method POST -Body $body -ContentType "application/json" -TimeoutSec 5
        return ($response.StatusCode -eq 200)
    } catch {
        return $false
    }
}

# Wait for peer discovery
Write-Host "`nWaiting for peer discovery (30 seconds)..." -ForegroundColor Cyan
$peerWait = 0
while ($peerWait -lt 30) {
    $h7070 = Get-NodeHeight 7070
    if ($h7070 -ne $null) {
        Write-Host "  Node 7070 healthy (height=$h7070)" -ForegroundColor Green
        break
    }
    $peerWait++
    Start-Sleep -Seconds 1
}

if ($peerWait -ge 30) {
    Write-Host "Timeout: Node 7070 not responding" -ForegroundColor Red
    Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    exit 1
}

# Give peers time to gossip and connect
Start-Sleep -Seconds 5

# Start mining ONLY on node 7070
Write-Host "`nStarting mining on node 7070 (4 threads)..." -ForegroundColor Yellow
if (Start-NodeMining 7070 4) {
    Write-Host "[OK] Mining started on node 7070" -ForegroundColor Green
} else {
    Write-Host "[FAIL] Failed to start mining on node 7070" -ForegroundColor Red
}

# Monitor heights
Write-Host "`nMonitoring block propagation for 90 seconds:" -ForegroundColor Cyan
Write-Host "Time     | 7070 | 8080 | 9090 | 10100 | 11110 | Status" -ForegroundColor White
Write-Host "---------+------+------+------+-------+-------+---------" -ForegroundColor White

$iterations = 0
$h7070_prev = 0

while ($iterations -lt 90) {
    $h7070 = Get-NodeHeight 7070
    $h8080 = Get-NodeHeight 8080
    $h9090 = Get-NodeHeight 9090
    $h10100 = Get-NodeHeight 10100
    $h11110 = Get-NodeHeight 11110
    
    # Only print if something changed or every 10 seconds
    if ($h7070 -ne $h7070_prev -or $iterations -eq 0 -or ($iterations % 10 -eq 0)) {
        $time = Get-Date -Format "HH:mm:ss"
        $h7070_disp = if ($h7070 -ne $null) { [string]$h7070 } else { "?" }
        $h8080_disp = if ($h8080 -ne $null) { [string]$h8080 } else { "?" }
        $h9090_disp = if ($h9090 -ne $null) { [string]$h9090 } else { "?" }
        $h10100_disp = if ($h10100 -ne $null) { [string]$h10100 } else { "?" }
        $h11110_disp = if ($h11110 -ne $null) { [string]$h11110 } else { "?" }
        
        # Determine status
        $status = ""
        if ($h7070 -gt 0) {
            if (($h8080 -eq $h7070) -and ($h9090 -eq $h7070) -and ($h10100 -eq $h7070) -and ($h11110 -eq $h7070)) {
                $status = "SYNCED"
            } elseif (($h8080 -eq $h7070) -or ($h9090 -eq $h7070) -or ($h10100 -eq $h7070) -or ($h11110 -eq $h7070)) {
                $status = "Syncing"
            } else {
                $status = "Waiting"
            }
        }
        
        Write-Host "$time | $h7070_disp     | $h8080_disp     | $h9090_disp     | $h10100_disp      | $h11110_disp      | $status"
        $h7070_prev = $h7070
    }
    
    $iterations = $iterations + 1
    Start-Sleep -Seconds 1
}

Write-Host "`n=== Final Status ===" -ForegroundColor Cyan
$h7070_final = Get-NodeHeight 7070
$h8080_final = Get-NodeHeight 8080
$h9090_final = Get-NodeHeight 9090
$h10100_final = Get-NodeHeight 10100
$h11110_final = Get-NodeHeight 11110

Write-Host "Node 7070 (Miner):  height=$h7070_final" -ForegroundColor Yellow
Write-Host "Node 8080:          height=$h8080_final" -ForegroundColor Green
Write-Host "Node 9090:          height=$h9090_final" -ForegroundColor Green
Write-Host "Node 10100:         height=$h10100_final" -ForegroundColor Green
Write-Host "Node 11110:         height=$h11110_final" -ForegroundColor Green

# Check for block integration
Write-Host "`nChecking block integration logs..." -ForegroundColor Cyan
@("7070","8080","9090","10100","11110") | ForEach-Object {
    $nodeId = $_
    $outLog = "c:\vision-node\localhost\node_${nodeId}.out.log"
    if (Test-Path $outLog) {
        $integratedCount = @(Select-String "Block #\d+ integrated into chain" $outLog -ErrorAction SilentlyContinue).Count
        $taitheCount = @(Select-String "block tithe:" $outLog -ErrorAction SilentlyContinue).Count
        Write-Host "Node $nodeId : $integratedCount blocks integrated, $taitheCount block tithes"
    }
}

# Cleanup
Write-Host "`n=== Test Complete - Stopping Nodes ===" -ForegroundColor Cyan
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host "All nodes stopped" -ForegroundColor Green
Write-Host "Logs available at: $logDir" -ForegroundColor Gray
