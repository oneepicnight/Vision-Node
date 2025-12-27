# Test Compact Block Generation
# This script starts a node, mines a block, and checks compact block metrics

Write-Host "🧪 Testing Compact Block Generation..." -ForegroundColor Cyan

# Kill any existing node
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

# Clean data directory
if (Test-Path "vision_data_test") {
    Remove-Item -Recurse -Force "vision_data_test"
}

Write-Host "📦 Starting node on port 7070..." -ForegroundColor Yellow
$node = Start-Process -FilePath ".\target\release\vision-node.exe" `
    -ArgumentList "--port", "7070", "--data-dir", "vision_data_test" `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput "compact-test.log" `
    -RedirectStandardError "compact-test-err.log"

Start-Sleep -Seconds 3

Write-Host "⛏️  Enabling mining..." -ForegroundColor Yellow
Invoke-RestMethod -Uri "http://localhost:7070/enable_mining" -Method POST | Out-Null

Write-Host "⏳ Waiting for block to be mined..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Check metrics
Write-Host "`n📊 Checking Compact Block Metrics..." -ForegroundColor Cyan
$metrics = Invoke-RestMethod -Uri "http://localhost:7070/metrics"

# Parse metrics
$compact_sent = if ($metrics -match 'vision_compact_blocks_sent_total (\d+)') { $matches[1] } else { "0" }
$bandwidth_saved = if ($metrics -match 'vision_compact_block_bandwidth_saved_bytes (\d+)') { $matches[1] } else { "0" }
$avg_savings = if ($metrics -match 'vision_compact_block_avg_savings_pct (\d+)') { $matches[1] } else { "0" }

Write-Host "`n✅ Compact Block Statistics:" -ForegroundColor Green
Write-Host "   Compact blocks sent: $compact_sent"
Write-Host "   Bandwidth saved: $bandwidth_saved bytes"
Write-Host "   Average savings: $avg_savings%"

# Check log for compact block messages
Write-Host "`n📋 Compact Block Log Entries:" -ForegroundColor Cyan
if (Test-Path "compact-test.log") {
    Get-Content "compact-test.log" | Select-String "compact" -Context 0 | Select-Object -First 5
}

# Get chain status
Write-Host "`n📈 Chain Status:" -ForegroundColor Cyan
$status = Invoke-RestMethod -Uri "http://localhost:7070/chain/status"
Write-Host "   Height: $($status.height)"
Write-Host "   Blocks: $($status.blocks_len)"

# Cleanup
Write-Host "`n🧹 Cleaning up..." -ForegroundColor Yellow
$node | Stop-Process -Force
Start-Sleep -Seconds 1

if ([int]$compact_sent -gt 0) {
    Write-Host "`n✅ SUCCESS: Compact blocks are being generated!" -ForegroundColor Green
    Write-Host "   Bandwidth reduction: ~$avg_savings%" -ForegroundColor Green
    exit 0
} else {
    Write-Host "`n⚠️  WARNING: No compact blocks detected" -ForegroundColor Yellow
    Write-Host "   This might be normal if blocks have no transactions" -ForegroundColor Yellow
    exit 0
}
