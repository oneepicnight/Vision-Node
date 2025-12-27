# P2P Debug Guide - Vision Node v2.7.0

## 🎯 Problem: Nodes Can't Connect

If your Vision Node v2.7.0 can't connect to other v2.7 nodes even though:
- P2P is listening on 0.0.0.0:7072
- Advertised address is correct
- Routers are configured properly
- v2.5 nodes could connect fine

Then the issue is likely **strict handshake validation** added in v2.7.

## 🔍 Quick Diagnosis

### Step 1: Check What Your Node Sees

Visit on **BOTH nodes**:
```
http://127.0.0.1:7070/api/p2p/debug
```

This returns JSON with all handshake validation constants:
```json
{
  "node_build": "v2.7-constellation",
  "node_version": 270,
  "protocol_version": 2,
  "chain_id": "abc123...",
  "bootstrap_prefix": "drop-2025-12-10",
  "bootstrap_checkpoint_height": 9,
  "bootstrap_checkpoint_hash": "e5bddd1e4081...",
  "min_protocol": 250,
  "max_protocol": 250,
  "min_node_version": "2.5.0",
  "advertised_p2p": "1.2.3.4:7072",
  "p2p_port": 7072,
  "debug_allow_all": false
}
```

### Step 2: Compare Fields

If **ANY** of these differ between nodes, they **CANNOT** connect:

1. **bootstrap_prefix** - Must match exactly
   - Different = different testnet drop
   - Value: "drop-2025-12-10"

2. **protocol_version** - Must be within min/max range
   - v2.7: Accepts protocol_version between 250-250 (zero tolerance)
   - v2.5: Used protocol_version=1
   - **This is why v2.7 can't talk to v2.5!**

3. **min_node_version** - Remote version must be >= this
   - v2.7: Requires "2.5.0" or higher
   - Blocks older builds

4. **chain_id** - Must match
   - Prevents mainnet/testnet mixing

## 🧪 Debug Override Mode

To prove that strict validation is the problem:

### Step 1: Enable Debug Mode

In `.env` file, uncomment:
```bash
VISION_P2P_DEBUG_ALLOW_ALL=1
```

### Step 2: Restart Node

Stop the node with Ctrl+C, then restart:
```bash
.\START-PUBLIC-NODE.bat
```

### Step 3: Watch the Logs

With debug mode ON, you'll see warnings like:
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
[P2P] 🔍 Validating handshake from node_id=abc build='v2.7-constellation' proto=2 node_ver=270 chain_id=def bootstrap_prefix='drop-2025-12-10'
[P2P] ⚠️  DEBUG: Allowing peer with mismatched bootstrap prefix (local='drop-2025-12-10', remote='drop-2025-12-09', build='v2.7-constellation')
```

If connections **START WORKING** with debug mode on, you've confirmed:
- The handshake validation is too strict
- One of the fields doesn't match between nodes

## 📋 What Changed Between v2.5 and v2.7

### v2.5 Handshake (Lenient)
- Protocol version: 1
- No bootstrap prefix check
- No minimum node version check
- Accepted almost any peer

### v2.7 Handshake (Strict)
- Protocol version: Must be 250 exactly (min=250, max=250)
- Bootstrap prefix: Must match "drop-2025-12-10"
- Minimum node version: Must be >= "2.5.0"
- Zero tolerance for mismatches

**This is why v2.7 nodes reject each other!**

## 🛠️ Fixes

### Option 1: Relax Constants (Temporary)

In `src/vision_constants.rs`:

```rust
// Before (strict):
pub const VISION_MIN_PROTOCOL_VERSION: u32 = 250;
pub const VISION_MAX_PROTOCOL_VERSION: u32 = 250;

