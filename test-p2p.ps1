# Test P2P functionality on Vision Node
# Public address: tcp://0.tcp.us-cal-1.ngrok.io:10213

Write-Host "`n================================" -ForegroundColor Cyan
Write-Host "  Vision Node P2P Test" -ForegroundColor Cyan
Write-Host "  Public: 0.tcp.us-cal-1.ngrok.io:10213" -ForegroundColor Cyan
Write-Host "================================`n" -ForegroundColor Cyan

$baseUrl = "http://localhost:7070/api"

# Test 1: Get current peers
Write-Host "[1/5] Getting current peer list..." -ForegroundColor Yellow
try {
    $peers = Invoke-RestMethod -Uri "$baseUrl/peers/list" -Method GET
    Write-Host "✓ Current peers: $($peers.Count)" -ForegroundColor Green
    if ($peers.Count -gt 0) {
        $peers | ForEach-Object { Write-Host "  - $_" -ForegroundColor Gray }
    }
} catch {
    Write-Host "✗ Failed to get peers: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 2: Get peer stats
Write-Host "`n[2/5] Getting peer statistics..." -ForegroundColor Yellow
try {
    $stats = Invoke-RestMethod -Uri "$baseUrl/peers/stats" -Method GET
    Write-Host "✓ Peer stats retrieved" -ForegroundColor Green
    Write-Host "  Total peers: $($stats.total_peers)" -ForegroundColor Gray
    Write-Host "  Active peers: $($stats.active_peers)" -ForegroundColor Gray
    Write-Host "  Banned peers: $($stats.banned_peers)" -ForegroundColor Gray
} catch {
    Write-Host "✗ Failed to get stats: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 3: Test sync endpoints
Write-Host "`n[3/5] Testing sync/pull endpoint..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$baseUrl/sync/pull?height=0" -Method GET -UseBasicParsing
    Write-Host "✓ Sync endpoint responsive (Status: $($response.StatusCode))" -ForegroundColor Green
} catch {
    Write-Host "✗ Sync endpoint error: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 4: Test block endpoint
Write-Host "`n[4/5] Getting current block height..." -ForegroundColor Yellow
try {
    $chain = Invoke-RestMethod -Uri "$baseUrl/chain/info" -Method GET
    Write-Host "✓ Chain height: $($chain.height)" -ForegroundColor Green
    Write-Host "  Total blocks: $($chain.blocks)" -ForegroundColor Gray
} catch {
    Write-Host "✗ Chain info error: $($_.Exception.Message)" -ForegroundColor Red
}

# Test 5: Test miner endpoints
Write-Host "`n[5/5] Testing miner endpoints..." -ForegroundColor Yellow
try {
    $minerConfig = Invoke-RestMethod -Uri "$baseUrl/miner/config" -Method GET
    Write-Host "✓ Miner configuration retrieved" -ForegroundColor Green
    Write-Host "  Threads: $($minerConfig.threads)" -ForegroundColor Gray
    Write-Host "  Enabled: $($minerConfig.enabled)" -ForegroundColor Gray
    Write-Host "  Max threads: $($minerConfig.max_threads)" -ForegroundColor Gray
} catch {
    Write-Host "✗ Miner config error: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host "`n================================" -ForegroundColor Cyan
Write-Host "  P2P Test Complete!" -ForegroundColor Cyan
Write-Host "================================`n" -ForegroundColor Cyan

Write-Host "Your public address for peer connections:" -ForegroundColor Yellow
Write-Host "  tcp://0.tcp.us-cal-1.ngrok.io:10213" -ForegroundColor White
Write-Host "`nMiners can connect with:" -ForegroundColor Yellow
Write-Host "  --p2p-peer 0.tcp.us-cal-1.ngrok.io:10213" -ForegroundColor White
Write-Host ""
