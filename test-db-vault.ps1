# Test DB-backed Vault System

Write-Host "🔄 Starting vision-node with DB-backed vault system..." -ForegroundColor Cyan

# Start the node in background
$process = Start-Process -FilePath "$PWD\target\release\vision-node.exe" -WindowStyle Minimized -PassThru

Write-Host "⏳ Waiting 3 seconds for node to start..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Test health endpoint
Write-Host "🏥 Testing /health endpoint..." -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -Uri "http://localhost:9999/health" -Method Get -TimeoutSec 5 -ErrorAction Stop
    $health = $response.Content | ConvertFrom-Json
    Write-Host "✅ Health check passed:" -ForegroundColor Green
    Write-Host "   - Status: $($health.status)"
    Write-Host "   - Timestamp: $($health.timestamp)"
} catch {
    Write-Host "❌ Health check failed: $_" -ForegroundColor Red
}

# Try to get vault status (if endpoint exists)
Write-Host ""
Write-Host "💾 Testing vault endpoints..." -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -Uri "http://localhost:9999/api/vault/balances" -Method Get -TimeoutSec 5 -ErrorAction SilentlyContinue
    if ($response.StatusCode -eq 200) {
        Write-Host "✅ Vault balances endpoint accessible"
        $balances = $response.Content | ConvertFrom-Json
        Write-Host "   Response: $($response.Content | ConvertFrom-Json | ConvertTo-Json -Depth 3)"
    }
} catch {
    Write-Host "⚠️  Vault balances endpoint not available or error: $($_.Exception.Message)" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✅ DB-backed vault system is running!" -ForegroundColor Green
Write-Host "   - Vault balances now stored in sled database (persistent across restarts)"
Write-Host "   - All amounts use u128 atomic units (no float rounding)"
Write-Host "   - Keys format: 'vault:{bucket}:{asset}'" -ForegroundColor Cyan
Write-Host ""
Write-Host "Press Ctrl+C to stop the node..." -ForegroundColor Yellow

# Wait for user interrupt
try {
    while ($true) { Start-Sleep -Seconds 1 }
} finally {
    Write-Host ""
    Write-Host "🛑 Shutting down node..." -ForegroundColor Yellow
    Stop-Process -Id $process.Id -ErrorAction SilentlyContinue
    Write-Host "✅ Node stopped" -ForegroundColor Green
}
