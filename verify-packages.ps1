# Vision Node Package Verification Script
# Verifies all download packages have updated binaries and wallets

Write-Host "=== VISION NODE PACKAGE VERIFICATION ===" -ForegroundColor Cyan
Write-Host ""

$sourceExe = "c:\vision-node\target\release\vision-node.exe"
$sourceWallet = "C:\Users\bighe\Desktop\guardian\wallet\dist"

# Check source binary
if (!(Test-Path $sourceExe)) {
    Write-Host "ERROR: Source binary not found!" -ForegroundColor Red
    Write-Host "Run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

$srcInfo = Get-Item $sourceExe
Write-Host "SOURCE BINARY:" -ForegroundColor Yellow
Write-Host "  Path: $sourceExe" -ForegroundColor White
Write-Host "  Size: $([math]::Round($srcInfo.Length/1MB, 2)) MB" -ForegroundColor White
Write-Host "  Modified: $($srcInfo.LastWriteTime)" -ForegroundColor White
Write-Host ""

# Check source wallet
if (!(Test-Path $sourceWallet)) {
    Write-Host "ERROR: Source wallet not found!" -ForegroundColor Red
    exit 1
}

$walletFiles = (Get-ChildItem "$sourceWallet\assets" -File).Count
Write-Host "SOURCE WALLET:" -ForegroundColor Yellow
Write-Host "  Path: $sourceWallet" -ForegroundColor White
Write-Host "  Asset Files: $walletFiles" -ForegroundColor White
Write-Host "  Has /wallet fix: " -NoNewline -ForegroundColor White
$testJs = Get-ChildItem "$sourceWallet\assets\index-64968bc5.js" -ErrorAction SilentlyContinue
if ($testJs) {
    $content = Get-Content $testJs.FullName -Raw
    if ($content -notmatch '"/home"') {
        Write-Host "YES" -ForegroundColor Green
    } else {
        Write-Host "NO - needs fix!" -ForegroundColor Red
    }
} else {
    Write-Host "UNKNOWN" -ForegroundColor Yellow
}
Write-Host ""

# Packages to verify
$packages = @(
    @{
        Name = "Testnet WIN64"
        ExePath = "C:\Users\bighe\Downloads\VisionNode-v0.8.0-testnet-WIN64\vision-node.exe"
        WalletPath = "C:\Users\bighe\Downloads\VisionNode-v0.8.0-testnet-WIN64\wallet\dist"
    },
    @{
        Name = "Guardian WIN64 (c:\vision-node\)"
        ExePath = "c:\vision-node\VisionNode-v0.8.0-guardian-WIN64\vision-node.exe"
        WalletPath = "c:\vision-node\VisionNode-v0.8.0-guardian-WIN64\wallet\dist"
    },
    @{
        Name = "Guardian WIN64 (Downloads)"
        ExePath = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-v0.8.0-guardian-WIN64\vision-node.exe"
        WalletPath = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-v0.8.0-guardian-WIN64\wallet\dist"
    },
    @{
        Name = "Constellation WIN64"
        ExePath = "C:\Users\bighe\Downloads\v0.8.0\constellation v0.8.0\vision-node.exe"
        WalletPath = "C:\Users\bighe\Downloads\v0.8.0\constellation v0.8.0\wallet\dist"
    }
)

Write-Host "=== PACKAGE VERIFICATION ===" -ForegroundColor Cyan
Write-Host ""

$allGood = $true
$needsUpdate = @()

foreach ($pkg in $packages) {
    Write-Host "Package: $($pkg.Name)" -ForegroundColor Yellow
    
    # Check binary
    if (Test-Path $pkg.ExePath) {
        $pkgInfo = Get-Item $pkg.ExePath
        $sizeMatch = [math]::Abs($pkgInfo.Length - $srcInfo.Length) -lt 1024
        $dateMatch = ($pkgInfo.LastWriteTime - $srcInfo.LastWriteTime).TotalSeconds -ge 0
        
        if ($sizeMatch -and ($pkgInfo.LastWriteTime -ge $srcInfo.LastWriteTime.AddMinutes(-5))) {
            Write-Host "  Binary: " -NoNewline -ForegroundColor White
            Write-Host "OK" -ForegroundColor Green
        } else {
            Write-Host "  Binary: " -NoNewline -ForegroundColor White
            Write-Host "OUTDATED" -ForegroundColor Red
            Write-Host "    Package: $([math]::Round($pkgInfo.Length/1MB, 2)) MB @ $($pkgInfo.LastWriteTime)" -ForegroundColor Gray
            Write-Host "    Source:  $([math]::Round($srcInfo.Length/1MB, 2)) MB @ $($srcInfo.LastWriteTime)" -ForegroundColor Gray
            $allGood = $false
            $needsUpdate += @{Pkg=$pkg.Name; Type="Binary"}
        }
    } else {
        Write-Host "  Binary: " -NoNewline -ForegroundColor White
        Write-Host "MISSING" -ForegroundColor Red
        $allGood = $false
        $needsUpdate += @{Pkg=$pkg.Name; Type="Binary"}
    }
    
    # Check wallet
    if (Test-Path $pkg.WalletPath) {
        $pkgWalletFiles = (Get-ChildItem "$($pkg.WalletPath)\assets" -File -ErrorAction SilentlyContinue).Count
        if ($pkgWalletFiles -eq $walletFiles) {
            Write-Host "  Wallet: " -NoNewline -ForegroundColor White
            Write-Host "OK ($pkgWalletFiles files)" -ForegroundColor Green
        } else {
            Write-Host "  Wallet: " -NoNewline -ForegroundColor White
            Write-Host "MISMATCH ($pkgWalletFiles vs $walletFiles files)" -ForegroundColor Red
            $allGood = $false
            $needsUpdate += @{Pkg=$pkg.Name; Type="Wallet"}
        }
    } else {
        Write-Host "  Wallet: " -NoNewline -ForegroundColor White
        Write-Host "MISSING" -ForegroundColor Red
        $allGood = $false
        $needsUpdate += @{Pkg=$pkg.Name; Type="Wallet"}
    }
    
    Write-Host ""
}

# Check Linux package
Write-Host "Package: Linux Constellation" -ForegroundColor Yellow
$linuxTarPath = "C:\Users\bighe\Downloads\VisionNode-Constellation-v0.8.0-LINUX64-FIXED-UPDATED.tar.gz"
if (Test-Path $linuxTarPath) {
    $tarInfo = Get-Item $linuxTarPath
    Write-Host "  Archive: " -NoNewline -ForegroundColor White
    Write-Host "EXISTS ($([math]::Round($tarInfo.Length/1MB, 2)) MB)" -ForegroundColor Green
    Write-Host "  Modified: $($tarInfo.LastWriteTime)" -ForegroundColor Gray
} else {
    Write-Host "  Archive: " -NoNewline -ForegroundColor White
    Write-Host "MISSING" -ForegroundColor Red
    $allGood = $false
}
Write-Host ""

# Check ZIP file
Write-Host "Package: Constellation WIN64 ZIP" -ForegroundColor Yellow
$zipPath = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-Constellation-v0.8.0-WIN64.zip"
if (Test-Path $zipPath) {
    $zipInfo = Get-Item $zipPath
    Write-Host "  Archive: " -NoNewline -ForegroundColor White
    Write-Host "EXISTS ($([math]::Round($zipInfo.Length/1MB, 2)) MB)" -ForegroundColor Green
    Write-Host "  Modified: $($zipInfo.LastWriteTime)" -ForegroundColor Gray
} else {
    Write-Host "  Archive: " -NoNewline -ForegroundColor White
    Write-Host "MISSING" -ForegroundColor Red
    $allGood = $false
}
Write-Host ""

# Summary
Write-Host "=== SUMMARY ===" -ForegroundColor Cyan
if ($allGood) {
    Write-Host "ALL PACKAGES UP TO DATE!" -ForegroundColor Green
} else {
    Write-Host "PACKAGES NEED UPDATES:" -ForegroundColor Yellow
    $needsUpdate | ForEach-Object {
        Write-Host "  - $($_.Pkg): $($_.Type)" -ForegroundColor Red
    }
}
