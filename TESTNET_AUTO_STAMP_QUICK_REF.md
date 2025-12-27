# Testnet Auto-Stamp Quick Reference

## What It Does

Auto-stamps the first 3 blocks on v2.0 testnet seed nodes to establish a canonical starting chain.

---

## Configuration

### Testnet Seed Nodes (3 Seeds)

```bash
VISION_NETWORK=testnet
VISION_IS_TESTNET_SEED=true
VISION_AUTO_STAMP_TESTNET=true
VISION_MINER_ADDRESS=land1seed_address_here
```

### Regular Testnet Miners

```bash
VISION_NETWORK=testnet
VISION_MINER_ADDRESS=land1miner_address_here
# (No seed flags needed - defaults to false)
```

### Mainnet (Guardian Launch - Separate Feature)

```bash
VISION_NETWORK=mainnet-full
VISION_LAUNCH_GUARDIAN_ENABLED=true  # Guardian only
VISION_GUARDIAN_ADDRESS=land1guard_address  # Guardian only
VISION_IS_GUARDIAN=true  # Guardian only
# (Testnet auto-stamp does NOT apply here)
```

---

## Expected Logs

### Seed Node (First Start)

```
[TESTNET_STAMP] 🌱 Auto-stamping 3 blocks for v2.0 Testnet on seed node...
[TESTNET_STAMP] 🔨 Stamping block 1 of 3...
[TESTNET_STAMP] ✅ Block 1 stamped successfully (height: 1)
[TESTNET_STAMP] 🔨 Stamping block 2 of 3...
[TESTNET_STAMP] ✅ Block 2 stamped successfully (height: 2)
[TESTNET_STAMP] 🔨 Stamping block 3 of 3...
[TESTNET_STAMP] ✅ Block 3 stamped successfully (height: 3)
[TESTNET_STAMP] 🎉 Testnet auto-stamp complete. Best height now 3.
```

### Seed Node (Restart)

```
[TESTNET_STAMP] Not required (current_height = 3, target = 3).
[TESTNET_STAMP] ✅ Testnet bootstrap complete. Chain height: 3
```

### Regular Miner

```
[TESTNET_STAMP] ℹ️  Regular testnet node - will sync from seeds
[BOOTSTRAP] 🚀 Executing unified bootstrap...
[SYNC] ✅ Synced block 1, 2, 3 from seeds
```

---

## Verification

### Check Seed Node Stamped

```bash
curl http://localhost:7070/height
# Expected: 3

curl http://localhost:7070/block/1
# Check miner field
```

### Check Miner Synced

```bash
curl http://localhost:7070/height
# Expected: 3+

curl http://localhost:7070/peers
# Should see seed nodes
```

---

## Troubleshooting

### Seed Not Stamping

**Check:**
```bash
echo $VISION_IS_TESTNET_SEED  # Must be "true"
echo $VISION_AUTO_STAMP_TESTNET  # Must be "true"
echo $VISION_NETWORK  # Must be "testnet"
```

**Log shows:**
```
[TESTNET_STAMP] ℹ️  Testnet seed node, but auto-stamp disabled
```
→ Set `VISION_AUTO_STAMP_TESTNET=true`

**Log shows:**
```
[TESTNET_STAMP] Not required (current_height = 3, target = 3).
```
→ Normal on restart (already stamped)

### Miner Not Syncing

**Check:**
```bash
echo $VISION_NETWORK  # Must be "testnet"
curl http://localhost:7070/peers  # Should see seeds
```

**Solution:**
- Ensure seeds are running and accessible
- Check firewall (port 7070)
- Verify protocol version 2

---

## Key Points

✅ **Testnet Only**: Only applies when `network = "testnet"`
✅ **Seed Nodes Only**: Only nodes with both flags enabled
✅ **Safe**: Won't overwrite existing blocks (height >= 3)
✅ **Automatic**: Happens on first startup
✅ **Separate**: Independent from mainnet Guardian launch

---

## Files Modified

- `src/constants.rs` - Added `TESTNET_STAMP_BLOCK_COUNT`
- `src/config/network.rs` - Added `is_testnet_seed` and `auto_stamp_testnet_blocks` fields
- `src/main.rs` - Added `bootstrap_testnet_stamp()` function and startup call

---

## Build Status

✅ Code compiles successfully (`cargo check` passed)
✅ No errors
✅ Ready for deployment

---

## Deployment Order

1. **Deploy v2.0 binary to 3 seed nodes**
2. **Configure seed nodes** with testnet seed flags
3. **Start all 3 seed nodes** simultaneously
4. **Verify all 3 stamped blocks 1-3** with matching hashes
5. **Deploy v2.0 binary to miners**
6. **Start miners** - they auto-sync from seeds
7. **Monitor** - miners should reach height 3+ and start mining

---

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `TESTNET_STAMP_BLOCK_COUNT` | `3` | Number of blocks to stamp |
| `NETWORK_TESTNET` | `"testnet"` | Testnet identifier |
| `NETWORK_MAINNET_FULL` | `"mainnet-full"` | Mainnet identifier |

---

## Environment Variables

| Variable | Seed Node | Miner | Guardian |
|----------|-----------|-------|----------|
| `VISION_NETWORK` | `testnet` | `testnet` | `mainnet-full` |
| `VISION_IS_TESTNET_SEED` | `true` | `false` | `false` |
| `VISION_AUTO_STAMP_TESTNET` | `true` | `false` | `false` |
| `VISION_LAUNCH_GUARDIAN_ENABLED` | N/A | N/A | `true` |
| `VISION_IS_GUARDIAN` | N/A | N/A | `true` |

---

## Success Criteria

- ✅ Seed nodes stamp blocks 1-3 on first start
- ✅ All 3 seeds have identical block hashes
- ✅ Miners sync from seeds (height 3+)
- ✅ Miners can mine new blocks (height 4+)
- ✅ v1.x nodes rejected on testnet
- ✅ Mainnet Guardian launch unaffected
