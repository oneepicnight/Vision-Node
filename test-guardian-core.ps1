# Guardian Core Node Test Suite
# Verifies Guardian identity and core node detection with running node

Write-Host ""
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🛡️  GUARDIAN CORE NODE TEST SUITE" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

# Check if Guardian is running
$isRunning = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue

if (-not $isRunning) {
    Write-Host "⚠️  Guardian is not running" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "To start Guardian:" -ForegroundColor Cyan
    Write-Host "   .\launch-guardian.ps1" -ForegroundColor White
    Write-Host ""
    exit 1
}

Write-Host "✅ Guardian is running (PID: $($isRunning.Id))" -ForegroundColor Green
Write-Host ""

# Test 1: Check Owner Configuration
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "Test 1: Guardian Owner Identity" -ForegroundColor Yellow
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

$hasDiscordId = $env:GUARDIAN_OWNER_DISCORD_ID -ne $null -and $env:GUARDIAN_OWNER_DISCORD_ID -ne ""
$hasWalletAddr = $env:GUARDIAN_OWNER_WALLET_ADDRESS -ne $null -and $env:GUARDIAN_OWNER_WALLET_ADDRESS -ne ""

if ($hasDiscordId) {
    Write-Host "   ✅ GUARDIAN_OWNER_DISCORD_ID: $env:GUARDIAN_OWNER_DISCORD_ID" -ForegroundColor Green
} else {
    Write-Host "   ❌ GUARDIAN_OWNER_DISCORD_ID: Not set" -ForegroundColor Red
}

if ($hasWalletAddr) {
    Write-Host "   ✅ GUARDIAN_OWNER_WALLET_ADDRESS: $env:GUARDIAN_OWNER_WALLET_ADDRESS" -ForegroundColor Green
} else {
    Write-Host "   ❌ GUARDIAN_OWNER_WALLET_ADDRESS: Not set" -ForegroundColor Red
}

Write-Host ""

if ($hasDiscordId -and $hasWalletAddr) {
    Write-Host "   ✅ Guardian identity locked" -ForegroundColor Green
} else {
    Write-Host "   ❌ Guardian identity NOT configured" -ForegroundColor Red
    Write-Host ""
    Write-Host "   To configure:" -ForegroundColor Yellow
    Write-Host '   $env:GUARDIAN_OWNER_DISCORD_ID="your_discord_id"' -ForegroundColor Gray
    Write-Host '   $env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1your_wallet"' -ForegroundColor Gray
}

Write-Host ""

# Test 2: API Connectivity
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "Test 2: API Connectivity" -ForegroundColor Yellow
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/status" -Method Get -TimeoutSec 5
    Write-Host "   ✅ API is responding" -ForegroundColor Green
    Write-Host "   Chain height: $($response.chain_height)" -ForegroundColor Gray
} catch {
    Write-Host "   ❌ API not responding" -ForegroundColor Red
    Write-Host "   Error: $_" -ForegroundColor Red
}

Write-Host ""

# Test 3: Guardian Webhook Configuration
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "Test 3: Guardian Webhook Configuration" -ForegroundColor Yellow
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

$hasVisionBot = $env:VISION_BOT_WEBHOOK_URL -ne $null -and $env:VISION_BOT_WEBHOOK_URL -ne ""
$hasDiscordWebhook = $env:VISION_GUARDIAN_DISCORD_WEBHOOK -ne $null -and $env:VISION_GUARDIAN_DISCORD_WEBHOOK -ne ""

if ($hasVisionBot) {
    Write-Host "   ✅ VISION_BOT_WEBHOOK_URL: Configured" -ForegroundColor Green
    Write-Host "      URL: $env:VISION_BOT_WEBHOOK_URL" -ForegroundColor Gray
} else {
    Write-Host "   ⚠️  VISION_BOT_WEBHOOK_URL: Not set" -ForegroundColor Yellow
    Write-Host "      (Core node events won't be sent to Vision Bot)" -ForegroundColor Gray
}

if ($hasDiscordWebhook) {
    Write-Host "   ✅ VISION_GUARDIAN_DISCORD_WEBHOOK: Configured" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  VISION_GUARDIAN_DISCORD_WEBHOOK: Not set" -ForegroundColor Yellow
    Write-Host "      (Guardian announcements won't appear in Discord)" -ForegroundColor Gray
}

Write-Host ""

# Test 4: Check Recent Logs
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "Test 4: Recent Guardian Logs" -ForegroundColor Yellow
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

