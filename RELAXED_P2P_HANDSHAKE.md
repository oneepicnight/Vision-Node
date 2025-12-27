# Relaxed P2P Handshake Architecture (v2.7.0+)

## Overview

Starting with v2.7.0, Vision Node uses a **relaxed P2P handshake** by default. This allows nodes on different versions, bootstrap prefixes, and protocol versions to connect freely. Chain consensus safety is provided by HTTP anchor truth (port 7070), so P2P connections (port 7072) can be permissive.

## Architecture

### Two-Port System

```
Port 7070 (HTTP):
  - Serves canonical chain truth from anchors
  - Provides peer lists via /api/p2p/seed_peers
  - Supplies network status via /api/status
  - Read-only, safe, no authentication needed
  - ANCHORS provide truth, EDGE nodes consume it

Port 7072 (P2P):
  - Permissive transport for blocks and transactions
  - Only hard safety checks (chain_id, genesis_hash)
  - Warnings for version/prefix mismatches (not rejections)
  - Fat pipe for data, not consensus arbiter
```

## Validation Rules

### Hard Safety Checks (Always Enforced)

These will **reject** the connection:

1. **chain_id**: Must match configured network
   - Prevents connecting to completely different chains
   - Computed from Network enum (testnet/mainnet)

2. **genesis_hash**: Must match locked genesis
   - Prevents chain forks from connecting
   - Uses "Genesis Door" pattern for new nodes

### Soft Checks (Warnings Only)

These will **log warnings** but **allow** the connection:

1. **bootstrap_prefix**: Expected to match `VISION_BOOTSTRAP_PREFIX`
   - Mismatch logs warning
   - Different testnet drops can connect
   - Useful during network upgrades

2. **protocol_version**: Expected in range [MIN, MAX]
   - Out-of-range logs warning
   - Forward/backward compatibility enabled
   - Allows gradual rollouts

3. **node_version**: Expected >= `VISION_MIN_NODE_VERSION`
   - Old version logs warning
   - Legacy nodes can participate
   - Smooth upgrade path

4. **bootstrap_checkpoint**: Expected to match checkpoint hash/height
   - Mismatch logs warning
   - Different bootstrap states can coexist

## Strict Mode (Opt-In)

If you need the old strict behavior:

```bash
# In .env file:
VISION_P2P_STRICT=1
```

With strict mode enabled:
- Bootstrap prefix mismatches → **REJECT**
- Protocol version out of range → **REJECT**
- Old node version → **REJECT**
- Bootstrap checkpoint mismatch → **REJECT**

Use this only if you want to enforce a homogeneous network (e.g., private testnet).

## Why Relaxed?

### Problem with Strict Validation

In v2.6 and earlier, P2P handshakes were strict:
- Version mismatch → connection rejected
- Bootstrap prefix mismatch → connection rejected
- Protocol version out of range → connection rejected

This caused:
- **Hard network splits** during upgrades
- **Slow adoption** of new versions
- **Fragmentation** into incompatible islands
- **Poor UX** - nodes couldn't connect to "wrong" version

### Solution: HTTP Anchor Truth

With HTTP anchor system:
- **Anchors** (publicly reachable nodes) serve canonical truth via HTTP
- **Edge nodes** query anchors for:
  - Current block height
  - Network tip hash
  - Mining eligibility
  - Healthy peer lists
- **P2P connections** are just transport
- **Chain safety** comes from HTTP consensus, not P2P validation

### Benefits

✅ **Smooth upgrades**: New versions can coexist with old versions  
✅ **No network splits**: Chain truth from anchors prevents forks  
✅ **Better connectivity**: More peers = more resilience  
✅ **Gradual rollout**: No forced "flag day" upgrades  
✅ **Testing flexibility**: Dev/test nodes can connect to production

## HTTP Bootstrap Flow

```
1. Node starts up
2. Reads VISION_ANCHOR_SEEDS from env
3. Queries each anchor: GET http://IP:7070/api/p2p/seed_peers
4. Receives JSON array of healthy peers:
   [
     {"address": "1.2.3.4:7072", "is_anchor": true},
     {"address": "5.6.7.8:7072", "is_anchor": false},
     ...
   ]
5. Inserts peers into peer book
6. Attempts P2P connections (port 7072)
7. Relaxed handshake accepts diverse peers
8. Node syncs blocks via P2P
9. Queries anchors periodically for network truth
```

## Logging

With relaxed handshakes, you'll see warnings like:

