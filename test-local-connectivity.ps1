# Test Local Network Connectivity to Vision Node
# Run this script from the MINER machine to test if it can reach the public node

param(
    [string]$PublicNodeIP = "192.168.1.123",
    [int]$Port = 7070
)

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "Vision Node Local Network Connectivity Test" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Test 1: Ping
Write-Host "[Test 1] Pinging $PublicNodeIP..." -ForegroundColor Yellow
$pingResult = Test-Connection -ComputerName $PublicNodeIP -Count 2 -Quiet
if ($pingResult) {
    Write-Host "✅ PING SUCCESS - Host is reachable" -ForegroundColor Green
} else {
    Write-Host "❌ PING FAILED - Host is not reachable" -ForegroundColor Red
    Write-Host "   Check if both machines are on the same network" -ForegroundColor Gray
    exit 1
}
Write-Host ""

# Test 2: TCP Port
Write-Host "[Test 2] Testing TCP connection to ${PublicNodeIP}:${Port}..." -ForegroundColor Yellow
try {
    $tcpClient = New-Object System.Net.Sockets.TcpClient
    $connect = $tcpClient.BeginConnect($PublicNodeIP, $Port, $null, $null)
    $wait = $connect.AsyncWaitHandle.WaitOne(3000, $false)
    
    if (!$wait) {
        Write-Host "❌ CONNECTION TIMEOUT - Port $Port is not accessible" -ForegroundColor Red
        Write-Host "   The public node may not be running or firewall is blocking" -ForegroundColor Gray
        $tcpClient.Close()
        exit 1
    } else {
        $tcpClient.EndConnect($connect)
        $tcpClient.Close()
        Write-Host "✅ TCP CONNECTION SUCCESS - Port $Port is open" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ CONNECTION FAILED - $_" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Test 3: HTTP Status Endpoint
Write-Host "[Test 3] Testing HTTP API at http://${PublicNodeIP}:${Port}/api/status..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "http://${PublicNodeIP}:${Port}/api/status" -TimeoutSec 5 -UseBasicParsing
    $status = $response.Content | ConvertFrom-Json
    
    Write-Host "✅ HTTP API SUCCESS" -ForegroundColor Green
    Write-Host "   Chain Height: $($status.height)" -ForegroundColor Cyan
    Write-Host "   Difficulty: $($status.difficulty)" -ForegroundColor Cyan
    Write-Host "   Peer Count: $($status.peers.Count)" -ForegroundColor Cyan
} catch {
    Write-Host "❌ HTTP API FAILED - $_" -ForegroundColor Red
    Write-Host "   The node may not be running properly" -ForegroundColor Gray
    exit 1
}
Write-Host ""

# Test 4: P2P Compact Block Endpoint
Write-Host "[Test 4] Testing P2P endpoint at http://${PublicNodeIP}:${Port}/p2p/compact_block..." -ForegroundColor Yellow
try {
    # Try a dummy POST (will fail but proves endpoint is reachable)
    $testData = @{
        header = @{
            hash = "test"
            height = 0
        }
    }
    $jsonData = $testData | ConvertTo-Json
    
    $response = Invoke-WebRequest -Uri "http://${PublicNodeIP}:${Port}/p2p/compact_block" `
        -Method POST `
        -Body $jsonData `
        -ContentType "application/json" `
        -TimeoutSec 5 `
        -UseBasicParsing `
        -ErrorAction SilentlyContinue
    
    Write-Host "✅ P2P ENDPOINT REACHABLE (returned status $($response.StatusCode))" -ForegroundColor Green
} catch {
    # We expect this to fail with 400/422 (bad data) but NOT connection errors
    if ($_.Exception.Response.StatusCode -eq 400 -or $_.Exception.Response.StatusCode -eq 422) {
        Write-Host "✅ P2P ENDPOINT REACHABLE (rejected test data as expected)" -ForegroundColor Green
    } elseif ($_.Exception.Message -match "Unable to connect|connection") {
        Write-Host "❌ P2P ENDPOINT NOT REACHABLE - Connection failed" -ForegroundColor Red
        exit 1
    } else {
        Write-Host "✅ P2P ENDPOINT REACHABLE (unexpected response but connection works)" -ForegroundColor Green
    }
}
Write-Host ""

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "✅ ALL TESTS PASSED" -ForegroundColor Green
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "The miner should be able to connect with:" -ForegroundColor Yellow
Write-Host "  .\vision-node.exe --miner --p2p-peer ${PublicNodeIP}:${Port}" -ForegroundColor White
Write-Host ""
