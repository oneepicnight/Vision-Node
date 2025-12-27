# reset-data.ps1
# Safely clear all vision_data_* directories for clean restart
# Preserves configuration files and logs

param(
    [switch]$Force,
    [switch]$PreserveKeys
)

$ErrorActionPreference = "Stop"

Write-Host "=== Vision Node Data Reset ===" -ForegroundColor Cyan

# Find all vision_data_* directories
$dataDirs = Get-ChildItem -Directory -Filter "vision_data_*" -ErrorAction SilentlyContinue

if ($dataDirs.Count -eq 0) {
    Write-Host "No data directories found (vision_data_*)" -ForegroundColor Green
    exit 0
}

Write-Host "Found $($dataDirs.Count) data directories:" -ForegroundColor Yellow
foreach ($dir in $dataDirs) {
    Write-Host "  - $($dir.Name)" -ForegroundColor White
}

if (-not $Force) {
    Write-Host ""
    $confirmation = Read-Host "Delete all data directories? This will erase the blockchain state! (yes/no)"
    if ($confirmation -ne "yes") {
        Write-Host "Operation cancelled" -ForegroundColor Yellow
        exit 0
    }
}

# Preserve keys if requested
if ($PreserveKeys) {
    Write-Host "Backing up keys..." -ForegroundColor Cyan
    $backupDir = "backups/keys_$(Get-Date -Format 'yyyyMMdd_HHmmss')"
    New-Item -ItemType Directory -Force -Path $backupDir | Out-Null
    
    foreach ($dir in $dataDirs) {
        $keysPath = Join-Path $dir.FullName "keys"
        if (Test-Path $keysPath) {
            $destPath = Join-Path $backupDir $dir.Name
            Copy-Item -Recurse -Force $keysPath $destPath
            Write-Host "  Backed up keys from $($dir.Name)" -ForegroundColor Green
        }
    }
    Write-Host "Keys backed up to: $backupDir" -ForegroundColor Green
}

# Stop any running vision-node processes
Write-Host "Stopping any running vision-node processes..." -ForegroundColor Yellow
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Delete data directories
Write-Host "Deleting data directories..." -ForegroundColor Yellow
foreach ($dir in $dataDirs) {
    try {
        Remove-Item -Recurse -Force $dir.FullName
        Write-Host "  Deleted $($dir.Name)" -ForegroundColor Green
    } catch {
        Write-Warning "Failed to delete $($dir.Name): $_"
    }
}

Write-Host ""
Write-Host "=== Data Reset Complete ===" -ForegroundColor Cyan
Write-Host "All vision_data_* directories have been removed" -ForegroundColor Green
if ($PreserveKeys) {
    Write-Host "Keys backed up to: $backupDir" -ForegroundColor Yellow
}
Write-Host "Next start will create fresh genesis blocks" -ForegroundColor White
