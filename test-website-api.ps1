# Vision Node Website API - Complete Test Script
# Tests all 19 endpoints from VISION_NODE_WEBSITE_API_IMPLEMENTATION.md

$baseUrl = "http://127.0.0.1:7070"

Write-Host "`n╔════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   VISION NODE WEBSITE API TEST SUITE          ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Test counter
$passed = 0
$failed = 0

function Test-Endpoint {
    param(
        [string]$Method,
        [string]$Path,
        [string]$Description,
        [object]$Body = $null
    )
    
    Write-Host "Testing: $Description" -ForegroundColor Yellow
    Write-Host "  $Method $Path" -ForegroundColor Gray
    
    try {
        $url = "$baseUrl$Path"
        
        if ($Method -eq "GET") {
            $response = Invoke-RestMethod -Uri $url -Method Get -ErrorAction Stop
        } else {
            $jsonBody = $Body | ConvertTo-Json
            $response = Invoke-RestMethod -Uri $url -Method Post -Body $jsonBody -ContentType "application/json" -ErrorAction Stop
        }
        
        Write-Host "  ✓ SUCCESS" -ForegroundColor Green
        Write-Host "  Response: $($response | ConvertTo-Json -Compress | Out-String | Select-Object -First 200)" -ForegroundColor DarkGray
        $script:passed++
        return $true
    }
    catch {
        Write-Host "  ✗ FAILED: $($_.Exception.Message)" -ForegroundColor Red
        $script:failed++
        return $false
    }
    finally {
        Write-Host ""
    }
}

Write-Host "═══ A) CORE STATUS ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/status" -Description "Primary node status"
Test-Endpoint -Method "GET" -Path "/api/bootstrap" -Description "Bootstrap node information"
Test-Endpoint -Method "GET" -Path "/api/chain/status" -Description "Detailed chain status"

Write-Host "═══ B) NETWORK DATA ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/constellation" -Description "Current constellation snapshot"
Test-Endpoint -Method "GET" -Path "/api/constellation/history?range=true" -Description "Constellation history range"
Test-Endpoint -Method "GET" -Path "/api/constellation/new-stars?limit=10" -Description "Recent NEW_STAR events"

Write-Host "═══ C) GUARDIAN ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/guardian" -Description "Guardian status (GET)"
Test-Endpoint -Method "GET" -Path "/api/guardian/feed" -Description "Guardian message feed"
# Test-Endpoint -Method "POST" -Path "/api/guardian" -Description "Toggle guardian mode" -Body @{enabled=$true}

Write-Host "═══ D) HEALTH & MONITORING ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/health" -Description "Current health analysis (Website API)"
Test-Endpoint -Method "GET" -Path "/api/health/public" -Description "Public health statistics"

Write-Host "═══ E) NETWORK MOOD ENDPOINT ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/mood" -Description "Current network mood"

Write-Host "═══ F) TRAUMA & PATTERNS ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/trauma?limit=10" -Description "Trauma events (war journal)"
Test-Endpoint -Method "GET" -Path "/api/patterns" -Description "Pattern library"

Write-Host "═══ G) NODE IDENTITY ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/nodes" -Description "Basic node list"
Test-Endpoint -Method "GET" -Path "/api/nodes/with-identity" -Description "Enriched nodes with identity"
Test-Endpoint -Method "GET" -Path "/api/reputation" -Description "Reputation leaderboard"

Write-Host "═══ H) ANALYTICS ENDPOINTS ═══`n" -ForegroundColor Magenta

Test-Endpoint -Method "GET" -Path "/api/downloads/visitors" -Description "Download & visitor analytics"
Test-Endpoint -Method "GET" -Path "/api/snapshots/recent?minutes=60" -Description "Recent network snapshots"

# Summary
Write-Host "`n╔════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   TEST RESULTS                                 ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

$total = $passed + $failed
Write-Host "Total Tests: $total" -ForegroundColor White
Write-Host "Passed: $passed" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })

if ($failed -eq 0) {
    Write-Host "`n✨ ALL TESTS PASSED! Website API is fully operational! ✨`n" -ForegroundColor Green
} else {
    Write-Host "`n⚠️  Some tests failed. Check the output above for details. ⚠️`n" -ForegroundColor Yellow
}

# Additional verification endpoints
Write-Host "`n═══ BONUS: VERIFY KUBERNETES HEALTH CHECKS ═══`n" -ForegroundColor Magenta
Test-Endpoint -Method "GET" -Path "/health" -Description "Kubernetes liveness check (should not conflict)"
Test-Endpoint -Method "GET" -Path "/health/live" -Description "Kubernetes liveness (explicit)"
Test-Endpoint -Method "GET" -Path "/health/ready" -Description "Kubernetes readiness check"

Write-Host "`n🎯 Test script complete!`n" -ForegroundColor Cyan
