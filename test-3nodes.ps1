Write-Host "=== Vision Node 3-Node Local Test (HTTP+P2P) ===" -ForegroundColor Cyan

function Write-Step($m) { Write-Host "[STEP] $m" -ForegroundColor Cyan }
function Write-Info($m) { Write-Host "[INFO] $m" -ForegroundColor DarkCyan }
function Write-Ok($m)   { Write-Host "[ OK ] $m" -ForegroundColor Green }
function Write-Err($m)  { Write-Host "[ERR ] $m" -ForegroundColor Red }

$root = "C:\vision-node"
$exe  = Join-Path $root "target\release\vision-node.exe"
if (!(Test-Path $exe)) {
    throw "vision-node.exe not found at $exe (build first: cargo build --release)"
}

$nodes = @(
    @{ Name = "node7070"; HttpPort = 7070; P2pPort = 17070; RunDir = (Join-Path $root "run-node7070") },
    @{ Name = "node7071"; HttpPort = 7071; P2pPort = 17071; RunDir = (Join-Path $root "run-node7071") },
    @{ Name = "node7072"; HttpPort = 7072; P2pPort = 17072; RunDir = (Join-Path $root "run-node7072") }
)

Write-Step "Stopping any running vision-node processes"
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

Write-Step "Preparing per-node run directories + local seed_peers.json"
foreach ($n in $nodes) {
    New-Item -ItemType Directory -Force -Path $n.RunDir | Out-Null

    $visionData = Join-Path $n.RunDir "vision_data"
    New-Item -ItemType Directory -Force -Path $visionData | Out-Null

    # Ensure relative config paths work when we run from RunDir
    $cfgLink = Join-Path $n.RunDir "config"
    if (!(Test-Path $cfgLink)) {
        cmd /c "mklink /J \"$cfgLink\" \"$root\config\"" | Out-Null
    }
    $pubLink = Join-Path $n.RunDir "public"
    if (!(Test-Path $pubLink)) {
        cmd /c "mklink /J \"$pubLink\" \"$root\public\"" | Out-Null
    }

    $peerSeeds = @($nodes | Where-Object { $_.Name -ne $n.Name } | ForEach-Object { "127.0.0.1:$($_.P2pPort)" })
    $seedCfg = @{
        version      = 1
        generated_at = (Get-Date).ToString("o")
        description  = "Local 3-node test seed list"
        peers        = $peerSeeds
    }
    $seedPath = Join-Path $visionData "seed_peers.json"
    $seedCfg | ConvertTo-Json -Depth 5 | Set-Content -Path $seedPath -Encoding UTF8
}

function Wait-Health($port) {
    for ($i = 0; $i -lt 60; $i++) {
        try {
            $null = Invoke-RestMethod "http://127.0.0.1:$port/health" -TimeoutSec 2
            return
        } catch {
            Start-Sleep -Milliseconds 300
        }
    }
    throw "Port $port did not become healthy in time"
}

function Get-Status($port) {
    try {
        return Invoke-RestMethod "http://127.0.0.1:$port/api/status" -TimeoutSec 2
    } catch {
        return $null
    }
}

Write-Step "Starting node7070 first (becomes local HTTP anchor)"
$n0 = $nodes[0]
$env:RUST_LOG = "info"
$env:VISION_ALLOW_PRIVATE_PEERS = "true"
$env:VISION_ANCHOR_SEEDS = "127.0.0.1"
$env:VISION_PUBLIC_IP = "127.0.0.1"
$env:VISION_PORT = "$($n0.HttpPort)"
$env:VISION_P2P_PORT = "$($n0.P2pPort)"
$env:VISION_PUBLIC_PORT = "$($n0.P2pPort)"
$env:VISION_DATA_DIR = "vision_data"
Start-Process -WorkingDirectory $n0.RunDir -FilePath $exe -RedirectStandardOutput (Join-Path $n0.RunDir "stdout.log") -RedirectStandardError (Join-Path $n0.RunDir "stderr.log") -WindowStyle Hidden | Out-Null
Wait-Health $n0.HttpPort
Write-Ok "node7070 is healthy on :$($n0.HttpPort)"

Write-Step "Starting node7071 + node7072"
foreach ($n in $nodes | Select-Object -Skip 1) {
    $env:RUST_LOG = "info"
    $env:VISION_ALLOW_PRIVATE_PEERS = "true"
    $env:VISION_ANCHOR_SEEDS = "127.0.0.1"
    $env:VISION_PUBLIC_IP = "127.0.0.1"
    $env:VISION_PORT = "$($n.HttpPort)"
    $env:VISION_P2P_PORT = "$($n.P2pPort)"
    $env:VISION_PUBLIC_PORT = "$($n.P2pPort)"
    $env:VISION_DATA_DIR = "vision_data"
    Start-Process -WorkingDirectory $n.RunDir -FilePath $exe -RedirectStandardOutput (Join-Path $n.RunDir "stdout.log") -RedirectStandardError (Join-Path $n.RunDir "stderr.log") -WindowStyle Hidden | Out-Null
    Wait-Health $n.HttpPort
    Write-Ok "$($n.Name) is healthy on :$($n.HttpPort)"
}

Write-Step "Checking /api/status on all nodes (repeat twice)"
1..2 | ForEach-Object {
    foreach ($n in $nodes) {
        $st = Get-Status $n.HttpPort
        if ($null -eq $st) {
            Write-Err "$($n.Name) :$($n.HttpPort) /api/status not available"
        } else {
            $peerCount = $st.peer_count
            $p2pHealth = $st.p2p_health
            $warmup    = $st.warmup_active
            Write-Info "$($n.Name) http=$($n.HttpPort) p2p=$($n.P2pPort) peers=$peerCount p2p_health=$p2pHealth warmup_active=$warmup"
        }
    }
    if ($_ -eq 1) { Start-Sleep -Seconds 8 }
}

Write-Host ""
Write-Host "Logs:" -ForegroundColor Yellow
foreach ($n in $nodes) {
    Write-Host "  $($n.RunDir)\\stdout.log" -ForegroundColor Gray
    Write-Host "  $($n.RunDir)\\stderr.log" -ForegroundColor Gray
}

$response = Read-Host "Stop nodes now? (Y/N)"
if ($response -eq "Y" -or $response -eq "y") {
    Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Ok "Nodes stopped"
} else {
    Write-Info "Nodes still running (stop via Task Manager or: Get-Process vision-node | Stop-Process -Force)"
}