// After (relaxed):
pub const VISION_MIN_PROTOCOL_VERSION: u32 = 1;   // Allow v2.5+
pub const VISION_MAX_PROTOCOL_VERSION: u32 = 250; // Accept up to v2.7
```

This allows v2.7 nodes to connect to v2.5 nodes.

### Option 2: Update Bootstrap Prefix

If nodes have different `bootstrap_prefix` values:

1. Check what each node has:
   ```bash
   curl http://node1:7070/api/p2p/debug | jq .bootstrap_prefix
   curl http://node2:7070/api/p2p/debug | jq .bootstrap_prefix
   ```

2. If they differ, rebuild with matching prefix:
   ```rust
   pub const VISION_BOOTSTRAP_PREFIX: &str = "drop-2025-12-10";
   ```

### Option 3: Keep Debug Mode On (Testing Only)

For temporary testing, just leave debug mode enabled:
```bash
VISION_P2P_DEBUG_ALLOW_ALL=1
```

**WARNING:** This allows ANY peer to connect, including:
- Incompatible protocol versions
- Different testnet drops
- Old/malicious builds

Only use for debugging!

## 🔬 Advanced Diagnostics

### Check Inbound TCP Accepts

Look for this log line when a peer connects:
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
```

If you DON'T see this, the problem is:
- Firewall blocking
- Port not forwarded
- Wrong advertised address

### Check Handshake Validation

After TCP accept, look for:
```
[P2P] 🔍 Validating handshake from node_id=...
```

If you see this followed by:
```
[P2P] Handshake rejected: ❌ BOOTSTRAP PREFIX MISMATCH
```

Then you know exactly which field is wrong.

### Check Rejection Reasons

All rejections now log clearly:
```
[P2P] Handshake rejected: ❌ PROTOCOL VERSION OUT OF RANGE
  Allowed range: 250-250
  Remote protocol: 2
  Remote build: v2.7-constellation
```

This tells you exactly what to fix.

## 📊 Log Patterns

### Successful Connection
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
[P2P] 🔍 Validating handshake from node_id=abc build='v2.7-constellation' proto=250 node_ver=270
[P2P] ✅ Handshake successful
```

### Bootstrap Prefix Mismatch
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
[P2P] 🔍 Validating handshake from node_id=abc build='v2.7-constellation' proto=250 node_ver=270
[P2P] Handshake rejected: ❌ BOOTSTRAP PREFIX MISMATCH
  Local prefix:  drop-2025-12-10
  Remote prefix: drop-2025-12-09
```

### Protocol Version Mismatch
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
[P2P] 🔍 Validating handshake from node_id=abc build='v2.5-constellation' proto=1 node_ver=250
[P2P] Handshake rejected: ❌ PROTOCOL VERSION OUT OF RANGE
  Allowed range: 250-250
  Remote protocol: 1
```

### Debug Mode Bypass
```
[P2P] 👋 Inbound TCP accepted from 1.2.3.4:50123
[P2P] 🔍 Validating handshake from node_id=abc build='v2.5-constellation' proto=1 node_ver=250
[P2P] ⚠️  DEBUG: Allowing peer with out-of-range protocol version (allowed=250-250, remote=1, build='v2.5-constellation')
[P2P] ✅ Handshake successful (debug override active)
```

## 🎯 Quick Fix for Early/Sparks Not Connecting

**Most likely cause:** Both nodes built on different days, so they have different `VISION_BOOTSTRAP_PREFIX` values.

**Quick fix:**

1. Enable debug mode on BOTH:
   ```bash
   VISION_P2P_DEBUG_ALLOW_ALL=1
   ```

2. Restart both nodes

3. Check if they connect

4. If YES: Rebuild both with same constants

5. If NO: Check `/api/p2p/debug` for other mismatches

## 📞 Support

If nodes still won't connect after debug mode:

1. Save output of `/api/p2p/debug` from BOTH nodes
2. Save logs showing handshake rejection
3. Check for firewall/router issues
4. Verify advertised_p2p addresses are correct

The problem is now **visible** in the logs with clear rejection reasons!
