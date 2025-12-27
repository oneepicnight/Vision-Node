# Guardian Sentinel - Launch Script
# The Watch Begins 🛡️

Write-Host ""
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🛡️  GUARDIAN SENTINEL - LAUNCH SEQUENCE" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

# Step 1: Verify binary exists
if (-not (Test-Path "c:\vision-node\target\release\vision-node.exe")) {
    Write-Host "❌ Binary not found!" -ForegroundColor Red
    Write-Host "Run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

$binary = Get-Item "c:\vision-node\target\release\vision-node.exe"
Write-Host "✅ Binary found: $([math]::Round($binary.Length/1MB,2)) MB" -ForegroundColor Green
Write-Host "   Built: $($binary.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))" -ForegroundColor Gray
Write-Host ""

# Step 2: Check Guardian owner configuration
Write-Host "🔍 Checking Guardian Owner Configuration..." -ForegroundColor Yellow
Write-Host ""

$hasDiscordId = $env:GUARDIAN_OWNER_DISCORD_ID -ne $null -and $env:GUARDIAN_OWNER_DISCORD_ID -ne ""
$hasWalletAddr = $env:GUARDIAN_OWNER_WALLET_ADDRESS -ne $null -and $env:GUARDIAN_OWNER_WALLET_ADDRESS -ne ""

if ($hasDiscordId) {
    Write-Host "   ✅ GUARDIAN_OWNER_DISCORD_ID: " -NoNewline -ForegroundColor Green
    Write-Host $env:GUARDIAN_OWNER_DISCORD_ID -ForegroundColor White
} else {
    Write-Host "   ✅ GUARDIAN_OWNER_DISCORD_ID: " -NoNewline -ForegroundColor Cyan
    Write-Host "309081088960233492 (default - canonical owner)" -ForegroundColor White
    Write-Host ""
    Write-Host "   Note: Discord ID defaults to Donnie's ID in all public builds" -ForegroundColor Gray
    Write-Host "   Override with: " -NoNewline -ForegroundColor Gray
    Write-Host '$env:GUARDIAN_OWNER_DISCORD_ID="your_id"' -ForegroundColor DarkGray
    Write-Host ""
}

if ($hasWalletAddr) {
    Write-Host "   ✅ GUARDIAN_OWNER_WALLET_ADDRESS: " -NoNewline -ForegroundColor Green
    Write-Host $env:GUARDIAN_OWNER_WALLET_ADDRESS -ForegroundColor White
} else {
    Write-Host "   ⚠️  GUARDIAN_OWNER_WALLET_ADDRESS: " -NoNewline -ForegroundColor Yellow
    Write-Host "Not set" -ForegroundColor Red
    Write-Host ""
    Write-Host "   To set your wallet address:" -ForegroundColor Yellow
    Write-Host '   $env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1your_wallet_here"' -ForegroundColor Gray
    Write-Host ""
}

Write-Host ""

if (-not $hasWalletAddr) {
    Write-Host "⚠️  Guardian wallet address not configured" -ForegroundColor Yellow
    Write-Host "   Guardian will use default Discord ID (309081088960233492)" -ForegroundColor Gray
    Write-Host "   But won't fully detect core node without wallet address" -ForegroundColor Gray
    Write-Host ""
    Write-Host "   Set wallet with: " -NoNewline -ForegroundColor Gray
    Write-Host '$env:GUARDIAN_OWNER_WALLET_ADDRESS="vision1your_wallet"' -ForegroundColor DarkGray
    Write-Host ""
    
    $continue = Read-Host "Continue anyway? (y/n)"
    if ($continue -ne 'y') {
        Write-Host ""
        Write-Host "Launch aborted." -ForegroundColor Red
        exit 0
    }
    Write-Host ""
}

# Step 3: Check other environment variables
Write-Host "🔍 Checking Optional Configuration..." -ForegroundColor Yellow
Write-Host ""

$hasVisionBot = $env:VISION_BOT_WEBHOOK_URL -ne $null -and $env:VISION_BOT_WEBHOOK_URL -ne ""
$hasDiscordWebhook = $env:VISION_GUARDIAN_DISCORD_WEBHOOK -ne $null -and $env:VISION_GUARDIAN_DISCORD_WEBHOOK -ne ""

if ($hasVisionBot) {
    Write-Host "   ✅ VISION_BOT_WEBHOOK_URL: Configured" -ForegroundColor Green
} else {
    Write-Host "   ℹ️  VISION_BOT_WEBHOOK_URL: Not set (events won't be sent to Vision Bot)" -ForegroundColor Gray
}

if ($hasDiscordWebhook) {
    Write-Host "   ✅ VISION_GUARDIAN_DISCORD_WEBHOOK: Configured" -ForegroundColor Green
} else {
    Write-Host "   ℹ️  VISION_GUARDIAN_DISCORD_WEBHOOK: Not set (no Discord announcements)" -ForegroundColor Gray
}

Write-Host ""

# Step 4: Kill existing processes
Write-Host "🔄 Checking for existing processes..." -ForegroundColor Yellow
$existing = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue

if ($existing) {
    Write-Host "   Found $($existing.Count) running process(es)" -ForegroundColor Yellow
    Write-Host "   Stopping..." -ForegroundColor Yellow
    Stop-Process -Name "vision-node" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    Write-Host "   ✅ Stopped" -ForegroundColor Green
} else {
    Write-Host "   No existing processes found" -ForegroundColor Gray
}

Write-Host ""

# Step 5: Launch Guardian
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🛡️  LAUNCHING GUARDIAN SENTINEL" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

Write-Host "Starting Vision Node..." -ForegroundColor Cyan
Write-Host ""

# Launch in new window
Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    @"
Set-Location 'c:\vision-node'
`$env:VISION_GUARDIAN_MODE='true'
`$env:VISION_UPSTREAM_HTTP_BASE='https://visionworld.tech'
`$env:GUARDIAN_OWNER_DISCORD_ID='$env:GUARDIAN_OWNER_DISCORD_ID'
`$env:GUARDIAN_OWNER_WALLET_ADDRESS='$env:GUARDIAN_OWNER_WALLET_ADDRESS'
`$env:VISION_BOT_WEBHOOK_URL='$env:VISION_BOT_WEBHOOK_URL'
`$env:VISION_GUARDIAN_DISCORD_WEBHOOK='$env:VISION_GUARDIAN_DISCORD_WEBHOOK'
Write-Host '🛡️  GUARDIAN SENTINEL' -ForegroundColor Cyan
Write-Host ''
.\target\release\vision-node.exe
"@
)

