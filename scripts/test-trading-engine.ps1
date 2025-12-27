# Test the Trading Engine

$baseUrl = "http://localhost:7070/api/market/exchange"

Write-Host "`n=== Vision Trading Engine Test ===" -ForegroundColor Cyan

# Test 1: Get empty order book
Write-Host "`n[1] Testing empty order book..." -ForegroundColor Yellow
try {
    $book = Invoke-RestMethod -Uri "$baseUrl/book?chain=BTC&depth=10" -Method GET
    Write-Host "Book: $($book.bids.Count) bids, $($book.asks.Count) asks" -ForegroundColor Green
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 2: Place a sell order (ask)
Write-Host "`n[2] Placing sell order at 50000..." -ForegroundColor Yellow
try {
    $sellOrder = @{
        owner = "seller1"
        chain = "BTC"
        price = 50000.0
        size = 1.0
        post_only = $false
        tif = "GTC"
    } | ConvertTo-Json

    $result = Invoke-RestMethod -Uri "$baseUrl/order" -Method POST -Body $sellOrder -ContentType "application/json"
    Write-Host "Order placed: $($result.order_id)" -ForegroundColor Green
    Write-Host "Trades: $($result.trades.Count)" -ForegroundColor Gray
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 3: Place another sell order at different price
Write-Host "`n[3] Placing sell order at 51000..." -ForegroundColor Yellow
try {
    $sellOrder2 = @{
        owner = "seller2"
        chain = "BTC"
        price = 51000.0
        size = 0.5
        post_only = $false
        tif = "GTC"
    } | ConvertTo-Json

    $result = Invoke-RestMethod -Uri "$baseUrl/order" -Method POST -Body $sellOrder2 -ContentType "application/json"
    Write-Host "Order placed: $($result.order_id)" -ForegroundColor Green
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 4: Get order book with orders
Write-Host "`n[4] Checking order book..." -ForegroundColor Yellow
try {
    $book = Invoke-RestMethod -Uri "$baseUrl/book?chain=BTC&depth=10" -Method GET
    Write-Host "Bids: $($book.bids.Count), Asks: $($book.asks.Count)" -ForegroundColor Green
    
    if ($book.asks.Count -gt 0) {
        Write-Host "`nAsks (sell orders):" -ForegroundColor Cyan
        foreach ($ask in $book.asks) {
            Write-Host "  Price: $($ask[0]), Size: $($ask[1])" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 5: Place a market buy order (should match)
Write-Host "`n[5] Placing market buy order (0.3 BTC)..." -ForegroundColor Yellow
try {
    $buyOrder = @{
        owner = "buyer1"
        chain = "BTC"
        size = 0.3
    } | ConvertTo-Json

    $result = Invoke-RestMethod -Uri "$baseUrl/buy" -Method POST -Body $buyOrder -ContentType "application/json"
    Write-Host "Market buy executed!" -ForegroundColor Green
    Write-Host "Filled: $($result.filled) BTC" -ForegroundColor Gray
    Write-Host "Avg Price: $($result.avg_price)" -ForegroundColor Gray
    Write-Host "Trades: $($result.trades.Count)" -ForegroundColor Gray
    
    if ($result.trades.Count -gt 0) {
        Write-Host "`nTrades:" -ForegroundColor Cyan
        foreach ($trade in $result.trades) {
            Write-Host "  $($trade.size) @ $($trade.price)" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 6: Check order book after trade
Write-Host "`n[6] Order book after trade..." -ForegroundColor Yellow
try {
    $book = Invoke-RestMethod -Uri "$baseUrl/book?chain=BTC&depth=10" -Method GET
    Write-Host "Bids: $($book.bids.Count), Asks: $($book.asks.Count)" -ForegroundColor Green
    
    if ($book.asks.Count -gt 0) {
        Write-Host "`nRemaining asks:" -ForegroundColor Cyan
        foreach ($ask in $book.asks) {
            Write-Host "  Price: $($ask[0]), Size: $($ask[1])" -ForegroundColor Gray
        }
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 7: Get recent trades
Write-Host "`n[7] Getting recent trades..." -ForegroundColor Yellow
try {
    $trades = Invoke-RestMethod -Uri "$baseUrl/trades?chain=BTC&limit=10" -Method GET
    Write-Host "Recent trades: $($trades.Count)" -ForegroundColor Green
    
    foreach ($trade in $trades) {
        Write-Host "  $($trade.size) @ $($trade.price) - $($trade.side)" -ForegroundColor Gray
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 8: Get ticker
Write-Host "`n[8] Getting ticker..." -ForegroundColor Yellow
try {
    $ticker = Invoke-RestMethod -Uri "$baseUrl/ticker?chain=BTC" -Method GET
    Write-Host "Last: $($ticker.last)" -ForegroundColor Green
    Write-Host "24h Change: $($ticker.change24h)%" -ForegroundColor Gray
    Write-Host "24h Volume: $($ticker.vol24h)" -ForegroundColor Gray
    Write-Host "24h High: $($ticker.high24h)" -ForegroundColor Gray
    Write-Host "24h Low: $($ticker.low24h)" -ForegroundColor Gray
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# Test 9: Get user's orders
Write-Host "`n[9] Getting seller1's open orders..." -ForegroundColor Yellow
try {
    $orders = Invoke-RestMethod -Uri "$baseUrl/my/orders?owner=seller1&chain=BTC" -Method GET
    Write-Host "Open orders: $($orders.Count)" -ForegroundColor Green
    
    foreach ($order in $orders) {
        $remaining = $order.size_total - $order.size_filled
        Write-Host "  Order $($order.id): $remaining @ $($order.price) - $($order.status)" -ForegroundColor Gray
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

Write-Host "`n=== Test Complete ===" -ForegroundColor Cyan
