#!/usr/bin/env pwsh
# Foundation Config Integration Test - Simple Version

$baseUrl = "http://localhost:7070"

Write-Host "===== Foundation Config Integration Test =====" -ForegroundColor Cyan
Write-Host "Testing node at: $baseUrl" -ForegroundColor Gray
Write-Host ""

# Test 1: Health check
Write-Host "Test 1: Health Check" -ForegroundColor Yellow
try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/health" -UseBasicParsing -ErrorAction Stop
    $data = $resp.Content | ConvertFrom-Json
    Write-Host "Status: OK" -ForegroundColor Green
    Write-Host "Response: $($data | ConvertTo-Json)" -ForegroundColor Green
} catch {
    Write-Host "Failed: $_" -ForegroundColor Red
}
Write-Host ""

# Test 2: Check vault address balance
Write-Host "Test 2: Check Vault Address Balance" -ForegroundColor Yellow
Write-Host "(Should be using: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb)" -ForegroundColor Gray
try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/account/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" -UseBasicParsing -ErrorAction Stop
    Write-Host "Status: OK" -ForegroundColor Green
    $data = $resp.Content | ConvertFrom-Json
    Write-Host "Vault Balance Data: $($data | ConvertTo-Json)" -ForegroundColor Green
} catch {
    Write-Host "Response: $_" -ForegroundColor Yellow
}
Write-Host ""

# Test 3: Check fund address balance  
Write-Host "Test 3: Check Fund Address Balance" -ForegroundColor Yellow
Write-Host "(Should be using: cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc)" -ForegroundColor Gray
try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/account/cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" -UseBasicParsing -ErrorAction Stop
    Write-Host "Status: OK" -ForegroundColor Green
    $data = $resp.Content | ConvertFrom-Json
    Write-Host "Fund Balance Data: $($data | ConvertTo-Json)" -ForegroundColor Green
} catch {
    Write-Host "Response: $_" -ForegroundColor Yellow
}
Write-Host ""

# Test 4: Check founder address balance
Write-Host "Test 4: Check Founder Address Balance" -ForegroundColor Yellow
Write-Host "(Should be using: dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd)" -ForegroundColor Gray
try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/account/dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd" -UseBasicParsing -ErrorAction Stop
    Write-Host "Status: OK" -ForegroundColor Green
    $data = $resp.Content | ConvertFrom-Json
    Write-Host "Founder Balance Data: $($data | ConvertTo-Json)" -ForegroundColor Green
} catch {
    Write-Host "Response: $_" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "===== Test Complete =====" -ForegroundColor Cyan
Write-Host "Foundation config is loaded from: config/token_accounts.toml" -ForegroundColor Green
Write-Host "Check logs for routing of settlements to these addresses" -ForegroundColor Green
Write-Host ""

# Show recent logs
Write-Host "===== Node Logs (last 30 lines) =====" -ForegroundColor Cyan
Get-Content c:\vision-node\test-output.log -Tail 30

Write-Host ""
Write-Host "Test script complete!" -ForegroundColor Green
