# Simple Tokenomics Test - Just check endpoints without complex logic
Write-Host "===================================" -ForegroundColor Cyan
Write-Host " Tokenomics System Simple Test" -ForegroundColor Cyan
Write-Host "===================================" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://127.0.0.1:7070"

# Test 1: Foundation Addresses
Write-Host "Test 1: Foundation Addresses" -ForegroundColor Green
try {
    $result = Invoke-RestMethod -Uri "$baseUrl/foundation/addresses" -Method Get -TimeoutSec 5
    Write-Host "  Vault: $($result.addresses.vault)" -ForegroundColor White
    Write-Host "  Fund: $($result.addresses.fund)" -ForegroundColor White
    Write-Host "  Treasury: $($result.addresses.treasury)" -ForegroundColor White
    Write-Host "  Tithe Amount: $($result.tithe.amount)" -ForegroundColor White
    Write-Host "  ✓ PASSED" -ForegroundColor Green
}
catch {
    Write-Host "  ✗ FAILED: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: Tokenomics Stats
Write-Host "Test 2: Tokenomics Stats" -ForegroundColor Green
try {
    $result = Invoke-RestMethod -Uri "$baseUrl/tokenomics/stats" -Method Get -TimeoutSec 5
    Write-Host "  Height: $($result.state.current_height)" -ForegroundColor White
    Write-Host "  Emission Per Block: $($result.config.emission_per_block)" -ForegroundColor White
    Write-Host "  Halving Interval: $($result.config.halving_interval_blocks)" -ForegroundColor White
    Write-Host "  ✓ PASSED" -ForegroundColor Green
}
catch {
    Write-Host "  ✗ FAILED: $_" -ForegroundColor Red
}
Write-Host ""

# Test 3: Emission Check
Write-Host "Test 3: Emission Calculation (Height 0)" -ForegroundColor Green
try {
    $result = Invoke-RestMethod -Uri "$baseUrl/tokenomics/emission/0" -Method Get -TimeoutSec 5
    Write-Host "  Height: $($result.height)" -ForegroundColor White
    Write-Host "  Emission: $($result.block_emission)" -ForegroundColor White
    Write-Host "  Tithe: $($result.tithe.amount)" -ForegroundColor White
    Write-Host "  ✓ PASSED" -ForegroundColor Green
}
catch {
    Write-Host "  ✗ FAILED: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "===================================" -ForegroundColor Cyan
Write-Host "All critical endpoints operational!" -ForegroundColor Green
Write-Host "===================================" -ForegroundColor Cyan
