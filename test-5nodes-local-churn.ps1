#!/usr/bin/env pwsh
# 5-Node Mesh Growth Soak Test
# Tests P2P mesh formation with minimal seeding - let gossip work naturally

param(
    [int]$SoakMinutes = 10,
    [int]$PollSeconds = 15,
    [int]$PeerWaitSeconds = 120,

[switch]$Churn = $false,
[ValidateSet("rolling","random","hubkill")][string]$ChurnMode = "rolling",
[int]$ChurnEverySeconds = 30,
[int]$ChurnDownSeconds = 20,
[int]$ChurnStartAfterSeconds = 60

    )

$ErrorActionPreference = "Stop"

# Node configurations (HTTP base, P2P = HTTP + 2)
$nodes = @(
    @{ HttpPort = 7070; P2PPort = 7072; Name = "NodeA" }
    @{ HttpPort = 8080; P2PPort = 8082; Name = "NodeB" }
    @{ HttpPort = 9090; P2PPort = 9092; Name = "NodeC" }
    @{ HttpPort = 10100; P2PPort = 10102; Name = "NodeD" }
    @{ HttpPort = 11110; P2PPort = 11112; Name = "NodeE" }
)

# Seed configuration: Node A only knows about B and C (let mesh form naturally)
$seedConfig = @{
    7070 = "127.0.0.1:8082,127.0.0.1:9092"   # A -> B,C
    8080 = "127.0.0.1:7072"                   # B -> A
    9090 = "127.0.0.1:7072"                   # C -> A
    10100 = ""                                 # D -> no seeds (learns via gossip)
    11110 = ""                                 # E -> no seeds (learns via gossip)
}

$processes = @()
$logDirs = @()
$startTime = Get-Date -Format "yyyyMMdd-HHmmss"

Write-Host "`n=== 5-Node Mesh Growth Test ===" -ForegroundColor Cyan
Write-Host "Soak: $SoakMinutes min | Poll: $PollSeconds sec | Peer wait: $PeerWaitSeconds sec`n"

