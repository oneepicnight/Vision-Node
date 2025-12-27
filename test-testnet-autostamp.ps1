#!/usr/bin/env pwsh
# Testnet Auto-Stamp Feature Test Script
# Tests the v2.0 testnet auto-stamp functionality

Write-Host "`n═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "     TESTNET AUTO-STAMP TEST SUITE" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

$ErrorActionPreference = "Continue"
$testDataDir = "c:\vision-node\test-testnet-data"
$binary = ".\target\release\vision-node.exe"

# Test counters
$testsPassed = 0
$testsFailed = 0
$testsTotal = 6

function Test-Result {
    param($testName, $passed, $details)
    if ($passed) {
        Write-Host "✅ PASS: $testName" -ForegroundColor Green
        $script:testsPassed++
    } else {
        Write-Host "❌ FAIL: $testName" -ForegroundColor Red
        $script:testsFailed++
    }
    if ($details) {
        Write-Host "   $details" -ForegroundColor Gray
    }
}

# Cleanup function
function Cleanup-TestData {
    if (Test-Path $testDataDir) {
        Write-Host "🧹 Cleaning up test data..." -ForegroundColor Yellow
        Remove-Item $testDataDir -Recurse -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }
}

Write-Host "📋 Test Plan:" -ForegroundColor Cyan
Write-Host "   1. Verify binary exists" -ForegroundColor White
Write-Host "   2. Test seed node with auto-stamp ENABLED" -ForegroundColor White
Write-Host "   3. Verify 3 blocks stamped" -ForegroundColor White
Write-Host "   4. Test seed node restart (should skip stamping)" -ForegroundColor White
Write-Host "   5. Test regular miner (should not stamp)" -ForegroundColor White
Write-Host "   6. Verify mainnet isolation (Guardian logic unchanged)`n" -ForegroundColor White

# ============================================================================
# TEST 1: Verify binary exists
# ============================================================================
Write-Host "TEST 1: Binary verification" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

if (Test-Path $binary) {
    $bin = Get-Item $binary
    $sizeMB = [math]::Round($bin.Length/1MB, 2)
    Test-Result "Binary exists" $true "$sizeMB MB, built: $($bin.LastWriteTime)"
} else {
    Test-Result "Binary exists" $false "Not found at $binary"
    Write-Host "`n❌ Cannot continue without binary. Run: cargo build --release`n" -ForegroundColor Red
    exit 1
}

# ============================================================================
# TEST 2: Seed node with auto-stamp ENABLED (fresh chain)
# ============================================================================
Write-Host "`nTEST 2: Seed node auto-stamp (first start)" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

Cleanup-TestData
New-Item -ItemType Directory -Path $testDataDir -Force | Out-Null

$env:VISION_NETWORK = "testnet"
$env:VISION_IS_TESTNET_SEED = "true"
$env:VISION_AUTO_STAMP_TESTNET = "true"
$env:VISION_MINER_ADDRESS = "land1testseed"
$env:VISION_DATA_DIR = $testDataDir
$env:VISION_PORT = "17070"
$env:VISION_P2P_PORT = "17072"
$env:RUST_LOG = "info"

Write-Host "   Starting testnet seed node (5 second startup)..." -ForegroundColor Gray
$process = Start-Process -FilePath $binary -NoNewWindow -PassThru -RedirectStandardOutput "$testDataDir\seed-stdout.log" -RedirectStandardError "$testDataDir\seed-stderr.log"
Start-Sleep -Seconds 5

# Check logs
$logs = Get-Content "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
$stderrLogs = Get-Content "$testDataDir\seed-stderr.log" -ErrorAction SilentlyContinue

$hasAutoStampStart = $logs | Select-String "TESTNET_STAMP.*Auto-stamping"
$hasBlock1 = $logs | Select-String "TESTNET_STAMP.*Block 1 stamped"
$hasBlock2 = $logs | Select-String "TESTNET_STAMP.*Block 2 stamped"
$hasBlock3 = $logs | Select-String "TESTNET_STAMP.*Block 3 stamped"
$hasComplete = $logs | Select-String "TESTNET_STAMP.*complete.*height now 3"

# Stop the process
Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

if ($hasAutoStampStart) {
    Test-Result "Auto-stamp initiated" $true "Found: $($hasAutoStampStart.Line.Trim())"
} else {
    Test-Result "Auto-stamp initiated" $false "Did not find auto-stamp start log"
}

