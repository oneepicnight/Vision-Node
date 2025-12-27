# test-guardian-integrity.ps1
# Guardian Binary Integrity Verification Test Script
#
# Purpose: Manually verify Guardian binary integrity by computing SHA-256
#          hash and comparing to guardian_integrity.json manifest.
#
# Usage:
#   .\test-guardian-integrity.ps1
#
# Exit Codes:
#   0 - Integrity OK (hash matches)
#   1 - Integrity FAILED (hash mismatch)
#   2 - Error (manifest or binary not found)

param(
    [string]$BinaryPath = ".\target\release\vision-node.exe",
    [string]$ManifestPath = ".\guardian_integrity.json"
)

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Guardian Binary Integrity Verification" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check if binary exists
if (!(Test-Path $BinaryPath)) {
    Write-Host "❌ ERROR: Binary not found: $BinaryPath" -ForegroundColor Red
    Write-Host ""
    Write-Host "Expected location:" -ForegroundColor Yellow
    Write-Host "  $BinaryPath" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Build the project first:" -ForegroundColor Yellow
    Write-Host "  cargo build --release" -ForegroundColor Yellow
    exit 2
}

# Check if manifest exists
if (!(Test-Path $ManifestPath)) {
    Write-Host "❌ ERROR: Manifest not found: $ManifestPath" -ForegroundColor Red
    Write-Host ""
    Write-Host "Expected location:" -ForegroundColor Yellow
    Write-Host "  $ManifestPath" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Create guardian_integrity.json with:" -ForegroundColor Yellow
    Write-Host '  {' -ForegroundColor Yellow
    Write-Host '    "version": "v0.8.1-testnet",' -ForegroundColor Yellow
    Write-Host '    "expected_sha256": "<hash>"' -ForegroundColor Yellow
    Write-Host '  }' -ForegroundColor Yellow
    exit 2
}

Write-Host "📁 Binary Path: $BinaryPath" -ForegroundColor Gray
Write-Host "📄 Manifest Path: $ManifestPath" -ForegroundColor Gray
Write-Host ""

# Compute SHA-256 hash of binary
Write-Host "🔒 Computing SHA-256 hash of binary..." -ForegroundColor Cyan
try {
    $hash = Get-FileHash -Path $BinaryPath -Algorithm SHA256 -ErrorAction Stop
    $actualHash = $hash.Hash.ToLower()
    Write-Host "   Actual Hash: $actualHash" -ForegroundColor White
} catch {
    Write-Host "❌ ERROR: Failed to compute hash: $_" -ForegroundColor Red
    exit 2
}

# Load manifest
Write-Host ""
Write-Host "📖 Loading manifest..." -ForegroundColor Cyan
try {
    $manifest = Get-Content $ManifestPath -Raw -ErrorAction Stop | ConvertFrom-Json
    $expectedHash = $manifest.expected_sha256.ToLower()
    $version = $manifest.version
    
    Write-Host "   Version: $version" -ForegroundColor White
    Write-Host "   Expected Hash: $expectedHash" -ForegroundColor White
} catch {
    Write-Host "❌ ERROR: Failed to load manifest: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Manifest should be valid JSON:" -ForegroundColor Yellow
    Write-Host '  {' -ForegroundColor Yellow
    Write-Host '    "version": "v0.8.1-testnet",' -ForegroundColor Yellow
    Write-Host '    "expected_sha256": "<hash>"' -ForegroundColor Yellow
    Write-Host '  }' -ForegroundColor Yellow
    exit 2
}

# Compare hashes
Write-Host ""
Write-Host "🔍 Verifying integrity..." -ForegroundColor Cyan

if ($actualHash -eq $expectedHash) {
    Write-Host ""
    Write-Host "✅ INTEGRITY OK" -ForegroundColor Green
    Write-Host ""
    Write-Host "   Version: $version" -ForegroundColor Green
    Write-Host "   Hash: $actualHash" -ForegroundColor Green
    Write-Host ""
    Write-Host "Guardian binary is verified and unmodified." -ForegroundColor Green
    Write-Host ""
    exit 0
} else {
    Write-Host ""
    Write-Host "❌ INTEGRITY FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "   Version: $version" -ForegroundColor Red
    Write-Host "   Expected: $expectedHash" -ForegroundColor Red
    Write-Host "   Actual:   $actualHash" -ForegroundColor Red
    Write-Host ""
    Write-Host "⚠️  WARNING: Guardian binary has been modified!" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Possible reasons:" -ForegroundColor Yellow
    Write-Host "  1. Binary was rebuilt (expected after development)" -ForegroundColor Yellow
    Write-Host "  2. Binary was tampered with (security concern)" -ForegroundColor Yellow
    Write-Host "  3. Manifest is outdated (update guardian_integrity.json)" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "If you just rebuilt, update the manifest:" -ForegroundColor Cyan
    Write-Host "  1. Copy the actual hash: $actualHash" -ForegroundColor Cyan
    Write-Host "  2. Update guardian_integrity.json with the new hash" -ForegroundColor Cyan
    Write-Host "  3. Run this script again to verify" -ForegroundColor Cyan
    Write-Host ""
    
    # Ask if user wants to update manifest
    $updateChoice = Read-Host "Update manifest with new hash? (y/n)"
    if ($updateChoice -eq "y" -or $updateChoice -eq "Y") {
        try {
            $manifest.expected_sha256 = $actualHash
            $manifest | ConvertTo-Json | Set-Content $ManifestPath
            Write-Host ""
            Write-Host "✅ Manifest updated successfully!" -ForegroundColor Green
            Write-Host ""
            Write-Host "New manifest:" -ForegroundColor Cyan
            Write-Host "  Version: $version" -ForegroundColor White
            Write-Host "  Hash: $actualHash" -ForegroundColor White
            Write-Host ""
            exit 0
        } catch {
            Write-Host ""
            Write-Host "❌ ERROR: Failed to update manifest: $_" -ForegroundColor Red
            exit 2
        }
    } else {
        Write-Host ""
        Write-Host "Manifest not updated. Integrity check failed." -ForegroundColor Yellow
        exit 1
    }
}