function Stop-AllNodes {
    Write-Host "[STOP] Shutting down all nodes..." -ForegroundColor Yellow
    foreach ($proc in $processes) {
        if ($proc -and !$proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
    Start-Sleep -Seconds 2
}


function Get-ErrKind([string]$msg) {
    $m = $msg.ToLowerInvariant()
    if ($m -match "timed out|timeout") { return "timeout" }
    if ($m -match "actively refused|refused") { return "conn_refused" }
    if ($m -match "cannot connect|no connection") { return "no_connect" }
    if ($m -match "forcibly closed|connection was closed") { return "conn_closed" }
    if ($m -match "name resolution|could not resolve") { return "dns" }
    return "http_err"
}

function Start-Node($node, $launchScriptPath) {
    # Starts node using its per-node launch script generated earlier in this run.
    $p = Start-Process -FilePath "pwsh" -ArgumentList "-NoProfile","-ExecutionPolicy","Bypass","-File",$launchScriptPath `
        -RedirectStandardOutput (Join-Path (Split-Path $launchScriptPath) "stdout.log") `
        -RedirectStandardError  (Join-Path (Split-Path $launchScriptPath) "stderr.log") `
        -PassThru -WindowStyle Hidden
    return $p
}

function Stop-Node($proc) {
    if ($proc -and !$proc.HasExited) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}

try {
    # Launch all nodes
    Write-Host "[START] Launching 5 nodes..." -ForegroundColor Green
    
    foreach ($node in $nodes) {
        $httpPort = $node.HttpPort
        $p2pPort = $node.P2PPort
        $name = $node.Name
        $seeds = $seedConfig[$httpPort]
        
        $logDir = "run-node$httpPort-$startTime"
        $logDirs += $logDir
        New-Item -ItemType Directory -Path $logDir -Force | Out-Null
        
        $dataDir = Join-Path (Join-Path $PSScriptRoot $logDir) "data"
        New-Item -ItemType Directory -Path $dataDir -Force | Out-Null  # Pre-create data dir
        
        Write-Host "  $name : HTTP=$httpPort P2P=$p2pPort Seeds=$seeds"
        Write-Host "           Data=$dataDir" -ForegroundColor DarkGray
        
        # Create environment hashtable for this node
        $nodeEnv = @{
            VISION_PORT = $httpPort.ToString()  # Used for default data dir
            VISION_HTTP_PORT = $httpPort.ToString()
            VISION_P2P_PORT = $p2pPort.ToString()
            VISION_P2P_SEEDS = $seeds
            VISION_DATA_DIR = $dataDir
            VISION_ALLOW_PRIVATE_PEERS = "true"
            VISION_LOCAL_TEST = "1"
            VISION_PEERBOOK_SCOPE = "local-5nodes"
            LOCAL_TEST_MODE = "true"
        }
        
        # Create a launch script for this node that sets env vars
        $launchScript = Join-Path $PSScriptRoot "$logDir\launch.ps1"
        $node.LaunchScript = $launchScript
        $node.LogDir = $logDir
        $node.DataDir = $dataDir
        $envVarScript = ($nodeEnv.GetEnumerator() | ForEach-Object { "`$env:$($_.Key) = '$($_.Value)'" }) -join "`n"
        @"
$envVarScript
& '.\target\release\vision-node.exe' run
"@ | Out-File -FilePath $launchScript -Encoding UTF8
        
        $proc = Start-Process -FilePath "powershell.exe" `
            -ArgumentList "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $launchScript `
            -RedirectStandardOutput "$logDir\stdout.log" `
            -RedirectStandardError "$logDir\stderr.log" `
            -PassThru `
            -NoNewWindow
        
        $processes += $proc
        Start-Sleep -Milliseconds 500
    }
    
    Write-Host "`n[WAIT] Waiting for health endpoints..." -ForegroundColor Yellow
    Start-Sleep -Seconds 20
    
    # Check processes are running
    $runningProcs = @(Get-Process vision-node -ErrorAction SilentlyContinue)
    Write-Host "  Running vision-node processes: $($runningProcs.Count)" -ForegroundColor Cyan
    
    # Check all nodes are healthy
    $allHealthy = $true
    foreach ($node in $nodes) {
        try {
            $health = Invoke-RestMethod "http://localhost:$($node.HttpPort)/health" -TimeoutSec 5
            if ($health.status -eq "alive") {
                Write-Host "  $($node.Name) (port $($node.HttpPort)): OK" -ForegroundColor Green
            } else {
                Write-Host "  $($node.Name) health check failed: status=$($health.status)" -ForegroundColor Red
                $allHealthy = $false
            }
        } catch {
            Write-Host "  $($node.Name) (port $($node.HttpPort)): Not responding - $_" -ForegroundColor Red
            $allHealthy = $false
        }
    }
    
    if (!$allHealthy) {
        throw "Not all nodes are healthy"
    }
    Write-Host "[OK] All 5 nodes are healthy." -ForegroundColor Green
    
    # Wait for mesh formation
    Write-Host "`n[MESH] Waiting for mesh to form (up to $PeerWaitSeconds seconds)..." -ForegroundColor Yellow
    
    $meshFormed = $false
    $waitStart = Get-Date
    while (((Get-Date) - $waitStart).TotalSeconds -lt $PeerWaitSeconds) {
        Start-Sleep -Seconds 5
        
        $peerCounts = @()
        foreach ($node in $nodes) {
            try {
                $status = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/status" -TimeoutSec 3
                $peerCounts += $status.peer_count
            } catch {
                $peerCounts += 0
            }
        }
        
        $avgPeers = ($peerCounts | Measure-Object -Average).Average
        $minPeers = ($peerCounts | Measure-Object -Minimum).Minimum
        
        Write-Host "  Peers: A=$($peerCounts[0]) B=$($peerCounts[1]) C=$($peerCounts[2]) D=$($peerCounts[3]) E=$($peerCounts[4]) | Min=$minPeers Avg=$([math]::Round($avgPeers,1))"
        
        # Mesh is "formed" when all nodes have at least 2 peers
        if ($minPeers -ge 2) {
            $meshFormed = $true
            Write-Host "[OK] Mesh formed! All nodes have 2+ peers." -ForegroundColor Green
            break
        }
    }
    
    if (!$meshFormed) {
        Write-Host "[WARN] Mesh didn't fully form, but continuing test..." -ForegroundColor Yellow
    }
    
    # Show detailed status
    Write-Host "`n[STATUS] Node details:" -ForegroundColor Cyan
    foreach ($node in $nodes) {
        try {
            $status = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/status" -TimeoutSec 3
            Write-Host "  $($node.Name): Height=$($status.chain_height) Peers=$($status.peer_count) P2P=$($status.p2p_health)"
        } catch {
            Write-Host "  $($node.Name): Not responding" -ForegroundColor Red
        }
    }
    
    # Set miner wallet and start mining on all nodes
    Write-Host "`n[MINER] Starting mining on all nodes..." -ForegroundColor Cyan
    foreach ($node in $nodes) {
        try {
            $null = Invoke-RestMethod -Method Post `
                -Uri "http://localhost:$($node.HttpPort)/api/miner/wallet" `
                -ContentType "application/json" `
                -Body '{"wallet":"VISION_MINER_5NODE_TEST"}' `
                -TimeoutSec 3
            
            $null = Invoke-RestMethod -Method Post `
                -Uri "http://localhost:$($node.HttpPort)/api/miner/start" `
                -ContentType "application/json" `
                -Body '{"threads":4}' `
                -TimeoutSec 3
            
            Write-Host "  $($node.Name): Mining started" -ForegroundColor Green
        } catch {
            Write-Host "  $($node.Name) miner setup failed: $_" -ForegroundColor Yellow
        }
    }
    
    Start-Sleep -Seconds 3
    
    # Check mining status
    foreach ($node in $nodes) {
        try {
            $mining = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/mining/status" -TimeoutSec 3
            Write-Host "  $($node.Name): Ready=$($mining.mining_ready) VPeers=$($mining.validated_peer_count)"
        } catch {
            Write-Host "  $($node.Name): Mining status unavailable" -ForegroundColor Yellow
        }
    }
    
    # Soak test loop
    Write-Host "`n[SOAK] Running soak test for $SoakMinutes minute(s); polling every $PollSeconds second(s)..." -ForegroundColor Green
    
    $iterations = 0
    $peerDropEvents = 0
    $miningNotReadyEvents = 0
    $httpFailEvents = 0
    $soakEnd = (Get-Date).AddMinutes($SoakMinutes)
    
    while ((Get-Date) -lt $soakEnd) {
        $iterations++
        Start-Sleep -Seconds $PollSeconds
        
        $timestamp = Get-Date -Format "HH:mm:ss"
        $statusLine = "[$timestamp]"
        
        foreach ($node in $nodes) {
            try {
                $status = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/status" -TimeoutSec 3
                $miner = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/miner/status" -TimeoutSec 3
                $mining = Invoke-RestMethod "http://localhost:$($node.HttpPort)/api/mining/status" -TimeoutSec 3
                
                $peers = $status.peer_count
                $ready = $mining.mining_ready
                $vpeers = $mining.validated_peer_count
                $height = $status.chain_height
                $found = $miner.blocks_found
                $hr = [math]::Round($miner.hashrate, 1)
                
                $statusLine += " $($node.Name): p=$peers vp=$vpeers h=$height f=$found hr=$hr |"
                
                if ($peers -eq 0) { $peerDropEvents++ }
                if (!$ready) { $miningNotReadyEvents++ }
                
            } catch {
                $kind = Get-ErrKind $_.Exception.Message
                $statusLine += " $($node.Name): ERR($kind) |"
                $httpFailEvents++
            }
        }
        
        Write-Host $statusLine -ForegroundColor $(if($httpFailEvents -eq 0){'Green'}else{'Yellow'})
    }
    
    Write-Host "`n[SOAK] Completed. iterations=$iterations peer_drop_events=$peerDropEvents mining_not_ready_events=$miningNotReadyEvents http_fail_events=$httpFailEvents" -ForegroundColor Cyan
    
    # Show dial failures from all nodes
    Write-Host "`n[DEBUG] Dial failures across the network:" -ForegroundColor Cyan
    foreach ($node in $nodes) {
        try {
            $debug = Invoke-RestMethod "http://localhost:$($node.HttpPort)/p2p/debug" -TimeoutSec 3
            Write-Host "  $($node.Name): $($debug.dial_failures.Count) failures | Connected: $($debug.connected_peers.Count) | PeerBook: $($debug.peer_book_counts.total)"
            
            if ($debug.dial_failures.Count -gt 0) {
                $debug.dial_failures | Select-Object -First 3 | ForEach-Object {
                    Write-Host "    - $($_.addr): $($_.reason) (source: $($_.source))" -ForegroundColor Yellow
                }
            }
        } catch {
            Write-Host "  $($node.Name): Debug endpoint unavailable" -ForegroundColor Red
        }
    }
    
    Write-Host "`n[DONE] Logs written to:"
    foreach ($logDir in $logDirs) {
        Write-Host " - $PSScriptRoot\$logDir\stdout.log"
    }
    
    Write-Host "`n[HINT] Ports used: 7070, 8080, 9090, 10100, 11110" -ForegroundColor Cyan
    Write-Host "[HINT] Check dial failures with: curl http://localhost:7070/p2p/debug | jq .dial_failures" -ForegroundColor Cyan
    
} catch {
    Write-Host "`n[ERROR] Test failed: $_" -ForegroundColor Red
    Write-Host $_.ScriptStackTrace -ForegroundColor Red
} finally {
    Stop-AllNodes
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
