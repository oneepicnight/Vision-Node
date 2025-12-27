# Test Vision Node Dashboard
# Starts a test node and opens the real-time dashboard

Write-Host "=== Vision Node Dashboard Test ===" -ForegroundColor Cyan
Write-Host ""

# Start the node
Write-Host "Starting Vision Node on port 7070..." -ForegroundColor Yellow
$node = Start-Process -FilePath ".\target\release\vision-node.exe" `
    -ArgumentList "--port", "7070", "--data-dir", "vision_data_dashboard_test" `
    -PassThru -WindowStyle Hidden

Start-Sleep -Seconds 3

try {
    Write-Host "Node started (PID: $($node.Id))" -ForegroundColor Green
    Write-Host ""
    
    # Check if node is responding
    try {
        $status = Invoke-RestMethod -Uri "http://localhost:7070/status" -Method Get -TimeoutSec 5
        Write-Host "Node Status:" -ForegroundColor Green
        Write-Host "  Block Height: $($status.blocks)" -ForegroundColor White
        Write-Host "  Peers: $($status.peers)" -ForegroundColor White
        Write-Host "  Mempool: $($status.mempool_critical + $status.mempool_bulk) txs" -ForegroundColor White
    } catch {
        Write-Host "Warning: Could not fetch status" -ForegroundColor Yellow
    }
    
    Write-Host ""
    Write-Host "Dashboard URLs:" -ForegroundColor Cyan
    Write-Host "  Real-Time Dashboard: http://localhost:7070/dashboard.html" -ForegroundColor Green
    Write-Host "  Block Explorer:      http://localhost:7070/explorer.html" -ForegroundColor Green
    Write-Host "  Miner Panel:         http://localhost:7070/panel.html" -ForegroundColor Green
    Write-Host "  Metrics:            http://localhost:7070/metrics" -ForegroundColor Green
    Write-Host ""
    
    Write-Host "Features:" -ForegroundColor Cyan
    Write-Host "  - Live block updates via WebSocket" -ForegroundColor White
    Write-Host "  - Real-time mempool visualization" -ForegroundColor White
    Write-Host "  - P2P network status" -ForegroundColor White
    Write-Host "  - Phase 2 metrics (Compact Blocks, TX Gossip, Reorgs)" -ForegroundColor White
    Write-Host ""
    
    # Open dashboard in default browser
    Write-Host "Opening dashboard in browser..." -ForegroundColor Yellow
    Start-Process "http://localhost:7070/dashboard.html"
    
    Write-Host ""
    Write-Host "Press Ctrl+C to stop the node..." -ForegroundColor Yellow
    Wait-Process -Id $node.Id
    
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
} finally {
    Write-Host ""
    Write-Host "Stopping node..." -ForegroundColor Yellow
    Stop-Process -Id $node.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Host "Node stopped" -ForegroundColor Green
}
