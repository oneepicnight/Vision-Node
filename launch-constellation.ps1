# Vision Constellation Node - Launch Script
# Join the network 🌟

Write-Host ""
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🌟  VISION CONSTELLATION NODE - LAUNCH SEQUENCE" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

# Step 1: Verify binary exists
if (-not (Test-Path "c:\vision-node\target\release\vision-node.exe")) {
    Write-Host "❌ Binary not found!" -ForegroundColor Red
    Write-Host "Run: cargo build --release" -ForegroundColor Yellow
    exit 1
}

$binary = Get-Item "c:\vision-node\target\release\vision-node.exe"
Write-Host "✅ Binary found: $([math]::Round($binary.Length/1MB,2)) MB" -ForegroundColor Green
Write-Host "   Built: $($binary.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))" -ForegroundColor Gray
Write-Host ""

# Step 2: Check configuration
Write-Host "🔍 Checking Configuration..." -ForegroundColor Yellow
Write-Host ""

$envFile = "c:\vision-node\.env"
if (Test-Path $envFile) {
    $beaconEndpoint = Get-Content $envFile | Select-String "BEACON_ENDPOINT" | Select-Object -First 1
    if ($beaconEndpoint) {
        Write-Host "   ✅ Beacon Endpoint configured" -ForegroundColor Green
        Write-Host "      $beaconEndpoint" -ForegroundColor Gray
    } else {
        Write-Host "   ⚠️  No BEACON_ENDPOINT in .env" -ForegroundColor Yellow
        Write-Host "      Node will run in standalone mode" -ForegroundColor Gray
    }
} else {
    Write-Host "   ⚠️  No .env file found" -ForegroundColor Yellow
}

Write-Host ""

# Step 3: Kill existing processes
Write-Host "🔄 Checking for existing processes..." -ForegroundColor Yellow
$existing = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue

if ($existing) {
    Write-Host "   Found $($existing.Count) running process(es)" -ForegroundColor Yellow
    Write-Host "   Stopping..." -ForegroundColor Yellow
    Stop-Process -Name "vision-node" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    Write-Host "   ✅ Stopped" -ForegroundColor Green
} else {
    Write-Host "   No existing processes found" -ForegroundColor Gray
}

Write-Host ""

# Step 4: Launch Constellation Node
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host "🌟  LAUNCHING CONSTELLATION NODE" -ForegroundColor Cyan
Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
Write-Host ""

Write-Host "Starting Vision Node..." -ForegroundColor Cyan
Write-Host ""

# Launch in new window
Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    @"
Set-Location 'c:\vision-node'
`$env:VISION_GUARDIAN_MODE='false'
`$env:VISION_UPSTREAM_HTTP_BASE='https://visionworld.tech'
Write-Host '🌟  VISION CONSTELLATION NODE' -ForegroundColor Cyan
Write-Host ''
.\target\release\vision-node.exe
"@
)

Start-Sleep -Seconds 3

# Step 5: Verify launch
Write-Host "🔍 Verifying launch..." -ForegroundColor Yellow
Start-Sleep -Seconds 2

$process = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue

if ($process) {
    Write-Host "   ✅ Constellation Node is ONLINE" -ForegroundColor Green
    Write-Host "   PID: $($process.Id)" -ForegroundColor Gray
    Write-Host ""
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host "✅ CONSTELLATION NODE LAUNCHED" -ForegroundColor Green
    Write-Host "=" -NoNewline; 1..69 | ForEach-Object { Write-Host "=" -NoNewline }; Write-Host ""
    Write-Host ""
    
    Write-Host "Endpoints:" -ForegroundColor Yellow
    Write-Host "   Dashboard: http://127.0.0.1:7070/dashboard.html" -ForegroundColor White
    Write-Host "   Wallet:    http://127.0.0.1:7070/app" -ForegroundColor White
    Write-Host "   Panel:     http://127.0.0.1:7070/panel.html" -ForegroundColor White
    Write-Host "   API:       http://127.0.0.1:7070/api/status" -ForegroundColor White
    Write-Host ""
    Write-Host "Network:" -ForegroundColor Yellow
    Write-Host "   Mode:      Constellation (connects to Guardian beacon)" -ForegroundColor White
    Write-Host "   Upstream:  https://visionworld.tech" -ForegroundColor White
    Write-Host ""
    Write-Host "To stop: Get-Process -Name 'vision-node' | Stop-Process" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Welcome to the constellation. 🌟" -ForegroundColor Cyan
    
} else {
    Write-Host "   ❌ Launch failed" -ForegroundColor Red
    Write-Host "   Check the node window for errors" -ForegroundColor Yellow
    Write-Host ""
}
