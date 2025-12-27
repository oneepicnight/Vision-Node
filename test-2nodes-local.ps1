# Two-node local connectivity + mining smoke test
# Ports:
# - Node A: HTTP 7070, P2P 7072
# - Node B: HTTP 8080, P2P 8082
#
# Requirements:
# - Build binary at target\release\vision-node.exe (this script will build if missing)
# - Uses localhost P2P; sets VISION_ALLOW_PRIVATE_PEERS=true

param(
    # If >0, keep nodes running and poll status for this many minutes.
    [int]$SoakMinutes = 0,
    # Poll interval during soak.
    [int]$PollSeconds = 10
)

$ErrorActionPreference = "Stop"

function Wait-HttpOk {
    param(
        [string]$Url,
        [int]$TimeoutSeconds = 45
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $resp = Invoke-WebRequest -UseBasicParsing -Uri $Url -TimeoutSec 2
            if ($resp.StatusCode -ge 200 -and $resp.StatusCode -lt 300) {
                return $true
            }
        } catch {
            Start-Sleep -Milliseconds 400
        }
    }

    throw "Timeout waiting for HTTP OK: $Url"
}

function Try-GetJson {
    param([string]$Url)
    try {
        return Invoke-RestMethod -Uri $Url -TimeoutSec 3
    } catch {
        return $null
    }
}

function Get-NodeSnapshot {
    param(
        [int]$HttpPort,
        [string]$Name
    )

    $status = Try-GetJson -Url "http://127.0.0.1:$HttpPort/api/status"
    $mining = Try-GetJson -Url "http://127.0.0.1:$HttpPort/api/mining/status"
    $miner = Try-GetJson -Url "http://127.0.0.1:$HttpPort/api/miner/status"

    $peerCount = if ($status -ne $null -and $status.peer_count -ne $null) { [int]$status.peer_count } else { -1 }
    $height = if ($status -ne $null -and $status.chain_height -ne $null) { [int]$status.chain_height } else { -1 }
    $miningReady = if ($mining -ne $null -and $mining.mining_ready -ne $null) { [bool]$mining.mining_ready } else { $false }
    $validatedPeers = if ($mining -ne $null -and $mining.validated_peer_count -ne $null) { [int]$mining.validated_peer_count } else { -1 }
    $blocksFound = if ($miner -ne $null -and $miner.blocks_found -ne $null) { [int]$miner.blocks_found } else { -1 }
    $hashrate = if ($miner -ne $null -and $miner.hashrate -ne $null) { [double]$miner.hashrate } else { 0 }

    return [PSCustomObject]@{
        name = $Name
        http = $HttpPort
        height = $height
        peer_count = $peerCount
        mining_ready = $miningReady
        validated_peer_count = $validatedPeers
        blocks_found = $blocksFound
        hashrate = $hashrate
    }
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $repoRoot "target\release\vision-node.exe"

if (-not (Test-Path $exe)) {
    Write-Host "[BUILD] target\\release\\vision-node.exe not found; building release..."
    Push-Location $repoRoot
    cargo build --release
    Pop-Location
}

if (-not (Test-Path $exe)) {
    throw "Build failed: $exe not found"
}

$runStamp = Get-Date -Format "yyyyMMdd-HHmmss"
$nodeA = @{ Name = "nodeA"; HttpPort = 7070; P2pPort = 7072; Dir = (Join-Path $repoRoot "run-node7070-$runStamp") }
$nodeB = @{ Name = "nodeB"; HttpPort = 8080; P2pPort = 8082; Dir = (Join-Path $repoRoot "run-node8080-$runStamp") }

# Prepare run dirs
foreach ($n in @($nodeA, $nodeB)) {
    New-Item -ItemType Directory -Path $n.Dir | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $n.Dir "vision_data") | Out-Null
    Copy-Item -Recurse -Force -Path (Join-Path $repoRoot "config") -Destination (Join-Path $n.Dir "config")
}

# Seed peers (connection maintainer reads config/seed_peers.toml)
$seedTomlA = @"
seed_peers = [
    "127.0.0.1:$($nodeB.P2pPort)",
]
min_outbound_connections = 1
max_outbound_connections = 4
connection_timeout_seconds = 10
reconnection_interval_seconds = 5
discovery_mode = "static"
"@

$seedTomlB = @"
seed_peers = [
    "127.0.0.1:$($nodeA.P2pPort)",
]
min_outbound_connections = 1
max_outbound_connections = 4
connection_timeout_seconds = 10
reconnection_interval_seconds = 5
discovery_mode = "static"
"@

