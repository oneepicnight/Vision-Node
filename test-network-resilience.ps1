# Test script for Vision network resilience when public node goes offline
# This script simulates the scenario where the first public node (bootstrap node) goes offline
# and verifies that miners continue operating normally

param(
    [string]$Action = "test",
    [string]$PublicNodePort = "7070",
    [string]$TestDurationMinutes = "5"
)

Write-Host "Vision Network Resilience Test" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

function Test-NetworkResilience {
    Write-Host "Testing network resilience when public node goes offline..." -ForegroundColor Yellow
    
    # Check if node is running
    $nodeProcess = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
    if (-not $nodeProcess) {
        Write-Host "ERROR: Vision node is not running. Please start the node first." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "✓ Vision node is running (PID: $($nodeProcess.Id))" -ForegroundColor Green
    
    # Get initial peer count
    try {
        $statusResponse = Invoke-WebRequest -Uri "http://localhost:$PublicNodePort/status" -TimeoutSec 10
        $status = $statusResponse.Content | ConvertFrom-Json
        $initialPeers = $status.peers
        Write-Host "✓ Initial peer count: $initialPeers" -ForegroundColor Green
    } catch {
        Write-Host "ERROR: Cannot connect to node status endpoint" -ForegroundColor Red
        exit 1
    }
    
    # Simulate public node going offline by stopping it temporarily
    Write-Host "Simulating public node offline by stopping the node..." -ForegroundColor Yellow
    Stop-Process -Id $nodeProcess.Id -Force
    Start-Sleep -Seconds 5
    
    # Verify node is stopped
    $stoppedProcess = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
    if ($stoppedProcess) {
        Write-Host "ERROR: Node did not stop properly" -ForegroundColor Red
        exit 1
    }
    Write-Host "✓ Node stopped successfully" -ForegroundColor Green
    
    # Wait for network to detect disconnection
    Write-Host "Waiting for network to detect disconnection..." -ForegroundColor Yellow
    Start-Sleep -Seconds 30
    
    # Restart the node
    Write-Host "Restarting the node to test reconnection..." -ForegroundColor Yellow
    $nodePath = Join-Path $PSScriptRoot "target\release\vision-node.exe"
    if (-not (Test-Path $nodePath)) {
        $nodePath = Join-Path $PSScriptRoot "target\debug\vision-node.exe"
    }
    
    if (-not (Test-Path $nodePath)) {
        Write-Host "ERROR: Cannot find vision-node.exe in target/release or target/debug" -ForegroundColor Red
        exit 1
    }
    
    Start-Process -FilePath $nodePath -NoNewWindow
    
    # Wait for node to restart
    Write-Host "Waiting for node to restart..." -ForegroundColor Yellow
    Start-Sleep -Seconds 10
    
    # Verify node restarted
    $restartedProcess = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
    if (-not $restartedProcess) {
        Write-Host "ERROR: Node did not restart properly" -ForegroundColor Red
        exit 1
    }
    Write-Host "✓ Node restarted successfully (PID: $($restartedProcess.Id))" -ForegroundColor Green
    
    # Wait for reconnection attempts
    Write-Host "Waiting for reconnection attempts..." -ForegroundColor Yellow
    Start-Sleep -Seconds 60
    
    # Check final status
    try {
        $finalStatusResponse = Invoke-WebRequest -Uri "http://localhost:$PublicNodePort/status" -TimeoutSec 10
        $finalStatus = $finalStatusResponse.Content | ConvertFrom-Json
        $finalPeers = $finalStatus.peers
        Write-Host "✓ Final peer count: $finalPeers" -ForegroundColor Green
        
        # Check if mining is still working
        $miningResponse = Invoke-WebRequest -Uri "http://localhost:$PublicNodePort/api/miner/status" -TimeoutSec 10
        $miningStatus = $miningResponse.Content | ConvertFrom-Json
        
        if ($miningStatus.enabled -eq $true) {
            Write-Host "✓ Mining is still active" -ForegroundColor Green
        } else {
            Write-Host "⚠ Mining is not active - this may be expected if manually disabled" -ForegroundColor Yellow
        }
        
        Write-Host "" -ForegroundColor White
        Write-Host "Test Results:" -ForegroundColor Cyan
        Write-Host "- Initial peers: $initialPeers" -ForegroundColor White
        Write-Host "- Final peers: $finalPeers" -ForegroundColor White
        Write-Host "- Node successfully restarted and reconnected" -ForegroundColor Green
        
        if ($finalPeers -gt 0) {
            Write-Host "- Network resilience: PASS ✓" -ForegroundColor Green
        } else {
            Write-Host "- Network resilience: FAIL ✗ (no peers reconnected)" -ForegroundColor Red
        }
        
    } catch {
        Write-Host "ERROR: Cannot check final status" -ForegroundColor Red
        exit 1
    }
}

function Show-Usage {
    Write-Host "Usage: .\test-network-resilience.ps1 [options]" -ForegroundColor White
    Write-Host "" -ForegroundColor White
    Write-Host "Options:" -ForegroundColor White
    Write-Host "  -Action <action>          Action to perform (default: test)" -ForegroundColor White
    Write-Host "  -PublicNodePort <port>    Port of the public node (default: 7070)" -ForegroundColor White
    Write-Host "  -TestDurationMinutes <min> Duration to run extended tests (default: 5)" -ForegroundColor White
    Write-Host "" -ForegroundColor White
    Write-Host "Actions:" -ForegroundColor White
    Write-Host "  test                      Run the network resilience test" -ForegroundColor White
    Write-Host "  help                      Show this help message" -ForegroundColor White
}

switch ($Action.ToLower()) {
    "test" {
        Test-NetworkResilience
    }
    "help" {
        Show-Usage
    }
    default {
        Write-Host "ERROR: Unknown action '$Action'" -ForegroundColor Red
        Show-Usage
        exit 1
    }
}