```
[P2P] ⚠️  Bootstrap prefix mismatch - allowing connection
         (local='drop-2025-12-10', remote='drop-2025-12-09', build='v2.6.0')

[P2P] ⚠️  Protocol version out of range - allowing connection
         (allowed=250-250, remote=249, build='v2.6.0')

[P2P] ⚠️  Old node version - allowing connection
         (min='2.7.0', remote='2.6.0', build='v2.6.0')
```

These are **expected** and **harmless**. The connection succeeds.

## Testing

### Test Relaxed Mode (Default)

1. Start a v2.7.0 node
2. Check logs - should accept connections from any version
3. Visit http://localhost:7070/api/status
4. Look for `connected_peers` > 0

### Test Strict Mode

1. Add `VISION_P2P_STRICT=1` to .env
2. Restart node
3. Check logs - should reject version/prefix mismatches
4. Only v2.7.0 nodes with matching prefix will connect

## Implementation Details

File: `src/p2p/connection.rs`

```rust
fn validate(&self) -> Result<(), String> {
    // Opt-in to strict mode (default is relaxed)
    let strict_mode = std::env::var("VISION_P2P_STRICT").is_ok();
    
    // Bootstrap prefix check
    if self.bootstrap_checkpoint_hash != VISION_BOOTSTRAP_PREFIX {
        if strict_mode {
            return Err("BOOTSTRAP PREFIX MISMATCH");
        } else {
            warn!("Bootstrap prefix mismatch - allowing connection");
        }
    }
    
    // Protocol version check
    if self.protocol_version out of range {
        if strict_mode {
            return Err("PROTOCOL VERSION OUT OF RANGE");
        } else {
            warn!("Protocol version out of range - allowing connection");
        }
    }
    
    // Node version check
    if self.node_version < MIN_NODE_VERSION {
        if strict_mode {
            return Err("NODE VERSION TOO OLD");
        } else {
            warn!("Old node version - allowing connection");
        }
    }
    
    // HARD SAFETY: always enforced
    if self.chain_id != expected_chain_id {
        return Err("CHAIN ID MISMATCH");
    }
    
    if self.genesis_hash != local_genesis {
        return Err("GENESIS HASH MISMATCH");
    }
    
    Ok(())
}
```

## FAQ

### Q: Is this secure?

**A:** Yes. Chain safety comes from:
1. **chain_id check** (prevents wrong chain)
2. **genesis_hash check** (prevents forks)
3. **HTTP anchor consensus** (canonical truth)

Version/prefix mismatches are cosmetic, not security issues.

### Q: Can a malicious node exploit this?

**A:** No. The relaxed checks only allow *connectivity*. Mining eligibility, block validation, and transaction verification are still strict. A node with wrong version/prefix can connect, but can't:
- Mine invalid blocks (PoW still required)
- Submit invalid transactions (validation unchanged)
- Corrupt chain state (genesis lock prevents forks)

### Q: Why not just remove version checks entirely?

**A:** We keep them as warnings for:
- **Observability**: Network operators see version distribution
- **Debugging**: Identify incompatibilities early
- **Metrics**: Track upgrade adoption rates
- **Opt-in strictness**: Private networks can enable VISION_P2P_STRICT

### Q: When should I use strict mode?

**A:** Only for:
- Private testnets
- Development clusters
- Controlled environments
- When you want homogeneous network

Public testnet/mainnet should use relaxed mode.

## Migration Guide

### From v2.6 (Strict) to v2.7 (Relaxed)

1. Update to v2.7.0
2. Restart node
3. Node will now accept connections from v2.6 nodes
4. Observe warnings in logs (expected)
5. Once most network is v2.7, old nodes will gradually upgrade

### Rolling Back

If you need to revert to strict validation:

1. Add `VISION_P2P_STRICT=1` to .env
2. Restart node
3. Only v2.7.0 nodes with matching prefix will connect

## Summary

| Aspect | v2.6 (Strict) | v2.7 (Relaxed) |
|--------|---------------|----------------|
| **Bootstrap prefix** | Must match → REJECT | Warning only → ALLOW |
| **Protocol version** | Must be in range → REJECT | Warning only → ALLOW |
| **Node version** | Must be >= MIN → REJECT | Warning only → ALLOW |
| **Chain ID** | Must match → REJECT | Must match → REJECT |
| **Genesis hash** | Must match → REJECT | Must match → REJECT |
| **Upgrade path** | Hard network split | Smooth coexistence |
| **Connectivity** | Fragmented islands | Full mesh |
| **Safety model** | P2P validation | HTTP anchor truth |

**Recommendation**: Use relaxed mode (default) for public networks.