$seedTomlA | Set-Content -Encoding UTF8 -Path (Join-Path $nodeA.Dir "config\seed_peers.toml")
$seedTomlB | Set-Content -Encoding UTF8 -Path (Join-Path $nodeB.Dir "config\seed_peers.toml")

Write-Host "[START] Launching nodes..."

$commonEnv = @{
    "VISION_ALLOW_PRIVATE_PEERS" = "true"
    # Used by /api/mining/status and mining readiness gating
    "VISION_MIN_PEERS_FOR_MINING" = "0"  # Allow mining with 0 peers for localhost testing
    "VISION_PUBLIC_IP" = "127.0.0.1"
    "VISION_DATA_DIR" = "vision_data"
    "VISION_ENABLE_MINER" = "true"       # Enable miner by default
    "VISION_MINER_ENABLED" = "true"      # Alternative flag
    "VISION_LOCAL_TEST_MODE" = "true"    # Force difficulty=1 for rapid block production
}

function Start-Node {
    param(
        $Node
    )

    # Set per-process environment (inherited at Start-Process time)
    foreach ($k in $commonEnv.Keys) {
        Set-Item -Path ("Env:{0}" -f $k) -Value $commonEnv[$k]
    }
    $env:VISION_PORT = "$($Node.HttpPort)"
    $env:VISION_P2P_PORT = "$($Node.P2pPort)"
    $env:VISION_PUBLIC_PORT = "$($Node.P2pPort)"

    $outFile = Join-Path $Node.Dir "stdout.log"
    $errFile = Join-Path $Node.Dir "stderr.log"
    return Start-Process -FilePath $exe -WorkingDirectory $Node.Dir -RedirectStandardOutput $outFile -RedirectStandardError $errFile -PassThru
}

$procA = Start-Node -Node $nodeA
$procB = Start-Node -Node $nodeB

