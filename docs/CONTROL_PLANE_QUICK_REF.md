# Control Plane Quick Reference

## What Changed in v2.7.0

**TL;DR**: HTTP 7070 is now the nervous system. P2P 7072 is optional muscle.

## Quick Test

### 1. Start node:
```bash
START-PUBLIC-NODE.bat  # Windows
./start-public-node.sh # Linux
```

### 2. Check logs for:
```
[BACKBONE] ✅ Connected to http://IP:7070 (XYZms) - tip=12345
[HEALING] 📥 Fetched 64 peers from anchor
[MINER] ⛏️ Mining enabled - will start when conditions allow
```

### 3. Visit panel: http://localhost:7070
- Look for 🌐 **Backbone (7070)** card
- Should show: ✅ Connected, anchor IP, latency, tip height

### 4. Click "Start Mining"
- Should **NOT freeze**
- Should **NOT reject** with errors
- Should log: "Mining enabled"
- If behind: Pauses cleanly, shows "Paused - syncing"

## Architecture

```
7070 (HTTP) = REQUIRED
  ✅ Peer discovery
  ✅ Network tip
  ✅ Health checks
  ✅ Identity/hello
  ✅ Exchange ready signal

7072 (P2P) = OPTIONAL
  📦 Block streaming
  💬 TX gossip
  🔧 Bulk transfer
```

## Key Files

| File | Purpose |
|------|---------|
| `src/control_plane.rs` | Control plane client + backbone state |
| `src/main.rs` | Starts probe/healing loops |
| `src/miner/manager.rs` | No blocking gate |
| `src/auto_sync.rs` | Uses backbone tip |
| `src/api/website_api.rs` | Exposes backbone status |
| `public/panel.html` | Shows 7070 connection |

## Env Vars

```bash
# Required: Anchor seeds for control plane
VISION_ANCHOR_SEEDS=16.163.123.221:7072,other:7072

# Optional: Enable strict P2P (default: relaxed)
VISION_P2P_STRICT=1
```

## API Response

GET http://localhost:7070/api/status

```json
{
  "http_backbone": {
    "connected": true,
    "anchor": "http://16.163.123.221:7070",
    "latency_ms": 128,
    "tip_height": 12345,
    "last_ok_unix": 1702334567,
    "last_error": null
  },
  "exchange_ok": true,
  "node_role": "Anchor",
  "can_mine": true
}
```

## Benefits

✅ **No freezes** - mining never blocks waiting  
✅ **Works behind CGNAT** - HTTP peer discovery  
✅ **Reliable tip** - from trusted anchors  
✅ **Clear status** - visible in panel  
✅ **Exchange ready** - API indicator  

## Troubleshooting

### Panel shows "⚠️ No response"
**Fix**: Check VISION_ANCHOR_SEEDS in .env

### Mining won't start
**Check**: Is backbone connected? Panel should show ✅

### Node isolated
**Check**: Backbone status in logs - should see "Connected to http://..."

### P2P not working
**Note**: That's OK! HTTP 7070 handles everything critical.

## One-Line Summary

**HTTP 7070 = nervous system (required), P2P 7072 = muscle (optional)**
