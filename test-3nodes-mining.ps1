# 3-Node Local Test Script for Vision Node v3.0.0
# Tests: Peer connections, mining system, and freeze prevention

Write-Host "ðŸ§ª Vision Node 3-Node Local Test" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

# Clean up any existing test data
Write-Host "ðŸ§¹ Cleaning up old test data..." -ForegroundColor Yellow
Remove-Item ".\vision_data_7070" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item ".\vision_data_7071" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item ".\vision_data_7072" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item ".\node1.log" -Force -ErrorAction SilentlyContinue
Remove-Item ".\node2.log" -Force -ErrorAction SilentlyContinue
Remove-Item ".\node3.log" -Force -ErrorAction SilentlyContinue

# Stop any running nodes
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

Write-Host "âœ… Cleanup complete" -ForegroundColor Green
Write-Host ""

# Create seed peers config for each node
Write-Host "ðŸ“ Creating seed peer configurations..." -ForegroundColor Yellow

# Node 1 seeds (points to Node 2 and 3)
$seed1 = @{
    seed_peers = @("127.0.0.1:7071", "127.0.0.1:7072")
    min_outbound_connections = 2
    max_outbound_connections = 10
}

# Node 2 seeds (points to Node 1 and 3)
$seed2 = @{
    seed_peers = @("127.0.0.1:7070", "127.0.0.1:7072")
    min_outbound_connections = 2
    max_outbound_connections = 10
}

# Node 3 seeds (points to Node 1 and 2)
$seed3 = @{
    seed_peers = @("127.0.0.1:7070", "127.0.0.1:7071")
    min_outbound_connections = 2
    max_outbound_connections = 10
}

# Create config directories
New-Item -ItemType Directory -Force -Path ".\vision_data_7070\config" | Out-Null
New-Item -ItemType Directory -Force -Path ".\vision_data_7071\config" | Out-Null
New-Item -ItemType Directory -Force -Path ".\vision_data_7072\config" | Out-Null

# Save seed configs
$seed1 | ConvertTo-Json | Set-Content ".\vision_data_7070\config\seed_peers.json"
$seed2 | ConvertTo-Json | Set-Content ".\vision_data_7071\config\seed_peers.json"
$seed3 | ConvertTo-Json | Set-Content ".\vision_data_7072\config\seed_peers.json"

Write-Host "âœ… Seed configurations created" -ForegroundColor Green
Write-Host ""

# Start Node 1 (Port 7070)
Write-Host "ðŸš€ Starting Node 1 (Port 7070)..." -ForegroundColor Cyan
$env:VISION_P2P_PORT = "7070"
$env:VISION_CONTROL_PORT = "8070"
$env:VISION_DATA_DIR = "vision_data_7070"
$env:GENESIS_MODE = "true"
$env:VISION_ALLOW_PRIVATE_PEERS = "true"  # Allow localhost connections
Start-Process -FilePath ".\target\release\vision-node.exe" -RedirectStandardOutput "node1.log" -RedirectStandardError "node1.log" -WindowStyle Hidden
Start-Sleep -Seconds 3

# Start Node 2 (Port 7071)
Write-Host "ðŸš€ Starting Node 2 (Port 7071)..." -ForegroundColor Cyan
$env:VISION_P2P_PORT = "7071"
$env:VISION_CONTROL_PORT = "8071"
$env:VISION_DATA_DIR = "vision_data_7071"
$env:GENESIS_MODE = "true"
Start-Process -FilePath ".\target\release\vision-node.exe" -RedirectStandardOutput "node2.log" -RedirectStandardError "node2.log" -WindowStyle Hidden
Start-Sleep -Seconds 3

# Start Node 3 (Port 7072)
Write-Host "ðŸš€ Starting Node 3 (Port 7072)..." -ForegroundColor Cyan
$env:VISION_P2P_PORT = "7072"
$env:VISION_CONTROL_PORT = "8072"
$env:VISION_DATA_DIR = "vision_data_7072"
$env:GENESIS_MODE = "true"
Start-Process -FilePath ".\target\release\vision-node.exe" -RedirectStandardOutput "node3.log" -RedirectStandardError "node3.log" -WindowStyle Hidden
Start-Sleep -Seconds 5

Write-Host ""
Write-Host "âœ… All 3 nodes started!" -ForegroundColor Green
Write-Host ""
Write-Host "ðŸ“Š Monitoring connections and mining (60 seconds)..." -ForegroundColor Yellow
Write-Host ""

# Monitor for 60 seconds
$startTime = Get-Date
$duration = 60

