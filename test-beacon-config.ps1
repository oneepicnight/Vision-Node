#!/usr/bin/env pwsh
# ================================================================
#  BEACON CONFIGURATION VALIDATION TEST
#  Verifies Guardian Beacon URL is hard-wired in all packages
# ================================================================

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$TARGET_URL = "https://visionworld.tech"

Write-Host "`n================================================================" -ForegroundColor Cyan
Write-Host "  BEACON CONFIGURATION VALIDATION TEST" -ForegroundColor Cyan
Write-Host "================================================================`n" -ForegroundColor Cyan
Write-Host "Target Guardian Beacon URL: $TARGET_URL" -ForegroundColor Green
Write-Host ""

$results = @()
$passed = 0
$failed = 0

function Test-File {
    param($Path, $SearchPattern, $Description)
    
    if (!(Test-Path $Path)) {
        $results += [PSCustomObject]@{
            File = $Path
            Status = "❌ NOT FOUND"
            Description = $Description
        }
        $script:failed++
        return
    }
    
    $content = Get-Content $Path -Raw
    if ($content -match $SearchPattern) {
        $results += [PSCustomObject]@{
            File = Split-Path $Path -Leaf
            Status = "✅ PASS"
            Description = $Description
        }
        $script:passed++
        
        if ($Verbose) {
            Write-Host "✅ $Description" -ForegroundColor Green
            Write-Host "   File: $Path" -ForegroundColor Gray
        }
    } else {
        $results += [PSCustomObject]@{
            File = Split-Path $Path -Leaf
            Status = "❌ FAIL"
            Description = $Description
        }
        $script:failed++
        Write-Host "❌ FAILED: $Description" -ForegroundColor Red
        Write-Host "   File: $Path" -ForegroundColor Gray
        Write-Host "   Missing: $SearchPattern" -ForegroundColor Yellow
    }
}

Write-Host "Testing Windows Packages..." -ForegroundColor Yellow
Write-Host ""

# Test Windows v0.8.1 Constellation
Test-File `
    -Path "c:\vision-node\VisionNode-v0.8.1-constellation-testnet-WIN64\START-VISION-NODE.bat" `
    -SearchPattern "set BEACON_ENDPOINT=https://visionworld\.tech" `
    -Description "Windows v0.8.1 Constellation - START-VISION-NODE.bat"

# Test Windows v0.8.1 Guardian
Test-File `
    -Path "c:\vision-node\VisionNode-v0.8.1-guardian-WIN64\START-GUARDIAN-NODE.bat" `
    -SearchPattern "set BEACON_ENDPOINT=https://visionworld\.tech" `
    -Description "Windows v0.8.1 Guardian - START-GUARDIAN-NODE.bat"

# Test Windows v0.8.0 Guardian
Test-File `
    -Path "c:\vision-node\VisionNode-v0.8.0-guardian-WIN64\START-GUARDIAN-NODE.bat" `
    -SearchPattern "set BEACON_ENDPOINT=https://visionworld\.tech" `
    -Description "Windows v0.8.0 Guardian - START-GUARDIAN-NODE.bat"

Write-Host ""
Write-Host "Testing Linux Packages..." -ForegroundColor Yellow
Write-Host ""

# Test Linux v0.8.1 Constellation startup script
Test-File `
    -Path "c:\vision-node\VisionNode-Constellation-v0.8.1-LINUX64\START-VISION-NODE.sh" `
    -SearchPattern 'export BEACON_ENDPOINT="https://visionworld\.tech"' `
    -Description "Linux v0.8.1 Constellation - START-VISION-NODE.sh"

# Test Linux v0.8.1 Constellation installer
Test-File `
    -Path "c:\vision-node\VisionNode-Constellation-v0.8.1-LINUX64\install.sh" `
    -SearchPattern 'export BEACON_ENDPOINT="https://visionworld\.tech"' `
    -Description "Linux v0.8.1 Constellation - install.sh"

# Test Linux v0.8.0 Constellation startup script
Test-File `
    -Path "c:\vision-node\VisionNode-Constellation-v0.8.0-LINUX64\START-VISION-NODE.sh" `
    -SearchPattern 'export BEACON_ENDPOINT="https://visionworld\.tech"' `
    -Description "Linux v0.8.0 Constellation - START-VISION-NODE.sh"

