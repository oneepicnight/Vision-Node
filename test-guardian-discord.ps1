# Guardian Discord Integration - Quick Test
# This script helps you test the Discord webhook integration

Write-Host "🛡️  Guardian Discord Integration Test" -ForegroundColor Cyan
Write-Host "=" -NoNewline; Write-Host ("=" * 69)
Write-Host ""

# Check if webhook URL is set
$webhookUrl = $env:VISION_GUARDIAN_DISCORD_WEBHOOK

if (-not $webhookUrl) {
    Write-Host "❌ Discord webhook not configured!" -ForegroundColor Red
    Write-Host ""
    Write-Host "To enable Discord integration:" -ForegroundColor Yellow
    Write-Host "1. Create a webhook in your Discord server:" -ForegroundColor White
    Write-Host "   Server Settings → Integrations → Webhooks → New Webhook" -ForegroundColor Gray
    Write-Host ""
    Write-Host "2. Set the environment variable:" -ForegroundColor White
    Write-Host '   $env:VISION_GUARDIAN_DISCORD_WEBHOOK="https://discord.com/api/webhooks/YOUR_WEBHOOK_URL"' -ForegroundColor Gray
    Write-Host ""
    Write-Host "3. Run this script again to test" -ForegroundColor White
    Write-Host ""
    exit 1
}

Write-Host "✅ Discord webhook configured" -ForegroundColor Green
Write-Host "   URL: " -NoNewline -ForegroundColor White
Write-Host $webhookUrl.Substring(0, [Math]::Min(50, $webhookUrl.Length)) -NoNewline -ForegroundColor Gray
if ($webhookUrl.Length -gt 50) { Write-Host "..." -ForegroundColor Gray } else { Write-Host "" }
Write-Host ""

# Test webhook with a simple message
Write-Host "🧪 Sending test message to Discord..." -ForegroundColor Yellow

try {
    $testMessage = @{
        content = "🛡️ **Guardian Test**`n`nThis is a test message from Vision Node.`n`nIf you see this, Discord integration is working! ✅"
    } | ConvertTo-Json

    $response = Invoke-RestMethod -Uri $webhookUrl -Method Post -Body $testMessage -ContentType "application/json"
    
    Write-Host "✅ Test message sent successfully!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Check your Discord channel - you should see the test message." -ForegroundColor White
    Write-Host ""
    Write-Host "Now start your Vision Node to see live Guardian announcements:" -ForegroundColor Cyan
    Write-Host "   cd c:\vision-node" -ForegroundColor Gray
    Write-Host "   .\target\release\vision-node.exe" -ForegroundColor Gray
    Write-Host ""
    
} catch {
    Write-Host "❌ Failed to send test message!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Error details:" -ForegroundColor Yellow
    Write-Host $_.Exception.Message -ForegroundColor Gray
    Write-Host ""
    Write-Host "Possible issues:" -ForegroundColor Yellow
    Write-Host "- Webhook URL is invalid or expired" -ForegroundColor White
    Write-Host "- Webhook was deleted from Discord" -ForegroundColor White
    Write-Host "- Network/firewall blocking HTTPS requests" -ForegroundColor White
    Write-Host "- Discord API rate limit (wait 1 minute and retry)" -ForegroundColor White
    Write-Host ""
    exit 1
}

Write-Host "=" -NoNewline; Write-Host ("=" * 69)
Write-Host ""
Write-Host "For more information, see: docs/GUARDIAN_DISCORD_INTEGRATION.md" -ForegroundColor Gray
