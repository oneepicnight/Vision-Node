# Tester Node Behavior - Vision v3.0.0

## Startup Flow (No Wallet Required)

### âœ… What Happens WITHOUT a Wallet Address

1. **Node Starts Immediately**
   - No wallet required to start the node
   - Node begins P2P networking and sync
   - All services activate (HTTP API, panel, dashboard)

2. **Node Syncs with Genesis**
   - Connects to genesis seed: `35.151.236.81:7072`
   - Downloads all blocks from genesis node
   - Validates chain and updates local state
   - **This happens automatically without any wallet**

3. **Node Status: Synced but Not Earning**
   - âœ… Fully synced with network
   - âœ… Connected to peers
   - âœ… Processing blocks
   - âœ… Can view chain data
   - âœ… Can submit transactions (if needed)
   - âŒ **NOT eligible for mining rewards** (empty `MINER_ADDRESS`)

4. **Node Logs**
   ```
   ðŸ” get_eligible_peer_pool: We ARE eligible!
   ðŸ” get_eligible_peer_pool: Our address: [EMPTY STRING]
   ðŸ” get_eligible_peer_pool: NOT added to pool (address empty)
   ```

### âœ… What Happens WHEN User Adds Wallet

1. **User Links Wallet via Panel**
   - Open `http://localhost:7777/panel.html`
   - Enter wallet address in "Miner Address" field
   - Click "Update"
   - API endpoint: `POST /api/update_miner` sets `MINER_ADDRESS`

2. **Immediate Mining Eligibility**
   - Next block cycle (2 seconds): Node checks eligibility
   - `get_eligible_peer_pool()` finds non-empty `MINER_ADDRESS`
   - Node is added to mining pool: `eligible.push(our_address)`
   - **Node can now win mining rewards!**

3. **No Restart Required**
   - Wallet address is updated in memory (`MINER_ADDRESS` global)
   - Next block production cycle uses new address
   - **Zero downtime, instant activation**

4. **Node Logs After Wallet Added**
   ```
   ðŸ” get_eligible_peer_pool: We ARE eligible!
   ðŸ” get_eligible_peer_pool: Our address: land1abc123...
   ðŸ” get_eligible_peer_pool: Added to pool!
   âœ… LEAF node mining eligible: synced=true, peers=1, lag=0
   ```

## Code Flow

### Mining Eligibility Check (Every Block)
```rust
// src/mining_eligibility.rs:217-243
fn get_eligible_peer_pool() -> Vec<String> {
    let mut eligible = Vec::new();
    
    // Check if WE are eligible first
    let snapshot = crate::auto_sync::SyncHealthSnapshot::current();
    
    if is_reward_eligible(&snapshot) {
        // Node is synced and connected to peers
        
        let our_address = crate::MINER_ADDRESS.lock().clone();
        
        if !our_address.is_empty() {
            // âœ… WALLET SET: Add to mining pool
            eligible.push(our_address);
        } else {
            // âŒ NO WALLET: Synced but not earning
        }
    }
    
    eligible
}
```

### Miner Address Update (No Restart)
```rust
// src/main.rs:1152 (API handler)
*MINER_ADDRESS.lock() = wallet.clone();
// âœ… Immediately available for next block mining
```

## Network Architecture

### Genesis Node (Your Node)
- **Address**: `35.151.236.81:7072`
- **Role**: First node, produces blocks every 2 seconds
- **Mode**: Genesis (no wallet required, uses placeholder)
- **Block Production**: Automatic mining system
- **Must Stay Online**: Testers sync from this node

### Tester Nodes (Constellation)
- **Bootstrap**: Connect to genesis seed automatically
- **Sync**: Download all blocks from genesis
- **Operation**: Run 24/7 to maintain sync
- **Rewards**: Only earned when wallet address is set

## User Experience Flow

### Phase 1: Download & Extract (1 minute)
1. Download `VisionNode-Constellation-v3.0.0-TESTERS.zip` (19.47 MB)
2. Extract to folder
3. See files: `vision-node.exe`, `.env`, `START-PUBLIC-NODE.bat`, etc.

### Phase 2: Start Node (Immediate)
1. Double-click `START-PUBLIC-NODE.bat`
2. Node starts, shows:
   ```
   ðŸŒŒ VISION CONSTELLATION NODE v3.0.0
   ðŸ“¡ Beacon: Constellation mode
   ðŸ”Œ P2P listener: 0.0.0.0:7072
   ðŸš€ Bootstrap: Starting unified bootstrap...
   âœ… Bootstrap completed successfully
   ```
