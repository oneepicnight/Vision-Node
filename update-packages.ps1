# Vision Node Package Update Script
# Updates all packages with latest binary and wallet

param(
    [switch]$BinaryOnly,
    [switch]$WalletOnly
)

Write-Host "=== VISION NODE PACKAGE UPDATER ===" -ForegroundColor Cyan
Write-Host ""

$sourceExe = "c:\vision-node\target\release\vision-node.exe"
$sourceWallet = "C:\Users\bighe\Desktop\guardian\wallet\dist"

# Verify source files
if (!(Test-Path $sourceExe)) {
    Write-Host "ERROR: Source binary not found at: $sourceExe" -ForegroundColor Red
    Write-Host "Run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

if (!(Test-Path $sourceWallet) -and !$BinaryOnly) {
    Write-Host "ERROR: Source wallet not found at: $sourceWallet" -ForegroundColor Red
    exit 1
}

$srcInfo = Get-Item $sourceExe
Write-Host "Source Binary: $([math]::Round($srcInfo.Length/1MB, 2)) MB @ $($srcInfo.LastWriteTime)" -ForegroundColor Green
if (!$BinaryOnly) {
    $walletCount = (Get-ChildItem "$sourceWallet\assets" -File).Count
    Write-Host "Source Wallet: $walletCount files" -ForegroundColor Green
}
Write-Host ""

# Package definitions
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

$updated = 0
$failed = 0

foreach ($pkg in $packages) {
    Write-Host "Updating: $($pkg.Name)" -ForegroundColor Yellow
    
    # Update binary
    if (!$WalletOnly) {
        try {
            $pkgDir = Split-Path $pkg.ExePath -Parent
            if (!(Test-Path $pkgDir)) {
                New-Item -ItemType Directory -Path $pkgDir -Force | Out-Null
            }
            Copy-Item -Path $sourceExe -Destination $pkg.ExePath -Force
            Write-Host "  Binary: UPDATED" -ForegroundColor Green
            $updated++
        } catch {
            Write-Host "  Binary: FAILED - $($_.Exception.Message)" -ForegroundColor Red
            $failed++
        }
    }
    
    # Update wallet
    if (!$BinaryOnly) {
        try {
            if (Test-Path $pkg.WalletPath) {
                Remove-Item $pkg.WalletPath -Recurse -Force
            }
            Copy-Item -Path $sourceWallet -Destination $pkg.WalletPath -Recurse -Force
            $count = (Get-ChildItem "$($pkg.WalletPath)\assets" -File).Count
            Write-Host "  Wallet: UPDATED ($count files)" -ForegroundColor Green
            $updated++
        } catch {
            Write-Host "  Wallet: FAILED - $($_.Exception.Message)" -ForegroundColor Red
            $failed++
        }
    }
    Write-Host ""
}

# Update Linux package
if (!$BinaryOnly) {
    Write-Host "Updating: Linux Constellation" -ForegroundColor Yellow
    try {
        $linuxDir = "C:\Users\bighe\Downloads\VisionNode-Constellation-v0.8.0-LINUX64"
        $linuxWallet = "$linuxDir\wallet\dist"
        
        if (Test-Path $linuxWallet) {
            Remove-Item $linuxWallet -Recurse -Force
        }
        Copy-Item -Path $sourceWallet -Destination $linuxWallet -Recurse -Force
        
        Write-Host "  Wallet: UPDATED" -ForegroundColor Green
        Write-Host "  Repackaging tar.gz..." -ForegroundColor Yellow
        
        Push-Location "C:\Users\bighe\Downloads"
        $tarPath = "VisionNode-Constellation-v0.8.0-LINUX64-FIXED-UPDATED.tar.gz"
        if (Test-Path $tarPath) { Remove-Item $tarPath -Force }
        tar -czf $tarPath VisionNode-Constellation-v0.8.0-LINUX64
        $tarSize = (Get-Item $tarPath).Length / 1MB
        Write-Host "  Archive: CREATED ($([math]::Round($tarSize, 2)) MB)" -ForegroundColor Green
        Pop-Location
        $updated++
    } catch {
        Write-Host "  Linux: FAILED - $($_.Exception.Message)" -ForegroundColor Red
        $failed++
    }
    Write-Host ""
}

# Update ZIP
if (!$BinaryOnly) {
    Write-Host "Updating: Constellation WIN64 ZIP" -ForegroundColor Yellow
    try {
        # Extract, update, repackage
        $zipPath = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-Constellation-v0.8.0-WIN64.zip"
        $tempDir = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-Constellation-v0.8.0-WIN64-TEMP"
        
        if (Test-Path $tempDir) { Remove-Item $tempDir -Recurse -Force }
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
        
        $zipWallet = "$tempDir\constellation v0.8.0\wallet\dist"
        Remove-Item $zipWallet -Recurse -Force
        Copy-Item -Path $sourceWallet -Destination $zipWallet -Recurse -Force
        
        Push-Location "C:\Users\bighe\Downloads\v0.8.0"
        Remove-Item $zipPath -Force
        Compress-Archive -Path "$tempDir\constellation v0.8.0" -DestinationPath $zipPath -CompressionLevel Optimal
        Remove-Item $tempDir -Recurse -Force
        $zipSize = (Get-Item $zipPath).Length / 1MB
        Write-Host "  Archive: UPDATED ($([math]::Round($zipSize, 2)) MB)" -ForegroundColor Green
        Pop-Location
        $updated++
    } catch {
        Write-Host "  ZIP: FAILED - $($_.Exception.Message)" -ForegroundColor Red
        $failed++
    }
    Write-Host ""
}

# Summary
Write-Host "=== SUMMARY ===" -ForegroundColor Cyan
Write-Host "Updated: $updated" -ForegroundColor Green
Write-Host "Failed: $failed" -ForegroundColor $(if ($failed -gt 0) { "Red" } else { "Gray" })
Write-Host ""
Write-Host "Run verify-packages.ps1 to confirm updates" -ForegroundColor Yellow
