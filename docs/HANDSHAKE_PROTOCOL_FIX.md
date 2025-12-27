# P2P Handshake Protocol Fix

## Problem Identified

**Issue**: Nodes were successfully establishing TCP connections, but the handshake was failing with generic error messages like "Peer handshake failed: Message..."

**Root Cause**: The handshake protocol lacked:
1. **Protocol versioning** - No way to detect version mismatches
2. **Chain validation** - No verification that nodes are on the same chain
3. **Genesis verification** - No check that both nodes started from the same genesis block
4. **Detailed logging** - Generic error messages didn't explain WHY handshakes failed

## Solution Implemented

### 1. Enhanced Handshake Structure

**Old Handshake** (3 fields):
```rust
Handshake {
    version: u32,
    chain_height: u64,
    peer_id: String,
}
```

**New Handshake** (6 fields with validation):
```rust
Handshake {
    protocol_version: u32,  // P2P protocol version (must match)
    chain_id: String,       // Chain identifier (e.g., "vision-testnet2")
    genesis_hash: String,   // Genesis block hash (must match)
    chain_height: u64,      // Current blockchain height
    peer_id: String,        // Node's unique identifier
    node_version: String,   // Software version (for debugging)
}
```

### 2. Protocol Constants

Added explicit constants for validation:
```rust
const P2P_PROTOCOL_VERSION: u32 = 1;
const CHAIN_ID: &str = "vision-testnet2";
```

### 3. Handshake Validation Function

Created `validate_handshake()` that performs three critical checks:

#### Check 1: Protocol Version
```rust
if protocol_version != P2P_PROTOCOL_VERSION {
    return Err(format!(
        "Protocol version mismatch (local={} remote={})",
        P2P_PROTOCOL_VERSION, protocol_version
    ));
}
```

#### Check 2: Chain ID
```rust
if chain_id != CHAIN_ID {
    return Err(format!(
        "Chain ID mismatch (local='{}' remote='{}')",
        CHAIN_ID, chain_id
    ));
}
```

#### Check 3: Genesis Hash
```rust
let our_genesis = CHAIN.lock().blocks[0].header.pow_hash.clone();
if genesis_hash != &our_genesis {
    return Err(format!(
        "Genesis hash mismatch (local={} remote={})",
        our_genesis, genesis_hash
    ));
}
```

### 4. Enhanced Logging

#### Message Reception Logging
- Log message length prefix
- Log first 200 bytes of received data (for debugging)
- Log raw bytes on deserialization failure

#### Handshake Progress Logging
```rust
info!(peer = %peer_addr, "Accepted inbound connection");
info!(peer = %peer_addr, "Waiting to receive handshake...");
info!(
    peer_id = %peer_id,
    protocol_version = protocol_version,
    chain_id = %chain_id,
    genesis_hash = %genesis_hash,
    chain_height = chain_height,
    node_version = %node_version,
    "Received handshake"
);
info!(peer_id = %peer_id, "Handshake validation successful");
```

#### Error Logging
Each failure type now logs the EXACT reason:
```rust
error!(peer = %peer_addr, error = %e, "Failed to receive handshake");
error!(peer = %peer_addr, error = %e, "Handshake validation failed");
```

### 5. Both Directions Updated

Applied validation to:
- **Inbound connections** (`handle_inbound_connection`)
- **Outbound connections** (`connect_to_peer`)

Both sides now:
1. Send complete handshake with all 6 fields
2. Receive peer's handshake
3. Validate protocol version, chain ID, and genesis hash
4. Log detailed information at each step
5. Provide specific error messages on failure

## Expected Behavior After Fix

### Successful Handshake
```
INFO  Accepted inbound connection peer=192.168.1.100:54321
INFO  Waiting to receive handshake...
INFO  Received handshake peer_id="abc123..." protocol_version=1 chain_id="vision-testnet2" genesis_hash="000000..." chain_height=42 node_version="0.1.0"
INFO  Handshake validation successful peer_id="abc123..."
INFO  Sending our handshake response peer=192.168.1.100:54321
INFO  Registered new peer connection address="192.168.1.100:54321" peer_id="abc123..." height=42 direction=Inbound
```

### Protocol Version Mismatch
```
INFO  Accepted inbound connection peer=192.168.1.100:54321
INFO  Waiting to receive handshake...
INFO  Received handshake peer_id="xyz789..." protocol_version=2 chain_id="vision-testnet2" genesis_hash="000000..." chain_height=42 node_version="0.2.0"
ERROR Handshake validation failed peer=192.168.1.100:54321 error="Protocol version mismatch (local=1 remote=2)"
```

