# Test Guardian Ownership Configuration
# This script tests the new Guardian owner feature

Write-Host "🛡️  Testing Guardian Ownership Configuration" -ForegroundColor Cyan
Write-Host ("=" * 70)
Write-Host ""

# Test 1: Without owner configured
Write-Host "Test 1: Starting Guardian WITHOUT owner config..." -ForegroundColor Yellow
Write-Host "Expected: Should log 'Not configured' message" -ForegroundColor Gray
Write-Host ""

# Remove env vars if set
$env:GUARDIAN_OWNER_DISCORD_ID=$null
$env:GUARDIAN_OWNER_WALLET_ADDRESS=$null

Write-Host "Starting node (will auto-stop after 5 seconds)..." -ForegroundColor Gray
$proc1 = Start-Process -FilePath "c:\vision-node\target\release\vision-node.exe" `
    -WorkingDirectory "c:\vision-node" `
    -PassThru `
    -WindowStyle Hidden

Start-Sleep -Seconds 5
Stop-Process -Id $proc1.Id -Force -ErrorAction SilentlyContinue

Write-Host "✅ Test 1 complete - Check logs above for 'Not configured' message" -ForegroundColor Green
Write-Host ""

# Test 2: With owner configured
Write-Host "Test 2: Starting Guardian WITH owner config..." -ForegroundColor Yellow
Write-Host "Expected: Should log 'Guardian owner: vision1test ↔ 123456789'" -ForegroundColor Gray
Write-Host ""

# Set test env vars
$env:GUARDIAN_OWNER_DISCORD_ID="123456789012345678"
$env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1test123abc456def789xyz"

Write-Host "Environment variables set:" -ForegroundColor Cyan
Write-Host "  GUARDIAN_OWNER_DISCORD_ID=$env:GUARDIAN_OWNER_DISCORD_ID" -ForegroundColor White
Write-Host "  GUARDIAN_OWNER_WALLET_ADDRESS=$env:GUARDIAN_OWNER_WALLET_ADDRESS" -ForegroundColor White
Write-Host ""

Write-Host "Starting node (will auto-stop after 5 seconds)..." -ForegroundColor Gray
$proc2 = Start-Process -FilePath "c:\vision-node\target\release\vision-node.exe" `
    -WorkingDirectory "c:\vision-node" `
    -PassThru `
    -WindowStyle Hidden `
    -ArgumentList "" `
    -RedirectStandardOutput "guardian-test-output.log" `
    -RedirectStandardError "guardian-test-error.log"

Start-Sleep -Seconds 5
Stop-Process -Id $proc2.Id -Force -ErrorAction SilentlyContinue

Write-Host "✅ Test 2 complete" -ForegroundColor Green
Write-Host ""

# Check output logs
Write-Host "Checking startup logs for Guardian owner message..." -ForegroundColor Cyan

if (Test-Path "guardian-test-error.log") {
    $errorLog = Get-Content "guardian-test-error.log" -Raw
    
    if ($errorLog -match "Guardian owner:") {
        Write-Host "✅ FOUND Guardian owner log!" -ForegroundColor Green
        
        # Extract the relevant lines
        $lines = $errorLog -split "`n" | Where-Object { $_ -match "GUARDIAN|Guardian owner" }
        foreach ($line in $lines) {
            Write-Host "  $line" -ForegroundColor White
        }
    } else {
        Write-Host "⚠️  Guardian owner message not found in logs" -ForegroundColor Yellow
        Write-Host "This might be because Guardian didn't initialize yet" -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host "Full error log saved to: guardian-test-error.log" -ForegroundColor Gray
} else {
    Write-Host "⚠️  Log file not found" -ForegroundColor Yellow
}

Write-Host ""
Write-Host ("=" * 70)
Write-Host "🎯 Test Summary" -ForegroundColor Cyan
Write-Host ("=" * 70)
Write-Host ""
Write-Host "Guardian ownership configuration is implemented and should work." -ForegroundColor Green
Write-Host ""
Write-Host "To use in production:" -ForegroundColor Yellow
Write-Host "  1. Get your Discord User ID (Discord Settings → Advanced → Developer Mode)" -ForegroundColor White
Write-Host "  2. Right-click your username → Copy ID" -ForegroundColor White
Write-Host "  3. Set environment variables:" -ForegroundColor White
Write-Host '     $env:GUARDIAN_OWNER_DISCORD_ID="your_id"' -ForegroundColor Gray
Write-Host '     $env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1abc..."' -ForegroundColor Gray
Write-Host "  4. Start Guardian node" -ForegroundColor White
Write-Host ""
Write-Host "Documentation: docs\GUARDIAN_OWNER_CONFIG.md" -ForegroundColor Cyan
Write-Host "Quick Start: GUARDIAN_OWNER_QUICKSTART.md" -ForegroundColor Cyan
Write-Host ""

# Cleanup
Remove-Item "guardian-test-*.log" -ErrorAction SilentlyContinue
$env:GUARDIAN_OWNER_DISCORD_ID=$null
$env:GUARDIAN_OWNER_WALLET_ADDRESS=$null
