# Handshake Framing Fix - Binary Protocol

## Problem: Message Framing Corruption

### Error Observed
```
Message too large: 121348160 bytes
```

### Root Cause
The handshake was using **JSON serialization** with **length-prefix framing**, but the receiving side was reading garbage bytes as the length. This happened because:

1. **Inconsistent serialization**: Sender serialized with JSON, receiver expected different format
2. **No size validation**: Accepted any length value, even absurd ones (121MB)
3. **Mixed protocols**: Regular messages used JSON, but handshake framing was broken
4. **No explicit structure**: Handshake buried in P2PMessage enum, not a dedicated binary message

## Solution: Binary Handshake with Bincode

### 1. Dedicated Handshake Structure

**New Binary Format** (fixed size, ~100 bytes):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeMessage {
    pub protocol_version: u32,    // Must be 1
    pub chain_id: [u8; 32],       // Blake3 hash of "vision-testnet2"
    pub genesis_hash: [u8; 32],   // Genesis block hash (must match)
    pub node_nonce: u64,          // Random nonce (detect self-connections)
    pub chain_height: u64,        // Current blockchain height
    pub node_version: u32,        // Software version (0.1.0 = 100)
}
```

**Why bincode?**
- Fixed-size fields = predictable length
- No string overhead
- Fast serialization
- Clear framing boundaries

### 2. Strict Length-Prefix Framing

**Wire Protocol**:
```
[4 bytes BE: length] [length bytes: bincode(HandshakeMessage)]
```

**Send Handshake**:
```rust
async fn send_handshake(&self, writer: &mut WriteHalf) -> Result<(), String> {
    let handshake = HandshakeMessage::new()?;
    let data = bincode::serialize(&handshake)?;  // ~100 bytes
    let len = data.len() as u32;
    
    // Log what we're sending
    info!(serialized_length = len, "Handshake serialized");
    
    // Validate before sending
    if len > MAX_HANDSHAKE_SIZE {  // 10KB limit
        return Err(format!("Handshake too large: {} bytes", len));
    }
    
    // Write length prefix (big-endian)
    writer.write_all(&len.to_be_bytes()).await?;
    
    // Write handshake data
    writer.write_all(&data).await?;
    writer.flush().await?;
    
    Ok(())
}
```

**Receive Handshake**:
```rust
async fn receive_handshake(&self, reader: &mut ReadHalf) -> Result<HandshakeMessage, String> {
    // Read length prefix
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes);
    
    // Log what we received
    info!(received_length = len, "Received handshake length prefix");
    
    // CRITICAL: Validate length before allocation
    if len == 0 {
        return Err("Handshake length is 0".to_string());
    }
    
    if len > MAX_HANDSHAKE_SIZE {  // 10KB max
        return Err(format!(
            "Invalid handshake length: {} bytes (max {} bytes)",
            len, MAX_HANDSHAKE_SIZE
        ));
    }
    
    // Now safe to allocate
    let mut data = vec![0u8; len as usize];
    reader.read_exact(&mut data).await?;
    
    // Deserialize with bincode
    let handshake = bincode::deserialize(&data)?;
    
    // Validate protocol fields
    handshake.validate()?;
    
    Ok(handshake)
}
```

### 3. Validation Logic

**Three-Phase Validation**:

#### Phase 1: Protocol Version
```rust
if self.protocol_version != P2P_PROTOCOL_VERSION {
    return Err(format!(
        "Handshake failed: protocol mismatch local={} remote={}",
        P2P_PROTOCOL_VERSION, self.protocol_version
    ));
}
```

#### Phase 2: Chain ID
```rust
let expected_chain_id = blake3::hash(b"vision-testnet2");
if self.chain_id != expected_chain_id.as_bytes() {
    return Err(format!(
        "Handshake failed: chain ID mismatch (local={} remote={})",
        hex::encode(expected_chain_id),
        hex::encode(self.chain_id)
    ));
}
```

#### Phase 3: Genesis Hash
```rust
let our_genesis = CHAIN.lock().blocks[0].header.pow_hash;
let our_genesis_bytes = hex::decode(&our_genesis)?;

if self.genesis_hash != our_genesis_bytes[..] {
    return Err(format!(
        "Handshake failed: genesis mismatch (local={} remote={})",
        hex::encode(our_genesis_bytes),
        hex::encode(self.genesis_hash)
    ));
}
```

### 4. Constants Added

```rust
/// P2P protocol version - MUST match between nodes
const P2P_PROTOCOL_VERSION: u32 = 1;

