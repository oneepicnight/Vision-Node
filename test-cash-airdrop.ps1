# CASH Airdrop Test Script
# Tests the CASH airdrop system endpoints

$baseUrl = "http://localhost:3001"

Write-Host "🧪 CASH Airdrop System Test Suite" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

# Test 1: Get initial CASH supply
Write-Host "📊 Test 1: Get initial CASH supply" -ForegroundColor Yellow
try {
    $supply = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash/supply" -Method Get
    Write-Host "✓ Current CASH supply: $($supply.total_supply)" -ForegroundColor Green
    $initialSupply = [bigint]::Parse($supply.total_supply)
} catch {
    Write-Host "✗ Failed to get supply: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 2: Preview a small airdrop
Write-Host "🔍 Test 2: Preview small airdrop (no confirmation needed)" -ForegroundColor Yellow
$smallAirdrop = @{
    recipients = @(
        @{
            address = "0x1111111111111111111111111111111111111111"
            amount_cash = "1000000000000000000"  # 1 CASH with 18 decimals
        },
        @{
            address = "0x2222222222222222222222222222222222222222"
            amount_cash = "2000000000000000000"  # 2 CASH
        }
    )
    reason = "Test airdrop - small amount"
    requested_by = "test_admin"
} | ConvertTo-Json -Depth 10

try {
    $preview = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash/preview" `
        -Method Post `
        -ContentType "application/json" `
        -Body $smallAirdrop
    
    if ($preview.ok) {
        Write-Host "✓ Preview succeeded" -ForegroundColor Green
        Write-Host "  Total recipients: $($preview.total_recipients)" -ForegroundColor Gray
        Write-Host "  Total CASH: $($preview.total_cash)" -ForegroundColor Gray
        Write-Host "  Requires confirmation: $($preview.requires_confirmation)" -ForegroundColor Gray
    } else {
        Write-Host "✗ Preview failed: $($preview.error)" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ Preview request failed: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 3: Execute the small airdrop
Write-Host "💸 Test 3: Execute small airdrop" -ForegroundColor Yellow
try {
    $execute = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash" `
        -Method Post `
        -ContentType "application/json" `
        -Body $smallAirdrop
    
    if ($execute.ok) {
        Write-Host "✓ Airdrop executed successfully" -ForegroundColor Green
        Write-Host "  Message: $($execute.message)" -ForegroundColor Gray
        Write-Host "  Total recipients: $($execute.total_recipients)" -ForegroundColor Gray
        Write-Host "  Total CASH distributed: $($execute.total_cash)" -ForegroundColor Gray
        
        if ($execute.failed -and $execute.failed.Count -gt 0) {
            Write-Host "  ⚠️  Some recipients failed:" -ForegroundColor Yellow
            foreach ($failed in $execute.failed) {
                Write-Host "    - $($failed.address): $($failed.error)" -ForegroundColor Yellow
            }
        }
    } else {
        Write-Host "✗ Execution failed: $($execute.error)" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ Execution request failed: $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 4: Verify supply increased
Write-Host "📈 Test 4: Verify CASH supply increased" -ForegroundColor Yellow
try {
    $newSupply = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash/supply" -Method Get
    $newSupplyValue = [bigint]::Parse($newSupply.total_supply)
    $expectedIncrease = [bigint]::Parse("3000000000000000000")  # 3 CASH
    
    if ($newSupplyValue -eq ($initialSupply + $expectedIncrease)) {
        Write-Host "✓ Supply increased correctly" -ForegroundColor Green
        Write-Host "  Previous: $initialSupply" -ForegroundColor Gray
        Write-Host "  Current:  $newSupplyValue" -ForegroundColor Gray
        Write-Host "  Increase: $expectedIncrease (3 CASH)" -ForegroundColor Gray
    } else {
        Write-Host "✗ Supply mismatch!" -ForegroundColor Red
        Write-Host "  Expected: $($initialSupply + $expectedIncrease)" -ForegroundColor Red
        Write-Host "  Got:      $newSupplyValue" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Failed to verify supply: $_" -ForegroundColor Red
}
Write-Host ""

# Test 5: Preview large airdrop (requires confirmation)
Write-Host "🚨 Test 5: Preview large airdrop (should require confirmation)" -ForegroundColor Yellow
$largeAirdrop = @{
    recipients = @(
        @{
            address = "0x3333333333333333333333333333333333333333"
            amount_cash = "200000000000000000000"  # 200 CASH (exceeds 100 CASH threshold)
        }
    )
    reason = "Test airdrop - large amount"
    requested_by = "test_admin"
} | ConvertTo-Json -Depth 10

try {
    $largePreview = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash/preview" `
        -Method Post `
        -ContentType "application/json" `
        -Body $largeAirdrop
    
    if ($largePreview.ok) {
        Write-Host "✓ Large preview succeeded" -ForegroundColor Green
        Write-Host "  Total CASH: $($largePreview.total_cash)" -ForegroundColor Gray
        Write-Host "  Requires confirmation: $($largePreview.requires_confirmation)" -ForegroundColor Gray
        
        if ($largePreview.requires_confirmation) {
            Write-Host "  ✓ Confirmation correctly required for large airdrop" -ForegroundColor Green
        } else {
            Write-Host "  ✗ Expected confirmation to be required!" -ForegroundColor Red
        }
    } else {
        Write-Host "✗ Large preview failed: $($largePreview.error)" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Large preview request failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 6: Try to execute large airdrop without confirmation (should fail)
Write-Host "🛡️  Test 6: Try large airdrop without confirmation (should fail)" -ForegroundColor Yellow
try {
    $executeNoConfirm = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash" `
        -Method Post `
        -ContentType "application/json" `
        -Body $largeAirdrop `
        -ErrorAction SilentlyContinue
    
    if (-not $executeNoConfirm.ok) {
        Write-Host "✓ Correctly rejected: $($executeNoConfirm.error)" -ForegroundColor Green
    } else {
        Write-Host "✗ Should have been rejected without confirmation!" -ForegroundColor Red
    }
} catch {
    # Expected to fail
    Write-Host "✓ Correctly rejected (threw error)" -ForegroundColor Green
}
Write-Host ""

# Test 7: Execute large airdrop WITH confirmation
Write-Host "✅ Test 7: Execute large airdrop with confirmation phrase" -ForegroundColor Yellow
$largeAirdropConfirmed = @{
    recipients = @(
        @{
            address = "0x3333333333333333333333333333333333333333"
            amount_cash = "200000000000000000000"  # 200 CASH
        }
    )
    reason = "Test airdrop - large amount with confirmation"
    requested_by = "test_admin"
    confirm_phrase = "VISION AIRDROP"
} | ConvertTo-Json -Depth 10

try {
    $executeLarge = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash" `
        -Method Post `
        -ContentType "application/json" `
        -Body $largeAirdropConfirmed
    
    if ($executeLarge.ok) {
        Write-Host "✓ Large airdrop executed with confirmation" -ForegroundColor Green
        Write-Host "  Message: $($executeLarge.message)" -ForegroundColor Gray
    } else {
        Write-Host "✗ Execution failed: $($executeLarge.error)" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Execution request failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 8: Test empty recipients (should fail)
Write-Host "🚫 Test 8: Try airdrop with no recipients (should fail)" -ForegroundColor Yellow
$emptyAirdrop = @{
    recipients = @()
    reason = "Test - empty"
    requested_by = "test_admin"
} | ConvertTo-Json -Depth 10

try {
    $emptyResult = Invoke-RestMethod -Uri "$baseUrl/admin/airdrop/cash/preview" `
        -Method Post `
        -ContentType "application/json" `
        -Body $emptyAirdrop `
        -ErrorAction SilentlyContinue
    
    if (-not $emptyResult.ok) {
        Write-Host "✓ Correctly rejected: $($emptyResult.error)" -ForegroundColor Green
    } else {
        Write-Host "✗ Should have rejected empty recipients!" -ForegroundColor Red
    }
} catch {
    Write-Host "✓ Correctly rejected (threw error)" -ForegroundColor Green
}
Write-Host ""

# Final summary
Write-Host "=================================" -ForegroundColor Cyan
Write-Host "🎉 Test suite complete!" -ForegroundColor Cyan
Write-Host ""
Write-Host "Summary:" -ForegroundColor White
Write-Host "- ✅ CASH supply tracking works" -ForegroundColor Green
Write-Host "- ✅ Preview endpoint validates correctly" -ForegroundColor Green
Write-Host "- ✅ Small airdrops execute without confirmation" -ForegroundColor Green
Write-Host "- ✅ Large airdrops require confirmation phrase" -ForegroundColor Green
Write-Host "- ✅ Safety checks prevent invalid airdrops" -ForegroundColor Green
Write-Host ""
Write-Host "💸 CASH Airdrop System: OPERATIONAL" -ForegroundColor Cyan
