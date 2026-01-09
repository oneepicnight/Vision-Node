# test-5node-single-miner.ps1
# Run 5-node test where only node 7070 mines
# Monitor heights of all nodes to verify block sync

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node 5-Node Single-Miner Test ===" -ForegroundColor Cyan
Write-Host "Node 7070: MINER (will mine blocks)" -ForegroundColor Yellow
Write-Host "Nodes 8080, 9090, 10100, 11110: VALIDATORS (receive/integrate blocks)" -ForegroundColor Green

# Clean old logs
$logDir = "c:\vision-node\localhost"
if (Test-Path $logDir) {
    Get-ChildItem $logDir -Filter "node_*.log" | Remove-Item -Force
}

# Start 5-node harness
Write-Host "`nStarting 5-node harness..." -ForegroundColor Cyan
& c:\vision-node\dev-3node.ps1
Start-Sleep -Seconds 8

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
Write-Host "`nMonitoring block propagation (90 seconds):" -ForegroundColor Cyan
Write-Host "Time | 7070 | 8080 | 9090 | 10100 | 11110 | Status" -ForegroundColor White
Write-Host "-----+------+------+------+-------+-------+--------" -ForegroundColor White

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
        $h7070_disp = if ($h7070 -ne $null) { $h7070 } else { "?" }
        $h8080_disp = if ($h8080 -ne $null) { $h8080 } else { "?" }
        $h9090_disp = if ($h9090 -ne $null) { $h9090 } else { "?" }
        $h10100_disp = if ($h10100 -ne $null) { $h10100 } else { "?" }
        $h11110_disp = if ($h11110 -ne $null) { $h11110 } else { "?" }
        
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
        $integratedCount = @(Select-String "Block #\d+ integrated into chain" $outLog).Count
        $taitheCount = @(Select-String "block tithe:" $outLog).Count
        Write-Host "Node $nodeId : $integratedCount blocks integrated, $taitheCount block tithes"
    }
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Logs available at: $logDir" -ForegroundColor Gray
