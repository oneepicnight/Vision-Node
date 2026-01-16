# 🔒 FORT KNOX MAINNET SECURITY LOCKDOWN

**Status:** ✅ COMPLETE  
**Date:** January 15, 2026  
**Version:** v1.0.3 Mainnet

---

## Overview

The Vision Node codebase has been **hardened for mainnet production** with all consensus-critical and security-sensitive parameters **hardcoded**. Environment variables are **disabled** to prevent runtime tampering and ensure all nodes run identical consensus rules.

## Security Benefits

✅ **Prevents Runtime Tampering** - No .env files can alter consensus  
✅ **Ensures Network Consensus** - All nodes run identical rules  
✅ **Eliminates Attack Surface** - No configuration-based exploits  
✅ **Forces Localhost API** - Prevents accidental internet exposure  
✅ **Disables Test Backdoors** - No debug/dev mode in production  

---

## Hardcoded Parameters

### Consensus Parameters
```rust
// Block production
target_block_time: 2 seconds
min_difficulty: 100
initial_difficulty: 100
block_weight_limit: 400_000
block_target_txs: 200

// Reorg protection
max_reorg: 36 blocks
max_reorg_bootstrap: 2048 blocks (sync mode)
retarget_window: 20 blocks

// Mempool
mempool_max: 10_000 transactions
mempool_ttl: 900 seconds (15 minutes)
mempool_sweep: 60 seconds

// Rate limiting
rate_submit_rps: 8 requests/second
rate_gossip_rps: 20 requests/second
```

### Tokenomics Parameters
```rust
enable_emission: true
emission_per_block: 32_000_000_000 (32 * 10^9)
halving_interval_blocks: 2_102_400 (~4 years @ 1.25s)
fee_burn_bps: 1000 (10%)
treasury_bps: 0 (disabled)
staking_epoch_blocks: 720
decimals: 9

// Vault addresses (hardcoded)
vault_addr: "b977c16e539670ddfecc0ac902fcb916ec4b944e"
fund_addr: "8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd"
treasury_addr: "df7a79291bb96e9dd1c77da089933767999eabf0"
```

### P2P Configuration
```rust
p2p_port: 7072 (mainnet P2P port)
p2p_host: "0.0.0.0" (accept connections from network)
min_outbound_connections: 1
max_outbound_connections: 50
connection_timeout: 10 seconds
reconnection_interval: 30 seconds

// Seed peers: Hardcoded from src/p2p/seed_peers.rs
// No VISION_P2P_SEEDS environment variable allowed
```

### HTTP API Configuration
```rust
http_host: "127.0.0.1" (localhost ONLY)
http_port: 7070
max_body_bytes: 262_144 (256 KB)
read_timeout: 10 seconds

// CORS: localhost only
allowed_origins: [
    "http://localhost:7070",
    "http://127.0.0.1:7070",
    "https://localhost:7070",
    "https://127.0.0.1:7070",
]

// No VISION_PUBLIC_NODE mode allowed
// No VISION_CORS_ORIGINS custom origins
```

### Storage & Archival
```rust
data_dir: "./vision_data" (single mainnet instance)
prune_depth: 0 (archival mode - no pruning)
min_blocks_to_keep: 1000
snapshot_every_blocks: 1000
snapshot_retention: 10 (last 10 snapshots)
prune_interval: 30 seconds
```

### Fee Configuration
```rust
fee_base: 1
fee_per_recipient: 0
initial_base_fee: 1_000_000_000 (1 Gwei, EIP-1559 style)
target_block_fullness: 0.5 (50%)
base_fee_max_change: 8 (12.5% per block)
```

### Mining Configuration
```rust
miner_address: "pow_miner" (default, can be changed via API)
miner_require_sync: false
miner_max_lag: 0
discovery_interval: 15 seconds
```

### Execution Configuration
```rust
parallel_execution: true
parallel_min_txs: 10
block_util_high: 0.8 (80%)
block_util_low: 0.3 (30%)
```

### Background Tasks
```rust
gossip_peer_gap: 50 milliseconds
ip_bucket_ttl: 300 seconds (5 minutes)
mempool_save_interval: 60 seconds
peer_diagnostics_interval: 30 seconds
reputation_summary_interval: 60 seconds
```

---

## Remaining Runtime Configuration

**Only 2 environment variables are honored:**

1. **`VISION_ADMIN_TOKEN`** - Admin API authentication  
   Required for `/admin/*` endpoints

2. **`VISION_LOG`** (or `RUST_LOG`) - Logging level  
   Controls tracing verbosity: `error`, `warn`, `info`, `debug`, `trace`

---

## Files Modified

### Core File Changes
- `src/main.rs` - Hardcoded all configuration (9 major sections)

### Archived Files
All `.env*` files moved to `archived-env-files/` with `.ARCHIVED` suffix:
- `.env`
- `.env.example`
- `.env.genesis`
- `.env.anchor-template`
- `.env.miner-template`
- `.env.tester.v3.0.0`
- `.env.tokenomics.sample`
- `.env.v3.0.0`

**⚠️ These files are NO LONGER USED**

---

## Code Sections Modified

