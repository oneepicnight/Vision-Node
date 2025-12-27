# Quick Fix: Copy wallet into public/ directory
# Run this on the computer where wallet isn't working

$installDir = Join-Path $env:LOCALAPPDATA "VisionBlockchain"
$publicWalletDir = Join-Path $installDir "public\wallet"

Write-Host "Fixing wallet access..." -ForegroundColor Yellow

if (Test-Path (Join-Path $installDir "wallet")) {
    New-Item -ItemType Directory -Path $publicWalletDir -Force | Out-Null
    Copy-Item -Path (Join-Path $installDir "wallet\*") -Destination $publicWalletDir -Force
    Write-Host "[OK] Wallet files copied to public/wallet/" -ForegroundColor Green
    Write-Host "`nWallet should now work at: http://localhost:7070/wallet/" -ForegroundColor Cyan
    Write-Host "`nIf node is running, restart it to see changes." -ForegroundColor Yellow
} else {
    Write-Host "[ERROR] Wallet directory not found in installation" -ForegroundColor Red
    Write-Host "Re-run the installer to get wallet files." -ForegroundColor Yellow
}

pause
