# Vision Node Rewards Diagnostic Script
Write-Host "=== Vision Node Rewards Check ===" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://127.0.0.1:7070"

# Check if node is running
try {
    Write-Host "1. Checking node status..." -ForegroundColor Yellow
    $height = Invoke-RestMethod -Uri "$baseUrl/api/height" -Method GET
    Write-Host "   ✓ Node is running at height: $height" -ForegroundColor Green
    Write-Host ""
} catch {
    Write-Host "   ✗ Node is not running or not responding" -ForegroundColor Red
    exit 1
}

# Check supply
try {
    Write-Host "2. Checking total supply..." -ForegroundColor Yellow
    $supply = Invoke-RestMethod -Uri "$baseUrl/api/supply" -Method GET
    Write-Host "   Supply response: $($supply | ConvertTo-Json)" -ForegroundColor Gray
    $supplyValue = $supply.total
    Write-Host "   Total supply: $supplyValue" -ForegroundColor $(if ($supplyValue -eq 0) { "Red" } else { "Green" })
    Write-Host ""
} catch {
    Write-Host "   ✗ Failed to get supply: $_" -ForegroundColor Red
}

# Check pow_miner balance
try {
    Write-Host "3. Checking pow_miner balance..." -ForegroundColor Yellow
    $balance = Invoke-RestMethod -Uri "$baseUrl/api/balance/pow_miner" -Method GET
    Write-Host "   Balance response: $($balance | ConvertTo-Json)" -ForegroundColor Gray
    Write-Host "   LAND balance: $($balance.LAND)" -ForegroundColor $(if ($balance.LAND -eq 0) { "Red" } else { "Green" })
    Write-Host ""
} catch {
    Write-Host "   ✗ Failed to get balance: $_" -ForegroundColor Red
}

# Check wallet info
try {
    Write-Host "4. Checking wallet info..." -ForegroundColor Yellow
    $wallet = Invoke-RestMethod -Uri "$baseUrl/api/wallet/info" -Method GET
    Write-Host "   Wallet response: $($wallet | ConvertTo-Json)" -ForegroundColor Gray
    Write-Host ""
} catch {
    Write-Host "   ✗ Failed to get wallet info: $_" -ForegroundColor Red
}

# Check status
try {
    Write-Host "5. Checking node status..." -ForegroundColor Yellow
    $status = Invoke-RestMethod -Uri "$baseUrl/api/status" -Method GET
    Write-Host "   Height: $($status.height)" -ForegroundColor Cyan
    Write-Host "   Mining: $($status.mining)" -ForegroundColor Cyan
    Write-Host ""
} catch {
    Write-Host "   ✗ Failed to get status: $_" -ForegroundColor Red
}

Write-Host "=== Diagnostic Summary ===" -ForegroundColor Cyan
if ($supplyValue -eq 0 -and $height -gt 0) {
    Write-Host "⚠ ISSUE FOUND: Node has mined $height blocks but supply is 0" -ForegroundColor Red
    Write-Host "This means block rewards are NOT being credited." -ForegroundColor Red
    Write-Host ""
    Write-Host "Possible causes:" -ForegroundColor Yellow
    Write-Host "1. Emission disabled via VISION_TOK_ENABLE_EMISSION=false" -ForegroundColor Yellow
    Write-Host "2. Bug in apply_tokenomics function" -ForegroundColor Yellow
    Write-Host "3. Balances not being persisted to database" -ForegroundColor Yellow
} elseif ($supplyValue -gt 0) {
    Write-Host "✓ Rewards are being credited! Supply: $supplyValue" -ForegroundColor Green
    $expectedMin = $height * 100000000000  # 100 tokens minimum per block
    if ($supplyValue -lt $expectedMin) {
        Write-Host "⚠ Supply seems lower than expected for $height blocks" -ForegroundColor Yellow
    }
} else {
    Write-Host "? Could not determine issue - check responses above" -ForegroundColor Yellow
}
