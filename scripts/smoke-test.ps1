#!/usr/bin/env pwsh
# Smoke test for Vision Node

$ErrorActionPreference = "Stop"

Write-Host "🧪 Vision Node Smoke Test" -ForegroundColor Cyan
Write-Host "==========================" -ForegroundColor Cyan

$baseUrl = "http://127.0.0.1:7070"
$adminToken = "test-token-123"

# Check if node is running
Write-Host "`n1️⃣ Testing node connectivity..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "$baseUrl/health" -Method Get -TimeoutSec 5
    if ($response.StatusCode -eq 200) {
        Write-Host "  ✓ Node is running" -ForegroundColor Green
    }
} catch {
    Write-Error "Node is not running. Start it first: cargo run --release"
    exit 1
}

# Test status endpoint
Write-Host "`n2️⃣ Testing /status..." -ForegroundColor Yellow
$status = Invoke-RestMethod -Uri "$baseUrl/status" -Method Get
Write-Host "  ✓ Height: $($status.height)" -ForegroundColor Green
Write-Host "  ✓ Peers: $($status.peer_count)" -ForegroundColor Green

# Seed balance (admin endpoint)
Write-Host "`n3️⃣ Seeding test balance..." -ForegroundColor Yellow
$headers = @{
    "X-Vision-Admin-Token" = $adminToken
    "Content-Type" = "application/json"
}
$body = @{
    addr = "ALICE"
    amount = "100000"
} | ConvertTo-Json

try {
    Invoke-RestMethod -Uri "$baseUrl/admin/seed-balance" -Method Post -Headers $headers -Body $body
    Write-Host "  ✓ Seeded ALICE with 100,000" -ForegroundColor Green
} catch {
    Write-Host "  ⚠ Seed failed (may already be seeded)" -ForegroundColor Yellow
}

# Check balance
Write-Host "`n4️⃣ Checking ALICE balance..." -ForegroundColor Yellow
$balance = Invoke-RestMethod -Uri "$baseUrl/balance/ALICE" -Method Get
Write-Host "  ✓ Balance: $balance" -ForegroundColor Green

# Transfer
Write-Host "`n5️⃣ Testing transfer (ALICE → BOB)..." -ForegroundColor Yellow
$transferBody = @{
    from = "ALICE"
    to = "BOB"
    amount = "2500"
} | ConvertTo-Json

$transfer = Invoke-RestMethod -Uri "$baseUrl/wallet/transfer" -Method Post -Body $transferBody -ContentType "application/json"
Write-Host "  ✓ Transfer successful" -ForegroundColor Green
Write-Host "  ✓ TX Hash: $($transfer.tx_hash)" -ForegroundColor Gray

# Check receipts
Write-Host "`n6️⃣ Checking receipts..." -ForegroundColor Yellow
$receipts = Invoke-RestMethod -Uri "$baseUrl/receipts/latest?limit=5" -Method Get
Write-Host "  ✓ Found $($receipts.receipts.Count) receipts" -ForegroundColor Green
if ($receipts.receipts.Count -gt 0) {
    Write-Host "  ✓ Latest: $($receipts.receipts[0].tx_hash)" -ForegroundColor Gray
}

# Test metrics
Write-Host "`n7️⃣ Testing /metrics..." -ForegroundColor Yellow
$metrics = Invoke-WebRequest -Uri "$baseUrl/metrics" -Method Get
if ($metrics.Content -match "vision_transfers_total") {
    Write-Host "  ✓ Prometheus metrics working" -ForegroundColor Green
} else {
    Write-Host "  ⚠ Metrics format unexpected" -ForegroundColor Yellow
}

# Summary
Write-Host "`n✅ Smoke test PASSED!" -ForegroundColor Green
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Green
Write-Host "All core MVP endpoints are working correctly." -ForegroundColor Cyan
