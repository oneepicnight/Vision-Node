# Vision Node Package Testing Script
param([string]$PackageName = "all")

$packages = @(
    @{ Name = "Testnet"; Path = "C:\Users\bighe\Downloads\VisionNode-v0.8.0-testnet-WIN64"; Port = 7070 },
    @{ Name = "Guardian-Local"; Path = "c:\vision-node\VisionNode-v0.8.0-guardian-WIN64"; Port = 7070 },
    @{ Name = "Guardian-Downloads"; Path = "C:\Users\bighe\Downloads\v0.8.0\VisionNode-v0.8.0-guardian-WIN64"; Port = 7070 },
    @{ Name = "Constellation"; Path = "C:\Users\bighe\Downloads\v0.8.0\constellation v0.8.0"; Port = 7070 }
)

$endpoints = @(
    "/api/status", "/api/bootstrap", "/api/chain/status",
    "/api/constellation", "/api/constellation/history", "/api/constellation/new-stars",
    "/api/guardian", "/api/guardian/feed",
    "/api/health", "/api/health/public",
    "/api/mood", "/api/trauma", "/api/patterns",
    "/api/nodes", "/api/nodes/with-identity", "/api/reputation",
    "/api/downloads/visitors", "/api/snapshots/recent"
)

function Test-Package {
    param($Package)
    
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Cyan
    Write-Host "Testing: $($Package.Name)" -ForegroundColor Cyan
    Write-Host "=============================================" -ForegroundColor Cyan
    
    $badDataDirs = Get-ChildItem -Path $Package.Path -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "vision_data*" }
    if ($badDataDirs -and $badDataDirs.Count -gt 0) {
        $names = ($badDataDirs | Select-Object -ExpandProperty Name) -join ", "
        Write-Host "[FAIL] Package contains DB/data folder(s): $names" -ForegroundColor Red
        Write-Host "       Packages must not ship any vision_data* directories." -ForegroundColor Red
        return $false
    }

    $exePath = Join-Path $Package.Path "vision-node.exe"
    
    if (!(Test-Path $exePath)) {
        Write-Host "[FAIL] Binary not found" -ForegroundColor Red
        return $false
    }
    
    $exeInfo = Get-Item $exePath
    $sizeMB = [math]::Round($exeInfo.Length/1MB, 2)
    Write-Host "[OK] Binary: $sizeMB MB @ $($exeInfo.LastWriteTime)" -ForegroundColor Green
    
    $walletPath = Join-Path $Package.Path "wallet\dist\assets"
    if (Test-Path $walletPath) {
        $walletCount = (Get-ChildItem $walletPath -File).Count
        Write-Host "[OK] Wallet: $walletCount files" -ForegroundColor Green
    }
    
    Write-Host ""
    Write-Host "Stopping existing processes..." -ForegroundColor Yellow
    Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    
    Write-Host "Starting node..." -ForegroundColor Cyan
    $env:VISION_PUBLIC_DIR = Join-Path $Package.Path "public"
    $env:VISION_WALLET_DIR = Join-Path $Package.Path "wallet\dist"
    
    $process = Start-Process -FilePath $exePath -WorkingDirectory $Package.Path -PassThru -WindowStyle Hidden
    
    if (!$process) {
        Write-Host "[FAIL] Failed to start process" -ForegroundColor Red
        return $false
    }
    
    Write-Host "Process started (PID: $($process.Id))" -ForegroundColor Green
    Write-Host "Waiting 8 seconds for initialization..." -ForegroundColor Yellow
    Start-Sleep -Seconds 8
    
    $runningProcess = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
    if (!$runningProcess) {
        Write-Host "[FAIL] Process died after startup" -ForegroundColor Red
        return $false
    }
    
    Write-Host "[OK] Process running" -ForegroundColor Green
    Write-Host ""
    Write-Host "Testing API endpoints..." -ForegroundColor Cyan
    
    $successCount = 0
    $failCount = 0
    
    foreach ($endpoint in $endpoints) {
        $url = "http://127.0.0.1:$($Package.Port)$endpoint"
        try {
            $response = Invoke-WebRequest -Uri $url -Method GET -TimeoutSec 5 -ErrorAction Stop
            if ($response.StatusCode -eq 200) {
                Write-Host "  [OK] $endpoint" -ForegroundColor Green
                $successCount++
            } else {
                Write-Host "  [FAIL] $endpoint (Status: $($response.StatusCode))" -ForegroundColor Red
                $failCount++
            }
        } catch {
            Write-Host "  [FAIL] $endpoint" -ForegroundColor Red
            $failCount++
        }
    }
    
    Write-Host ""
    Write-Host "Testing wallet..." -ForegroundColor Cyan
    try {
        $walletResponse = Invoke-WebRequest -Uri "http://127.0.0.1:$($Package.Port)/app" -Method GET -TimeoutSec 5 -ErrorAction Stop
        if ($walletResponse.StatusCode -eq 200) {
            Write-Host "  [OK] Wallet accessible at /app" -ForegroundColor Green
        }
    } catch {
        Write-Host "  [FAIL] Wallet not accessible" -ForegroundColor Red
    }
    
    Write-Host ""
    Write-Host "--- Summary for $($Package.Name) ---" -ForegroundColor Cyan
    Write-Host "API Endpoints: $successCount/$($endpoints.Count) passed" -ForegroundColor $(if ($successCount -eq $endpoints.Count) { "Green" } else { "Yellow" })
    Write-Host "Failed: $failCount" -ForegroundColor $(if ($failCount -gt 0) { "Red" } else { "Gray" })
    
    Write-Host ""
    Write-Host "Stopping test process..." -ForegroundColor Yellow
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    
    return ($successCount -eq $endpoints.Count)
}

Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "  VISION NODE PACKAGE TEST SUITE" -ForegroundColor Cyan
Write-Host "  Testing 4 Windows packages" -ForegroundColor Cyan
Write-Host "  Verifying 18 API endpoints + wallet" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

$allPassed = $true
$results = @()

foreach ($package in $packages) {
    $passed = Test-Package -Package $package
    $results += @{
        Name = $package.Name
        Passed = $passed
    }
    if (!$passed) {
        $allPassed = $false
    }
}

Write-Host ""
Write-Host "=============================================" -ForegroundColor Cyan
Write-Host "  FINAL TEST RESULTS" -ForegroundColor Cyan
Write-Host "=============================================" -ForegroundColor Cyan

foreach ($result in $results) {
    $status = if ($result.Passed) { "[PASS]" } else { "[FAIL]" }
    $color = if ($result.Passed) { "Green" } else { "Red" }
    Write-Host "$status $($result.Name)" -ForegroundColor $color
}

Write-Host ""
if ($allPassed) {
    Write-Host "ALL PACKAGES PASSED!" -ForegroundColor Green
} else {
    Write-Host "SOME PACKAGES FAILED" -ForegroundColor Yellow
}

Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

