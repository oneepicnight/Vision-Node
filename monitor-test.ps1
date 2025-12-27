# monitor-test.ps1 - Monitor ongoing 20-minute test
# Run this in a separate terminal to check progress

$nodeA = "http://localhost:7070"
$nodeB = "http://localhost:8080"

Write-Host "`n=== Two-Node Test Monitor ===" -ForegroundColor Cyan
Write-Host "Monitoring started: $(Get-Date)" -ForegroundColor Yellow
Write-Host "Press Ctrl+C to stop monitoring`n" -ForegroundColor Gray

while ($true) {
    try {
        Clear-Host
        Write-Host "=== Status at $(Get-Date) ===" -ForegroundColor Cyan
        
        # Node A Status
        Write-Host "`n--- Node A (localhost:7070) ---" -ForegroundColor Green
        try {
            $statusA = Invoke-RestMethod -Uri "$nodeA/api/status" -TimeoutSec 3
            Write-Host "  Height: $($statusA.chain_height)" -ForegroundColor White
            Write-Host "  Peers: $($statusA.peer_count)" -ForegroundColor White
            Write-Host "  P2P Health: $($statusA.p2p_health)" -ForegroundColor $(if ($statusA.p2p_health -eq 'healthy') {'Green'} else {'Yellow'})
            Write-Host "  Sync Status: $($statusA.sync_status)" -ForegroundColor White
            Write-Host "  Behind Blocks: $($statusA.behind_blocks)" -ForegroundColor White
            
            $minerA = Invoke-RestMethod -Uri "$nodeA/api/miner/status" -TimeoutSec 3
            Write-Host "  Mining: $($minerA.enabled)" -ForegroundColor White
            Write-Host "  Blocks Found: $($minerA.blocks_found)" -ForegroundColor White
            Write-Host "  Blocks Accepted: $($minerA.blocks_accepted)" -ForegroundColor White
            Write-Host "  Hashrate: $([math]::Round($minerA.hashrate, 2)) H/s" -ForegroundColor White
        } catch {
            Write-Host "  ERROR: Cannot reach Node A" -ForegroundColor Red
            Write-Host "  $($_.Exception.Message)" -ForegroundColor Red
        }
        
        # Node B Status
        Write-Host "`n--- Node B (localhost:8080) ---" -ForegroundColor Green
        try {
            $statusB = Invoke-RestMethod -Uri "$nodeB/api/status" -TimeoutSec 3
            Write-Host "  Height: $($statusB.chain_height)" -ForegroundColor White
            Write-Host "  Peers: $($statusB.peer_count)" -ForegroundColor White
            Write-Host "  P2P Health: $($statusB.p2p_health)" -ForegroundColor $(if ($statusB.p2p_health -eq 'healthy') {'Green'} else {'Yellow'})
            Write-Host "  Sync Status: $($statusB.sync_status)" -ForegroundColor White
            Write-Host "  Behind Blocks: $($statusB.behind_blocks)" -ForegroundColor White
            
            $minerB = Invoke-RestMethod -Uri "$nodeB/api/miner/status" -TimeoutSec 3
            Write-Host "  Mining: $($minerB.enabled)" -ForegroundColor White
            Write-Host "  Blocks Found: $($minerB.blocks_found)" -ForegroundColor White
            Write-Host "  Blocks Accepted: $($minerB.blocks_accepted)" -ForegroundColor White
            Write-Host "  Hashrate: $([math]::Round($minerB.hashrate, 2)) H/s" -ForegroundColor White
        } catch {
            Write-Host "  ERROR: Cannot reach Node B" -ForegroundColor Red
            Write-Host "  $($_.Exception.Message)" -ForegroundColor Red
        }
        
        # Consensus Check
        Write-Host "`n--- Consensus Check ---" -ForegroundColor Cyan
        if ($statusA -and $statusB) {
            $heightDiff = [math]::Abs($statusA.chain_height - $statusB.chain_height)
            if ($heightDiff -le 1) {
                Write-Host "  ✅ Heights in sync (diff: $heightDiff)" -ForegroundColor Green
            } else {
                Write-Host "  ⚠️ Height difference: $heightDiff blocks" -ForegroundColor Yellow
            }
            
            if ($statusA.chain_id -eq $statusB.chain_id) {
                Write-Host "  ✅ Same chain ID" -ForegroundColor Green
            } else {
                Write-Host "  ❌ FORK: Different chain IDs!" -ForegroundColor Red
            }
            
            # Compare top block hashes
            try {
                $blockA = Invoke-RestMethod -Uri "$nodeA/block/$($statusA.chain_height)" -TimeoutSec 3
                $blockB = Invoke-RestMethod -Uri "$nodeB/block/$($statusB.chain_height)" -TimeoutSec 3
                
                if ($statusA.chain_height -eq $statusB.chain_height) {
                    if ($blockA.header.pow_hash -eq $blockB.header.pow_hash) {
                        Write-Host "  ✅ Same tip hash at height $($statusA.chain_height)" -ForegroundColor Green
                    } else {
                        Write-Host "  ❌ FORK: Different tip hashes at same height!" -ForegroundColor Red
                        Write-Host "     Node A: $($blockA.header.pow_hash)" -ForegroundColor Gray
                        Write-Host "     Node B: $($blockB.header.pow_hash)" -ForegroundColor Gray
                    }
                }
            } catch {
                Write-Host "  ⚠️ Could not compare block hashes" -ForegroundColor Yellow
            }
        }
        
        # P2P Check
        Write-Host "`n--- P2P Health ---" -ForegroundColor Cyan
        if ($statusA -and $statusB) {
            $totalPeers = $statusA.peer_count + $statusB.peer_count
            if ($totalPeers -ge 2) {
                Write-Host "  ✅ P2P connected ($totalPeers total peer connections)" -ForegroundColor Green
            } else {
                Write-Host "  ⚠️ P2P weak ($totalPeers total peer connections)" -ForegroundColor Yellow
            }
        }
        
        Write-Host "`n--- Log Files ---" -ForegroundColor Cyan
        $logFiles = Get-ChildItem -Path "." -Filter "test-2node-20min-*.log" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($logFiles) {
            Write-Host "  Latest log: $($logFiles.Name)" -ForegroundColor White
            Write-Host "  Size: $([math]::Round($logFiles.Length / 1KB, 2)) KB" -ForegroundColor White
            Write-Host "  Last updated: $($logFiles.LastWriteTime)" -ForegroundColor White
        }
        
        Write-Host "`n(Refreshing every 30 seconds...)" -ForegroundColor Gray
        Start-Sleep -Seconds 30
        
    } catch {
        Write-Host "`nERROR in monitor: $($_.Exception.Message)" -ForegroundColor Red
        Start-Sleep -Seconds 10
    }
}
