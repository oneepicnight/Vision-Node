# ========================================
# Vision Node: Wallet & Receipts API Test
# ========================================
# Tests the new /wallet and /receipts endpoints
# Requires: Vision node running on http://127.0.0.1:7070

param(
    [string]$BaseUrl = "http://127.0.0.1:7070",
    [string]$TestAddr1 = "a".PadRight(64, '0'),  # 64-char hex address
    [string]$TestAddr2 = "b".PadRight(64, '0'),  # 64-char hex address
    [string]$AdminToken = $env:VISION_ADMIN_TOKEN
)

$ErrorActionPreference = "Continue"

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Vision Node: Wallet & Receipts API Test" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# Helper function to display test results
function Test-Result {
    param($Name, $Success, $Message = "")
    if ($Success) {
        Write-Host "[" -NoNewline
        Write-Host "✓" -ForegroundColor Green -NoNewline
        Write-Host "] $Name" -ForegroundColor White
        if ($Message) { Write-Host "    $Message" -ForegroundColor Gray }
    } else {
        Write-Host "[" -NoNewline
        Write-Host "✗" -ForegroundColor Red -NoNewline
        Write-Host "] $Name" -ForegroundColor White
        if ($Message) { Write-Host "    Error: $Message" -ForegroundColor Red }
    }
}

# ========================================
# Test 1: Seed initial balance via admin endpoint
# ========================================
Write-Host "`n[Test 1] Seeding initial balance via admin endpoint..." -ForegroundColor Yellow

if (-not $AdminToken) {
    Write-Host "    Warning: VISION_ADMIN_TOKEN not set, skipping seed" -ForegroundColor Yellow
    Write-Host "    Set with: `$env:VISION_ADMIN_TOKEN = 'your-token'" -ForegroundColor Gray
    $skipSeed = $true
} else {
    $skipSeed = $false
    $seedAmount = "10000000"
    $seedBody = @{
        address = $TestAddr1
        amount = $seedAmount
    } | ConvertTo-Json

    try {
        $headers = @{
            "Authorization" = "Bearer $AdminToken"
            "Content-Type" = "application/json"
        }
        $response = Invoke-RestMethod -Uri "$BaseUrl/admin/seed-balance" -Method Post -Headers $headers -Body $seedBody
        Test-Result "Seed balance for $TestAddr1" $true "Seeded: $seedAmount"
    } catch {
        Test-Result "Seed balance for $TestAddr1" $false $_.Exception.Message
        $skipSeed = $true
    }
}

# ========================================
# Test 2: Get balance for test address 1
# ========================================
Write-Host "`n[Test 2] Getting balance for $TestAddr1..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/$TestAddr1/balance" -Method Get -ContentType "application/json"
    $balance1 = if ($response.balance) { $response.balance } else { "0" }
    Test-Result "Get balance for $TestAddr1" $true "Balance: $balance1"
} catch {
    Test-Result "Get balance for $TestAddr1" $false $_.Exception.Message
    $balance1 = "0"
}

# ========================================
# Test 3: Get balance for test address 2
# ========================================
Write-Host "`n[Test 3] Getting balance for $TestAddr2..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/$TestAddr2/balance" -Method Get -ContentType "application/json"
    $balance2 = if ($response.balance) { $response.balance } else { "0" }
    Test-Result "Get balance for $TestAddr2" $true "Balance: $balance2"
} catch {
    Test-Result "Get balance for $TestAddr2" $false $_.Exception.Message
    $balance2 = "0"
}

# ========================================
# Test 4: Transfer from addr1 to addr2
# ========================================
Write-Host "`n[Test 4] Transferring 1000 tokens from $TestAddr1 to $TestAddr2..." -ForegroundColor Yellow

if ($skipSeed) {
    Write-Host "    Skipping transfer test (no balance seeded)" -ForegroundColor Yellow
} else {
    $transferAmount = "1000"
    $transferFee = "10"
    $transferBody = @{
        from = $TestAddr1
        to = $TestAddr2
        amount = $transferAmount
        fee = $transferFee
        memo = "Test transfer from PowerShell"
    } | ConvertTo-Json

    try {
        $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/transfer" -Method Post -ContentType "application/json" -Body $transferBody
        if ($response.status -eq "ok") {
            Test-Result "Transfer tokens" $true "Receipt ID: $($response.receipt_id)"
        } else {
            Test-Result "Transfer tokens" $false "Status: $($response.status)"
        }
    } catch {
        $errMsg = $_.Exception.Message
        if ($_ -match "insufficient_funds") {
            Test-Result "Transfer tokens" $false "Insufficient funds"
        } else {
            Test-Result "Transfer tokens" $false $errMsg
        }
    }
}

