# Testnet Reset v2.0 - Quick Reference

## Version Info
- **Node**: v2.0
- **Protocol**: 2
- **Min Testnet**: 2

## Default Seeds (Testnet Only)
- 69.173.206.211:7070
- 69.173.207.135:7072
- 159.203.0.215:7070

## Enforcement Rules

### Testnet
- ✅ Protocol v2: Connect + Mine
- ❌ Protocol v1: **REJECTED**

### Mainnet
- ✅ Protocol v2: Connect + Mine
- ⚠️ Protocol v1: Compatible (temp)

## Startup Command
```bash
# Testnet (auto-connects to seeds)
VISION_NETWORK=testnet ./vision-node

# Mainnet
VISION_NETWORK=mainnet-full ./vision-node
```

## Verify Deployment
```bash
# Check binary version in banner
# Should show: "Vision Node v2.0 - Testnet Reset"
# Should show: "Protocol Version: 2"
```

## Troubleshooting

**Peer rejected**: "Testnet Reset: Peer protocol X is outdated"
→ Upgrade peer to v2.0

**Mining blocked**: "Mining disabled: Node protocol vX is outdated"
→ Replace binary with v2.0

**No seeds connecting**: Check firewall allows outbound 7070/7072

## Key Log Messages

### Success
```
✅ Testnet peer accepted: protocol v2 (node v200)
🌐 Testnet detected - using default testnet seeds
```

### Rejection
```
❌ Testnet Reset: Peer protocol 1 is outdated. Please upgrade to v2.0.
```

## Files Changed
- src/constants.rs
- src/p2p/connection.rs
- src/miner/manager.rs
- src/p2p/seed_peers.rs
- src/main.rs

## Build
- **Binary**: target/release/vision-node.exe
- **Size**: 24.79 MB
- **Date**: 2025-12-05 12:39 PM

---
**Status**: ✅ READY FOR DEPLOYMENT
