# Test v2.7.0 New Endpoints
# Run this after starting vision-node to verify all new features

$baseUrl = "http://localhost:3030"

Write-Host "=== v2.7.0 Endpoint Tests ===" -ForegroundColor Cyan
Write-Host ""

# Test 1: /api/ready endpoint
Write-Host "1. Testing /api/ready (new endpoint)..." -ForegroundColor Yellow
try {
    $ready = Invoke-RestMethod -Uri "$baseUrl/api/ready" -Method Get -ErrorAction Stop
    Write-Host "   ✅ Status: $($ready.ready ? 'READY' : 'NOT READY')" -ForegroundColor Green
    Write-Host "   - Backbone Connected: $($ready.backbone_connected)" -ForegroundColor Gray
    Write-Host "   - Chain Synced: $($ready.chain_synced) (lag: $($ready.chain_lag))" -ForegroundColor Gray
    Write-Host "   - Website Reachable: $($ready.website_reachable)" -ForegroundColor Gray
    Write-Host "   - Node Approved: $($ready.node_approved)" -ForegroundColor Gray
    if ($ready.reasons) {
        Write-Host "   - Reasons: $($ready.reasons -join ', ')" -ForegroundColor Yellow
    }
} catch {
    Write-Host "   ❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: /api/status with new fields
Write-Host "2. Testing /api/status (extended with website fields)..." -ForegroundColor Yellow
try {
    $status = Invoke-RestMethod -Uri "$baseUrl/api/status" -Method Get -ErrorAction Stop
    Write-Host "   ✅ Status fetched successfully" -ForegroundColor Green
    Write-Host "   - Node ID: $($status.node_id)" -ForegroundColor Gray
    Write-Host "   - Pubkey Fingerprint: $($status.node_pubkey_fingerprint)" -ForegroundColor Gray
    Write-Host "   - Website Reachable: $($status.website_reachable)" -ForegroundColor Gray
    Write-Host "   - Website Registered: $($status.website_registered)" -ForegroundColor Gray
    Write-Host "   - Constellation URL: $($status.constellation_url)" -ForegroundColor Gray
} catch {
    Write-Host "   ❌ Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 3: /api/p2p/hello (without signature - should fail or debug mode)
Write-Host "3. Testing /api/p2p/hello (unsigned - should fail unless debug mode)..." -ForegroundColor Yellow
try {
    $hello = @{
        from_node_id = "test"
        ts_unix = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
        nonce_hex = -join ((1..32) | ForEach-Object { '{0:x2}' -f (Get-Random -Max 256) })
    } | ConvertTo-Json
    
    $response = Invoke-RestMethod -Uri "$baseUrl/api/p2p/hello" -Method Post -Body $hello -ContentType "application/json" -ErrorAction Stop
    Write-Host "   ⚠️ Accepted (debug mode enabled or signature check bypassed)" -ForegroundColor Yellow
    Write-Host "   - Response Node ID: $($response.node_id)" -ForegroundColor Gray
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    if ($statusCode -eq 400) {
        Write-Host "   ✅ Correctly rejected (signature verification working)" -ForegroundColor Green
    } else {
        Write-Host "   ❌ Unexpected error: $statusCode" -ForegroundColor Red
    }
}
Write-Host ""

# Test 4: Check mining eligibility (should not block)
Write-Host "4. Testing mining start (should not block)..." -ForegroundColor Yellow
try {
    $start = Invoke-RestMethod -Uri "$baseUrl/api/miner/start" -Method Post -ErrorAction Stop
    Write-Host "   ✅ Miner started (non-blocking)" -ForegroundColor Green
    Write-Host "   - Response: $($start | ConvertTo-Json -Compress)" -ForegroundColor Gray
} catch {
    Write-Host "   ⚠️ Mining not available: $_" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "=== Test Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  1. Check panel.html at http://localhost:3030/panel.html" -ForegroundColor Gray
Write-Host "  2. Verify 'Node Readiness' card appears" -ForegroundColor Gray
Write-Host "  3. Check 'View in Constellation' link in Website Integration card" -ForegroundColor Gray
Write-Host ""
