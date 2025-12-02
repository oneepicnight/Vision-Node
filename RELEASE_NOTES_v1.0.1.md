# Vision Node v1.0.1 Release Notes

**Release Date:** December 2, 2025  
**Package Version:** 1.0.1  
**Build Type:** Production Release

## 🎯 Critical Fixes

### 1. BEACON_ENDPOINT Configuration Fix
**Issue:** Linux constellation nodes were pointing to `localhost` instead of production beacon  
**Fix:** Updated all package `.env` files and startup scripts  
**Impact:** Linux nodes can now properly discover peers and join the constellation

**Before:**
```bash
BEACON_ENDPOINT=http://127.0.0.1:8080/api/beacon/register
```

**After:**
```bash
BEACON_ENDPOINT=https://visionworld.tech/api/beacon
```

**Files Updated:**
- `VisionNode-Constellation-v1.0.1-WIN64/.env`
- `VisionNode-Constellation-v1.0.1-LINUX64/.env`
- `VisionNode-Constellation-v1.0.1-LINUX64/START-VISION-NODE.sh`

### 2. P2P Networking Improvements

#### IPv4 Priority Everywhere
- Added `sort_ipv4_first()` helper function
- Beacon peer discovery now prioritizes IPv4 connections
- Reduces IPv6 connectivity issues

#### Anti-Garbage Handshake Protection
- Validates handshake packet size immediately (rejects > 10KB)
- Prevents `Invalid handshake length: 1347375956 bytes` errors
- Logs garbage packets at DEBUG level instead of ERROR

#### Quiet Logs - Reduced Log Spam
- Downgraded 5 handshake error logs from ERROR → DEBUG
- Normal P2P connection failures no longer spam console
- Cleaner log output for operators

#### Peer Reputation Memory (Anchor + Leaf System)
- **Anchor Peers:** Automatically promoted after 3+ successful connections
- **Leaf Peers:** Recent peers retained for 72 hours
- Enables autonomous network recovery
- Prevents "solo chain" mining scenarios

**Benefits:**
- Nodes automatically reconnect to reliable peers
- New nodes join constellation after meeting one anchor
- Network maintains connectivity even if guardian goes offline

### 3. Wallet SPA Asset Loading Fix
- Fixed `/app/assets/*` serving for wallet UI
- Build script now cleans old hashed assets before copying
- Vite config updated with `emptyOutDir: true`
- Prevents stale JavaScript/CSS files causing 404 errors

## 📦 Package Details

| Package | Platform | Size | Binary Hash |
|---------|----------|------|-------------|
| VisionNode-Constellation-v1.0.1-WIN64.zip | Windows x64 | 15.45 MB | SHA256: TBD |
| VisionNode-Constellation-v1.0.1-LINUX64.tar.gz | Linux x64 | 5.25 MB | SHA256: TBD |
| VisionNode-Guardian-v1.0.1-WIN64.zip | Windows x64 | 15.45 MB | SHA256: TBD |
| VisionNode-Guardian-v1.0.1-LINUX64.tar.gz | Linux x64 | 5.25 MB | SHA256: TBD |

## 🚀 Upgrade Instructions

### For Constellation Nodes (Miners)

**Windows:**
1. Stop your node: Close the terminal or press Ctrl+C
2. Extract `VisionNode-Constellation-v1.0.1-WIN64.zip`
3. Run `START-PUBLIC-NODE.bat`

**Linux:**
1. Stop your node: `pkill vision-node`
2. Extract: `tar -xzf VisionNode-Constellation-v1.0.1-LINUX64.tar.gz`
3. Run: `./START-VISION-NODE.sh`

### For Guardian Nodes

**Windows:**
1. Stop your node: Close the terminal or press Ctrl+C
2. Backup your `vision_data_7070` folder
3. Extract `VisionNode-Guardian-v1.0.1-WIN64.zip`
4. Run `START-GUARDIAN-NODE.bat`

**Linux:**
1. Stop your node: `pkill vision-node`
2. Backup your `vision_data_7070` folder
3. Extract: `tar -xzf VisionNode-Guardian-v1.0.1-LINUX64.tar.gz`
4. Run: `./START-GUARDIAN-NODE.sh`

## ✅ Testing Checklist

### Constellation Nodes
- [ ] Node starts and loads blockchain
- [ ] Connects to guardian beacon at `https://visionworld.tech/api/beacon`
- [ ] Discovers and connects to 3+ peer constellation nodes
- [ ] Mining activates and submits blocks
- [ ] Wallet UI loads at `http://localhost:7070/app` (no 404 errors)
- [ ] Panel loads at `http://localhost:7070/panel.html`

### Guardian Nodes
- [ ] Node starts in Guardian mode
- [ ] Beacon API responds at `/api/beacon/peers`
- [ ] Constellation nodes can register
- [ ] P2P connections show IPv4 addresses first
- [ ] No ERROR log spam from handshake failures
- [ ] Anchor peers accumulate over time

### Network Health
- [ ] Linux nodes appear in peer lists
- [ ] IPv4 connections prioritized over IPv6
- [ ] No "mining on solo chain" reports
- [ ] Peer count increases naturally over 24 hours

## 🐛 Known Issues

**Linux Binary Not Included:**
- Linux packages contain Windows binary placeholder
- Requires cross-compilation setup for Linux builds
- Testers should use WSL or native Linux builds

**Workaround:** Linux testers should build from source:
```bash
git clone <repo>
cd vision-node
cargo build --release
cp target/release/vision-node VisionNode-Constellation-v1.0.1-LINUX64/
```

## 📝 Files Changed

**Core:**
- `Cargo.toml` - Version bump to 1.0.1
- `src/main.rs` - Enhanced wallet asset serving comments
- `src/p2p/connection.rs` - IPv4 priority + anti-garbage handshake
- `src/p2p/beacon_bootstrap.rs` - IPv4 peer sorting
- `src/p2p/peer_memory.rs` - Anchor/leaf reputation system

**Configuration:**
- `.env` - BEACON_ENDPOINT → production URL
- `.env.example` - Added BEACON_ENDPOINT example
- All package `.env` files - Fixed beacon URLs

**Build:**
- `scripts/build-and-copy-wallet.ps1` - Improved build process
- `wallet-marketplace-source/vite.config.ts` - Added emptyOutDir
- `.gitignore` - Added /wallet/dist/

## 🔗 Related Documents

- `PATCH_v0.9.6_SUMMARY.md` - Detailed P2P patch implementation
- `BEACON_ENDPOINT` fix documentation in git history

## 💬 Support

For issues or questions:
- GitHub Issues: <repo URL>
- Discord: <invite link>
- Documentation: See README.txt in package

---

**Build Info:**
- Compiled: December 2, 2025
- Rust Version: 1.x.x
- Features: lite, multi-currency
- Variant: LITE (MVP)