if ($hasBlock1 -and $hasBlock2 -and $hasBlock3) {
    Test-Result "3 blocks stamped" $true "Blocks 1, 2, 3 all logged"
} else {
    Test-Result "3 blocks stamped" $false "Missing block stamp logs (found: $($hasBlock1 -ne $null), $($hasBlock2 -ne $null), $($hasBlock3 -ne $null))"
}

if ($hasComplete) {
    Test-Result "Bootstrap complete" $true "Height reached 3"
} else {
    Test-Result "Bootstrap complete" $false "Did not find completion log"
}

# ============================================================================
# TEST 3: Verify chain data persisted
# ============================================================================
Write-Host "`nTEST 3: Verify blocks persisted to disk" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

$dbExists = Test-Path "$testDataDir\vision_data\chain.db"
if ($dbExists) {
    Test-Result "Chain database created" $true "Found at $testDataDir\vision_data\chain.db"
} else {
    Test-Result "Chain database created" $false "Database not found"
}

# ============================================================================
# TEST 4: Seed node restart (should skip stamping)
# ============================================================================
Write-Host "`nTEST 4: Seed node restart (should skip auto-stamp)" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

Write-Host "   Restarting seed node (5 second startup)..." -ForegroundColor Gray
Remove-Item "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
Remove-Item "$testDataDir\seed-stderr.log" -ErrorAction SilentlyContinue

$process = Start-Process -FilePath $binary -NoNewWindow -PassThru -RedirectStandardOutput "$testDataDir\seed-stdout.log" -RedirectStandardError "$testDataDir\seed-stderr.log"
Start-Sleep -Seconds 5

$logs = Get-Content "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
$hasNotRequired = $logs | Select-String "TESTNET_STAMP.*Not required.*height"
$hasAutoStamp = $logs | Select-String "TESTNET_STAMP.*Auto-stamping"

Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

if ($hasNotRequired -and !$hasAutoStamp) {
    Test-Result "Skip stamping on restart" $true "Correctly detected existing blocks"
} else {
    Test-Result "Skip stamping on restart" $false "Should skip but logs show: notRequired=$($hasNotRequired -ne $null), autoStamp=$($hasAutoStamp -ne $null)"
}

# ============================================================================
# TEST 5: Regular miner (should NOT stamp)
# ============================================================================
Write-Host "`nTEST 5: Regular testnet miner (should NOT stamp)" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

Cleanup-TestData
New-Item -ItemType Directory -Path $testDataDir -Force | Out-Null

$env:VISION_NETWORK = "testnet"
$env:VISION_IS_TESTNET_SEED = "false"  # Regular miner
$env:VISION_AUTO_STAMP_TESTNET = "false"
$env:VISION_MINER_ADDRESS = "land1miner"
$env:VISION_DATA_DIR = $testDataDir
$env:VISION_PORT = "17071"
$env:VISION_P2P_PORT = "17073"

Write-Host "   Starting regular miner (5 second startup)..." -ForegroundColor Gray
Remove-Item "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
Remove-Item "$testDataDir\seed-stderr.log" -ErrorAction SilentlyContinue

$process = Start-Process -FilePath $binary -NoNewWindow -PassThru -RedirectStandardOutput "$testDataDir\seed-stdout.log" -RedirectStandardError "$testDataDir\seed-stderr.log"
Start-Sleep -Seconds 5

$logs = Get-Content "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
$hasRegularNode = $logs | Select-String "TESTNET_STAMP.*Regular.*node"
$hasAutoStamp = $logs | Select-String "TESTNET_STAMP.*Auto-stamping"

Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

if ($hasRegularNode -and !$hasAutoStamp) {
    Test-Result "Regular miner does NOT stamp" $true "Correctly identified as regular node"
} else {
    Test-Result "Regular miner does NOT stamp" $false "Should not stamp but logs show: regular=$($hasRegularNode -ne $null), autoStamp=$($hasAutoStamp -ne $null)"
}

# ============================================================================
# TEST 6: Mainnet isolation (Guardian logic unchanged)
# ============================================================================
Write-Host "`nTEST 6: Mainnet isolation verification" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────────────" -ForegroundColor Gray

Cleanup-TestData
New-Item -ItemType Directory -Path $testDataDir -Force | Out-Null

