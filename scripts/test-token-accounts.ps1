# Test script for token accounts settlement system
# Run this after starting the Vision node

$baseUrl = "http://127.0.0.1:7070"
$adminToken = $env:VISION_ADMIN_TOKEN

if (-not $adminToken) {
    Write-Host "ERROR: VISION_ADMIN_TOKEN environment variable not set" -ForegroundColor Red
    exit 1
}

Write-Host "=== Token Accounts Settlement Test ===" -ForegroundColor Cyan
Write-Host ""

# 1. Get current token accounts config
Write-Host "[1] Getting current token accounts config..." -ForegroundColor Yellow
$response = Invoke-RestMethod -Uri "$baseUrl/admin/token-accounts?token=$adminToken" -Method Get
Write-Host "Current config:" -ForegroundColor Green
$response.config | ConvertTo-Json
Write-Host ""

# 2. Test a market sale
Write-Host "[2] Testing market sale with 1000 tokens..." -ForegroundColor Yellow
$saleRequest = @{
    amount = 1000
} | ConvertTo-Json

$response = Invoke-RestMethod -Uri "$baseUrl/market/test-sale" -Method Post -Body $saleRequest -ContentType "application/json"
Write-Host "Sale result:" -ForegroundColor Green
$response | ConvertTo-Json
Write-Host ""

# Calculate expected amounts
$vault = 1000 * 0.50
$fund = 1000 * 0.30
$treasury = 1000 * 0.20
$founder1 = $treasury * 0.50
$founder2 = $treasury * 0.50

Write-Host "Expected distribution:" -ForegroundColor Cyan
Write-Host "  Vault (50%):     $vault"
Write-Host "  Fund (30%):      $fund"
Write-Host "  Founder1 (10%):  $founder1"
Write-Host "  Founder2 (10%):  $founder2"
Write-Host ""

# 3. Verify balances were updated
Write-Host "[3] Checking if balances were credited..." -ForegroundColor Yellow

$config = (Invoke-RestMethod -Uri "$baseUrl/admin/token-accounts?token=$adminToken" -Method Get).config

function Get-Balance($address) {
    $key = "balance:$address"
    # Note: This would need actual balance checking via your balance API
    # For now we just show the expected values
    return "N/A (implement balance check)"
}

Write-Host "Vault balance:    $(Get-Balance $config.vault_address)" -ForegroundColor Green
Write-Host "Fund balance:     $(Get-Balance $config.fund_address)" -ForegroundColor Green
Write-Host "Founder1 balance: $(Get-Balance $config.founder1_address)" -ForegroundColor Green
Write-Host "Founder2 balance: $(Get-Balance $config.founder2_address)" -ForegroundColor Green
Write-Host ""

# 4. Test updating config (optional)
Write-Host "[4] Testing config update (changing vault_pct to 51%)..." -ForegroundColor Yellow
$updateRequest = @{
    vault_pct = 51
    fund_pct = 29
    treasury_pct = 20
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/token-accounts/set?token=$adminToken" -Method Post -Body $updateRequest -ContentType "application/json"
    Write-Host "Update result:" -ForegroundColor Green
    $response | ConvertTo-Json
    Write-Host ""
    
    # Restore original config
    Write-Host "[5] Restoring original config..." -ForegroundColor Yellow
    $restoreRequest = @{
        vault_pct = 50
        fund_pct = 30
        treasury_pct = 20
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/token-accounts/set?token=$adminToken" -Method Post -Body $restoreRequest -ContentType "application/json"
    Write-Host "Restored successfully" -ForegroundColor Green
} catch {
    Write-Host "Config update test failed: $_" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== Test Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Update config/token_accounts.toml with real addresses"
Write-Host "2. Restart the node to load new addresses"
Write-Host "3. Process real market sales - proceeds will auto-route"
Write-Host "4. Check vault ledger: GET /vault/ledger"
Write-Host ""
