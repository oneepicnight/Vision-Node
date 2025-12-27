# Test script for Vision Peer Book API endpoints

Write-Host "🧪 Testing Vision Peer Book API Endpoints" -ForegroundColor Cyan
Write-Host "========================================`n"

Start-Sleep -Seconds 5

# Test 1: GET /api/peers/trusted
Write-Host "📊 Test 1: GET /api/peers/trusted" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://localhost:7070/api/peers/trusted" -Method Get -ErrorAction Stop
    Write-Host "✅ Success!" -ForegroundColor Green
    Write-Host "Response:" -ForegroundColor White
    $response | ConvertTo-Json -Depth 3
    Write-Host "`nTotal trusted peers: $($response.count)" -ForegroundColor Cyan
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}

Write-Host "`n----------------------------------------`n"

# Test 2: GET /api/peers/moods
Write-Host "📊 Test 2: GET /api/peers/moods" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "http://localhost:7070/api/peers/moods" -Method Get -ErrorAction Stop
    Write-Host "✅ Success!" -ForegroundColor Green
    Write-Host "Response:" -ForegroundColor White
    $response | ConvertTo-Json -Depth 3
    Write-Host "`nTotal peers with mood: $($response.total)" -ForegroundColor Cyan
    Write-Host "Distribution:" -ForegroundColor Cyan
    Write-Host "  - Calm: $($response.distribution.calm)" -ForegroundColor Green
    Write-Host "  - Warning: $($response.distribution.warning)" -ForegroundColor Yellow
    Write-Host "  - Storm: $($response.distribution.storm)" -ForegroundColor Magenta
    Write-Host "  - Wounded: $($response.distribution.wounded)" -ForegroundColor Red
    Write-Host "  - Celebration: $($response.distribution.celebration)" -ForegroundColor Cyan
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "✅ All tests complete!" -ForegroundColor Green
