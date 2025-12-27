# Quick Network Health Check
# Checks status of all running Vision nodes

$ErrorActionPreference = "SilentlyContinue"

function Get-NodeInfo($port) {
    try {
        $chain = Invoke-RestMethod -Uri "http://127.0.0.1:$port/chain" -TimeoutSec 2
        $peers = Invoke-RestMethod -Uri "http://127.0.0.1:$port/peers" -TimeoutSec 2
        
        return @{
            online = $true
            height = $chain.height
            hash = $chain.best_hash.Substring(0, 8)
            difficulty = $chain.difficulty
            peer_count = $peers.peers.Count
        }
    } catch {
        return @{ online = $false }
    }
}

Write-Host "`n🏥 Vision Node Network Health Check" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════`n" -ForegroundColor Cyan

$ports = @(7070, 7071, 7072)
$nodes = @()

foreach ($port in $ports) {
    $info = Get-NodeInfo $port
    $nodes += $info
    
    if ($info.online) {
        Write-Host "✅ Node $port" -ForegroundColor Green -NoNewline
        Write-Host " | H:$($info.height) | Hash:$($info.hash)... | D:$($info.difficulty) | Peers:$($info.peer_count)"
    } else {
        Write-Host "❌ Node $port - OFFLINE" -ForegroundColor Red
    }
}

$online_nodes = $nodes | Where-Object { $_.online }

if ($online_nodes.Count -eq 0) {
    Write-Host "`n⚠️  No nodes are running!" -ForegroundColor Yellow
    Write-Host "   Start with: .\test-3nodes-sync.ps1`n" -ForegroundColor Gray
    exit 0
}

Write-Host "`n📊 Network Status:" -ForegroundColor Cyan

# Check consensus
$heights = $online_nodes | ForEach-Object { $_.height } | Sort-Object -Unique
$hashes = $online_nodes | ForEach-Object { $_.hash } | Sort-Object -Unique

if ($heights.Count -eq 1 -and $hashes.Count -eq 1) {
    Write-Host "   ✅ Perfect consensus - all nodes synced" -ForegroundColor Green
} elseif ($heights.Count -eq 1) {
    Write-Host "   ⚠️  Same height but different hashes (racing blocks?)" -ForegroundColor Yellow
} else {
    $min_height = ($heights | Measure-Object -Minimum).Minimum
    $max_height = ($heights | Measure-Object -Maximum).Maximum
    $gap = $max_height - $min_height
    Write-Host "   ⚠️  Height mismatch - Gap: $gap blocks" -ForegroundColor Yellow
}

# Check if mining is happening
Start-Sleep -Seconds 3
$new_info = Get-NodeInfo 7070
if ($new_info.online -and $new_info.height -gt $online_nodes[0].height) {
    Write-Host "   ⛏️  Mining active - new blocks being produced" -ForegroundColor Green
} else {
    Write-Host "   ⚠️  No new blocks produced (mining may be off)" -ForegroundColor Yellow
}

Write-Host ""
