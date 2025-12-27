# Multi-Node Testing Guide
## Vision Node P2P Network Testing

This guide covers testing Vision Node's P2P capabilities, including block propagation, network synchronization, and consensus verification.

---

## 📋 Prerequisites

- **Built binary**: `cargo build --release --bin vision-node`
- **Multiple terminals**: Or use the provided PowerShell scripts
- **Ports available**: 7070, 7071, 7072 (or your chosen ports)
- **Windows PowerShell**: For test scripts

---

## 🚀 Quick Start: 3-Node Network

### Option 1: Automated Test Script

```powershell
# Start 3 nodes with mining on Node 1
.\test-3nodes-sync.ps1

# Start clean (wipe data)
.\test-3nodes-sync.ps1 -Clean

# Start without mining (for sync testing)
.\test-3nodes-sync.ps1 -NoMining

# Wait longer for more blocks
.\test-3nodes-sync.ps1 -WaitSeconds 60
```

### Option 2: Manual Setup

**Terminal 1 - Miner Node (Port 7070):**
```powershell
.\target\release\vision-node.exe --reset --enable-mining --port 7070
```

**Terminal 2 - Peer Node (Port 7071):**
```powershell
$env:VISION_PORT="7071"
$env:VISION_DATA_DIR="vision_data_7071"
.\target\release\vision-node.exe --reset --port 7071
```

**Terminal 3 - Peer Node (Port 7072):**
```powershell
$env:VISION_PORT="7072"
$env:VISION_DATA_DIR="vision_data_7072"
.\target\release\vision-node.exe --reset --port 7072
```

**Configure Peers:**
```powershell
# Add Node 2 and 3 as peers of Node 1
Invoke-RestMethod -Uri "http://127.0.0.1:7070/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7071"}' -ContentType "application/json"
Invoke-RestMethod -Uri "http://127.0.0.1:7070/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7072"}' -ContentType "application/json"

# Add Node 1 and 3 as peers of Node 2
Invoke-RestMethod -Uri "http://127.0.0.1:7071/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7070"}' -ContentType "application/json"
Invoke-RestMethod -Uri "http://127.0.0.1:7071/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7072"}' -ContentType "application/json"

# Add Node 1 and 2 as peers of Node 3
Invoke-RestMethod -Uri "http://127.0.0.1:7072/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7070"}' -ContentType "application/json"
Invoke-RestMethod -Uri "http://127.0.0.1:7072/peers" -Method POST -Body '{"peer":"http://127.0.0.1:7071"}' -ContentType "application/json"
```

---

## 🔍 Testing Scenarios

### 1. Block Propagation Test

Tests how quickly blocks propagate through the network.

```powershell
# Run after starting the 3-node network
.\test-block-propagation.ps1

# Monitor 20 blocks with 2-second sampling
.\test-block-propagation.ps1 -Blocks 20 -SampleInterval 2
```

**Expected Results:**
- ✅ Blocks propagate to all nodes within seconds
- ✅ Average propagation time < 5 seconds
- ✅ All nodes reach consensus on best hash

### 2. Network Synchronization Test

Test nodes syncing from scratch.

**Setup:**
1. Start Node 1 (miner) and let it mine 50+ blocks
2. Start Node 2 (syncer) without data
3. Configure Node 2 to peer with Node 1
4. Monitor sync progress

```powershell
# Check sync progress
while ($true) {
    $h1 = (Invoke-RestMethod http://127.0.0.1:7070/chain).height
    $h2 = (Invoke-RestMethod http://127.0.0.1:7071/chain).height
    Write-Host "Node1: $h1 | Node2: $h2 | Gap: $($h1-$h2)"
    Start-Sleep -Seconds 2
}
```

**Expected Results:**
- ✅ Node 2 syncs from genesis
- ✅ Gap closes over time
- ✅ Node 2 reaches same height as Node 1
- ✅ VisionX PoW verified for all synced blocks

