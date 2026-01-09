@echo off
REM Launch 5 Vision nodes in separate external windows

setlocal enabledelayedexpansion

echo === Launching 5-Node Vision Network Test ===
echo.

REM Kill existing nodes
taskkill /f /im vision-node.exe >nul 2>&1

REM Clean DB
echo Cleaning databases...
cd /d c:\vision-node
for /d %%d in (mainnet-*) do (
    rmdir /s /q "%%d" >nul 2>&1
)
timeout /t 2 /nobreak

echo.
echo Launching nodes in external windows...
echo.

REM Node 7070 - MINER (bootstrap node, no seeds)
start "Vision Node 7070 [MINER]" cmd /k "cd /d c:\vision-node && set VISION_PORT=7070 && set VISION_HTTP_PORT=7070 && set VISION_P2P_BIND=127.0.0.1:7072 && set VISION_P2P_PORT=7072 && set VISION_P2P_ADDR=127.0.0.1:7072 && set VISION_ALLOW_PRIVATE_PEERS=true && set VISION_LOCAL_TEST=1 && set VISION_MINER_ADDRESS=VISION_MINER_7070 && set VISION_MIN_PEERS_FOR_MINING=0 && set VISION_MINER_THREADS=8 && set VISION_MIN_DIFFICULTY=1 && set VISION_INITIAL_DIFFICULTY=1 && set VISION_TARGET_BLOCK_SECS=1 && set RUST_LOG=info && echo ===== Node 7070 [MINER] ===== && .\target\release\vision-node.exe"

timeout /t 1 /nobreak

REM Node 8080 - VALIDATOR
start "Vision Node 8080 [VALIDATOR]" cmd /k "cd /d c:\vision-node && set VISION_PORT=8080 && set VISION_HTTP_PORT=8080 && set VISION_P2P_BIND=127.0.0.1:8082 && set VISION_P2P_PORT=8082 && set VISION_P2P_ADDR=127.0.0.1:8082 && set VISION_P2P_SEEDS=127.0.0.1:7072 && set VISION_ALLOW_PRIVATE_PEERS=true && set VISION_MIN_DIFFICULTY=1 && set VISION_INITIAL_DIFFICULTY=1 && set VISION_TARGET_BLOCK_SECS=1 && set RUST_LOG=info && echo ===== Node 8080 [VALIDATOR] ===== && .\target\release\vision-node.exe"

timeout /t 1 /nobreak

REM Node 9090 - VALIDATOR
start "Vision Node 9090 [VALIDATOR]" cmd /k "cd /d c:\vision-node && set VISION_PORT=9090 && set VISION_HTTP_PORT=9090 && set VISION_P2P_BIND=127.0.0.1:9092 && set VISION_P2P_PORT=9092 && set VISION_P2P_ADDR=127.0.0.1:9092 && set VISION_P2P_SEEDS=127.0.0.1:7072 && set VISION_ALLOW_PRIVATE_PEERS=true && set VISION_MIN_DIFFICULTY=1 && set VISION_INITIAL_DIFFICULTY=1 && set VISION_TARGET_BLOCK_SECS=1 && set RUST_LOG=info && echo ===== Node 9090 [VALIDATOR] ===== && .\target\release\vision-node.exe"

timeout /t 1 /nobreak

REM Node 10100 - VALIDATOR
start "Vision Node 10100 [VALIDATOR]" cmd /k "cd /d c:\vision-node && set VISION_PORT=10100 && set VISION_HTTP_PORT=10100 && set VISION_P2P_BIND=127.0.0.1:10102 && set VISION_P2P_PORT=10102 && set VISION_P2P_ADDR=127.0.0.1:10102 && set VISION_P2P_SEEDS=127.0.0.1:7072 && set VISION_ALLOW_PRIVATE_PEERS=true && set VISION_MIN_DIFFICULTY=1 && set VISION_INITIAL_DIFFICULTY=1 && set VISION_TARGET_BLOCK_SECS=1 && set RUST_LOG=info && echo ===== Node 10100 [VALIDATOR] ===== && .\target\release\vision-node.exe"

timeout /t 1 /nobreak

REM Node 11110 - VALIDATOR
start "Vision Node 11110 [VALIDATOR]" cmd /k "cd /d c:\vision-node && set VISION_PORT=11110 && set VISION_HTTP_PORT=11110 && set VISION_P2P_BIND=127.0.0.1:11112 && set VISION_P2P_PORT=11112 && set VISION_P2P_ADDR=127.0.0.1:11112 && set VISION_P2P_SEEDS=127.0.0.1:7072 && set VISION_ALLOW_PRIVATE_PEERS=true && set VISION_MIN_DIFFICULTY=1 && set VISION_INITIAL_DIFFICULTY=1 && set VISION_TARGET_BLOCK_SECS=1 && set RUST_LOG=info && echo ===== Node 11110 [VALIDATOR] ===== && .\target\release\vision-node.exe"

echo.
echo ===================================================
echo ✅ All 5 nodes launched in separate windows!
echo ===================================================
echo.
echo Node 7070 [MINER]:      Watch for [MINER-FOUND] and [MINER-JOB] logs
echo Nodes 8080-11110 [VAL]: Watch for "block_validation CHAIN-POW" accepting 7070's blocks
echo.
echo Expected: All validators reach same height as miner 7070
echo ===================================================