try {
    Write-Host "[WAIT] Waiting for health endpoints..."
    Wait-HttpOk -Url "http://127.0.0.1:$($nodeA.HttpPort)/health" -TimeoutSeconds 60 | Out-Null
    Wait-HttpOk -Url "http://127.0.0.1:$($nodeB.HttpPort)/health" -TimeoutSeconds 60 | Out-Null

    Write-Host "[OK] Both nodes are healthy."

    Write-Host "[P2P] Waiting for peering on BOTH nodes (up to 90s)..."
    $deadline = (Get-Date).AddSeconds(90)
    while ((Get-Date) -lt $deadline) {
        $aStatus = Try-GetJson -Url "http://127.0.0.1:$($nodeA.HttpPort)/api/status"
        $bStatus = Try-GetJson -Url "http://127.0.0.1:$($nodeB.HttpPort)/api/status"
        $aPeers = 0
        $bPeers = 0
        if ($aStatus -ne $null -and $aStatus.peer_count -ne $null) { $aPeers = [int]$aStatus.peer_count }
        if ($bStatus -ne $null -and $bStatus.peer_count -ne $null) { $bPeers = [int]$bStatus.peer_count }
        if ($aPeers -gt 0 -and $bPeers -gt 0) { break }
        Start-Sleep -Seconds 2
    }

    if (-not ($aPeers -gt 0 -and $bPeers -gt 0)) {
        throw "P2P peering did not converge within 90s (nodeA peers=$aPeers, nodeB peers=$bPeers)"
    }

    # Check peer status if available
    $aStatus = Try-GetJson -Url "http://127.0.0.1:$($nodeA.HttpPort)/api/status"
    $bStatus = Try-GetJson -Url "http://127.0.0.1:$($nodeB.HttpPort)/api/status"

    if ($aStatus -ne $null) { Write-Host "[STATUS] nodeA /api/status:"; $aStatus | ConvertTo-Json -Depth 6 }
    if ($bStatus -ne $null) { Write-Host "[STATUS] nodeB /api/status:"; $bStatus | ConvertTo-Json -Depth 6 }

    # Configure miner wallet + start mining on both nodes
    # (Uses a placeholder address; update as needed)
    $minerAddr = "VISION_MINER_LOCAL_TEST"

    Write-Host "[MINER] Setting wallet + starting mining..."
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($nodeA.HttpPort)/api/miner/wallet" -Body (@{ wallet = $minerAddr } | ConvertTo-Json) -ContentType "application/json" -TimeoutSec 5 | Out-Null
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($nodeB.HttpPort)/api/miner/wallet" -Body (@{ wallet = $minerAddr } | ConvertTo-Json) -ContentType "application/json" -TimeoutSec 5 | Out-Null

    # Start mining with 4 threads for better hash rate
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($nodeA.HttpPort)/api/miner/start" -Body (@{ threads = 4 } | ConvertTo-Json) -ContentType "application/json" -TimeoutSec 5 | Out-Null
    Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:$($nodeB.HttpPort)/api/miner/start" -Body (@{ threads = 4 } | ConvertTo-Json) -ContentType "application/json" -TimeoutSec 5 | Out-Null

    Start-Sleep -Seconds 2

    $aMining = Try-GetJson -Url "http://127.0.0.1:$($nodeA.HttpPort)/api/miner/status"
    $bMining = Try-GetJson -Url "http://127.0.0.1:$($nodeB.HttpPort)/api/miner/status"

    if ($aMining -ne $null) { Write-Host "[MINER] nodeA /api/miner/status:"; $aMining | ConvertTo-Json -Depth 6 }
    if ($bMining -ne $null) { Write-Host "[MINER] nodeB /api/miner/status:"; $bMining | ConvertTo-Json -Depth 6 }

    $aMining2 = Try-GetJson -Url "http://127.0.0.1:$($nodeA.HttpPort)/api/mining/status"
    $bMining2 = Try-GetJson -Url "http://127.0.0.1:$($nodeB.HttpPort)/api/mining/status"

    if ($aMining2 -ne $null) { Write-Host "[MINING] nodeA /api/mining/status:"; $aMining2 | ConvertTo-Json -Depth 6 }
    if ($bMining2 -ne $null) { Write-Host "[MINING] nodeB /api/mining/status:"; $bMining2 | ConvertTo-Json -Depth 6 }

    if ($SoakMinutes -gt 0) {
        Write-Host "[SOAK] Running soak test for $SoakMinutes minute(s); polling every $PollSeconds second(s)..."

        $deadline = (Get-Date).AddMinutes($SoakMinutes)
        $poll = [Math]::Max(2, $PollSeconds)

        $peerDropEvents = 0
        $miningNotReadyEvents = 0
        $httpFailEvents = 0
        $iterations = 0

        while ((Get-Date) -lt $deadline) {
            $iterations++

            # Lightweight liveness check
            try {
                Wait-HttpOk -Url "http://127.0.0.1:$($nodeA.HttpPort)/health" -TimeoutSeconds 5 | Out-Null
                Wait-HttpOk -Url "http://127.0.0.1:$($nodeB.HttpPort)/health" -TimeoutSeconds 5 | Out-Null
            } catch {
                $httpFailEvents++
            }

            $snapA = Get-NodeSnapshot -HttpPort $nodeA.HttpPort -Name "nodeA"
            $snapB = Get-NodeSnapshot -HttpPort $nodeB.HttpPort -Name "nodeB"

            if ($snapA.peer_count -le 0 -or $snapB.peer_count -le 0) { $peerDropEvents++ }
            if (-not $snapA.mining_ready -or -not $snapB.mining_ready) { $miningNotReadyEvents++ }

            $ts = Get-Date -Format "HH:mm:ss"
            Write-Host ("[SOAK {0}] A: peers={1} ready={2} vpeers={3} h={4} found={5} hr={6:N1} | B: peers={7} ready={8} vpeers={9} h={10} found={11} hr={12:N1}" -f `
                $ts,
                $snapA.peer_count, $snapA.mining_ready, $snapA.validated_peer_count, $snapA.height, $snapA.blocks_found, $snapA.hashrate,
                $snapB.peer_count, $snapB.mining_ready, $snapB.validated_peer_count, $snapB.height, $snapB.blocks_found, $snapB.hashrate
            )

            Start-Sleep -Seconds $poll
        }

        Write-Host "[SOAK] Completed. iterations=$iterations peer_drop_events=$peerDropEvents mining_not_ready_events=$miningNotReadyEvents http_fail_events=$httpFailEvents"
    }

    Write-Host "[DONE] Logs written to:"
    Write-Host " - $($nodeA.Dir)\\stdout.log"
    Write-Host " - $($nodeB.Dir)\\stdout.log"

    Write-Host "[HINT] If peer_count stays 0, inspect stdout.log for [DIAL] SKIP reasons."
}
finally {
    Write-Host "[STOP] Shutting down nodes..."
    if ($procA -and -not $procA.HasExited) { $procA.Kill() }
    if ($procB -and -not $procB.HasExited) { $procB.Kill() }
}