### 3. Consensus Verification

Verify all nodes agree on chain state.

```powershell
# Get chain info from all nodes
$n1 = Invoke-RestMethod http://127.0.0.1:7070/chain
$n2 = Invoke-RestMethod http://127.0.0.1:7071/chain
$n3 = Invoke-RestMethod http://127.0.0.1:7072/chain

# Check consensus
Write-Host "Node 1: H=$($n1.height) Hash=$($n1.best_hash)"
Write-Host "Node 2: H=$($n2.height) Hash=$($n2.best_hash)"
Write-Host "Node 3: H=$($n3.height) Hash=$($n3.best_hash)"

if ($n1.best_hash -eq $n2.best_hash -and $n2.best_hash -eq $n3.best_hash) {
    Write-Host "✅ Consensus achieved!" -ForegroundColor Green
}
```

**Expected Results:**
- ✅ All nodes report same height
- ✅ All nodes report same best_hash
- ✅ All nodes report same difficulty

### 4. Epoch Boundary Test

Test network behavior across VisionX epoch boundaries (blocks 32, 64, 96...).

```powershell
# Monitor through epoch boundary
$target_height = 35  # Just past first epoch boundary at 32

while ($true) {
    $h1 = (Invoke-RestMethod http://127.0.0.1:7070/chain).height
    $h2 = (Invoke-RestMethod http://127.0.0.1:7071/chain).height
    $h3 = (Invoke-RestMethod http://127.0.0.1:7072/chain).height
    
    $epoch1 = [math]::Floor($h1 / 32)
    $epoch2 = [math]::Floor($h2 / 32)
    $epoch3 = [math]::Floor($h3 / 32)
    
    Write-Host "H: N1=$h1(E$epoch1) N2=$h2(E$epoch2) N3=$h3(E$epoch3)"
    
    if ($h1 -ge $target_height -and $h2 -ge $target_height -and $h3 -ge $target_height) {
        break
    }
    
    Start-Sleep -Seconds 2
}

Write-Host "✅ All nodes crossed epoch boundary successfully!" -ForegroundColor Green
```

**Expected Results:**
- ✅ Nodes cross epoch boundary (block 32)
- ✅ VisionX dataset rebuilds on all nodes
- ✅ Mining continues without errors
- ✅ All nodes stay in sync

### 5. Reorg Resistance Test

Test chain reorganization handling (advanced).

**Setup:**
1. Start 2 isolated networks (Node 1+2 vs Node 3)
2. Let both mine separately (fork)
3. Connect them
4. Observe reorg behavior

```powershell
# Coming soon - requires isolation and reconnection logic
```

---

## 📊 Monitoring Commands

### Check Chain Status
```powershell
# Node 1
Invoke-RestMethod http://127.0.0.1:7070/chain | ConvertTo-Json

# All nodes at once
7070,7071,7072 | ForEach-Object {
    $info = Invoke-RestMethod "http://127.0.0.1:$_/chain"
    Write-Host "Node $_: Height=$($info.height) Difficulty=$($info.difficulty)"
}
```

### Check Peer Lists
```powershell
Invoke-RestMethod http://127.0.0.1:7070/peers
Invoke-RestMethod http://127.0.0.1:7071/peers
Invoke-RestMethod http://127.0.0.1:7072/peers
```

### Check Mining Stats (Miner Node)
```powershell
Invoke-RestMethod http://127.0.0.1:7070/miner/stats | ConvertTo-Json
```

### Get Specific Block
```powershell
# Block 10 from Node 1
Invoke-RestMethod http://127.0.0.1:7070/block/10

# Compare same block across nodes
7070,7071,7072 | ForEach-Object {
    $block = Invoke-RestMethod "http://127.0.0.1:$_/block/10"
    Write-Host "Node $_: Hash=$($block.hash)"
}
```

### Monitor Prometheus Metrics
```powershell
Invoke-RestMethod http://127.0.0.1:7070/metrics
```

