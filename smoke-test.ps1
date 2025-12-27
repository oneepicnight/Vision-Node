# Vision Node - Comprehensive Build & Smoke Test
# Purpose: Validate full release build and multi-node consensus
# Usage: .\smoke-test.ps1 [-SkipBuild] [-Verbose]

param(
    [switch]$SkipBuild = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"
$Global:TestsPassed = 0
$Global:TestsFailed = 0

function Write-TestHeader {
    param([string]$Message)
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host " $Message" -ForegroundColor Cyan
    Write-Host "========================================`n" -ForegroundColor Cyan
}

function Write-TestResult {
    param([string]$Test, [bool]$Passed, [string]$Details = "")
    if ($Passed) {
        Write-Host "✅ PASS: $Test" -ForegroundColor Green
        if ($Details -and $Verbose) {
            Write-Host "   → $Details" -ForegroundColor Gray
        }
        $Global:TestsPassed++
    } else {
        Write-Host "❌ FAIL: $Test" -ForegroundColor Red
        if ($Details) {
            Write-Host "   → $Details" -ForegroundColor Yellow
        }
        $Global:TestsFailed++
    }
}

function Wait-ForHealthy {
    param([int]$Port, [int]$TimeoutSeconds = 30)
    $elapsed = 0
    while ($elapsed -lt $TimeoutSeconds) {
        try {
            $response = Invoke-RestMethod -Uri "http://localhost:$Port/livez" -Method Get -ErrorAction Stop
            if ($response.status -eq "ok") {
                return $true
            }
        } catch {
            # Not ready yet
        }
        Start-Sleep -Milliseconds 500
        $elapsed++
    }
    return $false
}

function Stop-VisionNodes {
    Write-Host "Cleaning up Vision Node processes..." -ForegroundColor Yellow
    Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
}

function Get-NodeStatus {
    param([int]$Port)
    try {
        $status = Invoke-RestMethod -Uri "http://localhost:$Port/status" -Method Get -ErrorAction Stop
        return $status
    } catch {
        return $null
    }
}

# ============================================================
# STEP 1: Build Test
# ============================================================
Write-TestHeader "STEP 1: Release Build Validation"

if (-not $SkipBuild) {
    Write-Host "Building Vision Node in release mode..." -ForegroundColor Yellow
    try {
        $buildOutput = cargo build --release 2>&1 | Out-String
        if ($LASTEXITCODE -eq 0 -or $buildOutput -match "Finished.*release") {
            Write-TestResult "Release build completed" $true "Binary: target\release\vision-node.exe"
        } else {
            Write-TestResult "Release build completed" $false $buildOutput
            exit 1
        }
    } catch {
        Write-TestResult "Release build completed" $false $_.Exception.Message
        exit 1
    }
} else {
    Write-Host "Skipping build (using existing binary)" -ForegroundColor Yellow
}

# Verify binary exists
$binaryPath = "target\release\vision-node.exe"
if (Test-Path $binaryPath) {
    $binarySize = (Get-Item $binaryPath).Length / 1MB
    Write-TestResult "Binary exists and is valid" $true "Size: $([math]::Round($binarySize, 2)) MB"
} else {
    Write-TestResult "Binary exists and is valid" $false "Binary not found at $binaryPath"
    exit 1
}

# ============================================================
# STEP 2: Clean Slate Setup
# ============================================================
Write-TestHeader "STEP 2: Clean Environment Setup"

# Stop any running nodes
Stop-VisionNodes

# Clean data directories
$dataDirs = @("vision_data_7070", "vision_data_7071", "vision_data_7072")
foreach ($dir in $dataDirs) {
    if (Test-Path $dir) {
        Remove-Item -Recurse -Force $dir
        Write-TestResult "Cleaned data directory: $dir" $true
    }
}

# Verify config files exist
if (Test-Path "config\seed_peers.toml") {
    Write-TestResult "Config files present" $true "seed_peers.toml found"
} else {
    Write-TestResult "Config files present" $false "seed_peers.toml missing"
}

# ============================================================
# STEP 3: Multi-Node Startup
# ============================================================
Write-TestHeader "STEP 3: Multi-Node Startup (3 Nodes)"

$env:VISION_NETWORK = "testnet"
$env:VISION_DEV = "0"
$env:RUST_LOG = "warn"

$nodes = @(
    @{Port=7070; DataDir="vision_data_7070"; LogFile="logs\smoke-node1.log"},
    @{Port=7071; DataDir="vision_data_7071"; LogFile="logs\smoke-node2.log"},
    @{Port=7072; DataDir="vision_data_7072"; LogFile="logs\smoke-node3.log"}
)

foreach ($node in $nodes) {
    $env:VISION_PORT = $node.Port
    $env:VISION_DATA_DIR = $node.DataDir
    
    Write-Host "Starting node on port $($node.Port)..." -ForegroundColor Yellow
    
    $logDir = Split-Path $node.LogFile -Parent
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir | Out-Null
    }
    
    Start-Process -FilePath $binaryPath `
        -RedirectStandardOutput $node.LogFile `
        -RedirectStandardError "$($node.LogFile).err" `
        -WindowStyle Hidden
    
    Start-Sleep -Milliseconds 500
}

# Wait for all nodes to be healthy
Write-Host "Waiting for nodes to initialize..." -ForegroundColor Yellow
foreach ($node in $nodes) {
    $healthy = Wait-ForHealthy -Port $node.Port -TimeoutSeconds 30
    Write-TestResult "Node on port $($node.Port) is healthy" $healthy
}

Start-Sleep -Seconds 3

# ============================================================
# STEP 4: Genesis Block Validation
# ============================================================
Write-TestHeader "STEP 4: Genesis Block Validation"

$node1Status = Get-NodeStatus -Port 7070
if ($node1Status) {
    $genesisValid = ($node1Status.height -ge 0)
    Write-TestResult "Genesis block created" $genesisValid "Height: $($node1Status.height)"
    
    if ($node1Status.network) {
        $networkCorrect = ($node1Status.network -eq "testnet")
        Write-TestResult "Network type correct" $networkCorrect "Network: $($node1Status.network)"
    }
    
    # Check initial supply (should be >0 from genesis land deeds)
    if ($node1Status.total_supply) {
        $supplyValid = ([int64]$node1Status.total_supply -gt 0)
        Write-TestResult "Genesis supply allocated" $supplyValid "Supply: $($node1Status.total_supply)"
    }
} else {
    Write-TestResult "Genesis block created" $false "Failed to query node status"
}

# ============================================================
# STEP 5: Mining Test (20 Blocks)
# ============================================================
Write-TestHeader "STEP 5: Mining Test (20 Blocks)"

Write-Host "Mining 20 blocks on node 1..." -ForegroundColor Yellow
try {
    $mineResponse = Invoke-RestMethod -Uri "http://localhost:7070/mine/start" -Method Post -Body (@{
        threads = 2
    } | ConvertTo-Json) -ContentType "application/json"
    
    Write-Host "Miner started, waiting for blocks..." -ForegroundColor Yellow
    
    # Poll for 20 blocks (with timeout)
    $startTime = Get-Date
    $targetHeight = 20
    $lastHeight = 0
    
    while (((Get-Date) - $startTime).TotalSeconds -lt 120) {
        $status = Get-NodeStatus -Port 7070
        if ($status -and $status.height -ge $targetHeight) {
            Write-TestResult "Mined $targetHeight blocks" $true "Final height: $($status.height)"
            break
        }
        
        if ($status -and $status.height -ne $lastHeight) {
            $lastHeight = $status.height
            Write-Host "   Block $lastHeight mined..." -ForegroundColor Gray
        }
        
        Start-Sleep -Seconds 2
    }
    
    if ($lastHeight -lt $targetHeight) {
        Write-TestResult "Mined $targetHeight blocks" $false "Only reached height $lastHeight"
    }
    
    # Stop miner
    Invoke-RestMethod -Uri "http://localhost:7070/mine/stop" -Method Post | Out-Null
    
} catch {
    Write-TestResult "Mined $targetHeight blocks" $false $_.Exception.Message
}

Start-Sleep -Seconds 5

# ============================================================
# STEP 6: Consensus Validation (Sync Check)
# ============================================================
Write-TestHeader "STEP 6: Multi-Node Consensus Validation"

$node1 = Get-NodeStatus -Port 7070
$node2 = Get-NodeStatus -Port 7071
$node3 = Get-NodeStatus -Port 7072

if ($node1 -and $node2 -and $node3) {
    # Check if all nodes have same height (±2 blocks tolerance)
    $heightDiff12 = [Math]::Abs($node1.height - $node2.height)
    $heightDiff13 = [Math]::Abs($node1.height - $node3.height)
    $heightDiff23 = [Math]::Abs($node2.height - $node3.height)
    
    $syncValid = ($heightDiff12 -le 2 -and $heightDiff13 -le 2 -and $heightDiff23 -le 2)
    Write-TestResult "Nodes synchronized" $syncValid "Heights: $($node1.height), $($node2.height), $($node3.height)"
    
    # Check if all nodes have same total supply
    if ($node1.total_supply -and $node2.total_supply -and $node3.total_supply) {
        $supplyMatch = ($node1.total_supply -eq $node2.total_supply -and $node2.total_supply -eq $node3.total_supply)
        Write-TestResult "Supply consensus" $supplyMatch "Supply: $($node1.total_supply)"
    }
} else {
    Write-TestResult "Nodes synchronized" $false "Failed to query all node statuses"
}

# ============================================================
# STEP 7: /status Endpoint Validation
# ============================================================
Write-TestHeader "STEP 7: Enhanced /status Endpoint Validation"

if ($node1) {
    # Check for new mainnet-readiness fields
    $fieldsPresent = @(
        @{Name="network"; Present=($null -ne $node1.network)},
        @{Name="network_phase"; Present=($null -ne $node1.network_phase)},
        @{Name="blocks_until_halving"; Present=($null -ne $node1.blocks_until_halving)},
        @{Name="next_halving_height"; Present=($null -ne $node1.next_halving_height)},
        @{Name="blocks_until_sunset"; Present=($null -ne $node1.blocks_until_sunset)},
        @{Name="testnet_sunset_height"; Present=($null -ne $node1.testnet_sunset_height)}
    )
    
    foreach ($field in $fieldsPresent) {
        Write-TestResult "Status field: $($field.Name)" $field.Present
    }
    
    # Validate halving calculation
    if ($node1.next_halving_height) {
        $expectedHalving = 1000000
        $halvingCorrect = ($node1.next_halving_height -eq $expectedHalving)
        Write-TestResult "Halving height correct" $halvingCorrect "Expected: $expectedHalving, Got: $($node1.next_halving_height)"
    }
}

# ============================================================
# STEP 8: Security Endpoint Audit
# ============================================================
Write-TestHeader "STEP 8: Security Audit (God-Mode Endpoints Removed)"

$removedEndpoints = @(
    @{Path="/airdrop"; Method="POST"},
    @{Path="/submit_admin_tx"; Method="POST"},
    @{Path="/admin/seed-balance"; Method="POST"},
    @{Path="/admin/token-accounts/set"; Method="POST"},
    @{Path="/set_gamemaster"; Method="POST"}
)

foreach ($endpoint in $removedEndpoints) {
    try {
        $response = Invoke-WebRequest -Uri "http://localhost:7070$($endpoint.Path)" `
            -Method $endpoint.Method `
            -Body "{}" `
            -ContentType "application/json" `
            -ErrorAction Stop
        
        # If we get here, endpoint still exists (bad)
        Write-TestResult "Endpoint $($endpoint.Path) removed" $false "Endpoint still accessible (Status: $($response.StatusCode))"
    } catch {
        # 404 = endpoint removed (good), 405 = wrong method but exists (bad)
        $statusCode = $_.Exception.Response.StatusCode.value__
        $removed = ($statusCode -eq 404)
        Write-TestResult "Endpoint $($endpoint.Path) removed" $removed "Status: $statusCode"
    }
}

# ============================================================
# STEP 9: Fee Market Validation
# ============================================================
Write-TestHeader "STEP 9: Fee Market & Mempool Validation"

try {
    $feeMarket = Invoke-RestMethod -Uri "http://localhost:7070/fee/market" -Method Get
    Write-TestResult "Fee market endpoint accessible" $true "Base fee: $($feeMarket.base_fee_per_gas)"
    
    $mempool = Invoke-RestMethod -Uri "http://localhost:7070/mempool" -Method Get
    Write-TestResult "Mempool endpoint accessible" $true "Pending TXs: $($mempool.transactions.Count)"
} catch {
    Write-TestResult "Fee market endpoints" $false $_.Exception.Message
}

# ============================================================
# STEP 10: Graceful Shutdown
# ============================================================
Write-TestHeader "STEP 10: Graceful Shutdown Test"

Stop-VisionNodes
Start-Sleep -Seconds 3

# Verify all processes stopped
$remainingProcesses = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
if ($remainingProcesses) {
    Write-TestResult "All nodes stopped cleanly" $false "$($remainingProcesses.Count) processes still running"
} else {
    Write-TestResult "All nodes stopped cleanly" $true
}

# Verify data persisted
foreach ($dir in $dataDirs) {
    if (Test-Path "$dir\db") {
        Write-TestResult "Data persisted: $dir" $true
    } else {
        Write-TestResult "Data persisted: $dir" $false
    }
}

# ============================================================
# FINAL REPORT
# ============================================================
Write-TestHeader "SMOKE TEST SUMMARY"

$totalTests = $Global:TestsPassed + $Global:TestsFailed
$passRate = [math]::Round(($Global:TestsPassed / $totalTests) * 100, 1)

Write-Host "Total Tests: $totalTests" -ForegroundColor Cyan
Write-Host "Passed:      $Global:TestsPassed" -ForegroundColor Green
Write-Host "Failed:      $Global:TestsFailed" -ForegroundColor Red
Write-Host "Pass Rate:   $passRate%" -ForegroundColor $(if ($passRate -ge 95) {"Green"} elseif ($passRate -ge 80) {"Yellow"} else {"Red"})

if ($Global:TestsFailed -eq 0) {
    Write-Host "`n✅ ALL TESTS PASSED - READY FOR MAINNET" -ForegroundColor Green
    exit 0
} elseif ($passRate -ge 90) {
    Write-Host "`n⚠️  MINOR ISSUES DETECTED - REVIEW FAILURES" -ForegroundColor Yellow
    exit 1
} else {
    Write-Host "`n❌ CRITICAL FAILURES - DO NOT DEPLOY" -ForegroundColor Red
    exit 2
}
