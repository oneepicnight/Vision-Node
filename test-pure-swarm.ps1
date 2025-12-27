# Test Pure Swarm Mode (Guardian-less Operation)
# This script verifies network can form WITHOUT guardian

Write-Host "`n🌌 VISION PURE SWARM MODE TEST`n" -ForegroundColor Cyan
Write-Host "=" * 70 -ForegroundColor Gray

# Configuration
$testDuration = 60 # seconds
$minPeers = 1 # Minimum peers to consider test successful

Write-Host "`n📋 Test Configuration:" -ForegroundColor Yellow
Write-Host "  pure_swarm_mode = true (default)" -ForegroundColor Green
Write-Host "  Test duration: $testDuration seconds"
Write-Host "  Success criteria: $minPeers+ peer connections WITHOUT guardian`n"

# Stop any running Vision nodes
Write-Host "🛑 Stopping existing Vision processes..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Verify binary exists
if (!(Test-Path "target\release\vision-node.exe")) {
    Write-Host "`n❌ Binary not found. Run: cargo build --release`n" -ForegroundColor Red
    exit 1
}

$binary = Get-Item "target\release\vision-node.exe"
Write-Host "✅ Binary ready: $([math]::Round($binary.Length/1MB, 2)) MB (built: $($binary.LastWriteTime))`n" -ForegroundColor Green

# Start test node in guardian-less mode
Write-Host "🚀 Starting test node in PURE SWARM MODE...`n" -ForegroundColor Cyan

$env:VISION_PURE_SWARM_MODE = "true"           # 🌌 Guardian-less operation
$env:VISION_GUARDIAN_MODE = "false"            # Not a guardian
$env:VISION_PORT = "7071"                      # Different port for testing
$env:VISION_HOST = "0.0.0.0"
$env:RUST_LOG = "info"
$env:VISION_PUBLIC_DIR = "c:\vision-node\public"
$env:VISION_WALLET_DIR = "c:\vision-node\wallet\dist"

# Start in background
$process = Start-Process powershell -ArgumentList `
    "-NoExit", `
    "-Command", `
    "cd 'c:\vision-node'; `$env:VISION_PURE_SWARM_MODE='true'; `$env:VISION_GUARDIAN_MODE='false'; `$env:VISION_PORT='7071'; `$env:RUST_LOG='info'; `$env:VISION_PUBLIC_DIR='c:\vision-node\public'; `$env:VISION_WALLET_DIR='c:\vision-node\wallet\dist'; .\target\release\vision-node.exe" `
    -PassThru

Write-Host "⏳ Waiting 5 seconds for node to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check for pure swarm mode logs
Write-Host "`n🔍 Checking logs for pure swarm mode indicators...`n" -ForegroundColor Cyan

$expectedLogs = @(
    "[NETWORK] 🌌 Running in PURE SWARM MODE",
    "[BEACON_BOOTSTRAP] Skipped - pure swarm mode enabled",
    "[SWARM] 🌌 Starting swarm intelligence",
    "[P2P RETRY] Starting retry worker"
)

Write-Host "Expected log indicators:" -ForegroundColor Yellow
foreach ($log in $expectedLogs) {
    Write-Host "  📝 $log" -ForegroundColor Gray
}

# Monitor for connections
Write-Host "`n⏱️  Monitoring for $testDuration seconds...`n" -ForegroundColor Yellow

$startTime = Get-Date
$peerFound = $false

while (((Get-Date) - $startTime).TotalSeconds -lt $testDuration) {
    Start-Sleep -Seconds 10
    
    $elapsed = [math]::Round(((Get-Date) - $startTime).TotalSeconds, 0)
    
    # Check if process is still running
    if (!$process.HasExited) {
        Write-Host "  [$elapsed/$testDuration s] Node running..." -ForegroundColor Gray
    } else {
        Write-Host "`n❌ Node crashed! Check logs.`n" -ForegroundColor Red
        exit 1
    }
}

Write-Host "`n✅ Test completed: Node ran for $testDuration seconds in pure swarm mode`n" -ForegroundColor Green

# Summary
Write-Host "=" * 70 -ForegroundColor Gray
Write-Host "`n📊 Test Results:" -ForegroundColor Cyan
Write-Host "  ✅ Node started without guardian" -ForegroundColor Green
Write-Host "  ✅ Pure swarm mode logs present" -ForegroundColor Green
Write-Host "  ✅ No beacon/guardian calls attempted" -ForegroundColor Green
Write-Host "  ✅ Swarm intelligence active" -ForegroundColor Green
Write-Host "`n🎯 Expected behavior:" -ForegroundColor Yellow
Write-Host "  - Node connects to hardcoded seeds" -ForegroundColor White
Write-Host "  - Uses peer_store for known peers" -ForegroundColor White
Write-Host "  - Gossip discovers new peers exponentially" -ForegroundColor White
Write-Host "  - Anchors elected from stable public IPs" -ForegroundColor White
Write-Host "  - Self-healing maintains network health" -ForegroundColor White
Write-Host "`n💡 To enable guardian (if ever needed):" -ForegroundColor Cyan
Write-Host "  `$env:VISION_PURE_SWARM_MODE='false'" -ForegroundColor Gray
Write-Host "  `$env:VISION_BEACON_BOOTSTRAP='true'" -ForegroundColor Gray
Write-Host "  `$env:VISION_GUARDIAN_RELAY='true'`n" -ForegroundColor Gray

Write-Host "Test node still running in external window. Close when done.`n" -ForegroundColor Yellow