---

## 🐛 Troubleshooting

### Nodes Not Syncing

**Check:**
1. Peer lists configured correctly: `GET /peers`
2. Auto-sync enabled (default)
3. No firewall blocking localhost ports
4. Check logs for sync errors

**Fix:**
```powershell
# Manually trigger sync
Invoke-RestMethod -Uri "http://127.0.0.1:7071/sync/pull" -Method POST -Body '{"src":"http://127.0.0.1:7070"}' -ContentType "application/json"
```

### Consensus Mismatch

**Check:**
1. All nodes started from clean state (--reset)
2. Same binary version on all nodes
3. VisionX verification passing

**Fix:**
```powershell
# Stop all nodes
Get-Process vision-node | Stop-Process -Force

# Clean data
Remove-Item vision_data_707* -Recurse -Force

# Restart
.\test-3nodes-sync.ps1 -Clean
```

### Blocks Not Propagating

**Check:**
1. Miner is actually mining (check `/miner/stats`)
2. Peers connected properly
3. Auto-sync running (check env var `VISION_AUTOSYNC_SECS`)

**Monitor:**
```powershell
# Watch sync metrics
while ($true) {
    $metrics = Invoke-RestMethod http://127.0.0.1:7071/metrics
    $sync_blocks = ($metrics -split "`n" | Select-String "vision_sync_blocks_downloaded_total").ToString()
    Write-Host $sync_blocks
    Start-Sleep -Seconds 5
}
```

---

## ✅ Success Criteria

A successful multi-node test should show:

1. ✅ **Block Production**: Miner node produces blocks at ~2s intervals
2. ✅ **Propagation**: Blocks reach peer nodes within 5 seconds
3. ✅ **Consensus**: All nodes agree on best hash
4. ✅ **Sync**: New nodes can sync full chain from genesis
5. ✅ **VisionX**: PoW verification passes on all nodes
6. ✅ **Epochs**: Network crosses epoch boundaries smoothly
7. ✅ **LWMA**: Difficulty adjusts properly across all nodes
8. ✅ **Zero Rejections**: No "Solution does not meet target" errors

---

## 🚀 Advanced: Multi-Machine Testing

To test across multiple physical/virtual machines:

1. **Change bind address**: `--host 0.0.0.0` (listen on all interfaces)
2. **Update peer URLs**: Use actual IP addresses instead of 127.0.0.1
3. **Configure firewall**: Allow inbound on chosen ports
4. **Test connectivity**: `curl http://<remote_ip>:7070/chain`

Example:
```powershell
# Machine 1 (192.168.1.100)
.\target\release\vision-node.exe --host 0.0.0.0 --port 7070 --enable-mining

# Machine 2 (192.168.1.101)
.\target\release\vision-node.exe --host 0.0.0.0 --port 7070

# Configure Machine 2 to peer with Machine 1
Invoke-RestMethod -Uri "http://192.168.1.101:7070/peers" -Method POST -Body '{"peer":"http://192.168.1.100:7070"}' -ContentType "application/json"
```

---

## 📝 Testing Checklist

- [ ] Build latest binary
- [ ] Start 3-node network
- [ ] Configure peer connections
- [ ] Verify block production
- [ ] Verify block propagation
- [ ] Verify network consensus
- [ ] Test epoch boundary crossing
- [ ] Test new node sync from scratch
- [ ] Monitor metrics
- [ ] Test under load (many transactions)
- [ ] Test network partition recovery

---

## 🎯 Next Steps

After successful multi-node testing:

1. **Stress Testing**: Add load generators for transactions
2. **Network Simulation**: Use tools like `tc` for latency/packet loss
3. **Long-Run Testing**: 24+ hour tests for stability
4. **Geographic Distribution**: Test across different regions
5. **Security Testing**: Test attack scenarios (51%, eclipse, etc.)

---

**Ready for Production Deployment! 🚀**