while (((Get-Date) - $startTime).TotalSeconds -lt $duration) {
    $elapsed = [math]::Round(((Get-Date) - $startTime).TotalSeconds)
    
    Clear-Host
    Write-Host "ðŸ§ª Vision Node 3-Node Local Test - Time: ${elapsed}s / ${duration}s" -ForegroundColor Cyan
    Write-Host "================================================================" -ForegroundColor Cyan
    Write-Host ""
    
    # Check Node 1
    if (Test-Path "node1.log") {
        $node1Peers = Select-String -Path "node1.log" -Pattern "Peers: \d+" | Select-Object -Last 1
        $node1Connected = Select-String -Path "node1.log" -Pattern "âœ… Connected to peer" | Measure-Object | Select-Object -ExpandProperty Count
        $node1Mining = Select-String -Path "node1.log" -Pattern "Won slot" | Measure-Object | Select-Object -ExpandProperty Count
        $node1Skip = Select-String -Path "node1.log" -Pattern "\[DIAL\] SKIP" | Measure-Object | Select-Object -ExpandProperty Count
        $node1Block = Select-String -Path "node1.log" -Pattern "Block \d+ broadcast" | Select-Object -Last 1
        
        Write-Host "Node 1 (7070):" -ForegroundColor Green
        if ($node1Peers) { 
            $peerCount = if ($node1Peers.Line -match "Peers: (\d+)") { $matches[1] } else { "?" }
            Write-Host "  Peer Count: $peerCount" -ForegroundColor White 
        }
        Write-Host "  Connections Made: $node1Connected" -ForegroundColor White
        Write-Host "  Mining Wins: $node1Mining" -ForegroundColor White
        Write-Host "  Dial Skips: $node1Skip" -ForegroundColor $(if ($node1Skip -gt 20) { "Red" } else { "Yellow" })
        if ($node1Block) { Write-Host "  Last: $($node1Block.Line -replace '.*(\[MINING\].*)', '$1')" -ForegroundColor Cyan }
    }
    
    Write-Host ""
    
    # Check Node 2
    if (Test-Path "node2.log") {
        $node2Peers = Select-String -Path "node2.log" -Pattern "Peers: \d+" | Select-Object -Last 1
        $node2Connected = Select-String -Path "node2.log" -Pattern "âœ… Connected to peer" | Measure-Object | Select-Object -ExpandProperty Count
        $node2Mining = Select-String -Path "node2.log" -Pattern "Won slot" | Measure-Object | Select-Object -ExpandProperty Count
        $node2Skip = Select-String -Path "node2.log" -Pattern "\[DIAL\] SKIP" | Measure-Object | Select-Object -ExpandProperty Count
        $node2Block = Select-String -Path "node2.log" -Pattern "Block \d+ broadcast" | Select-Object -Last 1
        
        Write-Host "Node 2 (7071):" -ForegroundColor Green
        if ($node2Peers) { 
            $peerCount = if ($node2Peers.Line -match "Peers: (\d+)") { $matches[1] } else { "?" }
            Write-Host "  Peer Count: $peerCount" -ForegroundColor White 
        }
        Write-Host "  Connections Made: $node2Connected" -ForegroundColor White
        Write-Host "  Mining Wins: $node2Mining" -ForegroundColor White
        Write-Host "  Dial Skips: $node2Skip" -ForegroundColor $(if ($node2Skip -gt 20) { "Red" } else { "Yellow" })
        if ($node2Block) { Write-Host "  Last: $($node2Block.Line -replace '.*(\[MINING\].*)', '$1')" -ForegroundColor Cyan }
    }
    
    Write-Host ""
    
    # Check Node 3
    if (Test-Path "node3.log") {
        $node3Peers = Select-String -Path "node3.log" -Pattern "Peers: \d+" | Select-Object -Last 1
        $node3Connected = Select-String -Path "node3.log" -Pattern "âœ… Connected to peer" | Measure-Object | Select-Object -ExpandProperty Count
        $node3Mining = Select-String -Path "node3.log" -Pattern "Won slot" | Measure-Object | Select-Object -ExpandProperty Count
        $node3Skip = Select-String -Path "node3.log" -Pattern "\[DIAL\] SKIP" | Measure-Object | Select-Object -ExpandProperty Count
        $node3Block = Select-String -Path "node3.log" -Pattern "Block \d+ broadcast" | Select-Object -Last 1
        
        Write-Host "Node 3 (7072):" -ForegroundColor Green
        if ($node3Peers) { 
            $peerCount = if ($node3Peers.Line -match "Peers: (\d+)") { $matches[1] } else { "?" }
            Write-Host "  Peer Count: $peerCount" -ForegroundColor White 
        }
        Write-Host "  Connections Made: $node3Connected" -ForegroundColor White
        Write-Host "  Mining Wins: $node3Mining" -ForegroundColor White
        Write-Host "  Dial Skips: $node3Skip" -ForegroundColor $(if ($node3Skip -gt 20) { "Red" } else { "Yellow" })
        if ($node3Block) { Write-Host "  Last: $($node3Block.Line -replace '.*(\[MINING\].*)', '$1')" -ForegroundColor Cyan }
    }
    
    Write-Host ""
    Write-Host "Press Ctrl+C to stop monitoring early" -ForegroundColor Gray
    
    Start-Sleep -Seconds 5
}

