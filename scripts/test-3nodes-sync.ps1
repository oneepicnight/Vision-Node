# Multi-Node P2P Testing Script
# Tests 3 nodes: one miner (7070), two syncing nodes (7071, 7072)
# Tests block propagation, sync, and network consensus

param(
    [switch]$Clean,
    [switch]$NoMining,
    [int]$WaitSeconds = 30
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Color($text, $color = "White") {
    Write-Host $text -ForegroundColor $color
}

Write-Color "`n🌐 Vision Node Multi-Node Test" "Cyan"
Write-Color "================================`n" "Cyan"

# Clean up old processes
Write-Color "🧹 Cleaning up old processes..." "Yellow"
Get-Process -Name "vision-node" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2

# Clean data directories if requested
if ($Clean) {
    Write-Color "🗑️  Cleaning data directories..." "Yellow"
    Remove-Item -Path "vision_data_7070" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "vision_data_7071" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "vision_data_7072" -Recurse -Force -ErrorAction SilentlyContinue
}

# Build if needed
if (-not (Test-Path ".\target\release\vision-node.exe")) {
    Write-Color "🔨 Building vision-node..." "Yellow"
    cargo build --release --bin vision-node
    if ($LASTEXITCODE -ne 0) {
        Write-Color "❌ Build failed!" "Red"
        exit 1
    }
}

Write-Color "`n🚀 Starting nodes..." "Green"
Write-Color "─────────────────────`n" "Green"

# Node 1: Miner on port 7070
Write-Color "📍 Node 1 (Miner): http://127.0.0.1:7070" "Green"
$env:VISION_PORT = "7070"
$env:VISION_DATA_DIR = "vision_data_7070"
$env:VISION_AUTOSYNC_SECS = "5"

if ($NoMining) {
    Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--reset", "--port", "7070" -WindowStyle Minimized
} else {
    Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--reset", "--enable-mining", "--port", "7070" -WindowStyle Minimized
}

Start-Sleep -Seconds 3

# Node 2: Peer on port 7071 (syncs from 7070)
Write-Color "📍 Node 2 (Peer):  http://127.0.0.1:7071" "Green"
$env:VISION_PORT = "7071"
$env:VISION_DATA_DIR = "vision_data_7071"
$env:VISION_AUTOSYNC_SECS = "3"

Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--reset", "--port", "7071" -WindowStyle Minimized

Start-Sleep -Seconds 3

# Node 3: Peer on port 7072 (syncs from 7070 and 7071)
Write-Color "📍 Node 3 (Peer):  http://127.0.0.1:7072" "Green"
$env:VISION_PORT = "7072"
$env:VISION_DATA_DIR = "vision_data_7072"
$env:VISION_AUTOSYNC_SECS = "3"

Start-Process -FilePath ".\target\release\vision-node.exe" -ArgumentList "--reset", "--port", "7072" -WindowStyle Minimized

Start-Sleep -Seconds 3

Write-Color "`n✅ All nodes started!`n" "Green"

# Configure peer connections
Write-Color "🔗 Configuring peer connections..." "Cyan"

# Node 1 knows about Node 2 and Node 3
Invoke-RestMethod -Uri "http://127.0.0.1:7070/peers" -Method POST -Body (@{peer="http://127.0.0.1:7071"} | ConvertTo-Json) -ContentType "application/json" | Out-Null
Invoke-RestMethod -Uri "http://127.0.0.1:7070/peers" -Method POST -Body (@{peer="http://127.0.0.1:7072"} | ConvertTo-Json) -ContentType "application/json" | Out-Null

# Node 2 knows about Node 1 and Node 3
Invoke-RestMethod -Uri "http://127.0.0.1:7071/peers" -Method POST -Body (@{peer="http://127.0.0.1:7070"} | ConvertTo-Json) -ContentType "application/json" | Out-Null
Invoke-RestMethod -Uri "http://127.0.0.1:7071/peers" -Method POST -Body (@{peer="http://127.0.0.1:7072"} | ConvertTo-Json) -ContentType "application/json" | Out-Null

# Node 3 knows about Node 1 and Node 2
Invoke-RestMethod -Uri "http://127.0.0.1:7072/peers" -Method POST -Body (@{peer="http://127.0.0.1:7070"} | ConvertTo-Json) -ContentType "application/json" | Out-Null
Invoke-RestMethod -Uri "http://127.0.0.1:7072/peers" -Method POST -Body (@{peer="http://127.0.0.1:7071"} | ConvertTo-Json) -ContentType "application/json" | Out-Null

Write-Color "✅ Peer connections configured`n" "Green"

# Display peer lists
Write-Color "📋 Peer Lists:" "Cyan"
$peers1 = Invoke-RestMethod -Uri "http://127.0.0.1:7070/peers"
$peers2 = Invoke-RestMethod -Uri "http://127.0.0.1:7071/peers"
$peers3 = Invoke-RestMethod -Uri "http://127.0.0.1:7072/peers"

Write-Color "   Node 1 peers: $($peers1.peers -join ', ')" "Gray"
Write-Color "   Node 2 peers: $($peers2.peers -join ', ')" "Gray"
Write-Color "   Node 3 peers: $($peers3.peers -join ', ')`n" "Gray"

# Wait for mining and sync
Write-Color "⏳ Waiting $WaitSeconds seconds for mining and sync..." "Yellow"
Write-Color "   (Miner on Node 1 should produce blocks, others sync)`n" "Gray"

for ($i = 1; $i -le $WaitSeconds; $i++) {
    Start-Sleep -Seconds 1
    if ($i % 5 -eq 0) {
        # Sample chain heights every 5 seconds
        try {
            $info1 = Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain"
            $info2 = Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain"
            $info3 = Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain"
            
            Write-Color "   [$i/$WaitSeconds] Heights: Node1=$($info1.height) | Node2=$($info2.height) | Node3=$($info3.height)" "Gray"
        } catch {
            Write-Color "   [$i/$WaitSeconds] Nodes still starting..." "Gray"
        }
    }
}

Write-Color "`n📊 Final Network Status" "Cyan"
Write-Color "═══════════════════════`n" "Cyan"

# Get final chain info from all nodes
$chain1 = Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain"
$chain2 = Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain"
$chain3 = Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain"

Write-Color "📍 Node 1 (Miner - 7070):" "Green"
Write-Color "   Height:     $($chain1.height)" "White"
Write-Color "   Best Hash:  $($chain1.best_hash)" "White"
Write-Color "   Difficulty: $($chain1.difficulty)" "White"

Write-Color "`n📍 Node 2 (Peer - 7071):" "Green"
Write-Color "   Height:     $($chain2.height)" "White"
Write-Color "   Best Hash:  $($chain2.best_hash)" "White"
Write-Color "   Difficulty: $($chain2.difficulty)" "White"

Write-Color "`n📍 Node 3 (Peer - 7072):" "Green"
Write-Color "   Height:     $($chain3.height)" "White"
Write-Color "   Best Hash:  $($chain3.best_hash)" "White"
Write-Color "   Difficulty: $($chain3.difficulty)" "White"

# Check consensus
Write-Color "`n🔍 Consensus Check:" "Cyan"
if ($chain1.best_hash -eq $chain2.best_hash -and $chain2.best_hash -eq $chain3.best_hash) {
    Write-Color "   ✅ All nodes agree on best hash!" "Green"
    Write-Color "   ✅ Network consensus achieved!" "Green"
} else {
    Write-Color "   ⚠️  Nodes have different best hashes" "Yellow"
    if ($chain1.height -eq $chain2.height -and $chain2.height -eq $chain3.height) {
        Write-Color "   ⚠️  But heights match - may need more time to sync" "Yellow"
    } else {
        Write-Color "   ⚠️  Heights differ - sync in progress" "Yellow"
    }
}

# Check if blocks were mined
if ($chain1.height -gt 0) {
    Write-Color "   ✅ Blocks mined: $($chain1.height)" "Green"
} else {
    Write-Color "   ⚠️  No blocks mined yet" "Yellow"
}

# Mining stats (if applicable)
if (-not $NoMining) {
    Write-Color "`n⛏️  Mining Stats (Node 1):" "Cyan"
    try {
        $stats = Invoke-RestMethod -Uri "http://127.0.0.1:7070/miner/stats"
        Write-Color "   Blocks Found: $($stats.blocks_found)" "White"
        Write-Color "   Accepted:     $($stats.blocks_accepted)" "White"
        Write-Color "   Rejected:     $($stats.blocks_rejected)" "White"
        Write-Color "   Total Reward: $($stats.total_rewards / 1000000) LAND" "White"
    } catch {
        Write-Color "   Stats unavailable" "Gray"
    }
}

Write-Color "`n🌐 Network Test Complete!" "Cyan"
Write-Color "══════════════════════════`n" "Cyan"

Write-Color "💡 Next Steps:" "Yellow"
Write-Color "   • Monitor with: curl http://127.0.0.1:707X/chain" "Gray"
Write-Color "   • View blocks:  curl http://127.0.0.1:707X/block/:height" "Gray"
Write-Color "   • Check peers:  curl http://127.0.0.1:707X/peers" "Gray"
Write-Color "   • Metrics:      curl http://127.0.0.1:707X/metrics`n" "Gray"

Write-Color "🛑 To stop all nodes: Get-Process -Name 'vision-node' | Stop-Process" "Gray"
