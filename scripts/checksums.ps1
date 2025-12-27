#!/usr/bin/env pwsh
# Checksum generator for release artifacts

Write-Host "🔐 Vision Node Release Checksums" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan

$artifacts = @(
    "dist\VisionNode-*-WIN64.zip",
    "dist\VisionNode-*-Linux.tar.gz"
)

foreach ($pattern in $artifacts) {
    $files = Get-Item $pattern -ErrorAction SilentlyContinue
    if ($files) {
        foreach ($file in $files) {
            Write-Host "`n📦 $($file.Name)" -ForegroundColor Yellow
            $hash = (Get-FileHash $file -Algorithm SHA256).Hash
            Write-Host "  SHA256: $hash" -ForegroundColor Green
            
            # Also display from .sha256 file if it exists
            $shaFile = "$file.sha256"
            if (Test-Path $shaFile) {
                $savedHash = (Get-Content $shaFile).Trim()
                if ($savedHash -eq $hash) {
                    Write-Host "  ✓ Matches saved checksum" -ForegroundColor Green
                } else {
                    Write-Host "  ⚠ MISMATCH with saved checksum!" -ForegroundColor Red
                }
            }
        }
    } else {
        Write-Host "`n⚠ No files matching: $pattern" -ForegroundColor Yellow
    }
}

Write-Host "`n✅ Checksum verification complete" -ForegroundColor Green
