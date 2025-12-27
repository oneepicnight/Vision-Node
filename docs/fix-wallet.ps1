# Fix wallet serving by removing conflicting public/app directory
Write-Host "=== Fixing Wallet Serving Issue ===" -ForegroundColor Cyan

# Backup and remove public/app (it's shadowing wallet/dist)
if (Test-Path "c:\vision-node\public\app") {
    Write-Host "Removing public/app (shadowing wallet)..." -ForegroundColor Yellow
    if (Test-Path "c:\vision-node\public\app-OLD") {
        Remove-Item "c:\vision-node\public\app-OLD" -Recurse -Force
    }
    Move-Item "c:\vision-node\public\app" "c:\vision-node\public\app-OLD" -Force
    Write-Host "Moved to public/app-OLD" -ForegroundColor Green
}

# Also check for exchange files
if (Test-Path "c:\vision-node\public\exchange") {
    Write-Host "`nFound public/exchange directory" -ForegroundColor Cyan
}

# Restart node
Write-Host "`nRestarting Vision Node..." -ForegroundColor Yellow
Stop-Process -Name "vision-node" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

$env:VISION_PUBLIC_DIR="c:\vision-node\public"
$env:VISION_WALLET_DIR="c:\vision-node\wallet\dist"

Start-Process -FilePath "c:\vision-node\target\release\vision-node.exe" `
              -WorkingDirectory "c:\vision-node" `
              -WindowStyle Hidden

Start-Sleep -Seconds 3

Write-Host "`n=== Fixed! ===" -ForegroundColor Green
Write-Host "Wallet (1 AM build): http://127.0.0.1:7070/app" -ForegroundColor Cyan
Write-Host "Do a hard refresh (Ctrl+Shift+R) to clear cache!" -ForegroundColor Yellow
