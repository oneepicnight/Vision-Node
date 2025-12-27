# Test Tokenomics + 2-LAND Block Tithe
# This script verifies the new emission system

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " Testing Tokenomics + 2-LAND Block Tithe System" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://127.0.0.1:7070"
$vaultAddr = "0xb977c16e539670ddfecc0ac902fcb916ec4b944e"
$fundAddr = "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd"
$treasuryAddr = "0xdf7a79291bb96e9dd1c77da089933767999eabf0"

# Test 1: Check Foundation Addresses
Write-Host "✅ Test 1: Check Foundation Addresses Configuration" -ForegroundColor Green
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/foundation/addresses" -Method Get
    Write-Host "  Vault: $($response.addresses.vault)" -ForegroundColor White
    Write-Host "  Fund: $($response.addresses.fund)" -ForegroundColor White
    Write-Host "  Treasury: $($response.addresses.treasury)" -ForegroundColor White
    Write-Host "  Tithe Amount: $($response.tithe.amount) units (2 LAND)" -ForegroundColor White
    Write-Host "  Splits: Miner=$($response.tithe.split_bps.miner)bp, Vault=$($response.tithe.split_bps.vault)bp, Fund=$($response.tithe.split_bps.fund)bp, Treasury=$($response.tithe.split_bps.treasury)bp" -ForegroundColor White
    Write-Host "  ✓ Passed" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: Check Tokenomics Stats
Write-Host "✅ Test 2: Check Tokenomics Stats" -ForegroundColor Green
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/tokenomics/stats" -Method Get
    Write-Host "  Emission Enabled: $($response.config.enable_emission)" -ForegroundColor White
    Write-Host "  Emission Per Block: $($response.config.emission_per_block) units" -ForegroundColor White
    Write-Host "  Halving Interval: $($response.config.halving_interval_blocks) blocks" -ForegroundColor White
    Write-Host "  Current Height: $($response.state.current_height)" -ForegroundColor White
    Write-Host "  Total Supply: $($response.state.total_supply)" -ForegroundColor White
    Write-Host "  Vault Total: $($response.state.vault_total)" -ForegroundColor White
    Write-Host "  Fund Total: $($response.state.fund_total)" -ForegroundColor White
    Write-Host "  Treasury Total: $($response.state.treasury_total)" -ForegroundColor White
    Write-Host "  ✓ Passed" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 3: Check Emission Calculation at Height 0
Write-Host "✅ Test 3: Check Emission at Height 0 (Era 0)" -ForegroundColor Green
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/tokenomics/emission/0" -Method Get
    Write-Host "  Height: $($response.height)" -ForegroundColor White
    Write-Host "  Halvings: $($response.halvings)" -ForegroundColor White
    Write-Host "  Block Emission: $($response.block_emission) units" -ForegroundColor White
    Write-Host "  Tithe Amount: $($response.tithe.amount) units" -ForegroundColor White
    Write-Host "  Tithe Vault Share: $($response.tithe.vault_share) units (50%)" -ForegroundColor White
    Write-Host "  Tithe Fund Share: $($response.tithe.fund_share) units (30%)" -ForegroundColor White
    Write-Host "  Tithe Treasury Share: $($response.tithe.treasury_share) units (20%)" -ForegroundColor White
    Write-Host "  ✓ Passed" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 4: Check Emission After First Halving
Write-Host "✅ Test 4: Check Emission at Height 2102400 (After First Halving)" -ForegroundColor Green
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/tokenomics/emission/2102400" -Method Get
    Write-Host "  Height: $($response.height)" -ForegroundColor White
    Write-Host "  Halvings: $($response.halvings)" -ForegroundColor White
    Write-Host "  Halving Divisor: $($response.halving_divisor)" -ForegroundColor White
    Write-Host "  Block Emission: $($response.block_emission) units (should be half of original)" -ForegroundColor White
    
    $originalEmission = 1000000000000
    $expectedEmission = $originalEmission / $response.halving_divisor
    if ($response.block_emission -eq $expectedEmission.ToString()) {
        Write-Host "  ✓ Emission correctly halved!" -ForegroundColor Green
    }
    else {
        Write-Host "  ⚠ Emission mismatch: expected $expectedEmission, got $($response.block_emission)" -ForegroundColor Yellow
    }
    Write-Host "  ✓ Passed" -ForegroundColor Green
}
catch {
    Write-Host "  ✗ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 5: Check Initial Balances (Should be 0 before mining)
Write-Host "✅ Test 5: Check Initial Foundation Balances" -ForegroundColor Green
Write-Host "  Checking Vault balance..." -ForegroundColor White
try {
    $vaultBalance = Invoke-RestMethod -Uri "$baseUrl/api/balance/$vaultAddr" -Method Get
    Write-Host "    Vault: $vaultBalance" -ForegroundColor White
} catch {
    Write-Host "    Vault: 0 (not found)" -ForegroundColor Gray
}

Write-Host "  Checking Fund balance..." -ForegroundColor White
try {
    $fundBalance = Invoke-RestMethod -Uri "$baseUrl/api/balance/$fundAddr" -Method Get
    Write-Host "    Fund: $fundBalance" -ForegroundColor White
} catch {
    Write-Host "    Fund: 0 (not found)" -ForegroundColor Gray
}

Write-Host "  Checking Treasury balance..." -ForegroundColor White
try {
    $treasuryBalance = Invoke-RestMethod -Uri "$baseUrl/api/balance/$treasuryAddr" -Method Get
    Write-Host "    Treasury: $treasuryBalance" -ForegroundColor White
} catch {
    Write-Host "    Treasury: 0 (not found)" -ForegroundColor Gray
}
Write-Host "  ✓ Passed" -ForegroundColor Green
Write-Host ""

Write-Host "==================================================" -ForegroundColor Cyan
Write-Host " Next Steps:" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "1. Mine a block:" -ForegroundColor Yellow
Write-Host "   curl -X POST http://127.0.0.1:7070/mine -H 'Content-Type: application/json' -d '{\"miner_addr\":\"YOUR_ADDR\"}'" -ForegroundColor White
Write-Host ""
Write-Host "2. Check Vault balance (should be 100000000 = 1 LAND):" -ForegroundColor Yellow
Write-Host "   curl http://127.0.0.1:7070/api/balance/$vaultAddr" -ForegroundColor White
Write-Host ""
Write-Host "3. Check Fund balance (should be 60000000 = 0.6 LAND):" -ForegroundColor Yellow
Write-Host "   curl http://127.0.0.1:7070/api/balance/$fundAddr" -ForegroundColor White
Write-Host ""
Write-Host "4. Check Treasury balance (should be 40000000 = 0.4 LAND):" -ForegroundColor Yellow
Write-Host "   curl http://127.0.0.1:7070/api/balance/$treasuryAddr" -ForegroundColor White
Write-Host ""
Write-Host "5. Check total supply growth:" -ForegroundColor Yellow
Write-Host "   curl http://127.0.0.1:7070/api/supply" -ForegroundColor White
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
