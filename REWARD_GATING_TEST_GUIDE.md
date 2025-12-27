# Reward Gating - Quick Test Guide

## Test 1: Isolated Node (Rewards Disabled)

1. **Start isolated node:**
   ```powershell
   # Stop any running nodes
   Get-Process -Name vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
   
   # Clear miner.json to use defaults
   Remove-Item miner.json -ErrorAction SilentlyContinue
   
   # Start node (no peers)
   $env:VISION_GUARDIAN_MODE='true'
   $env:BEACON_MODE='active'
   $env:VISION_PORT='7070'
   $env:RUST_LOG='info,rewards=info'
   .\target\release\vision-node.exe
   ```

2. **Expected behavior:**
   - Node starts with 0 peers
   - P2P health = "isolated"
   - Log shows: `⚠️ Block rewards DISABLED: node not yet eligible (unsynced / insufficient peers)`
   - Miner earnings = 0 LAND per block

3. **Verify status:**
   ```powershell
   # Check constellation status
   Invoke-RestMethod -Uri "http://127.0.0.1:7070/constellation/status" | ConvertTo-Json
   
   # Look for:
   # "connected_peers": 0
   # "p2p_health": "isolated"
   ```

4. **Check logs:**
   ```powershell
   # Look for reward status messages
   Get-Content -Path "vision-node.log" -Tail 50 | Select-String "rewards"
   ```

## Test 2: Network Join (Rewards Enabled)

1. **Connect to network:**
   ```powershell
   # Add bootstrap peers via environment variable
   $env:VISION_BOOTSTRAP='69.173.206.211:7070,69.173.207.135:7072'
   
   # Restart node
   Get-Process -Name vision-node -ErrorAction SilentlyContinue | Stop-Process -Force
   Start-Sleep 2
   .\target\release\vision-node.exe
   ```

2. **Expected behavior:**
   - Node connects to 2+ peers
   - P2P health changes to "ok" or "stable"
   - Log shows: `✅ Block rewards ENABLED: node is now eligible for rewards`
   - Miner starts earning rewards

3. **Verify eligibility transition:**
   ```powershell
   # Watch logs for state change
   Get-Content -Path "vision-node.log" -Tail 100 | Select-String "Block rewards"
   
   # Expected output:
   # [WARN] Block rewards DISABLED: node not yet eligible
   # ... (after peer connection)
   # [INFO] Block rewards ENABLED: node is now eligible for rewards
   ```

## Test 3: Custom Configuration

1. **Create custom miner.json:**
   ```json
   {
     "reward_address": "land1yourminingaddress",
     "auto_mine": false,
     "max_txs": 1000,
     "reward_eligibility": {
       "min_peers_for_rewards": 5,
       "max_reward_desync_blocks": 10,
       "reward_warmup_height": 100
     }
   }
   ```

2. **Restart node:**
   - With 5 peer requirement, rewards disabled until 5+ peers connected
   - With warmup height 100, no rewards until block 100

3. **Test scenarios:**
   ```powershell
   # Scenario A: Block < 100 → rewards disabled (warmup)
   # Scenario B: Block >= 100, peers < 5 → rewards disabled (insufficient peers)
   # Scenario C: Block >= 100, peers >= 5 → rewards enabled
   ```

## Test 4: Desync Scenario

1. **Simulate desync:**
   ```powershell
   # Stop node for 30 seconds (let network advance)
   Get-Process -Name vision-node | Stop-Process -Force
   Start-Sleep 30
   
   # Start node (will be behind network)
   .\target\release\vision-node.exe
   ```

2. **Expected behavior:**
   - Node syncs blocks
   - If >5 blocks behind: `⚠️ Block rewards DISABLED` (desync > max_reward_desync_blocks)
   - After catching up: `✅ Block rewards ENABLED`

## Monitoring Dashboard

### Check Eligibility Status