$env:VISION_NETWORK = "mainnet-full"
$env:VISION_IS_TESTNET_SEED = "true"  # Even with seed flag...
$env:VISION_AUTO_STAMP_TESTNET = "true"  # ...and auto-stamp flag...
$env:VISION_MINER_ADDRESS = "land1mainnet"
$env:VISION_DATA_DIR = $testDataDir
$env:VISION_PORT = "17072"
$env:VISION_P2P_PORT = "17074"

Write-Host "   Starting mainnet node (seed flags should be ignored)..." -ForegroundColor Gray
Remove-Item "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
Remove-Item "$testDataDir\seed-stderr.log" -ErrorAction SilentlyContinue

$process = Start-Process -FilePath $binary -NoNewWindow -PassThru -RedirectStandardOutput "$testDataDir\seed-stdout.log" -RedirectStandardError "$testDataDir\seed-stderr.log"
Start-Sleep -Seconds 5

$logs = Get-Content "$testDataDir\seed-stdout.log" -ErrorAction SilentlyContinue
$hasTestnetStamp = $logs | Select-String "TESTNET_STAMP"

Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

if (!$hasTestnetStamp) {
    Test-Result "Mainnet isolation" $true "No TESTNET_STAMP logs on mainnet-full (correct)"
} else {
    Test-Result "Mainnet isolation" $false "Found TESTNET_STAMP logs on mainnet - isolation broken!"
}

# ============================================================================
# RESULTS SUMMARY
# ============================================================================
Write-Host "`n═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "     TEST RESULTS" -ForegroundColor White
Write-Host "═══════════════════════════════════════════════════════════════" -ForegroundColor Cyan

Write-Host "`nTests Passed: $testsPassed / $testsTotal" -ForegroundColor $(if ($testsPassed -eq $testsTotal) { "Green" } else { "Yellow" })
Write-Host "Tests Failed: $testsFailed / $testsTotal" -ForegroundColor $(if ($testsFailed -eq 0) { "Green" } else { "Red" })

if ($testsPassed -eq $testsTotal) {
    Write-Host "`n✅ ALL TESTS PASSED - READY FOR DEPLOYMENT! 🚀`n" -ForegroundColor Green
    
    Write-Host "Deployment Checklist:" -ForegroundColor Cyan
    Write-Host "  ✅ Code compiles successfully" -ForegroundColor Green
    Write-Host "  ✅ Auto-stamp works on testnet seeds" -ForegroundColor Green
    Write-Host "  ✅ Blocks persist correctly" -ForegroundColor Green
    Write-Host "  ✅ Restart detection works" -ForegroundColor Green
    Write-Host "  ✅ Regular miners don't stamp" -ForegroundColor Green
    Write-Host "  ✅ Mainnet isolation verified" -ForegroundColor Green
    
    Write-Host "`nNext Steps:" -ForegroundColor Yellow
    Write-Host "  1. Package binary for deployment" -ForegroundColor White
    Write-Host "  2. Deploy to 3 testnet seed nodes with:" -ForegroundColor White
    Write-Host "     VISION_NETWORK=testnet" -ForegroundColor Gray
    Write-Host "     VISION_IS_TESTNET_SEED=true" -ForegroundColor Gray
    Write-Host "     VISION_AUTO_STAMP_TESTNET=true" -ForegroundColor Gray
    Write-Host "  3. Start seeds simultaneously" -ForegroundColor White
    Write-Host "  4. Verify all 3 stamp matching blocks 1-3" -ForegroundColor White
    Write-Host "  5. Deploy to miners (no seed flags)" -ForegroundColor White
    Write-Host "  6. Monitor sync and mining`n" -ForegroundColor White
    
    $exitCode = 0
} else {
    Write-Host "`n❌ SOME TESTS FAILED - REVIEW LOGS BEFORE DEPLOYMENT`n" -ForegroundColor Red
    Write-Host "Log files in: $testDataDir" -ForegroundColor Yellow
    Write-Host "  - seed-stdout.log" -ForegroundColor Gray
    Write-Host "  - seed-stderr.log`n" -ForegroundColor Gray
    $exitCode = 1
}

# Cleanup
Cleanup-TestData

Write-Host "Test data cleaned up`n" -ForegroundColor Gray
exit $exitCode