# Test Linux v0.8.0 Constellation installer
Test-File `
    -Path "c:\vision-node\VisionNode-Constellation-v0.8.0-LINUX64\install.sh" `
    -SearchPattern 'export BEACON_ENDPOINT="https://visionworld\.tech"' `
    -Description "Linux v0.8.0 Constellation - install.sh"

Write-Host ""
Write-Host "Testing Bootstrap Configuration..." -ForegroundColor Yellow
Write-Host ""

# Test seed_peers.toml files
$tomlFiles = @(
    "c:\vision-node\config\seed_peers.toml",
    "c:\vision-node\VisionNode-v0.8.1-constellation-testnet-WIN64\config\seed_peers.toml",
    "c:\vision-node\VisionNode-v0.8.1-guardian-WIN64\config\seed_peers.toml",
    "c:\vision-node\VisionNode-v0.8.0-guardian-WIN64\config\seed_peers.toml",
    "c:\vision-node\VisionNode-Constellation-v0.8.1-LINUX64\config\seed_peers.toml",
    "c:\vision-node\VisionNode-Constellation-v0.8.0-LINUX64\config\seed_peers.toml"
)

foreach ($tomlPath in $tomlFiles) {
    if (Test-Path $tomlPath) {
        $relPath = $tomlPath -replace [regex]::Escape("c:\vision-node\"), ""
        Test-File `
            -Path $tomlPath `
            -SearchPattern 'bootstrap_url = "https://visionworld\.tech/api/bootstrap"' `
            -Description "Bootstrap URL - $relPath"
    }
}

Write-Host ""
Write-Host "Testing Development Scripts..." -ForegroundColor Yellow
Write-Host ""

# Test root development scripts
Test-File `
    -Path "c:\vision-node\START-VISION-NODE.sh" `
    -SearchPattern 'export BEACON_ENDPOINT="https://visionworld\.tech"' `
    -Description "Root dev - START-VISION-NODE.sh"

Test-File `
    -Path "c:\vision-node\START-GUARDIAN-NODE.bat" `
    -SearchPattern "set BEACON_ENDPOINT=https://visionworld\.tech" `
    -Description "Root dev - START-GUARDIAN-NODE.bat"

# Results Summary
Write-Host ""
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "  TEST RESULTS" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host ""

$results | Format-Table -AutoSize

Write-Host ""
if ($failed -eq 0) {
    Write-Host "✅ ALL TESTS PASSED ($passed/$($passed + $failed))" -ForegroundColor Green
    Write-Host ""
    Write-Host "Guardian Beacon URL successfully hard-wired in all packages!" -ForegroundColor Green
    Write-Host ""
    Write-Host "VALIDATION:" -ForegroundColor Cyan
    Write-Host "  1. Windows packages include: set BEACON_ENDPOINT=https://visionworld.tech" -ForegroundColor White
    Write-Host "  2. Linux packages include:   export BEACON_ENDPOINT=\"https://visionworld.tech\"" -ForegroundColor White
    Write-Host "  3. Bootstrap configs use:    https://visionworld.tech/api/bootstrap" -ForegroundColor White
    Write-Host ""
    Write-Host "EXPECTED LOGS ON STARTUP:" -ForegroundColor Cyan
    Write-Host "  Constellation nodes: [NETWORK] Connecting to beacon at: https://visionworld.tech" -ForegroundColor White
    Write-Host "  Guardian nodes:      [BEACON] Guardian beacon started - broadcasting network heartbeats" -ForegroundColor White
    Write-Host ""
    exit 0
} else {
    Write-Host "❌ TESTS FAILED: $failed/$($passed + $failed)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Some files are missing the Guardian Beacon URL." -ForegroundColor Yellow
    Write-Host "Run the update script to fix configuration issues." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}