# ========================================
# Test 5: Verify balances changed (if transfer succeeded)
# ========================================
Write-Host "`n[Test 5] Verifying balances after transfer..." -ForegroundColor Yellow
try {
    $response1 = Invoke-RestMethod -Uri "$BaseUrl/wallet/$TestAddr1/balance" -Method Get -ContentType "application/json"
    $newBalance1 = if ($response1.balance) { $response1.balance } else { "0" }
    
    $response2 = Invoke-RestMethod -Uri "$BaseUrl/wallet/$TestAddr2/balance" -Method Get -ContentType "application/json"
    $newBalance2 = if ($response2.balance) { $response2.balance } else { "0" }
    
    Write-Host "    $TestAddr1 balance: $balance1 -> $newBalance1" -ForegroundColor Gray
    Write-Host "    $TestAddr2 balance: $balance2 -> $newBalance2" -ForegroundColor Gray
    Test-Result "Verify balance changes" $true
} catch {
    Test-Result "Verify balance changes" $false $_.Exception.Message
}

# ========================================
# Test 6: Get latest receipts
# ========================================
Write-Host "`n[Test 6] Getting latest receipts..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/receipts/latest?limit=10" -Method Get -ContentType "application/json"
    $count = if ($response) { $response.Count } else { 0 }
    Test-Result "Get latest receipts" $true "Found $count receipts"
    
    if ($count -gt 0) {
        Write-Host "`n    Recent receipts:" -ForegroundColor Gray
        foreach ($receipt in $response | Select-Object -First 3) {
            Write-Host "      - [$($receipt.kind)] $($receipt.from) -> $($receipt.to): $($receipt.amount) (fee: $($receipt.fee))" -ForegroundColor Gray
            if ($receipt.memo) {
                Write-Host "        Memo: $($receipt.memo)" -ForegroundColor DarkGray
            }
        }
    }
} catch {
    Test-Result "Get latest receipts" $false $_.Exception.Message
}

# ========================================
# Test 7: Invalid address validation
# ========================================
Write-Host "`n[Test 7] Testing address validation (short address)..." -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/short/balance" -Method Get -ContentType "application/json"
    Test-Result "Address validation" $false "Should have rejected short address"
} catch {
    if ($_ -match "invalid_address" -or $_ -match "400") {
        Test-Result "Address validation" $true "Correctly rejected invalid address"
    } else {
        Test-Result "Address validation" $false $_.Exception.Message
    }
}

# ========================================
# Test 7b: Invalid hex characters
# ========================================
Write-Host "`n[Test 7b] Testing address validation (invalid hex)..." -ForegroundColor Yellow
$invalidAddr = "z".PadRight(64, 'x')  # Non-hex characters
try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/$invalidAddr/balance" -Method Get -ContentType "application/json"
    Test-Result "Hex validation" $false "Should have rejected non-hex address"
} catch {
    if ($_ -match "invalid_address" -or $_ -match "400") {
        Test-Result "Hex validation" $true "Correctly rejected non-hex address"
    } else {
        Test-Result "Hex validation" $false $_.Exception.Message
    }
}

# ========================================
# Test 8: Invalid transfer (zero amount)
# ========================================
Write-Host "`n[Test 8] Testing zero amount transfer validation..." -ForegroundColor Yellow
$zeroBody = @{
    from = $TestAddr1
    to = $TestAddr2
    amount = "0"
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/transfer" -Method Post -ContentType "application/json" -Body $zeroBody
    Test-Result "Zero amount validation" $false "Should have rejected zero amount"
} catch {
    if ($_ -match "amount_zero" -or $_ -match "400") {
        Test-Result "Zero amount validation" $true "Correctly rejected zero amount"
    } else {
        Test-Result "Zero amount validation" $false $_.Exception.Message
    }
}

# ========================================
# Test 9: Invalid transfer (same sender/recipient)
# ========================================
Write-Host "`n[Test 9] Testing same sender/recipient validation..." -ForegroundColor Yellow
$sameBody = @{
    from = $TestAddr1
    to = $TestAddr1
    amount = "100"
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$BaseUrl/wallet/transfer" -Method Post -ContentType "application/json" -Body $sameBody
    Test-Result "Same address validation" $false "Should have rejected same sender/recipient"
} catch {
    if ($_ -match "same_sender_recipient" -or $_ -match "400") {
        Test-Result "Same address validation" $true "Correctly rejected same sender/recipient"
    } else {
        Test-Result "Same address validation" $false $_.Exception.Message
    }
}

# ========================================
# Summary
# ========================================
Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "Test Suite Complete" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

Write-Host "Tips for seeding balances:" -ForegroundColor Yellow
Write-Host "1. Use the node's airdrop endpoint (if admin token configured)" -ForegroundColor Gray
Write-Host "2. Manually write to sled DB 'balances' tree (u128 little-endian)" -ForegroundColor Gray
Write-Host "3. Add a /debug/seed_balance endpoint to your node for testing`n" -ForegroundColor Gray
