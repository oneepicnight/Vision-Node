# Create TESTERS zip from the local TESTERS folder.
# Fails if any vision_data* directory is present (packages must never ship a DB).
param(
    [string]$Version = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Version)) {
    if (Test-Path ".\VERSION") {
        $Version = (Get-Content ".\VERSION" -Raw).Trim()
    }
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    Write-Host "ERROR: Version not provided and VERSION file is missing/empty." -ForegroundColor Red
    exit 1
}

$folderName = "VisionNode-Constellation-v$Version-TESTERS"
$folderPath = Join-Path (Get-Location) $folderName
$zipPath = Join-Path (Get-Location) "$folderName.zip"

if (!(Test-Path $folderPath)) {
    Write-Host "ERROR: Folder not found: $folderPath" -ForegroundColor Red
    exit 1
}

$badDataDirs = Get-ChildItem -Path $folderPath -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "vision_data*" }
if ($badDataDirs -and $badDataDirs.Count -gt 0) {
    $names = ($badDataDirs | Select-Object -ExpandProperty Name) -join ", "
    Write-Host "ERROR: Refusing to package: found DB/data folder(s): $names" -ForegroundColor Red
    Write-Host "Delete them first (e.g. run RESET_WINDOWS.bat) and re-try." -ForegroundColor Yellow
    exit 2
}

Remove-Item -Force -ErrorAction SilentlyContinue $zipPath
Compress-Archive -Path $folderPath -DestinationPath $zipPath -CompressionLevel Optimal

Write-Host "OK: Created $zipPath" -ForegroundColor Green