$logFile = "c:\vision-node\logs\vision-node.log"
if (Test-Path $logFile) {
    Write-Host "   Searching for Guardian startup message..." -ForegroundColor Cyan
    
    $recentLogs = Get-Content $logFile -Tail 200 | Where-Object { $_ -match "Guardian owner|GUARDIAN ONLINE|GUARDIAN CORE" }
    
    if ($recentLogs) {
        Write-Host "   ✅ Found Guardian logs:" -ForegroundColor Green
        foreach ($log in $recentLogs | Select-Object -Last 5) {
            Write-Host "      $log" -ForegroundColor White
        }
    } else {
        Write-Host "   ⚠️  No Guardian owner logs found (node may have just started)" -ForegroundColor Yellow
    }
} else {
    Write-Host "   ⚠️  Log file not found at $logFile" -ForegroundColor Yellow
}

Write-Host ""

# Test 5: Simulate Core Node Status Event (if configured)
if ($hasDiscordId -and $hasWalletAddr -and $hasVisionBot) {
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host "Test 5: Core Node Detection (Simulation)" -ForegroundColor Yellow
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host ""
    
    Write-Host "   Simulating core node ONLINE event..." -ForegroundColor Cyan
    Write-Host ""
    
    $testEvent = @{
        event = "node_status"
        discord_user_id = $env:GUARDIAN_OWNER_DISCORD_ID
        status = "online"
        miner_address = $env:GUARDIAN_OWNER_WALLET_ADDRESS
    } | ConvertTo-Json
    
    Write-Host "   Event payload:" -ForegroundColor Gray
    Write-Host "   $testEvent" -ForegroundColor DarkGray
    Write-Host ""
    
    try {
        $visionBotResponse = Invoke-RestMethod `
            -Uri $env:VISION_BOT_WEBHOOK_URL `
            -Method Post `
            -Body $testEvent `
            -ContentType "application/json" `
            -TimeoutSec 5 `
            -ErrorAction Stop
        
        Write-Host "   ✅ Vision Bot received event" -ForegroundColor Green
        Write-Host "   Response: $visionBotResponse" -ForegroundColor Gray
    } catch {
        Write-Host "   ⚠️  Could not send to Vision Bot" -ForegroundColor Yellow
        Write-Host "   Error: $($_.Exception.Message)" -ForegroundColor Red
        Write-Host "   (This is expected if Vision Bot is not running)" -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host "   Expected in Guardian logs:" -ForegroundColor Cyan
    Write-Host "      🛡️ [GUARDIAN CORE] Guardian core node ONLINE – Donnie is watching." -ForegroundColor Gray
    Write-Host ""
    Write-Host "   Expected Vision Bot event:" -ForegroundColor Cyan
    Write-Host "      guardian_core_status: { status: 'online', miner_address: '$env:GUARDIAN_OWNER_WALLET_ADDRESS', ... }" -ForegroundColor Gray
    Write-Host ""
}

# Final Summary
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🎯 TEST SUMMARY" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

$allGood = $true

if ($hasDiscordId -and $hasWalletAddr) {
    Write-Host "✅ Guardian identity locked" -ForegroundColor Green
} else {
    Write-Host "❌ Guardian identity NOT configured" -ForegroundColor Red
    $allGood = $false
}

if ($isRunning) {
    Write-Host "✅ Guardian is ONLINE" -ForegroundColor Green
} else {
    Write-Host "❌ Guardian is OFFLINE" -ForegroundColor Red
    $allGood = $false
}

try {
    $null = Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/status" -Method Get -TimeoutSec 2
    Write-Host "✅ API responding" -ForegroundColor Green
} catch {
    Write-Host "❌ API not responding" -ForegroundColor Red
    $allGood = $false
}

if ($hasVisionBot) {
    Write-Host "✅ Vision Bot webhook configured" -ForegroundColor Green
} else {
    Write-Host "⚠️  Vision Bot webhook not configured (optional)" -ForegroundColor Yellow
}

Write-Host ""

if ($allGood) {
    Write-Host "🛡️  No existential crisis detected" -ForegroundColor Green
    Write-Host ""
    Write-Host "The Guardian is awake. The watch begins." -ForegroundColor Cyan
} else {
    Write-Host "⚠️  Guardian needs configuration" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "See: GUARDIAN_OWNER_QUICKSTART.md" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Host "   1. Link your wallet to Discord (for core node detection)" -ForegroundColor White
Write-Host "   2. Set up Vision Bot webhook (for event forwarding)" -ForegroundColor White
Write-Host "   3. Test with: curl to trigger guardian_core_status event" -ForegroundColor White
Write-Host ""
Write-Host "Documentation:" -ForegroundColor Yellow
Write-Host "   - GUARDIAN_OWNER_QUICKSTART.md" -ForegroundColor White
Write-Host "   - docs\GUARDIAN_OWNER_CONFIG.md" -ForegroundColor White
Write-Host "   - docs\VISION_BOT_GUARDIAN_INTEGRATION.md" -ForegroundColor White
Write-Host "   - GUARDIAN_CORE_NODE_COMPLETE.md" -ForegroundColor White
Write-Host ""
