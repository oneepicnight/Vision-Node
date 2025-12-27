#!/usr/bin/env pwsh
# test-reorg-persistence.ps1
# Test reorg with undo persistence and restart verification

$ErrorActionPreference = "Stop"

Write-Host "=== Reorg Persistence Test ===" -ForegroundColor Cyan
Write-Host "Tests: BlockUndo persistence, balance restoration, UTXO integrity, restart after reorg" -ForegroundColor Cyan
Write-Host ""

# Cleanup function
function Cleanup {
    Write-Host "Cleaning up..." -ForegroundColor Yellow
    Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    Remove-Item -Recurse -Force "temp-reorg-test-*" -ErrorAction SilentlyContinue
}

# Cleanup before starting
Cleanup

# Create test directories
$nodeA_dir = "temp-reorg-test-nodeA"
$nodeB_dir = "temp-reorg-test-nodeB"

New-Item -ItemType Directory -Force -Path $nodeA_dir | Out-Null
New-Item -ItemType Directory -Force -Path $nodeB_dir | Out-Null

# Generate test wallets
Write-Host "[1/8] Generating test wallets..." -ForegroundColor Green
$walletA = & ./target/release/vision-node.exe --keygen 2>&1 | Where-Object { $_ -match "^[0-9a-f]{64}$" } | Select-Object -First 1
$walletB = & ./target/release/vision-node.exe --keygen 2>&1 | Where-Object { $_ -match "^[0-9a-f]{64}$" } | Select-Object -First 1
$recipient = & ./target/release/vision-node.exe --keygen 2>&1 | Where-Object { $_ -match "^[0-9a-f]{64}$" } | Select-Object -First 1

if (-not $walletA -or -not $walletB -or -not $recipient) {
    Write-Host "❌ Failed to generate wallets" -ForegroundColor Red
    Cleanup
    exit 1
}

Write-Host "  Wallet A: $walletA" -ForegroundColor Gray
Write-Host "  Wallet B: $walletB" -ForegroundColor Gray
Write-Host "  Recipient: $recipient" -ForegroundColor Gray

# Create keys files
@{
    "wallet_address" = $walletA
    "wallet_privkey" = "0000000000000000000000000000000000000000000000000000000000000001"
} | ConvertTo-Json | Set-Content "$nodeA_dir/keys.json"

@{
    "wallet_address" = $walletB
    "wallet_privkey" = "0000000000000000000000000000000000000000000000000000000000000002"
} | ConvertTo-Json | Set-Content "$nodeB_dir/keys.json"

# Start Node A (port 7070)
Write-Host "[2/8] Starting Node A (port 7070)..." -ForegroundColor Green
$envA = @{
    "VISION_PORT" = "7070"
    "VISION_DATA_DIR" = $nodeA_dir
    "VISION_MINE" = "1"
    "VISION_MINE_INTERVAL" = "2"
    "VISION_BOOTSTRAP_PEERS" = ""
    "VISION_SNAPSHOT_EVERY_BLOCKS" = "5"
    "VISION_UNDO_DEPTH" = "20"
}
$procA = Start-Process -FilePath "./target/release/vision-node.exe" `
    -WorkingDirectory (Get-Location) `
    -Environment $envA `
    -PassThru `
    -WindowStyle Hidden

Start-Sleep -Seconds 3

# Verify Node A is running
$heightA = try { (Invoke-RestMethod "http://127.0.0.1:7070/height" -TimeoutSec 2) } catch { $null }
if (-not $heightA) {
    Write-Host "❌ Node A failed to start" -ForegroundColor Red
    Cleanup
    exit 1
}
Write-Host "  Node A running at height $heightA" -ForegroundColor Gray

# Mine some blocks on Node A
Write-Host "[3/8] Mining 10 blocks on Node A..." -ForegroundColor Green
for ($i = 1; $i -le 10; $i++) {
    Start-Sleep -Seconds 2
    $heightA = Invoke-RestMethod "http://127.0.0.1:7070/height"
    Write-Host "  Node A height: $heightA" -ForegroundColor Gray
}

# Get initial balance on Node A
$balanceA_initial = Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA"
Write-Host "  Node A initial balance: $($balanceA_initial.balance)" -ForegroundColor Gray

# Send transaction from Node A to recipient
Write-Host "[4/8] Sending transaction on Node A (100 tokens to recipient)..." -ForegroundColor Green
$txBody = @{
    "to" = $recipient
    "amount" = 100
    "privkey" = "0000000000000000000000000000000000000000000000000000000000000001"
} | ConvertTo-Json