/// Maximum handshake message size (10KB should be plenty)
const MAX_HANDSHAKE_SIZE: u32 = 10_000;
```

### 5. Connection Flow

**Inbound Connection** (accept peer):
```
1. Accept TCP connection
2. receive_handshake() - Read length prefix, validate, deserialize, validate fields
3. send_handshake() - Send our handshake response
4. Register peer
5. Start message loop for blocks/txs
```

**Outbound Connection** (connect to peer):
```
1. Connect TCP to peer
2. send_handshake() - Send our handshake first
3. receive_handshake() - Wait for peer response, validate
4. Register peer
5. Start message loop
```

## Expected Log Output

### Successful Handshake (Both Sides)

**Sender**:
```
INFO  Connecting to peer peer=192.168.1.100:7071
INFO  Sending handshake
INFO  Handshake serialized serialized_length=88
INFO  Handshake sent successfully
INFO  Waiting for peer handshake response...
INFO  Received handshake length prefix received_length=88
INFO  Handshake deserialized protocol_version=1 chain_height=42 node_version=100
INFO  Handshake validation successful
INFO  Peer handshake received and validated chain_height=42
INFO  Successfully connected to peer height=42
```

**Receiver**:
```
INFO  Accepted inbound connection peer=192.168.1.50:54321
INFO  Waiting to receive handshake...
INFO  Received handshake length prefix received_length=88
INFO  Handshake deserialized protocol_version=1 chain_height=7 node_version=100
INFO  Handshake validation successful
INFO  Handshake received and validated chain_height=7
INFO  Sending our handshake response
INFO  Handshake serialized serialized_length=88
INFO  Peer registered, starting message loop
```

### Failed Handshake: Invalid Length

**Old behavior**:
```
ERROR Message too large: 121348160 bytes
```

**New behavior**:
```
INFO  Received handshake length prefix received_length=121348160
ERROR Invalid handshake length: 121348160 bytes (max 10000 bytes)
ERROR Failed to receive handshake error="Invalid handshake length: 121348160 bytes (max 10000 bytes)"
```

### Failed Handshake: Protocol Mismatch

```
INFO  Received handshake length prefix received_length=88
INFO  Handshake deserialized protocol_version=2 chain_height=42 node_version=200
ERROR Handshake failed: protocol mismatch local=1 remote=2
ERROR Handshake validation failed error="Handshake failed: protocol mismatch local=1 remote=2"
```

### Failed Handshake: Genesis Mismatch

```
INFO  Received handshake length prefix received_length=88
INFO  Handshake deserialized protocol_version=1 chain_height=42 node_version=100
ERROR Handshake failed: genesis mismatch (local=000000abcd... remote=111111efgh...)
ERROR Handshake validation failed error="Handshake failed: genesis mismatch..."
```

## Key Improvements

### Before (Broken)
- ❌ JSON serialization (variable length, string overhead)
- ❌ No length validation (accepted 121MB)
- ❌ Mixed in P2PMessage enum
- ❌ Generic error messages
- ❌ No protocol versioning
- ❌ No chain validation

### After (Fixed)
- ✅ Bincode serialization (~88 bytes, fixed size)
- ✅ Strict length validation (max 10KB)
- ✅ Dedicated HandshakeMessage struct
- ✅ Detailed logging at each step
- ✅ Explicit protocol version check
- ✅ Chain ID and genesis validation
- ✅ Clear error messages with exact mismatch details

## Testing Verification

### 1. Check Handshake Length
```powershell
# Look for serialized_length in logs
Get-Content "stdout.log" | Select-String "serialized_length"

# Expected: serialized_length=88 (or similar small value)
# NOT: 121348160 or other huge numbers
```

### 2. Verify Handshake Exchange
```powershell
# Both nodes should log:
Get-Content "stdout.log" | Select-String "Handshake"

# Expected sequence:
# - "Sending handshake"
# - "Handshake serialized"
# - "Received handshake length prefix"
# - "Handshake deserialized"
# - "Handshake validation successful"
```

### 3. Check for Errors
```powershell
# Should NOT see:
Get-Content "stdout.log" | Select-String "Message too large|Invalid handshake length"

# If you DO see errors, they'll now be specific:
# - "protocol mismatch local=1 remote=2"
# - "chain ID mismatch"
# - "genesis mismatch"
```

### 4. Verify Peer Registration
```powershell
# Check /api/tcp_peers endpoint
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json

# Should show connected peers with:
# - peer_id (format: "peer-abcd123456789012")
# - height (should match other node)
# - direction (Inbound/Outbound)
```

## Compatibility

⚠️ **BREAKING CHANGE**: Cannot connect to old nodes

### Why?
- Old nodes: JSON handshake in P2PMessage enum
- New nodes: Binary handshake in HandshakeMessage struct
- Completely different wire format
- Different serialization (JSON vs bincode)

### Migration Required
**All nodes must be updated simultaneously**:
1. Stop all testnet nodes
2. Deploy updated binaries (both miner and public node)
3. Restart all nodes
4. Verify handshake logs show success

## Files Modified

1. **src/p2p/connection.rs**
   - Added `HandshakeMessage` struct (88 bytes binary)
   - Added `MAX_HANDSHAKE_SIZE` constant (10KB)
   - Added `HandshakeMessage::new()` - Create from local chain
   - Added `HandshakeMessage::validate()` - Three-phase validation
   - Added `HandshakeMessage::serialize()` - Bincode encoding
   - Added `HandshakeMessage::deserialize()` - Bincode decoding
   - Added `send_handshake()` - Length-prefix + bincode send
   - Added `receive_handshake()` - Length-prefix + bincode receive + validate
   - Updated `handle_inbound_connection()` - Use binary handshake
   - Updated `connect_to_peer()` - Use binary handshake

2. **Testnet Packages**
   - VisionNode-v0.1.6-testnet2-WIN64.zip (11.14 MB)
   - VisionNode-v0.1.6-testnet2-LINUX64.tar.gz (4.57 MB)

## Summary

### The Problem
```
Peer sends: [?? ?? ?? ??] [JSON garbage...]
Peer reads: [0x07 0x3C 0x72 0xC0] = 121348160 bytes (GARBAGE!)
Result: "Message too large: 121348160 bytes"
```

### The Fix
```
Peer sends: [0x00 0x00 0x00 0x58] [88 bytes bincode]
Peer reads: [0x00 0x00 0x00 0x58] = 88 bytes (VALID!)
Result: "Handshake validation successful"
```

### Next Steps
1. **Deploy both nodes** with updated binaries
2. **Watch logs** for handshake sequence
3. **Verify peers connect** via /api/tcp_peers
4. **Test block propagation** - miner finds block → public node receives it
5. **Celebrate** when you see "Handshake validation successful" on both sides! 🎉

**When this works, the Vision network will sync for the first time.** 🌅