Write-Host ""
Write-Host "ðŸ Test Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "ðŸ“‹ Final Analysis:" -ForegroundColor Yellow
Write-Host ""

# Final counts
$totalConnections = 0
$totalMiningWins = 0
$totalSkips = 0
$freezeDetected = $false
$nodes = @(
    @{Name="Node 1"; Port=7070; Log="node1.log"},
    @{Name="Node 2"; Port=7071; Log="node2.log"},
    @{Name="Node 3"; Port=7072; Log="node3.log"}
)

foreach ($node in $nodes) {
    if (Test-Path $node.Log) {
        $conn = (Select-String -Path $node.Log -Pattern "âœ… Connected to peer").Count
        $mining = (Select-String -Path $node.Log -Pattern "Won slot").Count
        $skips = (Select-String -Path $node.Log -Pattern "\[DIAL\] SKIP").Count
        $blocks = (Select-String -Path $node.Log -Pattern "Block \d+ broadcast").Count
        $duplicate = (Select-String -Path $node.Log -Pattern "duplicate.*first wins").Count
        
        $totalConnections += $conn
        $totalMiningWins += $mining
        $totalSkips += $skips
        
        Write-Host "$($node.Name) (Port $($node.Port)):" -ForegroundColor Cyan
        Write-Host "  Connections: $conn" -ForegroundColor White
        Write-Host "  Mining Wins: $mining" -ForegroundColor White
        Write-Host "  Blocks Broadcast: $blocks" -ForegroundColor White
        Write-Host "  Dial Skips: $skips" -ForegroundColor Yellow
        Write-Host "  Duplicates Prevented: $duplicate" -ForegroundColor Green
        
        # Check for freeze
        $lastBlock = Select-String -Path $node.Log -Pattern "Block \d+ broadcast" | Select-Object -Last 1
        $lastActivity = Select-String -Path $node.Log -Pattern "Won slot|Block.*broadcast|Connected to peer" | Select-Object -Last 1
        
        if ($blocks -gt 5) {
            $timeSinceBlock = ((Get-Date) - $lastBlock.Timestamp).TotalSeconds
            if ($timeSinceBlock -gt 30) {
                Write-Host "  âš ï¸  No blocks broadcast in last 30s - possible freeze" -ForegroundColor Red
                $freezeDetected = $true
            } else {
                Write-Host "  âœ… Active (last block ${timeSinceBlock}s ago)" -ForegroundColor Green
            }
        }
        
        Write-Host ""
    }
}

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "Total Connections Made: $totalConnections" -ForegroundColor White
Write-Host "Total Mining Wins: $totalMiningWins" -ForegroundColor White
Write-Host "Total Dial Skips: $totalSkips" -ForegroundColor Yellow

if ($freezeDetected) {
    Write-Host ""
    Write-Host "âŒ FREEZE DETECTED - Check logs for details" -ForegroundColor Red
} else {
    Write-Host ""
    Write-Host "âœ… NO FREEZES DETECTED - All nodes operating normally" -ForegroundColor Green
}

Write-Host ""
Write-Host "ðŸ“ Log files created:" -ForegroundColor Yellow
Write-Host "  - node1.log (Port 7070)"
Write-Host "  - node2.log (Port 7071)"
Write-Host "  - node3.log (Port 7072)"
Write-Host ""
Write-Host "ðŸ” To analyze specific issues:" -ForegroundColor Yellow
Write-Host "  Select-String -Path node1.log -Pattern 'SKIP|duplicate|MINING|normalized_key'" -ForegroundColor Gray
Write-Host "  Select-String -Path node1.log -Pattern 'Integrating block' | Select-Object -Last 5" -ForegroundColor Gray
Write-Host ""

# Offer to stop nodes
$response = Read-Host "Stop all nodes now? (Y/N)"
if ($response -eq "Y" -or $response -eq "y") {
    Write-Host "Stopping all nodes..." -ForegroundColor Yellow
    Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    Write-Host "All nodes stopped" -ForegroundColor Green
} else {
    Write-Host "Nodes still running. Stop manually with:" -ForegroundColor Gray
    Write-Host "  Get-Process vision-node | Stop-Process -Force" -ForegroundColor Gray
}

