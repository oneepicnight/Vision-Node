# Vision Node v1.0 Package Verification Script
# Verifies all files are present and ready to ship

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  VISION NODE v1.0 PACKAGE VERIFICATION" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$allGood = $true

# Check WIN64 Package
Write-Host "Checking WIN64 Package..." -ForegroundColor Yellow
if (Test-Path "VisionNode-Constellation-v1.0-WIN64.zip") {
    $size = [math]::Round((Get-Item "VisionNode-Constellation-v1.0-WIN64.zip").Length/1MB, 2)
    Write-Host "  ✅ VisionNode-Constellation-v1.0-WIN64.zip ($size MB)" -ForegroundColor Green
} else {
    Write-Host "  ❌ WIN64 ZIP not found!" -ForegroundColor Red
    $allGood = $false
}

# Check LINUX64 Package
Write-Host "Checking LINUX64 Package..." -ForegroundColor Yellow
if (Test-Path "VisionNode-Constellation-v1.0-LINUX64.tar.gz") {
    $size = [math]::Round((Get-Item "VisionNode-Constellation-v1.0-LINUX64.tar.gz").Length/1MB, 2)
    Write-Host "  ✅ VisionNode-Constellation-v1.0-LINUX64.tar.gz ($size MB)" -ForegroundColor Green
} else {
    Write-Host "  ❌ LINUX64 tar.gz not found!" -ForegroundColor Red
    $allGood = $false
}

Write-Host ""
Write-Host "Checking WIN64 Contents..." -ForegroundColor Yellow
$win64Files = @(
    "VisionNode-Constellation-v1.0-WIN64\vision-node.exe",
    "VisionNode-Constellation-v1.0-WIN64\VERSION",
    "VisionNode-Constellation-v1.0-WIN64\README.txt",
    "VisionNode-Constellation-v1.0-WIN64\.env",
    "VisionNode-Constellation-v1.0-WIN64\START-PUBLIC-NODE.bat",
    "VisionNode-Constellation-v1.0-WIN64\config",
    "VisionNode-Constellation-v1.0-WIN64\wallet",
    "VisionNode-Constellation-v1.0-WIN64\public"
)

foreach ($file in $win64Files) {
    if (Test-Path $file) {
        $name = Split-Path $file -Leaf
        Write-Host "  ✅ $name" -ForegroundColor Green
    } else {
        $name = Split-Path $file -Leaf
        Write-Host "  ❌ $name missing!" -ForegroundColor Red
        $allGood = $false
    }
}

Write-Host ""
Write-Host "Checking LINUX64 Contents..." -ForegroundColor Yellow
$linux64Files = @(
    "VisionNode-Constellation-v1.0-LINUX64\src",
    "VisionNode-Constellation-v1.0-LINUX64\VERSION",
    "VisionNode-Constellation-v1.0-LINUX64\README.txt",
    "VisionNode-Constellation-v1.0-LINUX64\BUILD_NOTES.txt",
    "VisionNode-Constellation-v1.0-LINUX64\.env",
    "VisionNode-Constellation-v1.0-LINUX64\START-VISION-NODE.sh",
    "VisionNode-Constellation-v1.0-LINUX64\install.sh",
    "VisionNode-Constellation-v1.0-LINUX64\config",
    "VisionNode-Constellation-v1.0-LINUX64\wallet",
    "VisionNode-Constellation-v1.0-LINUX64\public"
)

foreach ($file in $linux64Files) {
    if (Test-Path $file) {
        $name = Split-Path $file -Leaf
        Write-Host "  ✅ $name" -ForegroundColor Green
    } else {
        $name = Split-Path $file -Leaf
        Write-Host "  ❌ $name missing!" -ForegroundColor Red
        $allGood = $false
    }
}

Write-Host ""
Write-Host "Checking Documentation..." -ForegroundColor Yellow
$docs = @(
    "VisionNode-v1.0.0-PACKAGE-VERIFICATION.md",
    "READY-TO-SHIP-v1.0.txt",
    "RELEASE_NOTES_v1.0.md"
)

foreach ($doc in $docs) {
    if (Test-Path $doc) {
        Write-Host "  ✅ $doc" -ForegroundColor Green
    } else {
        Write-Host "  ❌ $doc missing!" -ForegroundColor Red
        $allGood = $false
    }
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
if ($allGood) {
    Write-Host "  ✅ ALL CHECKS PASSED!" -ForegroundColor Green
    Write-Host "  🚀 READY TO SHIP v1.0!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Packages:" -ForegroundColor Yellow
    Write-Host "  📦 VisionNode-Constellation-v1.0-WIN64.zip" -ForegroundColor White
    Write-Host "  📦 VisionNode-Constellation-v1.0-LINUX64.tar.gz" -ForegroundColor White
    Write-Host ""
    Write-Host "Next Steps:" -ForegroundColor Yellow
    Write-Host "  1. Upload to GitHub Releases" -ForegroundColor White
    Write-Host "  2. Announce on Discord" -ForegroundColor White
    Write-Host "  3. Update website" -ForegroundColor White
    Write-Host "  4. Deploy to mainnet" -ForegroundColor White
    Write-Host ""
    Write-Host "🎉 VISION NODE v1.0 IS READY! 🎉" -ForegroundColor Green
} else {
    Write-Host "  ❌ VERIFICATION FAILED!" -ForegroundColor Red
    Write-Host "  Please check missing files above" -ForegroundColor Red
}
Write-Host "========================================" -ForegroundColor Cyan
