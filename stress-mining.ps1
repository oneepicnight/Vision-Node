# stress-mining.ps1
# Stress test mining system with configurable parameters

param(
    [int]$Threads = 4,
    [int]$Duration = 300,  # seconds
    [int]$Port = 7070,
    [switch]$Testnet
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node Mining Stress Test ===" -ForegroundColor Cyan
Write-Host "Threads: $Threads" -ForegroundColor White
Write-Host "Duration: $Duration seconds" -ForegroundColor White
Write-Host "Port: $Port" -ForegroundColor White

$network = if ($Testnet) { "testnet" } else { "dev" }
Write-Host "Network: $network" -ForegroundColor White

# Check if node is running
try {
    $status = Invoke-RestMethod -Uri "http://localhost:$Port/status" -ErrorAction Stop
    Write-Host "Node is running - Height: $($status.height)" -ForegroundColor Green
} catch {
    Write-Error "Node is not running on port $Port. Start it first with: VISION_PORT=$Port .\target\release\vision-node.exe"
    exit 1
}

Write-Host ""
Write-Host "Starting stress test..." -ForegroundColor Cyan

$startTime = Get-Date
$endTime = $startTime.AddSeconds($Duration)
$initialHeight = $status.height
$blocksFound = 0
$lastHeight = $initialHeight

while ((Get-Date) -lt $endTime) {
    try {
        # Check current height
        $status = Invoke-RestMethod -Uri "http://localhost:$Port/status" -ErrorAction SilentlyContinue
        $currentHeight = $status.height
        
        if ($currentHeight -gt $lastHeight) {
            $blocksFound = $currentHeight - $initialHeight
            $lastHeight = $currentHeight
            $elapsed = ((Get-Date) - $startTime).TotalSeconds
            $rate = if ($elapsed -gt 0) { [math]::Round($blocksFound / $elapsed, 2) } else { 0 }
            Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Height: $currentHeight (+$blocksFound) - Rate: $rate blocks/sec" -ForegroundColor Green
        }
        
        Start-Sleep -Seconds 2
    } catch {
        Write-Warning "Failed to query status: $_"
    }
}

# Final stats
Write-Host ""
Write-Host "=== Stress Test Complete ===" -ForegroundColor Cyan
$totalElapsed = ((Get-Date) - $startTime).TotalSeconds
$finalStatus = Invoke-RestMethod -Uri "http://localhost:$Port/status"
$finalHeight = $finalStatus.height
$totalBlocks = $finalHeight - $initialHeight

Write-Host "Duration: $([math]::Round($totalElapsed, 2)) seconds" -ForegroundColor White
Write-Host "Initial Height: $initialHeight" -ForegroundColor White
Write-Host "Final Height: $finalHeight" -ForegroundColor White
Write-Host "Blocks Mined: $totalBlocks" -ForegroundColor Green
Write-Host "Average Rate: $([math]::Round($totalBlocks / $totalElapsed, 2)) blocks/sec" -ForegroundColor Green
Write-Host "Difficulty: $($finalStatus.difficulty)" -ForegroundColor White

# Check mempool
try {
    $mempoolStatus = Invoke-RestMethod -Uri "http://localhost:$Port/mempool/status"
    Write-Host "Mempool Size: $($mempoolStatus.size)" -ForegroundColor Yellow
} catch {
    Write-Host "Could not retrieve mempool status" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Mining stress test completed successfully" -ForegroundColor Green
