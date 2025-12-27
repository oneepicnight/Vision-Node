# Start Vision Node with Discord OAuth
# This script sets the required environment variables and starts the node

Write-Host "Starting Vision Node with Discord OAuth..." -ForegroundColor Cyan

# Set environment variables
$env:VISION_PUBLIC_DIR = "c:\vision-node\public"
$env:VISION_WALLET_DIR = "c:\vision-node\wallet\dist"
$env:DISCORD_CLIENT_ID = "1442594705748529335"
$env:DISCORD_CLIENT_SECRET = "yT2lVs_9x9I8gccZJ1nacTkL1bSEUn39"
$env:DISCORD_REDIRECT_URI = "http://127.0.0.1:7070/api/discord/callback"

Write-Host "✅ Environment variables set" -ForegroundColor Green
Write-Host ""
Write-Host "Discord OAuth Configuration:" -ForegroundColor Yellow
Write-Host "  Client ID: $env:DISCORD_CLIENT_ID" -ForegroundColor White
Write-Host "  Redirect URI: $env:DISCORD_REDIRECT_URI" -ForegroundColor White
Write-Host ""
Write-Host "Starting Vision Node..." -ForegroundColor Cyan
Write-Host ""

# Start the node
Set-Location "c:\vision-node"
.\target\release\vision-node.exe
