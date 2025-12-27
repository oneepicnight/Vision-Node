# Restart Vision Node with the 1 AM wallet build
Write-Host "Stopping Vision Node..." -ForegroundColor Yellow
Stop-Process -Name "vision-node" -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Write-Host "Starting Vision Node with updated wallet..." -ForegroundColor Yellow
$env:VISION_PUBLIC_DIR="c:\vision-node\public"
$env:VISION_WALLET_DIR="c:\vision-node\wallet\dist"

Start-Process -FilePath "c:\vision-node\target\release\vision-node.exe" `
              -WorkingDirectory "c:\vision-node" `
              -WindowStyle Hidden

Start-Sleep -Seconds 3

Write-Host "`n=== Vision Node Started ===" -ForegroundColor Green
Write-Host "Wallet: http://127.0.0.1:7070/app" -ForegroundColor Cyan
Write-Host "Panel:  http://127.0.0.1:7070/panel.html" -ForegroundColor Cyan
Write-Host "`nDo a hard refresh (Ctrl+Shift+R) to clear cache!" -ForegroundColor Yellow
