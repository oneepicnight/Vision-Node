# Vision Node Ghost Diagnosis Script
# Helps identify if wrong/old vision-node.exe is running

Write-Host "`n" "=" * 70 -ForegroundColor Cyan
Write-Host " VISION NODE GHOST DIAGNOSIS" -ForegroundColor Yellow
Write-Host " " "=" * 70 "`n" -ForegroundColor Cyan

Write-Host "[Step 1] Checking what's listening on port 7072..." -ForegroundColor Yellow
$listening = netstat -ano | Select-String ":7072.*LISTENING"
if ($listening) {
    Write-Host $listening -ForegroundColor Green
    
    # Extract PID
    $pid = ($listening -split '\s+')[-1]
    Write-Host "`nPID listening on 7072: $pid" -ForegroundColor Cyan
    
    Write-Host "`n[Step 2] Getting process details..." -ForegroundColor Yellow
    try {
        $process = Get-Process -Id $pid -ErrorAction Stop
        Write-Host "  Process Name: " -NoNewline -ForegroundColor White
        Write-Host $process.ProcessName -ForegroundColor Green
        Write-Host "  Process Path: " -NoNewline -ForegroundColor White
        Write-Host $process.Path -ForegroundColor Green
        Write-Host "  Start Time:   " -NoNewline -ForegroundColor White
        Write-Host $process.StartTime -ForegroundColor Green
        
        # Check if it's in current directory
        $currentDir = Get-Location
        if ($process.Path -like "$currentDir*") {
            Write-Host "`n✅ GOOD: Process is running from current directory" -ForegroundColor Green
        } else {
            Write-Host "`n❌ PROBLEM: Process is NOT from current directory!" -ForegroundColor Red
            Write-Host "   Expected: $currentDir\vision-node.exe" -ForegroundColor Yellow
            Write-Host "   Actual:   $($process.Path)" -ForegroundColor Red
            Write-Host "`n   This is the GHOST NODE! Kill it with:" -ForegroundColor Yellow
            Write-Host "   taskkill /PID $pid /F" -ForegroundColor Cyan
        }
    } catch {
        Write-Host "  Could not get process details: $_" -ForegroundColor Red
    }
} else {
    Write-Host "✅ Nothing is listening on port 7072" -ForegroundColor Green
}

Write-Host "`n[Step 3] Finding ALL vision-node.exe processes..." -ForegroundColor Yellow
$allProcesses = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
if ($allProcesses) {
    Write-Host "Found $($allProcesses.Count) vision-node process(es):" -ForegroundColor Cyan
    foreach ($proc in $allProcesses) {
        Write-Host "`n  PID: $($proc.Id)" -ForegroundColor White
        Write-Host "  Path: $($proc.Path)" -ForegroundColor Gray
        Write-Host "  Start: $($proc.StartTime)" -ForegroundColor Gray
    }
    
    Write-Host "`n[Step 4] Kill ALL vision-node processes?" -ForegroundColor Yellow
    Write-Host "  This will terminate all running vision-node.exe instances." -ForegroundColor Gray
    $confirm = Read-Host "  Kill all? (yes/no)"
    
    if ($confirm -eq "yes") {
        Write-Host "`n  Killing all vision-node processes..." -ForegroundColor Red
        taskkill /IM vision-node.exe /F 2>&1 | Out-Null
        Start-Sleep -Seconds 2
        
        # Verify
        $remaining = Get-Process -Name "vision-node" -ErrorAction SilentlyContinue
        if ($remaining) {
            Write-Host "  ❌ Some processes still running!" -ForegroundColor Red
        } else {
            Write-Host "  ✅ All vision-node processes terminated" -ForegroundColor Green
        }
        
        # Check port again
        $stillListening = netstat -ano | Select-String ":7072.*LISTENING"
        if ($stillListening) {
            Write-Host "  ❌ Port 7072 still in use!" -ForegroundColor Red
        } else {
            Write-Host "  ✅ Port 7072 is now free" -ForegroundColor Green
        }
    } else {
        Write-Host "  Skipped killing processes" -ForegroundColor Gray
    }
} else {
    Write-Host "✅ No vision-node processes found" -ForegroundColor Green
}

Write-Host "`n[Step 5] Check router port forwarding..." -ForegroundColor Yellow
Write-Host "  Get your local IP address:" -ForegroundColor White
$localIP = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -like "192.168.*" -or $_.IPAddress -like "10.*" } | Select-Object -First 1).IPAddress
if ($localIP) {
    Write-Host "  Local IPv4: " -NoNewline -ForegroundColor Gray
    Write-Host $localIP -ForegroundColor Cyan
    Write-Host "`n  Make sure your router forwards:" -ForegroundColor Yellow
    Write-Host "    External port 7072 → $localIP:7072" -ForegroundColor Green
} else {
    Write-Host "  Could not detect local IP" -ForegroundColor Red
}

Write-Host "`n[Step 6] Verify binary genesis hash..." -ForegroundColor Yellow
Write-Host "  Expected genesis (v1.0.1): " -NoNewline -ForegroundColor White
Write-Host "e7580fd06f67c98ab5e912f51c63b4f013a7bcbe37693fe9ec9cac57f5b8bb24" -ForegroundColor Green
Write-Host "`n  Start your node and check logs for:" -ForegroundColor Gray
Write-Host '    [GENESIS] compiled_genesis_hash=...' -ForegroundColor Cyan
Write-Host '    [CHAIN] db_genesis_hash=...' -ForegroundColor Cyan
Write-Host "`n  If these don't match expected, you have the wrong binary!" -ForegroundColor Yellow

Write-Host "`n" "=" * 70 -ForegroundColor Cyan
Write-Host " NEXT STEPS" -ForegroundColor Yellow
Write-Host " " "=" * 70 "`n" -ForegroundColor Cyan
Write-Host "1. If ghost node found, kill it and restart from correct directory" -ForegroundColor White
Write-Host "2. Verify router forwards 7072 to correct internal IP" -ForegroundColor White
Write-Host "3. Start node and check genesis logs match v1.0.1" -ForegroundColor White
Write-Host "4. Test connection: netstat -ano | findstr `":7072.*ESTABLISHED`"" -ForegroundColor White
Write-Host ""
