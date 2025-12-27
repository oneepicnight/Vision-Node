# 🌱 Genesis Seed Peers - Offline-First Bootstrap

## Overview

Vision nodes now support **offline-first P2P bootstrap** using genesis seed peers. This eliminates dependency on the Guardian beacon for initial peer discovery.

## Seed Peer File Location

### Windows
```
C:\vision-node\vision_data\seed_peers.json
```

### Linux
```
/opt/vision-node/vision_data/seed_peers.json
```

### macOS
```
~/vision-node/vision_data/seed_peers.json
```

## Genesis Seed Peers (v1)

```json
{
  "version": 1,
  "generated_at": "2025-12-03T00:00:00Z",
  "description": "Vision Genesis Seed Peers",
  "peers": [
    "16.163.123.221:7070",
    "69.173.206.211:7070",
    "69.173.207.135:7070",
    "74.125.212.204:7070",
    "75.128.156.69:7070",
    "98.97.137.74:7070",
    "182.106.66.15:7070"
  ]
}
```

## Bootstrap Sequence

When a constellation node starts, it follows this priority:

1. **Phase 0: Genesis Seed Peers** (OFFLINE-FIRST)
   - Loads from `vision_data/seed_peers.json` or uses hardcoded defaults
   - Attempts direct TCP connection to each seed (IPv4 only)
   - Seeds marked as `is_seed=true` (protected from rolling mesh eviction)
   - Seeds start with `health_score=100` and `trusted=true`

2. **Phase 1: Local Peer Book**
   - Uses `get_best_peers(64, min_health=30)` for healthy peers
   - Fastest reconnection method

3. **Phase 2: Environment Variable Seeds**
   - `VISION_BOOTSTRAP_PEERS` comma-separated list

4. **Phase 3: Guardian Beacon Discovery**
   - `/api/beacon/peers` endpoint (fallback)

## Key Features

### 🔒 Protected Seeds
- Genesis seeds are marked `is_seed=true`
- **NEVER evicted** from rolling mesh (even at 1000+ peer capacity)
- Always prioritized for reconnection

### 💚 High Health Score
- Seeds start at `health_score=100` (maximum)
- Success/failure tracking still applies
- But seed status protects them from eviction

### 🌐 Offline-First
- Works without Guardian beacon
- Direct P2P TCP connections
- No HTTP dependency for bootstrap

### 📚 Persistent Storage
- Seeds added to peer book on first run
- Future startups use cached seeds
- Rolling mesh maintains seed protection

## Implementation Details

### Code Files
- `src/p2p/seed_peers.rs` - Seed peer loader and bootstrap
- `src/p2p/bootstrap.rs` - Unified bootstrap sequence (Phase 0-3)
- `vision_data/seed_peers.json` - Seed peer configuration

### Log Messages

**Successful seed load:**
```
[SEED_PEERS] ✅ Loaded 7 genesis seed peers from vision_data/seed_peers.json
```

**Bootstrap initiation:**
```
[SEED_BOOTSTRAP] 🌱 Starting genesis seed bootstrap (7 seeds)
[SEED_BOOTSTRAP] Connecting to genesis seed: 16.163.123.221:7070
```

**Successful connection:**
```
[SEED_BOOTSTRAP] ✅ Connected to seed: 16.163.123.221:7070
[P2P] ✅ Connected to constellation peer GENESIS-SEED-0 at 16.163.123.221:7070
```

**Peer book integration:**
```
[PEER BOOK] 🌱 Bootstrapped 7 genesis seeds to peer book (protected from eviction)
```

## Testing

### Verify Seed File Exists
```powershell
Test-Path "C:\vision-node\vision_data\seed_peers.json"
```

### Check Peer Book Stats
```powershell
curl http://localhost:7070/p2p/peers/status | ConvertFrom-Json
```

Expected output:
```json
{
  "total": 7,
  "seeds": 7,
  "avg_health": 100.0,
  "top_sample": [...]
}
```

### Monitor Bootstrap Logs
```powershell
# Watch for seed bootstrap messages
Get-Process vision-node | ForEach-Object { 
    # Logs will show in node output
}
```

## Packaging Checklist

For v1.1.0 release packages:

- [ ] Include `vision_data/seed_peers.json` in Constellation packages
- [ ] Include `vision_data/seed_peers.json` in Guardian packages (for consistency)
- [ ] Update startup scripts to create `vision_data` directory
- [ ] Document seed peers in README
- [ ] Test offline startup (no beacon, only seeds)

## Benefits

### ✅ For Testers
- Can mine **TONIGHT** without waiting for beacon
- Direct P2P connections = faster startup
- No single point of failure

### ✅ For Network
- Constellation doesn't depend on beacon responses
- Decentralized peer discovery begins immediately
- Guardian becomes passport issuer, not bottleneck

### ✅ For Mainnet
- Stable root mesh from day 1
- Genesis seeds form backbone
- Organic growth from trusted seeds

## Future Enhancements

1. **DNS Seeds** - `seed.visionworld.tech` returns A records for seed IPs
2. **Seed Rotation** - Update mechanism for seed_peers.json
3. **Seed Health Monitoring** - Track seed uptime and quality
4. **Dynamic Seed Discovery** - Nodes share their best peers as seeds

---

**Status:** ✅ IMPLEMENTED - Ready for v1.1.0 release
