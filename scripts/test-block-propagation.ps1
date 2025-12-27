# Test Block Propagation Between Nodes
# Starts 3 nodes and monitors how quickly blocks propagate

param(
    [int]$Blocks = 10,
    [int]$SampleInterval = 1
)

$ErrorActionPreference = "Stop"

function Write-Status($msg, $color = "White") {
    $timestamp = Get-Date -Format "HH:mm:ss.fff"
    Write-Host "[$timestamp] $msg" -ForegroundColor $color
}

Write-Status "🚀 Starting Block Propagation Test" "Cyan"
Write-Status "Target: Monitor $Blocks blocks across 3 nodes" "Gray"

# Ensure nodes are running
$nodes = @(7070, 7071, 7072)
foreach ($port in $nodes) {
    try {
        $null = Invoke-RestMethod -Uri "http://127.0.0.1:$port/chain" -TimeoutSec 2
    } catch {
        Write-Status "❌ Node on port $port is not running!" "Red"
        Write-Status "Run: .\test-3nodes-sync.ps1 first" "Yellow"
        exit 1
    }
}

Write-Status "✅ All 3 nodes are online" "Green"

# Get starting heights
$start1 = (Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain").height
$start2 = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain").height
$start3 = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain").height

Write-Status "📊 Starting Heights: N1=$start1 | N2=$start2 | N3=$start3" "Cyan"
Write-Status "⏳ Monitoring block propagation...`n" "Yellow"

# Track propagation times
$propagation_times = @()
$last_height = $start1
$blocks_monitored = 0

while ($blocks_monitored -lt $Blocks) {
    Start-Sleep -Milliseconds ($SampleInterval * 1000)
    
    # Query all nodes
    $h1 = (Invoke-RestMethod -Uri "http://127.0.0.1:7070/chain").height
    $h2 = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain").height
    $h3 = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain").height
    
    # Check if new block appeared on Node 1 (miner)
    if ($h1 -gt $last_height) {
        $new_block = $h1
        $found_time = Get-Date
        
        Write-Status "📦 Block #$new_block found on Node 1" "Green"
        
        # Wait for it to propagate to other nodes
        $max_wait = 10 # seconds
        $waited = 0
        $n2_synced = $false
        $n3_synced = $false
        $n2_time = $null
        $n3_time = $null
        
        while ($waited -lt $max_wait) {
            Start-Sleep -Milliseconds 100
            $waited += 0.1
            
            $h2_current = (Invoke-RestMethod -Uri "http://127.0.0.1:7071/chain").height
            $h3_current = (Invoke-RestMethod -Uri "http://127.0.0.1:7072/chain").height
            
            if (-not $n2_synced -and $h2_current -ge $new_block) {
                $n2_synced = $true
                $n2_time = ((Get-Date) - $found_time).TotalMilliseconds
                Write-Status "   ✓ Node 2 synced in $([math]::Round($n2_time, 0)) ms" "Cyan"
            }
            
            if (-not $n3_synced -and $h3_current -ge $new_block) {
                $n3_synced = $true
                $n3_time = ((Get-Date) - $found_time).TotalMilliseconds
                Write-Status "   ✓ Node 3 synced in $([math]::Round($n3_time, 0)) ms" "Cyan"
            }
            
            if ($n2_synced -and $n3_synced) {
                break
            }
        }
        
        # Record result
        $result = @{
            block = $new_block
            node2_ms = $n2_time
            node3_ms = $n3_time
            both_synced = ($n2_synced -and $n3_synced)
        }
        $propagation_times += $result
        
        if (-not $result.both_synced) {
            Write-Status "   ⚠️  Propagation incomplete within ${max_wait}s" "Yellow"
        }
        
        $last_height = $new_block
        $blocks_monitored++
        
        Write-Host "" # blank line
    }
}

# Summary
Write-Status "`n📊 Propagation Test Summary" "Cyan"
Write-Status "═══════════════════════════" "Cyan"

$successful = $propagation_times | Where-Object { $_.both_synced }
$avg_n2 = ($successful | Where-Object { $_.node2_ms } | ForEach-Object { $_.node2_ms } | Measure-Object -Average).Average
$avg_n3 = ($successful | Where-Object { $_.node3_ms } | ForEach-Object { $_.node3_ms } | Measure-Object -Average).Average

Write-Status "Blocks Monitored:  $blocks_monitored" "White"
Write-Status "Successful Syncs:  $($successful.Count)" "Green"
Write-Status "Failed Syncs:      $($blocks_monitored - $successful.Count)" "Yellow"

if ($successful.Count -gt 0) {
    Write-Status "`nAverage Propagation Times:" "Cyan"
    Write-Status "  Node 2: $([math]::Round($avg_n2, 0)) ms" "White"
    Write-Status "  Node 3: $([math]::Round($avg_n3, 0)) ms" "White"
    
    $fastest = ($successful | ForEach-Object { [math]::Min($_.node2_ms, $_.node3_ms) } | Measure-Object -Minimum).Minimum
    $slowest = ($successful | ForEach-Object { [math]::Max($_.node2_ms, $_.node3_ms) } | Measure-Object -Maximum).Maximum
    
    Write-Status "`nPropagation Range:" "Cyan"
    Write-Status "  Fastest: $([math]::Round($fastest, 0)) ms" "Green"
    Write-Status "  Slowest: $([math]::Round($slowest, 0)) ms" "Yellow"
}

Write-Status "`n✅ Block Propagation Test Complete!" "Green"