### 1. Consensus Limits (`load_limits()`)
Hardcoded: block_weight_limit, block_target_txs, max_reorg, mempool_max, rate limits, target_block_time, retarget_window

### 2. Tokenomics (`load_tokenomics_cfg()`)
Hardcoded: emission, halving, fee burn/treasury, vault addresses

### 3. Pruning (`prune_depth()`, `is_archival_mode()`)
Hardcoded: Always archival mode (no pruning)

### 4. Mining Difficulty (`ACTIVE_MINER`)
Hardcoded: target_block_time (2s), min_difficulty (100)

### 5. Fee System
Hardcoded: fee_per_recipient, initial_base_fee, target_block_fullness, base_fee_max_change_denominator

### 6. Execution (`parallel_execution_enabled()`, etc.)
Hardcoded: Parallel execution enabled, min 10 txs

### 7. Miner Config
Hardcoded: miner_require_sync, miner_max_lag, discovery_secs, block_target_txs, block_util_high/low, mempool_max, mempool_ttl

### 8. Global Statics
- `FEE_BASE`: Hardcoded to 1
- `CHAIN`: Hardcoded data dir `./vision_data`
- `API_KEYS`: Empty (disabled)
- `MINER_ADDRESS`: Hardcoded to "pow_miner"

### 9. P2P Initialization
Hardcoded: seed_peers (from constants), min_peers (1), max_peers (50), p2p_port (7072), p2p_host (0.0.0.0)

### 10. HTTP Server
Hardcoded: http_host (127.0.0.1), port (7070), body_limit (256KB), timeout (10s)

### 11. CORS
Hardcoded: localhost origins only, removed VISION_CORS_ORIGINS and VISION_DEV mode

### 12. Data Directory
Hardcoded: `./vision_data` in `debug_chain_tip()` endpoint

### 13. Background Tasks
Hardcoded: gossip_peer_gap (50ms), ip_bucket_ttl (300s), mempool_save_interval (60s), prune_interval (30s), snapshot_retention (10)

### 14. Bootstrap
Removed: VISION_BOOTNODES environment variable (use hardcoded seed peers only)

---

## Testing Checklist

✅ **Build Test**: Verify no compilation errors  
✅ **Startup Test**: Confirm node starts with hardcoded config  
✅ **P2P Test**: Verify connection to hardcoded seed peers  
✅ **HTTP Test**: Confirm API only on localhost:7070  
✅ **Mining Test**: Verify mining with hardcoded difficulty  
✅ **Sync Test**: Confirm sync works with hardcoded timeouts  
✅ **Env Var Test**: Verify environment variables are ignored (except VISION_ADMIN_TOKEN, VISION_LOG)

---

## Deployment Notes

### Building for Production
```powershell
cargo build --release
```

### Running the Node
```powershell
# Minimal runtime config (optional):
$env:VISION_ADMIN_TOKEN="your-secret-token"
$env:VISION_LOG="info"

# Start node
.\vision-node.exe
```

### Expected Startup Logs
```
🔒 Vision node starting up
Vision node HTTP API listening on 127.0.0.1:7070 (localhost only for security)
🔌 Starting P2P listener on 0.0.0.0:7072
✅ P2P manager initialized with node_id: [8 chars]
🔧 Starting connection maintainer (min_peers: 1, max_peers: 50)
```

---

## Security Considerations

### What's Protected
- ✅ Consensus rules cannot be altered at runtime
- ✅ P2P configuration cannot be changed
- ✅ HTTP API is localhost-only by default
- ✅ No accidental public node exposure
- ✅ No test/dev mode backdoors

### What's NOT Protected
- ⚠️ Admin token still needs to be kept secure
- ⚠️ Wallet keys (keys.json) need filesystem protection
- ⚠️ Network-level DoS (use firewall/rate limiting)

### Attack Surface Reduced
- **Before**: ~50+ environment variables could alter behavior
- **After**: 2 environment variables (admin token, logging)
- **Reduction**: 96% smaller configuration attack surface

---

## Rollback Instructions

If you need to restore environment variable support (NOT RECOMMENDED for mainnet):

1. Restore original `src/main.rs` from git:
   ```powershell
   git checkout HEAD~1 src/main.rs
   ```

2. Restore `.env` files:
   ```powershell
   Copy-Item "archived-env-files\*" "." -Force
   Get-ChildItem ".env*.ARCHIVED" | Rename-Item -NewName { $_.Name -replace '\.ARCHIVED$', '' }
   ```

3. Rebuild:
   ```powershell
   cargo build --release
   ```

---

## Version History

- **v1.0.3** (2026-01-15) - Fort Knox lockdown implemented
- **v1.0.2** - Swarm intelligence enhancements (max peers 50)
- **v1.0.1** - Sync timeout fixes (20s/15s)
- **v1.0.0** - Mainnet launch

---

## Support

For questions or issues:
- Check logs with `VISION_LOG=debug`
- Review [NETWORK_SECURITY_FIXES_TODO.md](NETWORK_SECURITY_FIXES_TODO.md)
- Contact: support@visionworld.tech

---

**🔒 FORT KNOX LOCKDOWN: MAINNET PRODUCTION READY 🔒**