3. **Node is now syncing** (no wallet needed)

### Phase 3: Sync Chain (5-10 minutes)
1. Node connects to genesis: `35.151.236.81:7072`
2. Downloads blocks (depending on how many exist)
3. Console shows:
   ```
   [SYNC] Requesting blocks from height 10...
   [SYNC] Received 50 blocks from peer
   [SYNC] Applied block #60, new tip: 0x...
   ```
4. **Tester can browse panel/dashboard while syncing**

### Phase 4: Monitor Status (Anytime)
1. Open browser: `http://localhost:7777/panel.html`
2. View:
   - Current block height
   - Sync status
   - Connected peers
   - Network info
3. **Wallet section shows: "Not Set" or empty**

### Phase 5: Add Wallet (When Ready)
1. In panel, find "Update Miner Address" section
2. Enter wallet address: `land1abc123...`
3. Click "Update"
4. See confirmation: "Miner address updated successfully"
5. **Node immediately eligible for rewards** (next 2-second block cycle)

### Phase 6: Earn Rewards (Automatic)
1. Every 2 seconds, mining runs
2. All synced nodes with wallets are eligible
3. Random winner selected each block
4. Rewards paid to winner's wallet address
5. **No additional action needed - fully automatic**

## Troubleshooting

### Node Not Syncing
**Symptom**: Block height stuck at 10
**Cause**: Cannot reach genesis node
**Fix**:
1. Check internet connection
2. Verify port 7072 not blocked by firewall
3. Check genesis node is online: `telnet 35.151.236.81 7072`

### Node Synced but No Rewards
**Symptom**: Fully synced, connected, but never wins mining
**Cause**: Wallet address not set
**Fix**:
1. Open panel: `http://localhost:7777/panel.html`
2. Set miner address in "Update Miner Address" section
3. Confirm address saved
4. Wait for next mining cycle (2 seconds)

### Mining Winners Always Same Address
**Symptom**: Same wallet wins every block
**Cause**: Only one node has wallet address set
**Solution**: This is normal if you're the only tester with a wallet!
- More testers with wallets = more mining competition
- Each block randomly selects from eligible pool

## Key Differences from Genesis Node

| Feature | Genesis Node | Tester Node |
|---------|-------------|-------------|
| **Wallet Required** | âŒ No (uses placeholder) | âŒ No (but needed for rewards) |
| **Syncs from** | N/A (first node) | Genesis node |
| **Bootstrap** | Disabled | Enabled |
| **MIN_PEERS** | 0 (genesis mode) | 1 (normal mode) |
| **Block Production** | Automatic (every 2s) | Via mining only |
| **Mining Eligible** | Always (with placeholder) | Only when wallet set |

## Environment Variables (Tester .env)

```bash
VISION_PURE_SWARM_MODE=true          # Enable bootstrap
VISION_MIN_PEERS_FOR_MINING=1        # Require 1 peer for mining
VISION_BEACON_BOOTSTRAP=true         # Use beacon discovery
VISION_MIN_HEALTHY_CONNECTIONS=1     # Maintain 1 connection
```

## API Endpoints (For Advanced Users)

### Check Current Miner Address
```bash
curl http://localhost:7070/api/wallet_info
```
Response:
```json
{
  "wallet_address": "land1abc123..." // or "" if not set
}
```

### Update Miner Address
```bash
curl -X POST http://localhost:7070/api/update_miner \
  -H "Content-Type: application/json" \
  -d '{"wallet":"land1abc123..."}'
```

### Check Mining Eligibility
```bash
curl http://localhost:7070/api/mining/eligibility
```
Response:
```json
{
  "eligible": true,
  "role": "edge-miner",
  "lag_blocks": 0,
  "peer_count": 1,
  "reason": "âœ… Synced and eligible for mining"
}
```

## Summary

**âœ… Tester nodes sync immediately without wallet** - This already works!

**âœ… Wallet can be added later for rewards** - This already works!

**âœ… No restart required when adding wallet** - This already works!

**Your concern was already addressed in the code!** The node behavior is exactly as you wanted:
1. Start without wallet â†’ Sync happens
2. Add wallet later â†’ Rewards start immediately
3. No downtime or restart needed

The only thing needed is clear documentation for testers, which this document provides.

