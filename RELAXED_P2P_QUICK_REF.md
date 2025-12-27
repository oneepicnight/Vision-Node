# Relaxed P2P Handshake - Quick Reference

## What Changed in v2.7.0

### Old Behavior (v2.6 - Strict)
```
❌ Bootstrap prefix mismatch → Connection REJECTED
❌ Protocol version out of range → Connection REJECTED
❌ Node version too old → Connection REJECTED
❌ Chain ID mismatch → Connection REJECTED
❌ Genesis hash mismatch → Connection REJECTED
```

### New Behavior (v2.7.0 - Relaxed)
```
⚠️  Bootstrap prefix mismatch → WARNING only, connection ALLOWED
⚠️  Protocol version out of range → WARNING only, connection ALLOWED
⚠️  Node version too old → WARNING only, connection ALLOWED
❌ Chain ID mismatch → Connection REJECTED (hard safety)
❌ Genesis hash mismatch → Connection REJECTED (hard safety)
```

## Quick Commands

### Check Current Mode
```bash
# Look for strict_mode=true or strict_mode=false in logs
grep "strict_mode=" vision-node.log
```

### Enable Strict Mode
```bash
# Add to .env file:
VISION_P2P_STRICT=1
```

### Disable Strict Mode (Default)
```bash
# Remove or comment out from .env:
# VISION_P2P_STRICT=1
```

## Expected Log Messages

### Relaxed Mode (Default)
```
[P2P] 🔍 Validating handshake from node_id=abc... build='v2.6.0' (strict_mode=false)
[P2P] ⚠️  Bootstrap prefix mismatch - allowing connection
[P2P] ⚠️  Protocol version out of range - allowing connection
[P2P] ⚠️  Old node version - allowing connection
[P2P] Handshake successful with peer abc...
```

### Strict Mode
```
[P2P] 🔍 Validating handshake from node_id=abc... build='v2.6.0' (strict_mode=true)
[P2P] Handshake rejected: ❌ BOOTSTRAP PREFIX MISMATCH - peer is on a different testnet drop
```

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│                    Vision Node v2.7.0                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Port 7070 (HTTP)          Port 7072 (P2P)                 │
│  ================          ================                │
│                                                             │
│  📡 ANCHOR TRUTH           🔗 RELAXED HANDSHAKE            │
│  - Canonical chain         - Only chain_id matters          │
│  - Peer lists              - Only genesis_hash matters      │
│  - Network status          - Warnings for version/prefix    │
│  - Read-only               - Permissive connectivity        │
│                                                             │
│  Safety = HTTP anchors     Transport = P2P fat pipe        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## When to Use Each Mode

### Use Relaxed Mode (Default) When:
✅ Running on public testnet  
✅ Running on mainnet  
✅ Want smooth upgrades  
✅ Need maximum connectivity  
✅ Dealing with version diversity  

### Use Strict Mode When:
✅ Running private testnet  
✅ Development/testing cluster  
✅ Controlled environment  
✅ Want homogeneous network  
✅ Debugging version issues  

## Troubleshooting

### Problem: Nodes won't connect
**Solution**: Check you're using relaxed mode (no VISION_P2P_STRICT=1 in .env)

### Problem: Too many version warnings
**Solution**: This is expected during network upgrades - warnings are harmless

### Problem: Want to block old versions
**Solution**: Enable strict mode: `VISION_P2P_STRICT=1`

### Problem: Chain forked
**Solution**: Check genesis_hash matches. Relaxed mode doesn't allow genesis forks.

## HTTP Bootstrap Flow

```
1. Node starts
2. Queries VISION_ANCHOR_SEEDS → GET http://anchor:7070/api/p2p/seed_peers
3. Receives peer list: [{"address": "1.2.3.4:7072", "is_anchor": true}, ...]
4. Inserts peers into peer book
5. Connects to peers (port 7072) with relaxed handshake
6. Syncs blocks via P2P
7. Queries anchors for network truth
```

## Safety Guarantees

### What's Protected (Hard Checks)
✅ **chain_id**: Can't connect to wrong chain  
✅ **genesis_hash**: Can't connect to forked chain  

### What's Relaxed (Soft Checks)
⚠️  **bootstrap_prefix**: Can connect to different testnet drops  
⚠️  **protocol_version**: Can connect to different protocol versions  
⚠️  **node_version**: Can connect to older/newer node versions  

### Why This Is Safe
- Chain consensus comes from HTTP anchors (port 7070)
- P2P is just transport (port 7072)
- Block validation still strict
- Mining eligibility still strict
- Transaction validation still strict

## Implementation Files

```
src/p2p/connection.rs       # Handshake validation logic
src/p2p/bootstrap.rs        # HTTP seed peer hydration
src/p2p/peer_store.rs       # Peer export for HTTP endpoint
src/p2p/api.rs              # GET /api/p2p/seed_peers endpoint
```

## Environment Variables

```bash
# Anchor seeds for HTTP bootstrap
VISION_ANCHOR_SEEDS=16.163.123.221:7072,other.anchor:7072

# Strict mode (opt-in)
VISION_P2P_STRICT=1

# Pure swarm mode (recommended)
VISION_PURE_SWARM_MODE=true
```

## Testing

### Test Default (Relaxed) Behavior
```bash
# Start node
./vision-node

# Check logs for warnings (expected):
grep "allowing connection" vision-node.log

# Verify connections:
curl http://localhost:7070/api/status | jq .connected_peers
```

### Test Strict Behavior
```bash
# Add to .env:
echo "VISION_P2P_STRICT=1" >> .env

# Restart node
./vision-node

# Check logs for rejections:
grep "Handshake rejected" vision-node.log
```

## FAQ

**Q: Does relaxed mode reduce security?**  
A: No. Chain safety from chain_id + genesis_hash + HTTP anchor truth.

**Q: Can malicious nodes exploit relaxed validation?**  
A: No. Only connectivity is relaxed. Block/tx validation still strict.

**Q: Why warnings instead of silent acceptance?**  
A: Observability - operators see version distribution, upgrade progress.

**Q: Will this cause chain forks?**  
A: No. genesis_hash check prevents forks. HTTP anchors provide consensus.

**Q: Should I use strict mode?**  
A: Only for private/dev networks. Public networks use relaxed mode.

## One-Line Summary

**v2.7.0**: HTTP anchors provide chain truth (port 7070), P2P provides transport (port 7072). Only chain_id and genesis_hash cause handshake rejection - everything else is warnings.
