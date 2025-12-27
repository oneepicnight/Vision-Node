# Vision Wallet Integration Test Script
# Tests wallet functionality with the Vision Node

param(
    [switch]$Open
)

$ErrorActionPreference = "Continue"

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "VISION WALLET - INTEGRATION TEST" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

# Clean up old processes
Write-Host "🧹 Cleaning up old processes..." -ForegroundColor Yellow
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Start node
Write-Host "🚀 Starting Vision Node on port 7070..." -ForegroundColor Yellow
Start-Process -FilePath "cargo" -ArgumentList "run --release -- --port 7070" -NoNewWindow

# Wait for node to be ready
Write-Host "⏳ Waiting for node to start..." -ForegroundColor Yellow
$maxAttempts = 60
$attempt = 0

while ($attempt -lt $maxAttempts) {
    try {
        $response = Invoke-RestMethod -Uri "http://127.0.0.1:7070/status" -TimeoutSec 2 -ErrorAction Stop
        Write-Host "✅ Node is ready!" -ForegroundColor Green
        break
    } catch {
        $attempt++
        Start-Sleep -Milliseconds 500
    }
}

if ($attempt -ge $maxAttempts) {
    Write-Host "❌ Node failed to start" -ForegroundColor Red
    exit 1
}

Start-Sleep -Seconds 2

Write-Host "`n📝 Testing Wallet Endpoints..." -ForegroundColor Cyan

# Test 1: Check if /wallet/sign endpoint exists
Write-Host "`n1️⃣  Testing /wallet/sign endpoint..." -ForegroundColor Yellow

$testPrivateKey = "0000000000000000000000000000000000000000000000000000000000000001"
$testPublicKey = "0000000000000000000000000000000000000000000000000000000000000002"

$testTx = @{
    nonce = 0
    sender_pubkey = $testPublicKey
    access_list = @()
    module = "test"
    method = "ping"
    args = @()
    tip = 1000
    fee_limit = 10000
    sig = ""
    max_priority_fee_per_gas = 0
    max_fee_per_gas = 0
}

$signRequest = @{
    tx = $testTx
    private_key = $testPrivateKey
} | ConvertTo-Json -Depth 10

try {
    $signResult = Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/sign" -Method Post -Body $signRequest -ContentType "application/json"
    Write-Host "✅ Signing works!" -ForegroundColor Green
    Write-Host "   Signature: $($signResult.signature.Substring(0, 32))..." -ForegroundColor Gray
    Write-Host "   TX Hash: $($signResult.tx_hash.Substring(0, 32))..." -ForegroundColor Gray
} catch {
    Write-Host "❌ Signing failed: $_" -ForegroundColor Red
}

# Test 2: Check balance endpoint
Write-Host "`n2️⃣  Testing balance endpoint..." -ForegroundColor Yellow

try {
    $balance = Invoke-RestMethod -Uri "http://127.0.0.1:7070/balance/$testPublicKey"
    Write-Host "✅ Balance query works!" -ForegroundColor Green
    Write-Host "   Balance: $($balance.balance)" -ForegroundColor Gray
} catch {
    Write-Host "⚠️  Balance query failed: $_" -ForegroundColor Yellow
}

# Test 3: Check nonce endpoint
Write-Host "`n3️⃣  Testing nonce endpoint..." -ForegroundColor Yellow

try {
    $nonce = Invoke-RestMethod -Uri "http://127.0.0.1:7070/nonce/$testPublicKey"
    Write-Host "✅ Nonce query works!" -ForegroundColor Green
    Write-Host "   Nonce: $($nonce.nonce)" -ForegroundColor Gray
} catch {
    Write-Host "⚠️  Nonce query failed: $_" -ForegroundColor Yellow
}

# Test 4: Full transaction flow (sign + submit)
Write-Host "`n4️⃣  Testing full transaction flow (sign + submit)..." -ForegroundColor Yellow

try {
    # Sign the transaction
    $signResult = Invoke-RestMethod -Uri "http://127.0.0.1:7070/wallet/sign" -Method Post -Body $signRequest -ContentType "application/json"
    
    # Add signature to transaction
    $signedTx = $testTx.Clone()
    $signedTx.sig = $signResult.signature
    
    $submitRequest = @{ tx = $signedTx } | ConvertTo-Json -Depth 10
    
    # Submit signed transaction
    $submitResult = Invoke-RestMethod -Uri "http://127.0.0.1:7070/submit_tx" -Method Post -Body $submitRequest -ContentType "application/json"
    
    Write-Host "✅ Full flow works!" -ForegroundColor Green
    Write-Host "   Status: $($submitResult.status)" -ForegroundColor Gray
    Write-Host "   TX Hash: $($submitResult.tx_hash.Substring(0, 32))..." -ForegroundColor Gray
} catch {
    Write-Host "⚠️  Full flow had issues (expected for test tx): $_" -ForegroundColor Yellow
}

# Test 5: Check if wallet UI is accessible
Write-Host "`n5️⃣  Checking wallet UI..." -ForegroundColor Yellow

try {
    $walletPage = Invoke-WebRequest -Uri "http://127.0.0.1:7070/wallet/" -TimeoutSec 3
    if ($walletPage.Content -like "*Vision Wallet*") {
        Write-Host "✅ Wallet UI is accessible!" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Wallet page loaded but content unexpected" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  Wallet UI not accessible: $_" -ForegroundColor Yellow
}

Write-Host "`n✅ Integration tests complete!" -ForegroundColor Green

Write-Host "`n📊 Available URLs:" -ForegroundColor Cyan
Write-Host "  🎨 Wallet:      http://127.0.0.1:7070/wallet/" -ForegroundColor White
Write-Host "  🖥️  Panel:       http://127.0.0.1:7070/panel.html" -ForegroundColor White
Write-Host "  📊 Dashboard:   http://127.0.0.1:7070/dashboard.html" -ForegroundColor White
Write-Host "  🔍 Explorer:    http://127.0.0.1:7070/explorer.html" -ForegroundColor White
Write-Host "  📡 Status:      http://127.0.0.1:7070/status" -ForegroundColor White

Write-Host "`n🧪 API Endpoints:" -ForegroundColor Cyan
Write-Host "  POST /wallet/sign      - Sign transactions" -ForegroundColor Gray
Write-Host "  GET  /balance/:addr    - Query balance" -ForegroundColor Gray
Write-Host "  GET  /nonce/:addr      - Query nonce" -ForegroundColor Gray
Write-Host "  POST /submit_tx        - Submit signed tx" -ForegroundColor Gray
Write-Host "  GET  /mempool          - List pending txs" -ForegroundColor Gray
Write-Host "  GET  /tx/:hash         - Query transaction" -ForegroundColor Gray

if ($Open) {
    Write-Host "`n🌐 Opening wallet in browser..." -ForegroundColor Cyan
    Start-Process "http://127.0.0.1:7070/wallet/"
}

Write-Host "`nPress Ctrl+C to stop node" -ForegroundColor Yellow

# Keep running
while ($true) { Start-Sleep -Seconds 10 }
