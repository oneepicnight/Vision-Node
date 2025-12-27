#!/usr/bin/env pwsh
# Test script for Vault Epoch Payouts to Land Stakers
# Usage: .\test-vault-epoch.ps1 [-BaseUrl "http://127.0.0.1:7070"]

param(
    [string]$BaseUrl = "http://127.0.0.1:7070",
    [int]$EpochBlocks = 30,
    [int]$ParcelWeightMult = 1
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node: Vault Epoch Payout Test ===" -ForegroundColor Cyan
Write-Host ""

# Configure epoch settings
$env:VISION_EPOCH_BLOCKS = $EpochBlocks
$env:VISION_PARCEL_WEIGHT_MULT = $ParcelWeightMult

Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Epoch Length: $EpochBlocks blocks"
Write-Host "  Parcel Weight Multiplier: $ParcelWeightMult"
Write-Host "  Base URL: $BaseUrl"
Write-Host ""

# Test 1: Check epoch status
Write-Host "[1] Checking epoch status..." -ForegroundColor Green
try {
    $epoch = Invoke-RestMethod -Uri "$BaseUrl/vault/epoch" -Method Get
    Write-Host "  ✓ Epoch Index: $($epoch.epoch_index)" -ForegroundColor Green
    Write-Host "  ✓ Last Payout Height: $($epoch.last_payout_height)" -ForegroundColor Green
    Write-Host "  ✓ Next Payout Height: $($epoch.next_payout_height)" -ForegroundColor Green
    Write-Host "  ✓ Vault Balance: $($epoch.vault_balance)" -ForegroundColor Green
    Write-Host "  ✓ Total Weight: $($epoch.total_weight)" -ForegroundColor Green
    Write-Host "  ✓ Due: $($epoch.due)" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to get epoch status: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 2: Check current height
Write-Host "[2] Checking chain height..." -ForegroundColor Green
try {
    $heightResp = Invoke-RestMethod -Uri "$BaseUrl/height" -Method Get
    $currentHeight = $heightResp.height
    Write-Host "  ✓ Current Height: $currentHeight" -ForegroundColor Green
    $blocksUntilPayout = $epoch.next_payout_height - $currentHeight
    Write-Host "  ✓ Blocks Until Next Payout: $blocksUntilPayout" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to get chain height: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 3: Check vault stats
Write-Host "[3] Checking vault stats..." -ForegroundColor Green
try {
    $vault = Invoke-RestMethod -Uri "$BaseUrl/vault" -Method Get
    Write-Host "  ✓ Vault Split: vault=$($vault.split.vault)%, ops=$($vault.split.ops)%, founders=$($vault.split.founders)%" -ForegroundColor Green
    Write-Host "  ✓ LAND Totals: vault=$($vault.totals.LAND.vault), ops=$($vault.totals.LAND.ops), founders=$($vault.totals.LAND.founders)" -ForegroundColor Green
    Write-Host "  ✓ CASH Totals: vault=$($vault.totals.CASH.vault), ops=$($vault.totals.CASH.ops), founders=$($vault.totals.CASH.founders)" -ForegroundColor Green
} catch {
    Write-Host "  ✗ Failed to get vault stats: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 4: Check for vault_payout receipts
Write-Host "[4] Checking for vault payout receipts..." -ForegroundColor Green
try {
    $receipts = Invoke-RestMethod -Uri "$BaseUrl/receipts/latest?limit=50" -Method Get
    $vaultPayouts = $receipts | Where-Object { $_.kind -eq "vault_payout" }
    
    if ($vaultPayouts.Count -gt 0) {
        Write-Host "  ✓ Found $($vaultPayouts.Count) vault payout receipt(s)" -ForegroundColor Green
        foreach ($payout in $vaultPayouts | Select-Object -First 5) {
            Write-Host "    - Epoch $($payout.note -replace 'epoch=',''): $($payout.amount) to $($payout.to)" -ForegroundColor Cyan
        }
    } else {
        Write-Host "  ℹ No vault payout receipts found (payouts may not have occurred yet)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "  ✗ Failed to get receipts: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 5: Simulate epoch boundary (optional - requires mining capability)
Write-Host "[5] Testing epoch payout mechanics..." -ForegroundColor Green
Write-Host "  ℹ To trigger a payout:" -ForegroundColor Yellow
Write-Host "    1. Ensure vault_total > 0 in tokenomics tree" -ForegroundColor Gray
Write-Host "    2. Ensure land_owners tree has entries" -ForegroundColor Gray
Write-Host "    3. Mine blocks until height >= next_payout_height ($($epoch.next_payout_height))" -ForegroundColor Gray
Write-Host "    4. Check receipts for 'vault_payout' entries" -ForegroundColor Gray
Write-Host ""

# Summary
Write-Host "=== Test Summary ===" -ForegroundColor Cyan
Write-Host "  ✓ Epoch status API working" -ForegroundColor Green
Write-Host "  ✓ Vault stats API working" -ForegroundColor Green
Write-Host "  ✓ Receipt tracking operational" -ForegroundColor Green
Write-Host ""

# Tips for manual testing
Write-Host "=== Manual Testing Tips ===" -ForegroundColor Yellow
Write-Host ""
Write-Host "1. Seed land ownership (in sled):" -ForegroundColor Cyan
Write-Host '   db.open_tree("land_owners").insert(b"parcel_001", b"alice12345678")' -ForegroundColor Gray
Write-Host '   db.open_tree("land_owners").insert(b"parcel_002", b"bob987654321")' -ForegroundColor Gray
Write-Host ""
Write-Host "2. Rebuild owner weights:" -ForegroundColor Cyan
Write-Host '   land_stake::rebuild_owner_weights(&db)' -ForegroundColor Gray
Write-Host ""
Write-Host "3. Seed vault balance:" -ForegroundColor Cyan
Write-Host '   db.open_tree("tokenomics").insert(b"vault_total", 1_000_000_u128.to_le_bytes())' -ForegroundColor Gray
Write-Host ""
Write-Host "4. Mine blocks to trigger payout:" -ForegroundColor Cyan
Write-Host "   curl -X POST $BaseUrl/mine_block" -ForegroundColor Gray
Write-Host ""
Write-Host "5. Verify payouts in receipts:" -ForegroundColor Cyan
Write-Host '   curl "$BaseUrl/receipts/latest?limit=25"' -ForegroundColor Gray
Write-Host ""
Write-Host "Done!" -ForegroundColor Green
