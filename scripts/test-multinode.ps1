# Multi-node testing script
Write-Host "Starting 3-node network test..." -ForegroundColor Green

# Clean up old data
Write-Host "Cleaning up old data directories..." -ForegroundColor Yellow
Remove-Item -Recurse -Force vision_data_7070, vision_data_7071, vision_data_7072 -ErrorAction SilentlyContinue

# Start Node 1 (miner on port 7070)
Write-Host "Starting Node 1 (Miner) on port 7070..." -ForegroundColor Cyan
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; `$env:VISION_PORT=7070; `$env:VISION_ENABLE_MINING='true'; Write-Host 'NODE 1 - MINER (Port 7070)' -ForegroundColor Green; .\target\release\vision-node.exe"

# Start Node 2 (sync only on port 7071)
Write-Host "Starting Node 2 (Sync) on port 7071..." -ForegroundColor Cyan
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; `$env:VISION_PORT=7071; `$env:VISION_PEERS='http://127.0.0.1:7070'; Write-Host 'NODE 2 - SYNC (Port 7071)' -ForegroundColor Yellow; .\target\release\vision-node.exe"

# Start Node 3 (sync only on port 7072)
Write-Host "Starting Node 3 (Sync) on port 7072..." -ForegroundColor Cyan
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; `$env:VISION_PORT=7072; `$env:VISION_PEERS='http://127.0.0.1:7070'; Write-Host 'NODE 3 - SYNC (Port 7072)' -ForegroundColor Magenta; .\target\release\vision-node.exe"

Write-Host "`nWaiting 20 seconds for nodes to initialize..." -ForegroundColor Yellow
Start-Sleep -Seconds 20

# Monitor node heights
Write-Host "`nMonitoring node synchronization..." -ForegroundColor Green
Write-Host "Press Ctrl+C to stop monitoring`n" -ForegroundColor Gray

for ($i = 0; $i -lt 60; $i++) {
    try {
        $h1 = (Invoke-RestMethod -Uri "http://127.0.0.1:7070/height" -TimeoutSec 2 -ErrorAction SilentlyContinue)
        $h2 = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/height" -TimeoutSec 2 -ErrorAction SilentlyContinue)
        $h3 = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/height" -TimeoutSec 2 -ErrorAction SilentlyContinue)
        
        $timestamp = Get-Date -Format "HH:mm:ss"
        Write-Host "[$timestamp] Node1: $h1 | Node2: $h2 | Node3: $h3" -ForegroundColor White
    }
    catch {
        Write-Host "." -NoNewline -ForegroundColor DarkGray
    }
    
    Start-Sleep -Seconds 3
}

Write-Host "`n`nTest complete! Check the node windows for details." -ForegroundColor Green
Write-Host "To stop nodes: Get-Process vision-node | Stop-Process -Force" -ForegroundColor Yellow