Start-Sleep -Seconds 3

# Step 6: Verify launch
Write-Host "🔍 Verifying launch..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

$process = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue

if ($process) {
    Write-Host "   ✅ Guardian is ONLINE" -ForegroundColor Green
    Write-Host "   PID: $($process.Id)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host "✅ GUARDIAN SENTINEL LAUNCHED" -ForegroundColor Green
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host ""
    
    if ($hasDiscordId -and $hasWalletAddr) {
        Write-Host "🛡️  Guardian owner: $env:GUARDIAN_OWNER_WALLET_ADDRESS ↔ $env:GUARDIAN_OWNER_DISCORD_ID" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "   The Guardian knows who he serves." -ForegroundColor Gray
    } else {
        Write-Host "⚠️  Guardian running without owner identity" -ForegroundColor Yellow
    }
    
    Write-Host ""
    Write-Host "Endpoints:" -ForegroundColor Yellow
    Write-Host "   Dashboard: http://127.0.0.1:7070/dashboard.html" -ForegroundColor White
    Write-Host "   Wallet:    http://127.0.0.1:7070/app" -ForegroundColor White
    Write-Host "   API:       http://127.0.0.1:7070/api/status" -ForegroundColor White
    Write-Host ""
    Write-Host "Next Steps:" -ForegroundColor Yellow
    Write-Host "   1. Run: .\test-guardian-ownership.ps1" -ForegroundColor White
    Write-Host "   2. Test core node detection" -ForegroundColor White
    Write-Host "   3. Verify Vision Bot integration" -ForegroundColor White
    Write-Host ""
    Write-Host "To stop: Get-Process -Name 'vision-node' | Stop-Process" -ForegroundColor Gray
    Write-Host ""
    Write-Host "The watch begins. 🛡️" -ForegroundColor Cyan
    
} else {
    Write-Host "   ❌ Launch failed" -ForegroundColor Red
    Write-Host "   Check the Guardian window for errors" -ForegroundColor Yellow
    Write-Host ""
}