```powershell
# PowerShell monitoring script
while ($true) {
    Clear-Host
    Write-Host "=== REWARD ELIGIBILITY STATUS ===" -ForegroundColor Cyan
    
    $status = Invoke-RestMethod -Uri "http://127.0.0.1:7070/constellation/status"
    
    Write-Host "`nPeer Status:" -ForegroundColor Yellow
    Write-Host "  Connected: $($status.connected_peers)"
    Write-Host "  Health: $($status.p2p_health)"
    
    Write-Host "`nSync Status:" -ForegroundColor Yellow
    Write-Host "  Local Height: $($status.sync_height)"
    Write-Host "  Network Height: $($status.network_estimated_height)"
    Write-Host "  Desync: $($status.network_estimated_height - $status.sync_height) blocks"
    
    # Check eligibility based on defaults
    $eligible = $status.connected_peers -ge 3 -and
                $status.p2p_health -in @('ok', 'stable', 'immortal') -and
                ([Math]::Abs($status.network_estimated_height - $status.sync_height) -le 5)
    
    Write-Host "`nReward Status:" -ForegroundColor Yellow
    if ($eligible) {
        Write-Host "  ✅ ENABLED" -ForegroundColor Green
    } else {
        Write-Host "  ⛔ DISABLED" -ForegroundColor Red
        
        # Show reasons
        if ($status.connected_peers -lt 3) {
            Write-Host "    Reason: Insufficient peers ($($status.connected_peers) < 3)" -ForegroundColor Gray
        }
        if ($status.p2p_health -notin @('ok', 'stable', 'immortal')) {
            Write-Host "    Reason: P2P health is '$($status.p2p_health)'" -ForegroundColor Gray
        }
        $desync = [Math]::Abs($status.network_estimated_height - $status.sync_height)
        if ($desync -gt 5) {
            Write-Host "    Reason: Desync $desync blocks (> 5)" -ForegroundColor Gray
        }
    }
    
    Start-Sleep 5
}
```

## Expected Log Output

### Rewards Disabled
```
[2025-12-05T14:30:15Z WARN  rewards] ⚠️ Block rewards DISABLED: node not yet eligible (unsynced / insufficient peers)
    height: 1523
    connected_peers: 1
    p2p_health: "weak"
    sync_height: 1523
    network_estimated_height: 1528
```

### Rewards Enabled
```
[2025-12-05T14:32:40Z INFO  rewards] ✅ Block rewards ENABLED: node is now eligible for rewards
    height: 1529
    connected_peers: 4
    p2p_health: "ok"
```

### No Spam (State Tracking)
- Logs only appear when eligibility **changes**
- No repeated warnings every block
- Clean, actionable information

## Verification

### Check Miner Balance

```powershell
# Get miner address from miner.json
$minerAddr = (Get-Content miner.json | ConvertFrom-Json).reward_address

# Check balance
$balance = Invoke-RestMethod -Uri "http://127.0.0.1:7070/balance/$minerAddr"
Write-Host "Miner Balance: $($balance.balance / 100000000) LAND"
```

### Expected Results

| Scenario | Peers | Health | Desync | Rewards | Miner Balance Changes |
|----------|-------|--------|--------|---------|----------------------|
| Isolated | 0 | isolated | 0 | ❌ Disabled | No change |
| Weak | 1 | weak | 0 | ❌ Disabled | No change |
| Syncing | 3 | ok | 8 | ❌ Disabled | No change |
| Healthy | 3 | ok | 2 | ✅ Enabled | +5 LAND per block |
| Stable | 10 | stable | 0 | ✅ Enabled | +5 LAND per block |

## Troubleshooting

### Issue: Rewards still disabled after connecting peers

**Solution:**
1. Check actual peer count: `curl http://127.0.0.1:7070/p2p/peers | jq '.connected_peers'`
2. Verify P2P health: `curl http://127.0.0.1:7070/constellation/status | jq '.p2p_health'`
3. Check desync: Compare sync_height vs network_estimated_height
4. Review miner.json configuration (may have stricter requirements)

### Issue: No log messages appearing

**Solution:**
1. Ensure `RUST_LOG=info,rewards=info` is set
2. Check log file location: `vision-node.log` in current directory
3. Increase verbosity: `RUST_LOG=debug,rewards=debug`

### Issue: Want to test without peer requirement

**Solution:**
Temporarily set `min_peers_for_rewards: 0` in miner.json:
```json
{
  "reward_eligibility": {
    "min_peers_for_rewards": 0,
    "max_reward_desync_blocks": 99999,
    "reward_warmup_height": 0
  }
}
```

⚠️ **Warning:** Only for local testing! Never use these settings in production.

## Next Steps

After successful testing:
1. Set appropriate thresholds for your network
2. Monitor logs for legitimate vs. illegitimate reward disabling
3. Adjust configuration based on network conditions
4. Document any edge cases encountered
5. Consider implementing gradual reward scaling (future enhancement)
