<#
Build a release of the vision-node and the vision-panel, set VISION_RELEASE=1 for release-mode behavior.

Usage: Run from repository root (scripts folder is inside repo):
  powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\make-release.ps1

This script will:
 - set VISION_RELEASE=1 in the environment for the duration of the script
 - build the Rust binary in --release mode
 - build the vision-panel (npm build) and copy `vision-panel/dist` into `public/`
 - create a zip artifact under `artifacts/`
#>

Set-StrictMode -Version Latest

param(
    [string]$OutputDir = "artifacts",
    [switch]$SkipPanelBuild
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

Write-Output "Setting VISION_RELEASE=1 for this build (used by start scripts and runtime behavior)."
$env:VISION_RELEASE = '1'

Push-Location $repoRoot
try {
    Write-Output "Building Rust release..."
    cargo build --release

    if (-not $SkipPanelBuild) {
        if (Test-Path "vision-panel") {
            Write-Output "Building vision-panel..."
            Push-Location "vision-panel"
            npm ci
            npm run build
            Pop-Location

            # Copy built dist into public/
            $dist = Join-Path $repoRoot "vision-panel\dist"
            $public = Join-Path $repoRoot "public"
            if (Test-Path $dist) {
                Write-Output "Copying panel dist to public/"
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $public '*')
                Copy-Item -Path (Join-Path $dist '*') -Destination $public -Recurse -Force
            } else {
                Write-Warning "vision-panel/dist not found; skipping copy."
            }
        } else {
            Write-Warning "vision-panel directory not found; skipping panel build."
        }
    }

    # Create artifacts directory and zip release
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    $ts = (Get-Date).ToString('yyyyMMdd-HHmmss')
    $zip = Join-Path $OutputDir ("vision-node-release-$ts.zip")
    Write-Output "Packaging release into $zip"
    if (Test-Path $zip) { Remove-Item $zip -Force }

    # Include release binary and public/ assets
    $releaseExe = Join-Path $repoRoot "target\release\vision-node.exe"
    $publicDir = Join-Path $repoRoot "public"
    $tempDir = Join-Path $env:TEMP ("vision-release-$ts")
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tempDir
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    if (Test-Path $releaseExe) { Copy-Item $releaseExe -Destination $tempDir -Force }
    if (Test-Path $publicDir) { Copy-Item -Path (Join-Path $publicDir '*') -Destination (Join-Path $tempDir 'public') -Recurse -Force }

    Compress-Archive -Path (Join-Path $tempDir '*') -DestinationPath $zip -Force
    Remove-Item -Recurse -Force $tempDir

    Write-Output "Release created: $zip"
} finally {
    Pop-Location
}

Write-Output "Done. Note: VISION_RELEASE=1 was set for this build process. When running the packaged binary on a host, you may also set VISION_RELEASE=1 in the environment to use the exe-relative public/ fallback."
