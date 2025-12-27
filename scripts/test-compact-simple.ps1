# Test Compact Block P2P - Simple Version
Write-Host "Testing Compact Block P2P Propagation..." -ForegroundColor Cyan

# Kill existing nodes
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Start Node 1 (miner)
Write-Host "Starting Node 1 on port 7070..." -ForegroundColor Yellow
$node1 = Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--port", "7070" -PassThru -NoNewWindow

# Start Node 2 (sync)
Write-Host "Starting Node 2 on port 7071..." -ForegroundColor Yellow
Start-Sleep -Seconds 3
$node2 = Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--port", "7071", "--peers", "http://localhost:7070" -PassThru -NoNewWindow

Start-Sleep -Seconds 5

# Connect nodes
Write-Host "Connecting nodes..." -ForegroundColor Yellow
try {
    Invoke-RestMethod -Uri "http://localhost:7070/add_peer" -Method POST -ContentType "application/json" -Body '{"peer": "http://localhost:7071"}' | Out-Null
    Write-Host "Nodes connected" -ForegroundColor Green
} catch {
    Write-Host "Failed to connect: $_" -ForegroundColor Red
}

# Enable mining
Write-Host "Enabling mining..." -ForegroundColor Yellow
try {
    Invoke-RestMethod -Uri "http://localhost:7070/enable_mining" -Method POST | Out-Null
    Write-Host "Mining enabled" -ForegroundColor Green
} catch {
    Write-Host "Failed to enable mining: $_" -ForegroundColor Red
}

Write-Host "Waiting for blocks..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

# Check metrics
Write-Host "`nNode 1 Metrics:" -ForegroundColor Cyan
try {
    $m1 = Invoke-RestMethod -Uri "http://localhost:7070/metrics"
    $sent = if ($m1 -match 'vision_compact_blocks_sent_total (\d+)') { $matches[1] } else { "0" }
    $bw = if ($m1 -match 'vision_compact_block_bandwidth_saved_bytes (\d+)') { $matches[1] } else { "0" }
    Write-Host "  Compact blocks sent: $sent"
    Write-Host "  Bandwidth saved: $bw bytes"
} catch {
    Write-Host "  Could not fetch metrics" -ForegroundColor Red
}

Write-Host "`nNode 2 Metrics:" -ForegroundColor Cyan
try {
    $m2 = Invoke-RestMethod -Uri "http://localhost:7071/metrics"
    $rcvd = if ($m2 -match 'vision_compact_blocks_received_total (\d+)') { $matches[1] } else { "0" }
    Write-Host "  Compact blocks received: $rcvd"
} catch {
    Write-Host "  Could not fetch metrics" -ForegroundColor Red
}

# Cleanup
Write-Host "`nCleaning up..." -ForegroundColor Yellow
$node1 | Stop-Process -Force -ErrorAction SilentlyContinue
$node2 | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Host "`nTest complete!" -ForegroundColor Green
if ([int]$sent -gt 0 -and [int]$rcvd -gt 0) {
    Write-Host "SUCCESS: Sent $sent, Received $rcvd" -ForegroundColor Green
} else {
    Write-Host "No compact blocks detected (sent: $sent, received: $rcvd)" -ForegroundColor Yellow
}
