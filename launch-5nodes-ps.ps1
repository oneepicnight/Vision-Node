Write-Host "=== Launching 5-Node Vision Network Test ===" -ForegroundColor Cyan
Write-Host ""

# Kill existing nodes
Write-Host "Cleaning up existing nodes..." -ForegroundColor Yellow
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# Clean databases
Write-Host "Cleaning databases..." -ForegroundColor Yellow
cd c:\vision-node
Get-ChildItem -Directory -Filter "mainnet-*" | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

Write-Host ""
Write-Host "Launching nodes in separate PowerShell windows..." -ForegroundColor Cyan
Write-Host ""

# Node 7070 - MINER
$miner_cmd = @"
cd c:\vision-node
`$env:VISION_PORT='7070'
`$env:VISION_HTTP_PORT='7070'
`$env:VISION_P2P_BIND='127.0.0.1:7072'
`$env:VISION_P2P_PORT='7072'
`$env:VISION_P2P_ADDR='127.0.0.1:7072'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_LOCAL_TEST='1'
`$env:VISION_MINER_ADDRESS='VISION_MINER_7070'
`$env:VISION_MIN_PEERS_FOR_MINING='0'
`$env:VISION_MINER_THREADS='8'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:RUST_LOG='info'
Write-Host "===== Node 7070 [MINER] =====" -ForegroundColor Green
.\target\release\vision-node.exe
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $miner_cmd -WindowStyle Normal
Start-Sleep -Seconds 1

# Node 8080 - VALIDATOR
$val_8080_cmd = @"
cd c:\vision-node
`$env:VISION_PORT='8080'
`$env:VISION_HTTP_PORT='8080'
`$env:VISION_P2P_BIND='127.0.0.1:8082'
`$env:VISION_P2P_PORT='8082'
`$env:VISION_P2P_ADDR='127.0.0.1:8082'
`$env:VISION_P2P_SEEDS='127.0.0.1:7072'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:RUST_LOG='info'
Write-Host "===== Node 8080 [VALIDATOR] =====" -ForegroundColor Blue
.\target\release\vision-node.exe
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $val_8080_cmd -WindowStyle Normal
Start-Sleep -Seconds 1

# Node 9090 - VALIDATOR
$val_9090_cmd = @"
cd c:\vision-node
`$env:VISION_PORT='9090'
`$env:VISION_HTTP_PORT='9090'
`$env:VISION_P2P_BIND='127.0.0.1:9092'
`$env:VISION_P2P_PORT='9092'
`$env:VISION_P2P_ADDR='127.0.0.1:9092'
`$env:VISION_P2P_SEEDS='127.0.0.1:7072'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:RUST_LOG='info'
Write-Host "===== Node 9090 [VALIDATOR] =====" -ForegroundColor Blue
.\target\release\vision-node.exe
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $val_9090_cmd -WindowStyle Normal
Start-Sleep -Seconds 1

# Node 10100 - VALIDATOR
$val_10100_cmd = @"
cd c:\vision-node
`$env:VISION_PORT='10100'
`$env:VISION_HTTP_PORT='10100'
`$env:VISION_P2P_BIND='127.0.0.1:10102'
`$env:VISION_P2P_PORT='10102'
`$env:VISION_P2P_ADDR='127.0.0.1:10102'
`$env:VISION_P2P_SEEDS='127.0.0.1:7072'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:RUST_LOG='info'
Write-Host "===== Node 10100 [VALIDATOR] =====" -ForegroundColor Blue
.\target\release\vision-node.exe
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $val_10100_cmd -WindowStyle Normal
Start-Sleep -Seconds 1

# Node 11110 - VALIDATOR
$val_11110_cmd = @"
cd c:\vision-node
`$env:VISION_PORT='11110'
`$env:VISION_HTTP_PORT='11110'
`$env:VISION_P2P_BIND='127.0.0.1:11112'
`$env:VISION_P2P_PORT='11112'
`$env:VISION_P2P_ADDR='127.0.0.1:11112'
`$env:VISION_P2P_SEEDS='127.0.0.1:7072'
`$env:VISION_ALLOW_PRIVATE_PEERS='true'
`$env:VISION_MIN_DIFFICULTY='1'
`$env:VISION_INITIAL_DIFFICULTY='1'
`$env:VISION_TARGET_BLOCK_SECS='1'
`$env:RUST_LOG='info'
Write-Host "===== Node 11110 [VALIDATOR] =====" -ForegroundColor Blue
.\target\release\vision-node.exe
"@

Start-Process powershell -ArgumentList "-NoExit", "-Command", $val_11110_cmd -WindowStyle Normal

Write-Host ""
Write-Host "====================================================" -ForegroundColor Green
Write-Host "✅ All 5 nodes launched in separate windows!" -ForegroundColor Green
Write-Host "====================================================" -ForegroundColor Green
Write-Host ""
Write-Host "Starting miner on node 7070..." -ForegroundColor Yellow
Start-Sleep -Seconds 3

# Start the miner
$miner_response = Invoke-WebRequest -Uri "http://127.0.0.1:7070/api/miner/start" `
    -Method POST `
    -Body '{"threads": 8}' `
    -ContentType 'application/json' `
    -UseBasicParsing `
    -ErrorAction SilentlyContinue

Write-Host "Miner started on node 7070!" -ForegroundColor Green
Write-Host ""
Write-Host "Node 7070 [MINER]:      Watch for [MINER-FOUND] and blocks_found increasing" -ForegroundColor Yellow
Write-Host "Nodes 8080-11110 [VAL]: Watch for block_validation and height increases" -ForegroundColor Yellow
Write-Host ""
Write-Host "Expected: All validators reach same height as miner 7070" -ForegroundColor Yellow
Write-Host "====================================================" -ForegroundColor Green
