# Copilot Patch: Peer Connection Diagnostics & Fixes

**Problem**: Peers connecting then disappearing, "can't connect" loops, duplicate connections

**Root Causes**: 
1. No visibility into WHY connections are skipped
2. Peers keyed by ephemeral ports (53565, etc.) instead of normalized IP:7072
3. Duplicate connections replacing existing healthy ones
4. Handshake timeout too aggressive (3s) for slower peers

---

## âœ… Fix 1: Log WHY You Reject/Skip Connections

**Files Modified**: 
- `src/p2p/connection_maintainer.rs`
- `src/p2p/connection.rs`

**Before**: Silent skips with no visibility
```rust
if is_already_connected(&peer_store, seed_addr).await {
    debug!("[CONN_MAINTAINER] Already connected to seed: {}", seed_addr);
    continue;
}
```

**After**: Clear rejection logs
```rust
if is_already_connected(&peer_store, &normalized_seed).await {
    info!("[DIAL] SKIP already-connected key={}", normalized_seed);
    continue;
}

if let Some(reason) = validate_ip_for_dial(&seed_addr, &local_ips) {
    info!("[DIAL] SKIP {} ip={}", reason, seed_addr);
    continue;
}
```

**New Log Messages**:
- `[DIAL] SKIP already-connected key=192.168.1.100:7072`
- `[DIAL] SKIP non-public ip=10.0.0.5`
- `[DIAL] SKIP self ip=192.168.1.50`
- `[DIAL] SKIP ipv6 ip=[::1]:7072`
- `[DIAL] SKIP non-ipv4-or-loopback ip=127.0.0.1:7072`

**Impact**: 
- âœ… Instant visibility into connection failures
- âœ… Can immediately see if all dials are being skipped
- âœ… Easy to debug "won't connect" issues

---

## âœ… Fix 2: Normalize Peer Keys to IP:7072

**File Modified**: `src/p2p/connection.rs`

**Problem**: Peers stored with ephemeral ports causing:
- Duplicate connections from same IP
- "Already connected" checks failing
- Peer removal targeting wrong connection
- Peer count reads wrong

**Before**: Using ephemeral remote port as key
```rust
pub async fn register_peer(&self, address: String, ...) -> Arc<Mutex<PeerConnection>> {
    let peer = Arc::new(Mutex::new(PeerConnection { ... }));
    let mut peers = self.peers.lock().await;
    peers.insert(address.clone(), peer.clone());  // âŒ Uses 192.168.1.100:53565
}
```

**After**: Normalized to IP:7072
```rust
pub async fn register_peer(&self, address: String, ...) -> Arc<Mutex<PeerConnection>> {
    // Fix 2: Normalize peer key to IP:7072 (not ephemeral port)
    let normalized_key = if let Ok(sock_addr) = address.parse::<std::net::SocketAddr>() {
        format!("{}:7072", sock_addr.ip())
    } else {
        address.clone()
    };
    
    let peer = Arc::new(Mutex::new(PeerConnection { ... }));
    let mut peers = self.peers.lock().await;
    peers.insert(normalized_key.clone(), peer.clone());  // âœ… Uses 192.168.1.100:7072
}
```

**Outbound Dial**: Always dial IP:7072
```rust
// Fix 2: Normalize dial address to IP:7072 format
let dial_addr = if let Ok(sock_addr) = address.parse::<std::net::SocketAddr>() {
    format!("{}:7072", sock_addr.ip())
} else {
    address.clone()
};

info!(peer = %address, normalized = %dial_addr, "Connecting to peer");
let stream = match TcpStream::connect(&dial_addr).await { ... }
```

**Impact**: 
- âœ… One peer per IP, no duplicate connections
- âœ… Duplicate checks work correctly
- âœ… Peer count accurate
- âœ… No "connect, disappear, can't connect" loops

---

## âœ… Fix 3: First-Wins Duplicate Policy

**File Modified**: `src/p2p/connection.rs`

**Problem**: New duplicate connections replacing existing healthy ones

**Before**: Last connection wins (or both kept causing confusion)
```rust
// Old behavior: always register new peer
peers.insert(address.clone(), peer.clone());
```

**After**: First connection wins
```rust
// Fix 3: First-wins duplicate policy - keep existing connection
if peers.contains_key(&normalized_key) {
    info!(
        "[P2P] SKIP duplicate connection to {} (first connection wins)",
        normalized_key
    );
    // Return existing peer instead of replacing
    return peers.get(&normalized_key).unwrap().clone();
}

peers.insert(normalized_key.clone(), peer.clone());
```

