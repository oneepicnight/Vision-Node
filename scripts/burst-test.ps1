#!/usr/bin/env pwsh
# Burst test - send 1000 transfers as fast as possible

$ErrorActionPreference = "Stop"

Write-Host "💥 Vision Node Burst Test" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan

$baseUrl = "http://127.0.0.1:7070"
$count = 1000

Write-Host "`nSending $count transfers..." -ForegroundColor Yellow
Write-Host "Started: $(Get-Date -Format 'HH:mm:ss')" -ForegroundColor Gray

$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$errors = 0

for ($i = 1; $i -le $count; $i++) {
    try {
        $body = @{
            from = "ALICE"
            to = "BOB"
            amount = "1"
        } | ConvertTo-Json
        
        Invoke-RestMethod -Uri "$baseUrl/wallet/transfer" -Method Post -Body $body -ContentType "application/json" -TimeoutSec 5 | Out-Null
        
        if ($i % 100 -eq 0) {
            Write-Host "  Progress: $i/$count" -ForegroundColor Gray
        }
    } catch {
        $errors++
    }
}

$stopwatch.Stop()

# Summary
Write-Host "`n✅ Burst test complete!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "  Total: $count transfers" -ForegroundColor Cyan
Write-Host "  Success: $($count - $errors)" -ForegroundColor Green
Write-Host "  Errors: $errors" -ForegroundColor $(if ($errors -eq 0) { "Green" } else { "Red" })
Write-Host "  Duration: $($stopwatch.Elapsed.TotalSeconds) seconds" -ForegroundColor Cyan
Write-Host "  Rate: $([math]::Round($count / $stopwatch.Elapsed.TotalSeconds, 2)) tx/sec" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green

# Check metrics
Write-Host "`nFetching final metrics..." -ForegroundColor Yellow
$metrics = Invoke-WebRequest -Uri "$baseUrl/metrics" -Method Get
if ($metrics.Content -match 'vision_transfers_total\s+(\d+)') {
    Write-Host "  Total transfers: $($Matches[1])" -ForegroundColor Green
}