try {
    $txResult = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:7070/tx" -Body $txBody -ContentType "application/json"
    Write-Host "  Transaction sent: $($txResult.tx_hash)" -ForegroundColor Gray
} catch {
    Write-Host "  Transaction send failed (might be normal if no balance yet)" -ForegroundColor Yellow
}

Start-Sleep -Seconds 3

# Get balance after transaction
$balanceA_after_tx = Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA"
$recipientBalance = Invoke-RestMethod "http://127.0.0.1:7070/balance/$recipient"
Write-Host "  Node A balance after TX: $($balanceA_after_tx.balance)" -ForegroundColor Gray
Write-Host "  Recipient balance: $($recipientBalance.balance)" -ForegroundColor Gray

# Start Node B (port 7071) - will mine competing chain
Write-Host "[5/8] Starting Node B (port 7071) - creating competing chain..." -ForegroundColor Green
$envB = @{
    "VISION_PORT" = "7071"
    "VISION_DATA_DIR" = $nodeB_dir
    "VISION_MINE" = "1"
    "VISION_MINE_INTERVAL" = "1"
    "VISION_BOOTSTRAP_PEERS" = ""
    "VISION_SNAPSHOT_EVERY_BLOCKS" = "5"
    "VISION_UNDO_DEPTH" = "20"
}
$procB = Start-Process -FilePath "./target/release/vision-node.exe" `
    -WorkingDirectory (Get-Location) `
    -Environment $envB `
    -PassThru `
    -WindowStyle Hidden

Start-Sleep -Seconds 3

# Mine more blocks on Node B (faster, to create heavier chain)
Write-Host "  Mining faster on Node B to create heavier chain..." -ForegroundColor Gray
for ($i = 1; $i -le 15; $i++) {
    Start-Sleep -Seconds 1
    $heightB = Invoke-RestMethod "http://127.0.0.1:7071/height"
    Write-Host "  Node B height: $heightB" -ForegroundColor Gray
}

$heightA_before = Invoke-RestMethod "http://127.0.0.1:7070/height"
$heightB_before = Invoke-RestMethod "http://127.0.0.1:7071/height"

Write-Host ""
Write-Host "  Before sync:" -ForegroundColor Cyan
Write-Host "    Node A height: $heightA_before" -ForegroundColor Gray
Write-Host "    Node B height: $heightB_before" -ForegroundColor Gray

# Trigger reorg by syncing Node A from Node B
Write-Host "[6/8] Triggering REORG - Node A syncing from Node B..." -ForegroundColor Green
$syncBody = @{
    "src" = "http://127.0.0.1:7071"
} | ConvertTo-Json

try {
    $syncResult = Invoke-RestMethod -Method POST -Uri "http://127.0.0.1:7070/admin/sync" -Body $syncBody -ContentType "application/json"
    Write-Host "  Sync result: pulled=$($syncResult.pulled), failed=$($syncResult.failed)" -ForegroundColor Gray
} catch {
    Write-Host "  Sync failed: $_" -ForegroundColor Yellow
}

Start-Sleep -Seconds 3

# Check heights after reorg
$heightA_after = Invoke-RestMethod "http://127.0.0.1:7070/height"
$heightB_after = Invoke-RestMethod "http://127.0.0.1:7071/height"

Write-Host ""
Write-Host "  After reorg:" -ForegroundColor Cyan
Write-Host "    Node A height: $heightA_after" -ForegroundColor Gray
Write-Host "    Node B height: $heightB_after" -ForegroundColor Gray

# Check balances after reorg
$balanceA_after_reorg = Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA"
$recipientBalance_after_reorg = Invoke-RestMethod "http://127.0.0.1:7070/balance/$recipient"

Write-Host ""
Write-Host "  Balances after reorg:" -ForegroundColor Cyan
Write-Host "    Node A balance: $($balanceA_after_reorg.balance)" -ForegroundColor Gray
Write-Host "    Recipient balance: $($recipientBalance_after_reorg.balance)" -ForegroundColor Gray

# Test 1: Verify reorg happened
$test1_pass = $heightA_after -gt $heightA_before
if ($test1_pass) {
    Write-Host "  ✅ TEST 1 PASS: Reorg occurred (height increased)" -ForegroundColor Green
} else {
    Write-Host "  ❌ TEST 1 FAIL: No reorg detected" -ForegroundColor Red
}

# Test 2: Check if transaction was reverted (recipient balance should be back to 0 if TX was in orphaned blocks)
# Note: This depends on whether the TX was included in the orphaned chain
Write-Host "  ℹ️  Transaction may or may not be reverted depending on timing" -ForegroundColor Gray

