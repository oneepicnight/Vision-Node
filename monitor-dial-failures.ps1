#!/usr/bin/env pwsh
# Monitor dial failures from both nodes

param(
    [int]$IntervalSeconds = 10,
    [int]$MaxChecks = 30
)

Write-Host "`n=== Dial Failure Monitor ===" -ForegroundColor Cyan
Write-Host "Checking both nodes every $IntervalSeconds seconds...`n"

for ($i = 1; $i -le $MaxChecks; $i++) {
    Write-Host "[Check $i at $(Get-Date -Format 'HH:mm:ss')]" -ForegroundColor Yellow
    
    # Check Node A (7070)
    try {
        $debugA = Invoke-RestMethod "http://localhost:7070/api/p2p/debug" -TimeoutSec 3
        $statusA = Invoke-RestMethod "http://localhost:7070/api/status" -TimeoutSec 3
        
        Write-Host "  Node A:" -ForegroundColor Green
        Write-Host "    Height: $($statusA.chain_height) | Peers: $($statusA.peer_count) | P2P: $($statusA.p2p_health)"
        Write-Host "    Connected: $($debugA.connected_peers.Count) | Dial failures: $($debugA.dial_failures.Count)"
        
        if ($debugA.dial_failures.Count -gt 0) {
            Write-Host "    Recent failures:" -ForegroundColor Red
            $debugA.dial_failures | Select-Object -First 3 | ForEach-Object {
                $time = (Get-Date -UnixTimeSeconds $_.timestamp_unix).ToString('HH:mm:ss')
                Write-Host "      [$time] $($_.addr) - $($_.reason) (source: $($_.source))"
            }
        }
    } catch {
        Write-Host "  Node A: Not responding" -ForegroundColor Red
    }
    
    # Check Node B (8080)
    try {
        $debugB = Invoke-RestMethod "http://localhost:8080/api/p2p/debug" -TimeoutSec 3
        $statusB = Invoke-RestMethod "http://localhost:8080/api/status" -TimeoutSec 3
        
        Write-Host "  Node B:" -ForegroundColor Green
        Write-Host "    Height: $($statusB.chain_height) | Peers: $($statusB.peer_count) | P2P: $($statusB.p2p_health)"
        Write-Host "    Connected: $($debugB.connected_peers.Count) | Dial failures: $($debugB.dial_failures.Count)"
        
        if ($debugB.dial_failures.Count -gt 0) {
            Write-Host "    Recent failures:" -ForegroundColor Red
            $debugB.dial_failures | Select-Object -First 3 | ForEach-Object {
                $time = (Get-Date -UnixTimeSeconds $_.timestamp_unix).ToString('HH:mm:ss')
                Write-Host "      [$time] $($_.addr) - $($_.reason) (source: $($_.source))"
            }
        }
    } catch {
        Write-Host "  Node B: Not responding" -ForegroundColor Red
    }
    
    Write-Host ""
    
    if ($i -lt $MaxChecks) {
        Start-Sleep -Seconds $IntervalSeconds
    }
}

Write-Host "`n=== Monitoring Complete ===" -ForegroundColor Cyan
