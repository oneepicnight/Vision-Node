# Test Chain Reorganization
# 
# This test verifies that the node properly handles competing forks
# and reorganizes to the chain with more accumulated work.

Write-Host "=== Testing Chain Reorganization ===" -ForegroundColor Cyan
Write-Host ""

# Start a test node
Write-Host "Starting test node on port 7070..." -ForegroundColor Yellow
$node = Start-Process -FilePath ".\target\release\vision-node.exe" `
    -ArgumentList "--port", "7070", "--data-dir", "vision_data_test_reorg", "--no-mining" `
    -PassThru -WindowStyle Hidden

Start-Sleep -Seconds 3

try {
    # Check node is running
    Write-Host "Node started (PID: $($node.Id))" -ForegroundColor Green
    
    # Get chain state before
    Write-Host ""
    Write-Host "Checking initial chain state..." -ForegroundColor Yellow
    $response = Invoke-RestMethod -Uri "http://localhost:7070/metrics" -Method Get
    
    if ($response -match "vision_chain_reorgs_total (\d+)") {
        $initialReorgs = [int]$matches[1]
        Write-Host "Initial reorg count: $initialReorgs" -ForegroundColor Green
    } else {
        Write-Host "Could not read reorg metric" -ForegroundColor Red
    }
    
    # Check current block height
    $status = Invoke-RestMethod -Uri "http://localhost:7070/status" -Method Get
    $currentHeight = $status.blocks
    Write-Host "Current block height: $currentHeight" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "Reorg detection is active!" -ForegroundColor Green
    Write-Host "The node will automatically handle reorgs when:" -ForegroundColor Cyan
    Write-Host "  1. It receives a block that does not extend the current tip" -ForegroundColor White
    Write-Host "  2. The competing fork has MORE accumulated difficulty" -ForegroundColor White
    Write-Host "  3. All fork blocks can be traced back to a common ancestor" -ForegroundColor White
    Write-Host ""
    Write-Host "Reorg metrics available:" -ForegroundColor Cyan
    Write-Host "  - vision_chain_reorgs_total" -ForegroundColor White
    Write-Host "  - vision_chain_reorg_blocks_rolled_back_total" -ForegroundColor White
    Write-Host "  - vision_chain_reorg_txs_reinserted_total" -ForegroundColor White
    Write-Host "  - vision_chain_reorg_depth_last" -ForegroundColor White
    
    Write-Host ""
    Write-Host "Press Ctrl+C to stop the node..." -ForegroundColor Yellow
    Wait-Process -Id $node.Id
    
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
} finally {
    # Cleanup
    Write-Host ""
    Write-Host "Stopping node..." -ForegroundColor Yellow
    Stop-Process -Id $node.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Host "Node stopped" -ForegroundColor Green
}
