# Test what endpoints actually return
param(
    [int]$Port = 7070
)

Write-Host "=== Testing Endpoints on port $Port ===" -ForegroundColor Cyan

# Test /api/status
Write-Host "`n[1] Testing /api/status" -ForegroundColor Yellow
try {
    $raw = curl.exe -s "http://localhost:$Port/api/status"
    Write-Host "Raw response:" -ForegroundColor Gray
    Write-Host $raw | Out-String
    
    $json = $raw | ConvertFrom-Json
    Write-Host "Parsed JSON properties:" -ForegroundColor Green
    $json | Get-Member -MemberType NoteProperty | ForEach-Object { Write-Host "  - $($_.Name): $($json.($_.Name))" }
} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
}

# Test /api/health
Write-Host "`n[2] Testing /api/health" -ForegroundColor Yellow
try {
    $raw = curl.exe -s "http://localhost:$Port/api/health"
    Write-Host "Raw response:" -ForegroundColor Gray
    Write-Host $raw | Out-String
} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
}

# Test /api/miner/status
Write-Host "`n[3] Testing /api/miner/status" -ForegroundColor Yellow
try {
    $raw = curl.exe -s "http://localhost:$Port/api/miner/status"
    Write-Host "Raw response:" -ForegroundColor Gray
    Write-Host $raw | Out-String
} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
}

# Test /api/peers or similar
Write-Host "`n[4] Testing /api/peers" -ForegroundColor Yellow
try {
    $raw = curl.exe -s "http://localhost:$Port/api/peers"
    Write-Host "Raw response:" -ForegroundColor Gray
    Write-Host $raw | Out-String
} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
}

Write-Host "`nDone testing endpoints" -ForegroundColor Cyan
