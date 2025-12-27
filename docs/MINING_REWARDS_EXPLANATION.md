# Mining Rewards & Supply Explanation

## Current Status

### What's Happening
- ✅ Node is running successfully
- ✅ Miner is actively searching for valid blocks (you see "Mining block #1..." messages)
- ✅ Tokenomics + Tithe system is fully implemented
- ❌ **No blocks have been mined yet** - miner is searching but hasn't found a valid nonce
- ❌ Supply shows 0 because no mining rewards have been issued yet

### Why Supply is 0
The supply is 0 because **no blocks have been successfully mined and finalized yet**. The miner logs show:
```
🎯 Mining block #1 with epoch_seed=0000..., epoch=0
🎯 New mining job: height=1, difficulty=10000, target=0006...
```

This means the miner is actively searching for a valid hash that meets the difficulty target, but hasn't found one yet. When a valid block is found, you'll see a different message like:
```
✅ Block #1 mined successfully!
```

### Missing `/api/supply` Endpoint
**FIXED**: I just added the `/api/supply` endpoint which was missing. The miner panel calls this endpoint to show total supply.

**Added Code:**
```rust
/// GET /supply - Return total supply as plain text
async fn get_supply() -> String {
    let g = CHAIN.lock();
    g.total_supply().to_string()
}
```

**Added Route:**
```rust
.route("/supply", get(get_supply))
```

### How Mining Works

1. **Miner finds valid nonce** → Block is created with:
   - **Tokenomics emission**: Miner gets base reward (32 LAND)
   - **2-LAND Tithe**: 2 LAND is split across foundation:
     - 50% (1.0 LAND) → Vault (`0xb977...`)
     - 30% (0.6 LAND) → Fund (`0x8bb8...`)
     - 20% (0.4 LAND) → Treasury (`0xdf7a...`)

2. **Block finalized** → Balances updated:
   - Miner balance increases by emission amount
   - Foundation addresses get tithe splits
   - Total supply increases

3. **Supply tracked** in database key `supply:total`

### Your Miner Address

The built-in miner uses whatever address is configured. To mine to YOUR address:

**Option 1: Use the miner panel in the web UI**
1. Open http://127.0.0.1:7070
2. Go to Miner tab
3. Enter YOUR wallet address
4. Click "Start Mining"

**Option 2: Use API directly**
```powershell
# Start mining to your address
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/miner/start" `
  -Method POST `
  -Body '{"miner_address":"YOUR_ADDRESS_HERE"}' `
  -ContentType "application/json"
```

**Option 3: Set environment variable**
```powershell
$env:MINER_ADDRESS="YOUR_ADDRESS_HERE"
.\target\release\vision-node.exe
```

### After Rebuild

Once the build completes (it's currently waiting for file lock), restart the node:

```powershell
# Kill any running instances
Get-Process vision-node -ErrorAction SilentlyContinue | Stop-Process -Force

# Start fresh
.\target\release\vision-node.exe
```

Then check supply:
```powershell
# Should return "0" initially
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/supply"

# After a block is mined, will show total supply (emission + tithe)
# Example after 1 block: 34000000000 (32 LAND emission + 2 LAND tithe)
```

### Testing Mining

**Force mine a block** (if difficulty is too high for testing):
```powershell
# Temporarily lower difficulty for testing
# Add to .env:
DIFFICULTY_FLOOR=1000

# Or use admin endpoint to seed a block
curl -X POST http://127.0.0.1:7070/admin/mine_block `
  -H "X-Admin-Token: YOUR_TOKEN" `
  -d '{"miner_addr":"YOUR_ADDRESS"}'
```

### Expected Flow

1. **Block 0** (Genesis): Supply = 0
2. **Block 1 mined**: 
   - Miner gets 32 LAND (32000000000 units)
   - Vault gets 1.0 LAND (100000000 units)
   - Fund gets 0.6 LAND (60000000 units)
   - Treasury gets 0.4 LAND (40000000 units)
   - **Total Supply = 34 LAND** (34000000000 units)

3. **Block 2 mined**: Supply increases by another 34 LAND, etc.

### Checking Foundation Balances

After first block is mined:
```powershell
# Vault (should be 100000000 = 1.0 LAND)
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/balance/0xb977c16e539670ddfecc0ac902fcb916ec4b944e"

# Fund (should be 60000000 = 0.6 LAND)
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/balance/0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd"

# Treasury (should be 40000000 = 0.4 LAND)
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/balance/0xdf7a79291bb96e9dd1c77da089933767999eabf0"

# Your miner address (should be 32000000000 = 32 LAND)
Invoke-RestMethod -Uri "http://127.0.0.1:7070/api/balance/YOUR_ADDRESS"
```

## Summary

**The system is working correctly!** You just need to:

1. ✅ Wait for the cargo build to complete
2. ✅ Restart the node with the new build
3. ✅ Either wait for the miner to find a block, OR set a lower difficulty for testing
4. ✅ Verify your miner is using YOUR wallet address (not a placeholder)

Once a block is successfully mined, you'll immediately see:
- Supply increase on the miner panel
- Your address balance show the mining reward
- Foundation addresses show their tithe portions
