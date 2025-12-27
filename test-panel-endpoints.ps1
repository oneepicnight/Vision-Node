# Test Panel.html API Endpoints
# Verifies all endpoints called by panel.html are working correctly

Write-Host "`n" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  PANEL.HTML ENDPOINT VERIFICATION" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$baseUrl = "http://127.0.0.1:7070"
$passCount = 0
$failCount = 0
$errors = @()

function Test-Endpoint {
    param(
        [string]$Name,
        [string]$Method = "GET",
        [string]$Path,
        [hashtable]$Body = $null,
        [bool]$ExpectJson = $true
    )
    
    $url = "$baseUrl$Path"
    Write-Host "  Testing: $Name" -NoNewline
    Write-Host " ... " -NoNewline -ForegroundColor Gray
    
    try {
        if ($Method -eq "GET") {
            $response = Invoke-WebRequest -Uri $url -Method GET -UseBasicParsing -ErrorAction Stop
        } else {
            $jsonBody = $Body | ConvertTo-Json -Depth 10
            $response = Invoke-WebRequest -Uri $url -Method $Method -Body $jsonBody -ContentType "application/json" -UseBasicParsing -ErrorAction Stop
        }
        
        if ($response.StatusCode -eq 200) {
            if ($ExpectJson) {
                $json = $response.Content | ConvertFrom-Json
                Write-Host "✅ PASS" -ForegroundColor Green
                $script:passCount++
                return $json
            } else {
                Write-Host "✅ PASS" -ForegroundColor Green
                $script:passCount++
                return $response.Content
            }
        } else {
            Write-Host "❌ FAIL (HTTP $($response.StatusCode))" -ForegroundColor Red
            $script:failCount++
            $script:errors += "$Name - HTTP $($response.StatusCode)"
            return $null
        }
    } catch {
        Write-Host "❌ FAIL ($($_.Exception.Message))" -ForegroundColor Red
        $script:failCount++
        $script:errors += "$Name - $($_.Exception.Message)"
        return $null
    }
}

Write-Host "🔍 Testing Core API Endpoints:" -ForegroundColor Cyan
Write-Host ""

# Core Status Endpoints (called by panel.html)
$status = Test-Endpoint -Name "Node Status" -Path "/api/status"
$supply = Test-Endpoint -Name "Supply" -Path "/api/supply" -ExpectJson $false
$mempoolSize = Test-Endpoint -Name "Mempool Size" -Path "/api/mempool_size" -ExpectJson $false

Write-Host ""
Write-Host "⚡ Testing Miner API Endpoints:" -ForegroundColor Cyan
Write-Host ""

# Miner Endpoints (called by panel.html initMinerControls and polling)
$minerStatus = Test-Endpoint -Name "Miner Status" -Path "/api/miner/status"
$minerWallet = Test-Endpoint -Name "Miner Wallet (GET)" -Path "/api/miner/wallet"

# Test wallet configuration
# $walletSet = Test-Endpoint -Name "Miner Wallet (POST)" -Method "POST" -Path "/api/miner/wallet" -Body @{ wallet = "test_wallet_123" }

Write-Host ""
Write-Host "🌐 Testing Constellation Endpoints:" -ForegroundColor Cyan
Write-Host ""

# Constellation Endpoints (called by panel.html fetchConstellationStatus and fetchPeers)
$constStatus = Test-Endpoint -Name "Constellation Status" -Path "/constellation/status"
$constPeers = Test-Endpoint -Name "Constellation Peers" -Path "/api/constellation/peers"

Write-Host ""
Write-Host "📊 Testing Mining Info Endpoints:" -ForegroundColor Cyan
Write-Host ""

# Mining Info (optional, used by some panels)
$miningInfo = Test-Endpoint -Name "Mining Info" -Path "/api/mining/info"
$miningStatus = Test-Endpoint -Name "Mining Status" -Path "/mining/status"
$miningLeaderboard = Test-Endpoint -Name "Mining Leaderboard" -Path "/mining/leaderboard"

Write-Host ""
Write-Host "🎯 Testing Beacon Endpoints:" -ForegroundColor Cyan
Write-Host ""

# Beacon Endpoints (called by some panels)
$beaconStatus = Test-Endpoint -Name "Beacon Status" -Path "/api/beacon/status"
$beaconHealth = Test-Endpoint -Name "Beacon Health" -Path "/api/beacon/health"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  TEST RESULTS" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "  Total Tests: $($passCount + $failCount)" -ForegroundColor White
Write-Host "  ✅ Passed: $passCount" -ForegroundColor Green
Write-Host "  ❌ Failed: $failCount" -ForegroundColor Red
Write-Host ""

if ($failCount -gt 0) {
    Write-Host "Failed Endpoints:" -ForegroundColor Yellow
    foreach ($error in $errors) {
        Write-Host "   - $error" -ForegroundColor Red
    }
    Write-Host ""
}

# Display key data samples
if ($status) {
    Write-Host "📊 Sample Data:" -ForegroundColor Cyan
    Write-Host "   Height: $($status.height)" -ForegroundColor White
    Write-Host "   Peers: $($status.peers.Length)" -ForegroundColor White
    Write-Host "   Mempool: $($status.mempool)" -ForegroundColor White
}

if ($constPeers) {
    $inCount = $constPeers.inbound
    $outCount = $constPeers.outbound
    $peerMsg = "   Active Peers: $($constPeers.active) - $inCount inbound, $outCount outbound"
    Write-Host $peerMsg -ForegroundColor White
}

if ($minerStatus) {
    Write-Host "   Miner Threads: $($minerStatus.threads)" -ForegroundColor White
    Write-Host "   Miner Enabled: $($minerStatus.enabled)" -ForegroundColor White
    Write-Host "   Hashrate: $($minerStatus.hashrate) H/s" -ForegroundColor White
}

Write-Host ""

if ($failCount -eq 0) {
    Write-Host "🎉 ALL ENDPOINTS WORKING!" -ForegroundColor Green
    Write-Host "   Panel.html is fully operational" -ForegroundColor Green
    Write-Host ""
    exit 0
} else {
    Write-Host "⚠️  SOME ENDPOINTS FAILED" -ForegroundColor Yellow
    Write-Host "   Check errors above and verify Guardian is running" -ForegroundColor Yellow
    Write-Host ""
    exit 1
}