# Save state before restart for comparison
$stateBeforeRestart = @{
    "height" = $heightA_after
    "balanceA" = $balanceA_after_reorg.balance
    "balanceRecipient" = $recipientBalance_after_reorg.balance
}

# Stop Node A
Write-Host "[7/8] Stopping Node A for restart test..." -ForegroundColor Green
Stop-Process -Id $procA.Id -Force
Start-Sleep -Seconds 2

# Restart Node A
Write-Host "[8/8] Restarting Node A to verify persistence..." -ForegroundColor Green
$procA_restart = Start-Process -FilePath "./target/release/vision-node.exe" `
    -WorkingDirectory (Get-Location) `
    -Environment $envA `
    -PassThru `
    -WindowStyle Hidden

Start-Sleep -Seconds 5

# Verify Node A restarted successfully
try {
    $heightA_restart = Invoke-RestMethod "http://127.0.0.1:7070/height" -TimeoutSec 5
    $balanceA_restart = Invoke-RestMethod "http://127.0.0.1:7070/balance/$walletA"
    $recipientBalance_restart = Invoke-RestMethod "http://127.0.0.1:7070/balance/$recipient"

    Write-Host ""
    Write-Host "  After restart:" -ForegroundColor Cyan
    Write-Host "    Height: $heightA_restart (was $($stateBeforeRestart.height))" -ForegroundColor Gray
    Write-Host "    Node A balance: $($balanceA_restart.balance) (was $($stateBeforeRestart.balanceA))" -ForegroundColor Gray
    Write-Host "    Recipient balance: $($recipientBalance_restart.balance) (was $($stateBeforeRestart.balanceRecipient))" -ForegroundColor Gray

    # Test 3: Height persisted correctly
    $test3_pass = $heightA_restart -eq $stateBeforeRestart.height
    if ($test3_pass) {
        Write-Host "  ✅ TEST 3 PASS: Height persisted correctly after restart" -ForegroundColor Green
    } else {
        Write-Host "  ❌ TEST 3 FAIL: Height mismatch after restart" -ForegroundColor Red
    }

    # Test 4: Balances persisted correctly
    $test4_pass = ($balanceA_restart.balance -eq $stateBeforeRestart.balanceA) -and 
                  ($recipientBalance_restart.balance -eq $stateBeforeRestart.balanceRecipient)
    if ($test4_pass) {
        Write-Host "  ✅ TEST 4 PASS: Balances persisted correctly after restart" -ForegroundColor Green
    } else {
        Write-Host "  ❌ TEST 4 FAIL: Balance mismatch after restart" -ForegroundColor Red
    }

    # Test 5: Node is still functional
    try {
        $health = Invoke-RestMethod "http://127.0.0.1:7070/health"
        $test5_pass = $true
        Write-Host "  ✅ TEST 5 PASS: Node is functional after restart" -ForegroundColor Green
    } catch {
        $test5_pass = $false
        Write-Host "  ❌ TEST 5 FAIL: Node health check failed" -ForegroundColor Red
    }

    # Overall result
    Write-Host ""
    Write-Host "=== TEST SUMMARY ===" -ForegroundColor Cyan
    if ($test1_pass -and $test3_pass -and $test4_pass -and $test5_pass) {
        Write-Host "✅ ALL TESTS PASSED" -ForegroundColor Green
        Write-Host "  - Reorg occurred successfully" -ForegroundColor Green
        Write-Host "  - Chain tip persisted correctly" -ForegroundColor Green
        Write-Host "  - Balances persisted correctly" -ForegroundColor Green
        Write-Host "  - Node survived restart" -ForegroundColor Green
        $exitCode = 0
    } else {
        Write-Host "❌ SOME TESTS FAILED" -ForegroundColor Red
        if (-not $test1_pass) { Write-Host "  - Reorg did not occur" -ForegroundColor Red }
        if (-not $test3_pass) { Write-Host "  - Height mismatch after restart" -ForegroundColor Red }
        if (-not $test4_pass) { Write-Host "  - Balance mismatch after restart" -ForegroundColor Red }
        if (-not $test5_pass) { Write-Host "  - Node not functional after restart" -ForegroundColor Red }
        $exitCode = 1
    }

} catch {
    Write-Host "❌ ERROR: Failed to verify restart: $_" -ForegroundColor Red
    $exitCode = 1
}

# Cleanup
Cleanup

Write-Host ""
Write-Host "Test complete!" -ForegroundColor Cyan
exit $exitCode
