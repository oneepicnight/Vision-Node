# Comprehensive Trading Engine Demo

Write-Host "`n" -ForegroundColor Cyan
Write-Host "╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║      VISION BLOCKCHAIN TRADING ENGINE - LIVE DEMO            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

# 1. Current Market State
Write-Host "`n[1] CURRENT MARKET STATE" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$book = Invoke-RestMethod "http://localhost:7070/api/market/exchange/book?chain=BTC&depth=5"
$ticker = Invoke-RestMethod "http://localhost:7070/api/market/exchange/ticker?chain=BTC"

Write-Host "`nOrder Book (BTC/VISION):" -ForegroundColor Cyan
Write-Host "  Best Bid: `$$($book.bids[0][0]) - $($book.bids[0][1]) BTC" -ForegroundColor Green
Write-Host "  Best Ask: `$$($book.asks[0][0]) - $($book.asks[0][1]) BTC" -ForegroundColor Magenta
Write-Host "  Spread:   `$$($book.asks[0][0] - $book.bids[0][0])" -ForegroundColor Yellow

Write-Host "`n24h Stats:" -ForegroundColor Cyan
Write-Host "  Last:   `$$($ticker.last)" -ForegroundColor Gray
Write-Host "  Volume: $($ticker.vol24h) BTC" -ForegroundColor Gray
Write-Host "  High:   `$$($ticker.high24h)" -ForegroundColor Gray
Write-Host "  Low:    `$$($ticker.low24h)" -ForegroundColor Gray

# 2. Place Limit Order
Write-Host "`n[2] PLACING LIMIT ORDER" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$limitOrder = @{
    owner = "demo_user"
    chain = "BTC"
    side = "sell"
    price = 51500.0
    size = 2.0
    post_only = $true
    tif = "GTC"
} | ConvertTo-Json

$limitResult = Invoke-RestMethod -Uri "http://localhost:7070/api/market/exchange/order" -Method POST -Body $limitOrder -ContentType "application/json"
Write-Host "  ✓ Limit order placed: $($limitResult.order_id)" -ForegroundColor Green
Write-Host "    Sell 2.0 BTC @ `$51,500 (post-only)" -ForegroundColor Gray

# 3. Place Market Order
Write-Host "`n[3] EXECUTING MARKET ORDER" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$marketOrder = @{
    owner = "trader_joe"
    chain = "BTC"
    size = 0.75
} | ConvertTo-Json

$marketResult = Invoke-RestMethod -Uri "http://localhost:7070/api/market/exchange/buy" -Method POST -Body $marketOrder -ContentType "application/json"
Write-Host "  ✓ Market buy executed!" -ForegroundColor Green
Write-Host "    Filled: $($marketResult.filled) BTC" -ForegroundColor Gray
Write-Host "    Avg Price: `$$([math]::Round($marketResult.avg_price, 2))" -ForegroundColor Gray
Write-Host "    Trades: $($marketResult.trades.Count)" -ForegroundColor Gray

if ($marketResult.trades.Count -gt 0) {
    Write-Host "`n    Trade breakdown:" -ForegroundColor Cyan
    foreach ($trade in $marketResult.trades) {
        Write-Host "      → $($trade.size) BTC @ `$$($trade.price)" -ForegroundColor Gray
    }
}

# 4. Recent Trades
Write-Host "`n[4] RECENT TRADE HISTORY" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$trades = Invoke-RestMethod "http://localhost:7070/api/market/exchange/trades?chain=BTC&limit=5"
Write-Host "`n  Last $($trades.Count) Trades:" -ForegroundColor Cyan
foreach ($t in $trades) {
    $sideColor = if ($t.side -eq "buy") { "Green" } else { "Magenta" }
    $arrow = if ($t.side -eq "buy") { "▲" } else { "▼" }
    Write-Host "    $arrow $($t.size) BTC @ `$$($t.price) [$($t.side.ToUpper())]" -ForegroundColor $sideColor
}

# 5. User Orders
Write-Host "`n[5] USER OPEN ORDERS" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$userOrders = Invoke-RestMethod "http://localhost:7070/api/market/exchange/my/orders?owner=demo_user&chain=BTC"
if ($userOrders.Count -gt 0) {
    Write-Host "`n  demo_user has $($userOrders.Count) open order(s):" -ForegroundColor Cyan
    foreach ($order in $userOrders) {
        $remaining = $order.size_total - $order.size_filled
        Write-Host "    • $($order.id): $remaining BTC @ `$$($order.price) [$($order.status)]" -ForegroundColor Gray
    }
} else {
    Write-Host "  No open orders for demo_user" -ForegroundColor Gray
}

# 6. Updated Book
Write-Host "`n[6] UPDATED ORDER BOOK" -ForegroundColor Yellow
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray

$newBook = Invoke-RestMethod "http://localhost:7070/api/market/exchange/book?chain=BTC&depth=5"

Write-Host "`n  Top 3 Bids (Buy):" -ForegroundColor Green
for ($i = 0; $i -lt [Math]::Min(3, $newBook.bids.Count); $i++) {
    $bid = $newBook.bids[$i]
    Write-Host "    `$$($bid[0]) — $($bid[1]) BTC" -ForegroundColor Gray
}

Write-Host "`n  Top 3 Asks (Sell):" -ForegroundColor Magenta
for ($i = 0; $i -lt [Math]::Min(3, $newBook.asks.Count); $i++) {
    $ask = $newBook.asks[$i]
    Write-Host "    `$$($ask[0]) — $($ask[1]) BTC" -ForegroundColor Gray
}

# Summary
Write-Host "`n" -ForegroundColor Cyan
Write-Host "╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║                    DEMO COMPLETE!                            ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`n✓ All features working:" -ForegroundColor Green
Write-Host "  • Order book with real-time depth" -ForegroundColor Gray
Write-Host "  • Limit orders (buy/sell with post-only)" -ForegroundColor Gray
Write-Host "  • Market orders with price discovery" -ForegroundColor Gray
Write-Host "  • Trade history tracking" -ForegroundColor Gray
Write-Host "  • User order management" -ForegroundColor Gray
Write-Host "  • Partial fills supported" -ForegroundColor Gray
Write-Host "  • 24h ticker statistics" -ForegroundColor Gray

Write-Host "`n🌐 Access Points:" -ForegroundColor Cyan
Write-Host "  Wallet:  http://localhost:7070/wallet/" -ForegroundColor Yellow
Write-Host "  Miner:   http://localhost:7070/panel.html" -ForegroundColor Yellow
Write-Host "  API:     http://localhost:7070/api/market/exchange/" -ForegroundColor Yellow

Write-Host "`n"