**Inbound Handler**:
```rust
// Fix 2 & 3: Check for duplicate before handshake
let normalized_addr = format!("{}:7072", peer_addr.ip());
let already_connected = {
    let peers = self.peers.lock().await;
    peers.contains_key(&normalized_addr)
};

if already_connected {
    info!(
        "[P2P] SKIP duplicate inbound from {} (first connection wins)",
        normalized_addr
    );
    return Err(format!("Duplicate connection from {} (first wins)", normalized_addr));
}
```

**Impact**: 
- âœ… Stable connections (no thrashing)
- âœ… No reconnect storms
- âœ… Existing healthy connection preserved

---

## âœ… Fix 4: Separate Handshake Timeout (12s)

**File Modified**: `src/p2p/connection.rs`

**Problem**: 3-5s timeout too aggressive for handshakes, dropping slow but valid peers

**Before**: 3-second handshake timeout
```rust
const HANDSHAKE_TIMEOUT_MS: u64 = 3000; // 3 seconds
```

**After**: 12-second handshake timeout
```rust
// Fix 4: Separate handshake timeout (12s) from normal message timeout (5s)
const HANDSHAKE_TIMEOUT_MS: u64 = 12000; // 12 seconds (handshakes can be slower)
```

**Retry Wrapper**: 36-second total timeout
```rust
// Fix 4: Handshake timeout (12s per attempt, 36s total for 3 attempts)
let peer_handshake = tokio::time::timeout(
    Duration::from_secs(36), // 12s per attempt * 3 attempts
    self.perform_handshake_with_retry(&mut reader, &mut writer, &sock_addr, true)
).await
```

**Timeout Separation**:
- **Handshake read/write**: 12 seconds per attempt
- **Normal message send**: 5 seconds (unchanged)
- **Block broadcast**: 10 seconds (from previous fix)

**Impact**: 
- âœ… Slow peers can complete handshake
- âœ… Fewer "handshake timeout" errors
- âœ… Better compatibility with peers doing extra work (logging, reachability checks)
- âœ… Fast rejection of non-Vision scanners still works

---

## Testing

**Expected Behavior**:

**Successful Connection**:
```
[INFO] Connecting to peer 192.168.1.100:7072
[INFO] Connected via IPv4
[INFO] Initiating handshake with retry logic...
[INFO] âœ… Handshake success with peer VNODE-abc123 at 192.168.1.100:7072
[INFO] Registered new peer connection (normalized_key=192.168.1.100:7072)
```

**Duplicate Connection (First Wins)**:
```
[INFO] Accepted inbound IPv4 connection from 192.168.1.100:53565
[INFO] [P2P] SKIP duplicate inbound from 192.168.1.100:7072 (first connection wins)
```

**Rejected Connection**:
```
[INFO] [DIAL] SKIP non-public ip=10.0.0.5
[INFO] [DIAL] SKIP already-connected key=192.168.1.100:7072
[INFO] [DIAL] SKIP ipv6 ip=[::1]:7072
```

**Slow Handshake (Now Succeeds)**:
```
[INFO] Initiating handshake with retry logic...
[INFO] âœ… Handshake success with peer VNODE-def456 at 192.168.1.101:7072 (8s)
```

---

## Summary

| Fix | Description | Status |
|-----|-------------|--------|
| **1** | Log WHY connections are skipped/rejected | âœ… FIXED |
| **2** | Normalize peer keys to IP:7072 format | âœ… FIXED |
| **3** | First-wins duplicate connection policy | âœ… FIXED |
| **4** | Increase handshake timeout to 12s | âœ… FIXED |

---

## Debugging Commands

```powershell
# Watch connection logs
.\target\release\vision-node.exe 2>&1 | Select-String "DIAL|SKIP|duplicate|normalized"

# Check for rejection patterns
.\target\release\vision-node.exe 2>&1 | Select-String "SKIP"

# Monitor handshake timing
.\target\release\vision-node.exe 2>&1 | Select-String "Handshake"

# Verify normalized keys
.\target\release\vision-node.exe 2>&1 | Select-String "normalized_key"
```

---

## Version Info

- **Vision Node**: v3.0.0 MINING TESTNET
- **Date**: December 15, 2025
- **Build**: Release (optimized)
- **Compile Status**: âœ… SUCCESS (8m 17s)
- **Patch**: Copilot Diagnostics & Connection Normalization

