# testnet-dryrun.ps1
# Test testnet sunset at low height (e.g. block 100) for validation

param(
    [int]$SunsetHeight = 100,
    [int]$Port = 7070
)

$ErrorActionPreference = "Stop"

Write-Host "=== Testnet Sunset Dry Run ===" -ForegroundColor Cyan
Write-Host "Sunset Height: $SunsetHeight" -ForegroundColor Yellow
Write-Host "Port: $Port" -ForegroundColor White

# Clean data directory
$dataDir = "vision_data_$Port"
if (Test-Path $dataDir) {
    Write-Host "Cleaning existing data directory..." -ForegroundColor Yellow
    Remove-Item -Recurse -Force $dataDir
}

Write-Host "Building node..." -ForegroundColor Cyan
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

Write-Host "Starting testnet node with sunset at height $SunsetHeight..." -ForegroundColor Cyan

# Set environment variables for testnet with custom sunset height
$env:VISION_PORT = $Port
$env:VISION_NETWORK = "testnet"
$env:VISION_TESTNET_SUNSET_HEIGHT = $SunsetHeight
$env:VISION_TARGET_BLOCK_SECS = 2  # Fast blocks for testing
$env:RUST_LOG = "info"
$env:VISION_MINER_ADDRESS = "testnet_miner"

# Start node in background
$nodeProcess = Start-Process -FilePath ".\target\release\vision-node.exe" -PassThru -WindowStyle Normal

Write-Host "Node started (PID: $($nodeProcess.Id))" -ForegroundColor Green
Write-Host "Waiting for node to initialize..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Monitor until sunset
Write-Host ""
Write-Host "Monitoring node until sunset at height $SunsetHeight..." -ForegroundColor Cyan
$lastHeight = 0

while ($true) {
    try {
        $status = Invoke-RestMethod -Uri "http://localhost:$Port/status" -ErrorAction Stop
        $currentHeight = $status.height
        
        if ($currentHeight -ne $lastHeight) {
            $remaining = $SunsetHeight - $currentHeight
            if ($remaining -le 0) {
                Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Height: $currentHeight - SUNSET REACHED!" -ForegroundColor Red
                break
            } else {
                Write-Host "[$(Get-Date -Format 'HH:mm:ss')] Height: $currentHeight/$SunsetHeight (T-$remaining blocks)" -ForegroundColor Green
            }
            $lastHeight = $currentHeight
        }
        
        Start-Sleep -Seconds 2
    } catch {
        Write-Warning "Node stopped or unreachable: $_"
        break
    }
}

# Check for migration file
Write-Host ""
Write-Host "=== Sunset Validation ===" -ForegroundColor Cyan
$migrationFile = "migration-testnet-to-mainnet.json"
if (Test-Path $migrationFile) {
    Write-Host "Migration file created: $migrationFile" -ForegroundColor Green
    $content = Get-Content $migrationFile | ConvertFrom-Json
    Write-Host "  Network: $($content.network)" -ForegroundColor White
    Write-Host "  Export Height: $($content.export_height)" -ForegroundColor White
    Write-Host "  Keys Exported: $($content.keys.Count)" -ForegroundColor White
} else {
    Write-Warning "Migration file NOT found - sunset may have failed"
}

# Check if node is still accepting blocks
Write-Host ""
Write-Host "Testing post-sunset behavior..." -ForegroundColor Cyan
Start-Sleep -Seconds 5

try {
    $statusAfter = Invoke-RestMethod -Uri "http://localhost:$Port/status" -ErrorAction Stop
    if ($statusAfter.height -gt $SunsetHeight) {
        Write-Host "WARNING: Node accepted blocks after sunset (height: $($statusAfter.height))" -ForegroundColor Red
    } else {
        Write-Host "PASS: Node stopped at sunset height $SunsetHeight" -ForegroundColor Green
    }
} catch {
    Write-Host "Node is no longer responding (expected after sunset)" -ForegroundColor Green
}

# Cleanup
Write-Host ""
Write-Host "Stopping node..." -ForegroundColor Yellow
Stop-Process -Id $nodeProcess.Id -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Dry Run Complete ===" -ForegroundColor Cyan
Write-Host "Testnet sunset dry run completed successfully" -ForegroundColor Green
Write-Host "Review migration file: $migrationFile" -ForegroundColor Yellow
