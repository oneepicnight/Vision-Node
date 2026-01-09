# Launch 5 nodes in separate external PowerShell windows for live monitoring
# Only node 7070 mines; others are validators

$ErrorActionPreference = 'Stop'

Write-Host '=== Launching 5-Node Test in External Windows ===' -ForegroundColor Cyan

# Kill existing nodes
Get-Process -Name 'vision-node' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# Clean DB
Write-Host 'Cleaning databases...' -ForegroundColor Yellow
Remove-Item -Path "./mainnet-*" -Recurse -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

$nodes = @(
    @{ Port = 7070; P2PPort = 7072; Seeds = '127.0.0.1:8082,127.0.0.1:9092'; Role = 'MINER' },
    @{ Port = 8080; P2PPort = 8082; Seeds = '127.0.0.1:7072'; Role = 'VALIDATOR' },
    @{ Port = 9090; P2PPort = 9092; Seeds = '127.0.0.1:7072'; Role = 'VALIDATOR' },
    @{ Port = 10100; P2PPort = 10102; Seeds = '127.0.0.1:7072'; Role = 'VALIDATOR' },
    @{ Port = 11110; P2PPort = 11112; Seeds = '127.0.0.1:7072'; Role = 'VALIDATOR' }
)

Write-Host "`nLaunching 5 nodes in external windows...`n" -ForegroundColor Green

foreach ($node in $nodes) {
    $port = $node.Port
    $p2p = $node.P2PPort
    $seeds = $node.Seeds
    $role = $node.Role
    
    # Base environment variables
    $env = @(
        "`$env:VISION_PORT='$port'",
        "`$env:VISION_HTTP_PORT='$port'",
        "`$env:VISION_P2P_BIND='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_PORT='$p2p'",
        "`$env:VISION_P2P_ADDR='127.0.0.1:$p2p'",
        "`$env:VISION_P2P_SEEDS='$seeds'",
        "`$env:VISION_ALLOW_PRIVATE_PEERS='true'",
        "`$env:VISION_MIN_DIFFICULTY='1'",
        "`$env:VISION_INITIAL_DIFFICULTY='1'",
        "`$env:VISION_TARGET_BLOCK_SECS='1'",
        "`$env:RUST_LOG='info'"
    )
    
    # Only miner node gets these settings
    if ($role -eq 'MINER') {
        $env += @(
            "`$env:VISION_LOCAL_TEST='1'",
            "`$env:VISION_MINER_ADDRESS='VISION_MINER_7070'",
            "`$env:VISION_MIN_PEERS_FOR_MINING='0'",
            "`$env:VISION_MINER_THREADS='8'"
        )
    }
    
    # Build command
    $cmd = ($env -join '; ') + '; ' + "Write-Host 'Starting Node $port [$role]...' -ForegroundColor Cyan; Set-Location '$PSScriptRoot'; ./target/release/vision-node.exe"
    
    # Launch in new window
    $windowTitle = "Vision Node $port [$role]"
    Start-Process powershell -ArgumentList "-NoExit", "-Command", $cmd -WindowStyle Normal
    
    Write-Host "  ✓ Node $port [$role] launched in external window" -ForegroundColor Green
    Start-Sleep -Seconds 1
}

Write-Host "`n✅ All 5 nodes launched in separate PowerShell windows!" -ForegroundColor Cyan
Write-Host "   Monitor them live and watch the mining + sync activity`n" -ForegroundColor Yellow
Write-Host "Expected behavior:" -ForegroundColor Cyan
Write-Host "  • Node 7070: Shows [MINER-FOUND] and [MINER-JOB] logs" -ForegroundColor White
Write-Host "  • Nodes 8080-11110: Show 'block_validation CHAIN-POW' for blocks from 7070" -ForegroundColor White
Write-Host "  • All nodes should eventually reach the same height as 7070" -ForegroundColor White