### Chain ID Mismatch
```
INFO  Accepted inbound connection peer=192.168.1.100:54321
INFO  Waiting to receive handshake...
INFO  Received handshake peer_id="xyz789..." protocol_version=1 chain_id="vision-mainnet" genesis_hash="000000..." chain_height=42 node_version="0.1.0"
ERROR Handshake validation failed peer=192.168.1.100:54321 error="Chain ID mismatch (local='vision-testnet2' remote='vision-mainnet')"
```

### Genesis Hash Mismatch
```
INFO  Accepted inbound connection peer=192.168.1.100:54321
INFO  Waiting to receive handshake...
INFO  Received handshake peer_id="xyz789..." protocol_version=1 chain_id="vision-testnet2" genesis_hash="111111..." chain_height=42 node_version="0.1.0"
ERROR Handshake validation failed peer=192.168.1.100:54321 error="Genesis hash mismatch (local=000000... remote=111111...)"
```

### Deserialization Failure
```
INFO  Accepted inbound connection peer=192.168.1.100:54321
INFO  Waiting to receive handshake...
DEBUG Received message length prefix: 256 bytes
DEBUG Received message data: {"protocol_version":1,"chain_id":"vision-test... (256 total bytes)
ERROR Deserialization failed: missing field `genesis_hash` at line 1 column 123
ERROR Raw bytes (first 500): [123, 34, 112, 114, 111, 116, 111, 99, 111, ...]
ERROR Failed to receive handshake peer=192.168.1.100:54321 error="Handshake receive failed: Failed to deserialize message: missing field `genesis_hash` at line 1 column 123"
```

## Testing the Fix

### 1. Verify Both Nodes Are Updated
```powershell
# Check binary timestamp
Get-Item ".\vision-node.exe" | Select-Object LastWriteTime

# Or check version in logs
# Look for "Received handshake" with "node_version" field
```

### 2. Check Handshake Logs
```powershell
# On public node
Get-Content "logs\*.log" | Select-String -Pattern "Handshake|handshake"

# You should see:
# - "Waiting to receive handshake..."
# - "Received handshake" with all 6 fields
# - "Handshake validation successful" OR specific error
```

### 3. Verify Peer Connection
```powershell
# Check /api/tcp_peers endpoint
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json

# Should show connected peers with:
# - address
# - peer_id
# - height
# - direction (Inbound/Outbound)
# - last_activity_secs
```

### 4. Confirm Block Propagation
```powershell
# On miner
Invoke-WebRequest -Uri "http://localhost:7070/api/status" | ConvertFrom-Json
# Note the height

# On public node
Invoke-WebRequest -Uri "http://localhost:7070/api/status" | ConvertFrom-Json
# Height should match miner (or be close)
```

## Compatibility

⚠️ **BREAKING CHANGE**: Old nodes cannot connect to new nodes

- **Old handshake format**: 3 fields (version, chain_height, peer_id)
- **New handshake format**: 6 fields (protocol_version, chain_id, genesis_hash, chain_height, peer_id, node_version)

**Impact**: All nodes on the network MUST be updated simultaneously

**Migration Strategy**:
1. Stop all nodes
2. Update all binaries to new version
3. Restart all nodes
4. Verify handshakes succeed with new logging

## Files Modified

1. **src/p2p/connection.rs**
   - Added `P2P_PROTOCOL_VERSION` and `CHAIN_ID` constants
   - Enhanced `P2PMessage::Handshake` struct (3 → 6 fields)
   - Created `validate_handshake()` function
   - Updated `handle_inbound_connection()` with validation
   - Updated `connect_to_peer()` with validation
   - Enhanced `receive_message()` logging

2. **Testnet Packages**
   - VisionNode-v0.1.6-testnet2-WIN64.zip (11.13 MB)
   - VisionNode-v0.1.6-testnet2-LINUX64.tar.gz (4.57 MB)

## Summary

**Before**: "Peer handshake failed: Message ..."
- No idea why it failed
- Could be version mismatch, wrong chain, bad data, etc.
- Generic error, impossible to debug

**After**: Explicit validation and detailed errors
- "Protocol version mismatch (local=1 remote=2)"
- "Chain ID mismatch (local='vision-testnet2' remote='vision-mainnet')"
- "Genesis hash mismatch (local=abc... remote=xyz...)"
- Clear, actionable error messages

**Result**: When handshake fails, logs will tell you EXACTLY why, making debugging trivial.

## Next Steps

1. Deploy updated packages to all testnet nodes
2. Restart all nodes simultaneously
3. Check logs for successful handshake messages
4. Verify `/api/tcp_peers` shows connected peers
5. Confirm blocks propagate between miner and public node
6. Monitor for any new handshake failures with specific error messages
