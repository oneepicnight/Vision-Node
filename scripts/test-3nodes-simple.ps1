# Kill any existing nodes
Get-Process | Where-Object {$_.ProcessName -eq "vision-node"} | Stop-Process -Force
Start-Sleep -Seconds 2

# Clean data
Remove-Item -Recurse -Force vision_data_7070 -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force vision_data_7071 -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force vision_data_7072 -ErrorAction SilentlyContinue

Write-Host "Starting Node 1 (7070) - WITH MINING" -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; .\target\release\vision-node.exe --reset --enable-mining --port 7070"
Start-Sleep -Seconds 3

Write-Host "Starting Node 2 (7071) - SYNC ONLY" -ForegroundColor Blue  
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; .\target\release\vision-node.exe --reset --port 7071"
Start-Sleep -Seconds 3

Write-Host "Starting Node 3 (7072) - SYNC ONLY" -ForegroundColor Blue
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd C:\vision-node; .\target\release\vision-node.exe --reset --port 7072"
Start-Sleep -Seconds 10

Write-Host "Nodes started. Waiting for initialization..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check status
$node1 = (Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain/tip").height
$node2 = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain/tip").height
$node3 = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain/tip").height

Write-Host "Initial: Node1=$node1 Node2=$node2 Node3=$node3" -ForegroundColor Cyan

# Add peers
$body = '{"url":"http://127.0.0.1:7070"}'
Invoke-RestMethod -Uri "http://127.0.0.1:7071/sync/add-peer" -Method Post -Body $body -ContentType "application/json" | Out-Null
Invoke-RestMethod -Uri "http://127.0.0.1:7072/sync/add-peer" -Method Post -Body $body -ContentType "application/json" | Out-Null

Write-Host "Peers configured. Monitoring for 60s..." -ForegroundColor Yellow

for ($i = 1; $i -le 12; $i++) {
    Start-Sleep -Seconds 5
    $h1 = (Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain/tip").height
    $h2 = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain/tip").height
    $h3 = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain/tip").height
    Write-Host "[$($i*5)s] Node1: $h1 | Node2: $h2 | Node3: $h3"
}

Write-Host "Test complete! Close node windows manually." -ForegroundColor Green
