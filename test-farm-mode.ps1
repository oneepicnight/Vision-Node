# Farm Mode V2 - Test Script
# Run this after the node has started successfully

Write-Host "================================" -ForegroundColor Cyan
Write-Host "Farm Mode V2 - API Test Script" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://localhost:7070/api"

# Test 1: Check mining info (public endpoint)
Write-Host "Test 1: GET /api/mining/info" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/mining/info" -Method Get
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: Get runtime modes status
Write-Host "Test 2: GET /api/admin/modes" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/modes" -Method Get
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 3: Enable Farm Mode
Write-Host "Test 3: POST /api/admin/modes/farm/enable" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/modes/farm/enable" -Method Post -ContentType "application/json" -Body "{}"
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 4: Get farm rigs (should be empty initially)
Write-Host "Test 4: GET /api/admin/farm/rigs" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/farm/rigs" -Method Get
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 5: Configure mining endpoints
Write-Host "Test 5: POST /api/admin/mining/endpoints" -ForegroundColor Yellow
try {
    $body = @{
        public_pool_url = "http://pool.example.com:7070"
        local_node_url = "http://localhost:7070"
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/mining/endpoints" -Method Post -ContentType "application/json" -Body $body
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 6: Get mining endpoints
Write-Host "Test 6: GET /api/admin/mining/endpoints" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/mining/endpoints" -Method Get
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 7: Try to get config for a non-existent rig (should return default)
Write-Host "Test 7: GET /api/admin/farm/rigs/test-rig-1/config" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/farm/rigs/test-rig-1/config" -Method Get
    Write-Host "✅ Success (returns default config):" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 8: Update rig config (even though rig doesn't exist yet)
Write-Host "Test 8: POST /api/admin/farm/rigs/test-rig-1/config" -ForegroundColor Yellow
try {
    $body = @{
        profile = @{
            profile_type = "Performance"
        }
        auto_restart_on_error = $true
        min_hashrate_threshold = 500000
    } | ConvertTo-Json -Depth 3
    
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/farm/rigs/test-rig-1/config" -Method Post -ContentType "application/json" -Body $body
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 9: Verify config was saved
Write-Host "Test 9: GET /api/admin/farm/rigs/test-rig-1/config (verify saved)" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$baseUrl/admin/farm/rigs/test-rig-1/config" -Method Get
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 10: Try join pool endpoint
Write-Host "Test 10: POST /api/wallet/mining/join-pool" -ForegroundColor Yellow
try {
    $body = @{
        wallet_address = "test-wallet-123"
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "$baseUrl/wallet/mining/join-pool" -Method Post -ContentType "application/json" -Body $body
    Write-Host "✅ Success:" -ForegroundColor Green
    $response | ConvertTo-Json -Depth 3
} catch {
    Write-Host "❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "================================" -ForegroundColor Cyan
Write-Host "All API Tests Complete!" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Host "1. Open browser to http://localhost:7070" -ForegroundColor White
Write-Host "2. Click '🚜 Farm' mode button" -ForegroundColor White
Write-Host "3. View the Farm dashboard (will be empty without connected rigs)" -ForegroundColor White
Write-Host "4. Connect a rig via WebSocket: ws://localhost:7070/farm/ws" -ForegroundColor White
Write-Host ""
Write-Host "WebSocket Connection Example:" -ForegroundColor Yellow
Write-Host @"
const ws = new WebSocket('ws://localhost:7070/farm/ws');
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'register',
    rig_id: 'test-rig-1',
    wallet: 'your-wallet-address',
    threads: 8
  }));
};
"@ -ForegroundColor Gray
