#!/usr/bin/env pwsh
# Simple reorg test - verify undo persistence works

Write-Host "=== Reorg Persistence Test ===" -ForegroundColor Cyan

# Cleanup
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force "temp-reorg-*" -ErrorAction SilentlyContinue

# Test directories
$nodeA = "temp-reorg-nodeA"
$nodeB = "temp-reorg-nodeB"
New-Item -ItemType Directory -Force -Path $nodeA, $nodeB | Out-Null

# Generate wallets
Write-Host "[1/6] Generating wallets..." -ForegroundColor Green
$walletA = & ./target/release/vision-node.exe --keygen 2>&1 | Where-Object { $_ -match "^[0-9a-f]{64}$" } | Select-Object -First 1
$walletB = & ./target/release/vision-node.exe --keygen 2>&1 | Where-Object { $_ -match "^[0-9a-f]{64}$" } | Select-Object -First 1

@{ "wallet_address" = $walletA; "wallet_privkey" = "0000000000000000000000000000000000000000000000000000000000000001" } | ConvertTo-Json | Set-Content "$nodeA/keys.json"
@{ "wallet_address" = $walletB; "wallet_privkey" = "0000000000000000000000000000000000000000000000000000000000000002" } | ConvertTo-Json | Set-Content "$nodeB/keys.json"

Write-Host "  Wallet A: $walletA" -ForegroundColor Gray
Write-Host "  Wallet B: $walletB" -ForegroundColor Gray

# Start Node A
Write-Host "[2/6] Starting Node A (port 7070)..." -ForegroundColor Green
$envA = @{
    "VISION_PORT" = "7070"
    "VISION_DATA_DIR" = $nodeA
    "VISION_MINE" = "1"
    "VISION_MINE_INTERVAL" = "2"
    "VISION_SNAPSHOT_EVERY_BLOCKS" = "5"
}

$procA = Start-Process -FilePath "./target/release/vision-node.exe" -Environment $envA -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3

$heightA = Invoke-RestMethod "http://127.0.0.1:7070/height"
Write-Host "  Node A started at height $heightA" -ForegroundColor Gray

# Mine blocks on A
Write-Host "[3/6] Mining 10 blocks on Node A..." -ForegroundColor Green
for ($i = 1; $i -le 5; $i++) {
    Start-Sleep -Seconds 2
    $h = Invoke-RestMethod "http://127.0.0.1:7070/height"
    Write-Host "  Height: $h" -ForegroundColor Gray
}

$balanceA_before = (Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA").balance
Write-Host "  Balance before: $balanceA_before" -ForegroundColor Gray

# Start Node B (competing chain)
Write-Host "[4/6] Starting Node B (port 7071) - competing chain..." -ForegroundColor Green
$envB = @{
    "VISION_PORT" = "7071"
    "VISION_DATA_DIR" = $nodeB
    "VISION_MINE" = "1"
    "VISION_MINE_INTERVAL" = "1"
    "VISION_SNAPSHOT_EVERY_BLOCKS" = "5"
}

$procB = Start-Process -FilePath "./target/release/vision-node.exe" -Environment $envB -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 3

# Mine faster on B
Write-Host "  Mining faster on B..." -ForegroundColor Gray
for ($i = 1; $i -le 8; $i++) {
    Start-Sleep -Seconds 1
    $h = Invoke-RestMethod "http://127.0.0.1:7071/height"
    Write-Host "  Node B height: $h" -ForegroundColor Gray
}

$heightA_before = Invoke-RestMethod "http://127.0.0.1:7070/height"
$heightB = Invoke-RestMethod "http://127.0.0.1:7071/height"
Write-Host ""
Write-Host "  Before sync: A=$heightA_before, B=$heightB" -ForegroundColor Cyan

# Trigger reorg
Write-Host "[5/6] Triggering REORG..." -ForegroundColor Green
$syncBody = @{ "src" = "http://127.0.0.1:7071" } | ConvertTo-Json
Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:7070/admin/sync" -Body $syncBody -ContentType "application/json" | Out-Null
Start-Sleep -Seconds 3

$heightA_after = Invoke-RestMethod "http://127.0.0.1:7070/height"
$balanceA_after = (Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA").balance

Write-Host "  After reorg: height=$heightA_after, balance=$balanceA_after" -ForegroundColor Cyan

$test1 = $heightA_after -gt $heightA_before
if ($test1) {
    Write-Host "  ✅ Reorg occurred" -ForegroundColor Green
} else {
    Write-Host "  ❌ No reorg" -ForegroundColor Red
}

# Save state
$savedHeight = $heightA_after
$savedBalance = $balanceA_after

# Restart Node A
Write-Host "[6/6] Restarting Node A..." -ForegroundColor Green
Stop-Process -Id $procA.Id -Force
Start-Sleep -Seconds 2

$procA2 = Start-Process -FilePath "./target/release/vision-node.exe" -Environment $envA -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5

$heightA_restart = Invoke-RestMethod "http://127.0.0.1:7070/height"
$balanceA_restart = (Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA").balance

Write-Host ""
Write-Host "  After restart: height=$heightA_restart (was $savedHeight)" -ForegroundColor Cyan
Write-Host "  After restart: balance=$balanceA_restart (was $savedBalance)" -ForegroundColor Cyan

$test2 = $heightA_restart -eq $savedHeight
$test3 = $balanceA_restart -eq $savedBalance

if ($test2) {
    Write-Host "  ✅ Height persisted" -ForegroundColor Green
} else {
    Write-Host "  ❌ Height mismatch" -ForegroundColor Red
}

if ($test3) {
    Write-Host "  ✅ Balance persisted" -ForegroundColor Green
} else {
    Write-Host "  ❌ Balance mismatch" -ForegroundColor Red
}

# Cleanup
Write-Host ""
Write-Host "Cleaning up..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
Remove-Item -Recurse -Force "temp-reorg-*" -ErrorAction SilentlyContinue

# Summary
Write-Host ""
Write-Host "=== SUMMARY ===" -ForegroundColor Cyan
if ($test1 -and $test2 -and $test3) {
    Write-Host "✅ ALL TESTS PASSED" -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ SOME TESTS FAILED" -ForegroundColor Red
    exit 1
}
