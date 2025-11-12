Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  VISION WALLET MARKETPLACE v1.0" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Check if vision-market.exe exists
if (-not (Test-Path ".\vision-market.exe")) {
    Write-Host "ERROR: vision-market.exe not found!" -ForegroundColor Red
    Write-Host "Make sure all files are extracted together." -ForegroundColor Yellow
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}

# Check if dist folder exists
if (-not (Test-Path ".\dist")) {
    Write-Host "ERROR: dist folder not found!" -ForegroundColor Red
    Write-Host "Make sure all files are extracted together." -ForegroundColor Yellow
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "Starting Vision Wallet Marketplace..." -ForegroundColor Green
Write-Host ""
Write-Host "Components:" -ForegroundColor Yellow
Write-Host "  - Frontend:   http://localhost:4173" -ForegroundColor White
Write-Host "  - Backend:    http://localhost:8080" -ForegroundColor White
Write-Host "  - Vision Node: http://localhost:7070 (must be running!)" -ForegroundColor White
Write-Host ""
Write-Host "Features:" -ForegroundColor Yellow
Write-Host "  - LAND Token Marketplace (Buy/Sell)" -ForegroundColor White
Write-Host "  - Exchange Trading (BTC/BCH/DOGE to LAND)" -ForegroundColor White
Write-Host "  - Cash Orders (LAND for Cash)" -ForegroundColor White
Write-Host "  - Wallet & Vault Management" -ForegroundColor White
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

try {
    # Start the backend
    Write-Host "Starting market backend on port 8080..." -ForegroundColor Green
    $backend = Start-Process -FilePath ".\vision-market.exe" -PassThru -WindowStyle Normal
    
    Write-Host "Waiting for backend to initialize (2 seconds)..." -ForegroundColor Yellow
    Start-Sleep -Seconds 2
    
    Write-Host "Backend running! (PID: $($backend.Id))" -ForegroundColor Green
    Write-Host ""
    
    # Start the frontend server on port 4173
    Write-Host "Starting frontend server on port 4173..." -ForegroundColor Green
    
    # Check if frontend_server.exe exists (bundled static file server)
    if (Test-Path ".\frontend_server.exe") {
        Write-Host "Using bundled frontend server..." -ForegroundColor Yellow
        $frontend = Start-Process -FilePath ".\frontend_server.exe" -PassThru -WindowStyle Normal
        Start-Sleep -Seconds 2
        Write-Host "Frontend running! (PID: $($frontend.Id))" -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "ERROR: frontend_server.exe not found!" -ForegroundColor Red
        Write-Host "Make sure frontend_server.exe is extracted with other files." -ForegroundColor Yellow
        Write-Host ""
        Stop-Process -Id $backend.Id -Force
        Read-Host "Press Enter to exit"
        exit 1
    }
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "WALLET IS READY!" -ForegroundColor Green -BackgroundColor DarkGreen
    Write-Host ""
    Write-Host "Services Running:" -ForegroundColor Yellow
    Write-Host "  * Vision Node:  http://localhost:7070 (make sure it is running!)" -ForegroundColor White
    Write-Host "  * Backend:      http://localhost:8080 (PID: $($backend.Id))" -ForegroundColor White
    Write-Host "  * Frontend:     http://localhost:4173 (PID: $($frontend.Id))" -ForegroundColor White
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host " Open browser: http://localhost:4173" -ForegroundColor Cyan -BackgroundColor DarkBlue
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "TO STOP: Press Ctrl+C in this window" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
    
    # Keep both processes running
    Write-Host "Servers running. Press Ctrl+C to stop both..." -ForegroundColor Green
    
    # Wait for either process to exit
    try {
        Wait-Process -Id $backend.Id, $frontend.Id
    } finally {
        # Clean up - stop both if one crashes
        Write-Host ""
        Write-Host "Stopping servers..." -ForegroundColor Yellow
        Stop-Process -Id $backend.Id -ErrorAction SilentlyContinue -Force
        Stop-Process -Id $frontend.Id -ErrorAction SilentlyContinue -Force
        Write-Host "Servers stopped." -ForegroundColor Green
    }
    
} catch {
    Write-Host ""
    Write-Host "ERROR: Failed to start wallet marketplace" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}
