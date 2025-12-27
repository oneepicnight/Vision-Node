# Execute a test trade to see the matching engine in action

Write-Host "`n=== Live Trading Test ===" -ForegroundColor Cyan

# Get current order book
$book = Invoke-RestMethod "http://localhost:7070/api/market/exchange/book?chain=BTC&depth=5"
Write-Host "`nCurrent Best Ask: `$$($book.asks[0][0])" -ForegroundColor Yellow
Write-Host "Current Best Bid: `$$($book.bids[0][0])" -ForegroundColor Yellow

# Place a market buy order that will match against the asks
Write-Host "`n[TEST] Placing market buy for 1.5 BTC..." -ForegroundColor Cyan

$marketBuy = @{
    owner = "test_trader"
    chain = "BTC"
    size = 1.5
} | ConvertTo-Json

try {
    $result = Invoke-RestMethod -Uri "http://localhost:7070/api/market/exchange/buy" -Method POST -Body $marketBuy -ContentType "application/json"
    
    Write-Host "`n[SUCCESS] Trade Executed!" -ForegroundColor Green
    Write-Host "  Filled: $($result.filled) BTC" -ForegroundColor Gray
    Write-Host "  Avg Price: `$$($result.avg_price)" -ForegroundColor Gray
    Write-Host "  Trades: $($result.trades.Count)" -ForegroundColor Gray
    
    if ($result.trades.Count -gt 0) {
        Write-Host "`n  Trade Details:" -ForegroundColor Cyan
        foreach ($trade in $result.trades) {
            Write-Host "    - $($trade.size) BTC @ `$$($trade.price)" -ForegroundColor Gray
        }
    }
    
    # Check updated order book
    Start-Sleep -Milliseconds 500
    $newBook = Invoke-RestMethod "http://localhost:7070/api/market/exchange/book?chain=BTC&depth=5"
    
    Write-Host "`n[UPDATED] Order Book:" -ForegroundColor Cyan
    Write-Host "  Best Ask: `$$($newBook.asks[0][0]) (was `$$($book.asks[0][0]))" -ForegroundColor Yellow
    Write-Host "  Ask Size: $($newBook.asks[0][1]) BTC (was $($book.asks[0][1]) BTC)" -ForegroundColor Gray
    
    # Get recent trades
    $trades = Invoke-RestMethod "http://localhost:7070/api/market/exchange/trades?chain=BTC&limit=5"
    Write-Host "`n[RECENT] Last $($trades.Count) Trades:" -ForegroundColor Cyan
    foreach ($t in $trades) {
        $time = [DateTimeOffset]::FromUnixTimeMilliseconds($t.ts).LocalDateTime
        Write-Host "  $($t.size) @ `$$($t.price) - $($t.side) - $($time.ToString('HH:mm:ss'))" -ForegroundColor Gray
    }
    
} catch {
    Write-Host "`n[ERROR] $($_)" -ForegroundColor Red
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
Write-Host "Refresh the wallet to see updated order book!" -ForegroundColor Yellow
