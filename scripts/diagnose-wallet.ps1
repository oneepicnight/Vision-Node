# Vision Wallet - Diagnostic Script
# Run this to check why the wallet isn't loading

$ErrorActionPreference = "Continue"

Write-Host "`n+--------------------------------------------------------------+" -ForegroundColor Cyan
Write-Host "|          VISION WALLET - Diagnostic Check                     |" -ForegroundColor Cyan
Write-Host "+--------------------------------------------------------------+`n" -ForegroundColor Cyan

$installDir = Join-Path $env:LOCALAPPDATA "VisionBlockchain"
$walletDir = Join-Path $installDir "public\wallet"

# Check 1: Is Vision Node installed?
Write-Host "[1] Checking Vision Node installation..." -ForegroundColor Yellow
if (Test-Path $installDir) {
    Write-Host "    [OK] Node directory found: $installDir" -ForegroundColor Green
    
    if (Test-Path (Join-Path $installDir "vision-node.exe")) {
        Write-Host "    [OK] vision-node.exe found" -ForegroundColor Green
    } else {
        Write-Host "    [ERROR] vision-node.exe NOT FOUND!" -ForegroundColor Red
        Write-Host "    ACTION: Install Vision Node first (VisionNode-v1.0.zip)" -ForegroundColor Yellow
    }
} else {
    Write-Host "    [ERROR] Vision Node NOT installed!" -ForegroundColor Red
    Write-Host "    ACTION: Install Vision Node first (VisionNode-v1.0.zip)`n" -ForegroundColor Yellow
    pause
    exit 1
}

# Check 2: Is Vision Node running?
Write-Host "`n[2] Checking if Vision Node is running..." -ForegroundColor Yellow
try {
    $response = Invoke-WebRequest -Uri "http://localhost:7070/api/admin/ping" -Method GET -TimeoutSec 2 -ErrorAction Stop
    Write-Host "    [OK] Node is running and responding" -ForegroundColor Green
} catch {
    Write-Host "    [ERROR] Node is NOT running!" -ForegroundColor Red
    Write-Host "    ACTION: Start the node by double-clicking 'Vision Node' desktop icon" -ForegroundColor Yellow
    Write-Host "    Or run: $installDir\Start-VisionNode.bat`n" -ForegroundColor Gray
}

# Check 3: Is wallet installed?
Write-Host "`n[3] Checking wallet installation..." -ForegroundColor Yellow
if (Test-Path $walletDir) {
    Write-Host "    [OK] Wallet directory found: $walletDir" -ForegroundColor Green
    
    $requiredFiles = @(
        @{Name="index.html"; Path=(Join-Path $walletDir "index.html")},
        @{Name="vite.svg"; Path=(Join-Path $walletDir "vite.svg")},
        @{Name="assets"; Path=(Join-Path $walletDir "assets")}
    )
    
    $allFound = $true
    foreach ($file in $requiredFiles) {
        if (Test-Path $file.Path) {
            Write-Host "    [OK] $($file.Name) found" -ForegroundColor Green
        } else {
            Write-Host "    [ERROR] $($file.Name) NOT FOUND!" -ForegroundColor Red
            $allFound = $false
        }
    }
    
    if (-not $allFound) {
        Write-Host "`n    ACTION: Reinstall wallet (run INSTALL.bat from VisionWallet-v1.0.zip)" -ForegroundColor Yellow
    }
} else {
    Write-Host "    [ERROR] Wallet NOT installed!" -ForegroundColor Red
    Write-Host "    ACTION: Run INSTALL.bat from VisionWallet-v1.0.zip`n" -ForegroundColor Yellow
}

# Check 4: Test wallet access
Write-Host "`n[4] Testing wallet access..." -ForegroundColor Yellow
try {
    $walletResponse = Invoke-WebRequest -Uri "http://localhost:7070/wallet/" -Method GET -TimeoutSec 2 -ErrorAction Stop
    if ($walletResponse.StatusCode -eq 200) {
        Write-Host "    [OK] Wallet is accessible at http://localhost:7070/wallet/" -ForegroundColor Green
        Write-Host "`n+--------------------------------------------------------------+" -ForegroundColor Green
        Write-Host "|                  WALLET IS WORKING!                           |" -ForegroundColor Green
        Write-Host "+--------------------------------------------------------------+" -ForegroundColor Green
        Write-Host "`nOpen in browser: http://localhost:7070/wallet/`n" -ForegroundColor Cyan
    }
} catch {
    $statusCode = $_.Exception.Response.StatusCode.Value__
    if ($statusCode -eq 404) {
        Write-Host "    [ERROR] 404 - Wallet files not found by web server" -ForegroundColor Red
        Write-Host "`n    This means:" -ForegroundColor Yellow
        Write-Host "    - Node is running (good)" -ForegroundColor Gray
        Write-Host "    - But wallet files are missing from public/wallet/ directory" -ForegroundColor Gray
        Write-Host "`n    ACTION: Run INSTALL.bat from VisionWallet-v1.0.zip" -ForegroundColor Yellow
    } else {
        Write-Host "    [ERROR] HTTP $statusCode" -ForegroundColor Red
    }
}

# Summary
Write-Host "`n+--------------------------------------------------------------+" -ForegroundColor Cyan
Write-Host "|                        SUMMARY                                |" -ForegroundColor Cyan
Write-Host "+--------------------------------------------------------------+" -ForegroundColor Cyan
Write-Host "`nFor the wallet to work, you need:" -ForegroundColor White
Write-Host "  1. Vision Node INSTALLED (VisionNode-v1.0.zip)" -ForegroundColor Gray
Write-Host "  2. Vision Node RUNNING (double-click desktop icon)" -ForegroundColor Gray
Write-Host "  3. Vision Wallet INSTALLED (VisionWallet-v1.0.zip)" -ForegroundColor Gray
Write-Host "`nThe wallet is served BY the node at http://localhost:7070/wallet/" -ForegroundColor White
Write-Host "It's not a separate server - it needs the node running!`n" -ForegroundColor White

Write-Host "Press any key to exit..." -ForegroundColor Cyan
pause | Out-Null
