# Dial Failure Tracking

## Overview

Global tracking system for P2P connection failures. Turns "why aren't peers connecting?" from ghost hunting into dashboard debugging.

## Features

- **Global In-Memory Tracker**: `dial_tracker.rs` with lazy_static + Mutex
- **100 Most Recent Failures**: Auto-evicts old entries (FIFO queue)
- **Categorized Reasons**: connection_refused, timeout, handshake_timeout, version_mismatch, etc.
- **Source Attribution**: Tracks where peer came from (seed, peer_book, direct, gossip, handshake)
- **HTTP API Endpoint**: `/api/p2p/debug` exposes failures in real-time

## Failure Categories

### Connection Failures (TCP level)
- `connection_refused` - Port closed or firewall blocking
- `timeout` - No response within TCP timeout
- `connection_reset` - Connection dropped by remote
- `connection_aborted` - Connection aborted locally
- `not_connected` - Socket not connected
- `addr_not_available` - Address not available
- `network_unreachable` - Network routing issue

### Handshake Failures (Protocol level)
- `handshake_timeout` - Handshake didn't complete in 36s
- `version_mismatch` - Protocol version incompatible
- `genesis_mismatch` - Different chain genesis
- `banned` - Peer is banned
- `ticket_invalid` - Invalid connection ticket
- `handshake_error` - Generic handshake failure

### Pre-Dial Skips (Policy blocks)
- `ipv6_blocked` - IPv6 not supported (IPv4-only mode)
- `invalid_ipv4_endpoint` - Not a valid IPv4 endpoint
- `already_connected` - Already have active connection
- `private_ip_blocked` - Private IP in production mode
- `loopback_blocked` - Loopback address blocked
- `self_dial_blocked` - Attempted to dial own address

## Sources

- **seed** - Hardcoded bootstrap seed peer
- **peer_book** - Peer from stored peer book (hot/warm/cold)
- **direct** - Direct connection attempt (manual or API)
- **gossip** - Peer learned from PeerList gossip
- **handshake** - Peer learned during handshake exchange

## HTTP API

### GET /api/p2p/debug

Returns comprehensive P2P debug info including recent failures:

```json
{
  "connected_peers": [...],
  "peer_book_counts": {
    "hot": 3,
    "warm": 12,
    "cold": 45,
    "total": 60
  },
  "dial_failures": [
    {
      "addr": "1.2.3.4:7072",
      "reason": "connection_refused",
      "timestamp_unix": 1735000000,
      "source": "seed"
    },
    {
      "addr": "5.6.7.8:7072",
      "reason": "handshake_timeout",
      "timestamp_unix": 1735000100,
      "source": "peer_book"
    }
  ]
}
```

### Example Query

```bash
# Get all failures
curl http://localhost:7070/api/p2p/debug | jq '.dial_failures'

# Get failures from last 5 minutes
curl http://localhost:7070/api/p2p/debug | jq '.dial_failures[] | select(.timestamp_unix > (now - 300))'

# Group by reason
curl http://localhost:7070/api/p2p/debug | jq '.dial_failures | group_by(.reason) | map({reason: .[0].reason, count: length})'

# Show only timeouts
curl http://localhost:7070/api/p2p/debug | jq '.dial_failures[] | select(.reason | contains("timeout"))'
```

## Implementation Details

### dial_tracker.rs

```rust
pub struct DialTracker {
    failures: VecDeque<DialFailure>,  // FIFO queue, max 100 entries
}

pub fn record_dial_failure(addr: String, reason: String, source: String)
pub fn get_dial_failures() -> Vec<DialFailure>
```

### Tracking Points

#### connection.rs
- **Line ~2360**: TCP connection failures (connection_refused, timeout, etc.)
- **Line ~2390**: Handshake timeout (36s retry window)
- **Line ~2410**: Handshake errors (version_mismatch, genesis_mismatch, etc.)
- **Line ~2300**: IPv6 blocked
- **Line ~2310**: Invalid IPv4 endpoint
- **Line ~2330**: Private IP / self-dial blocked

#### connection_maintainer.rs
- **Line ~213**: Seed already connected
- **Line ~220**: Seed IP validation failure
- **Line ~309**: Peer book already connected
- **Line ~316**: Peer book IP validation failure

## Debugging Workflow

### 1. Check Current State
```bash
curl http://localhost:7070/api/p2p/debug | jq '{
  connected: .connected_peers | length,
  peer_book: .peer_book_counts.total,
  failures: .dial_failures | length
}'
```

### 2. Analyze Failure Patterns
```bash
# Most common failure reasons
curl http://localhost:7070/api/p2p/debug | jq '
  .dial_failures | group_by(.reason) | 
  map({reason: .[0].reason, count: length}) | 
  sort_by(.count) | reverse
'

# Failures by source
curl http://localhost:7070/api/p2p/debug | jq '
  .dial_failures | group_by(.source) | 
  map({source: .[0].source, count: length})
'
```

### 3. Identify Problem Peers
```bash
# Show repeat offenders
curl http://localhost:7070/api/p2p/debug | jq '
  .dial_failures | group_by(.addr) | 
  map({addr: .[0].addr, failures: length, reasons: [.[].reason] | unique}) | 
  sort_by(.failures) | reverse | .[0:10]
'
```

### 4. Time-Based Analysis
```bash
# Failures in last 2 minutes
curl http://localhost:7070/api/p2p/debug | jq --arg now "$(date +%s)" '
  .dial_failures[] | 
  select(.timestamp_unix > (($now | tonumber) - 120))
'
```

## Common Patterns

### "Seeds Not Connecting"
**Symptom**: Many `connection_refused` failures from `source: "seed"`
**Likely Cause**: Seeds offline, wrong ports, or firewall blocking
**Solution**: Check seed addresses in config, verify ports, check firewall rules

### "Handshake Timeouts"
**Symptom**: Many `handshake_timeout` failures
**Likely Cause**: Network latency, congested nodes, or protocol bugs
**Solution**: Check node CPU/memory, verify network connectivity, check for protocol mismatches

### "Version Mismatches"
**Symptom**: Many `version_mismatch` failures
**Likely Cause**: Nodes running different protocol versions
**Solution**: Upgrade nodes to latest version, check MIN_PROTOCOL_VERSION settings

### "Private IP Blocks"
**Symptom**: Many `private_ip_blocked` failures from `source: "seed"`
**Likely Cause**: Seeds using private IPs (10.x, 192.168.x, 172.16-31.x)
**Solution**: Update seed list to public IPs only, or enable LAN mode for testing

### "Already Connected"
**Symptom**: Many `already_connected` skips
**Likely Cause**: Normal - connection maintainer trying to dial existing peers
**Solution**: No action needed, this is expected behavior

## Testing

### Unit Tests
```bash
cargo test dial_tracker
```

### Integration Test
```bash
# Start node
./target/release/vision-node

# Generate some failures (bad seeds)
curl -X POST http://localhost:7070/api/p2p/connect \
  -H "Content-Type: application/json" \
  -d '{"peer_addr": "127.0.0.1:9999"}'

# Check tracked failures
curl http://localhost:7070/api/p2p/debug | jq '.dial_failures'
```

## Performance

- **Memory**: ~100KB (100 failures × ~1KB each)
- **CPU**: Negligible (mutex lock only during record/read)
- **Latency**: <1ms per operation
- **Storage**: In-memory only (cleared on restart)

## Future Enhancements

### Time-Based Expiry (optional)
Could add TTL to clear failures older than 5 minutes:

```rust
pub fn clear_old_failures(&mut self, max_age_secs: u64) {
    let now = SystemTime::now()...;
    self.failures.retain(|f| now - f.timestamp_unix < max_age_secs);
}
```

### Persistent Storage (optional)
Could write failures to SQLite for long-term analysis:

```rust
pub fn persist_to_db(&self, db: &Connection) { ... }
```

### Metrics Export (optional)
Could expose Prometheus metrics:

```rust
dial_failures_total{reason="timeout",source="seed"} 42
```

## Related Files

- `src/p2p/dial_tracker.rs` - Core tracking module
- `src/p2p/api.rs` - HTTP API endpoint
- `src/p2p/connection.rs` - Connection failure tracking
- `src/p2p/connection_maintainer.rs` - Seed/peer_book failure tracking
- `src/p2p/mod.rs` - Module registration

## Related Documentation

- `P2P_DEBUG_GUIDE.md` - Overall P2P debugging strategies
- `CONTROL_PLANE_7070_COMPLETE.md` - HTTP API documentation
- `AUTO_SYNC_INDEPENDENT.md` - Peer discovery and auto-sync

## Status

✅ **COMPLETE** - Dial failure tracking is production-ready
- Global tracker with 100-entry FIFO queue
- Categorized reasons with source attribution
- HTTP API endpoint for real-time debugging
- Tracking wired into all connection failure points
- Built and tested successfully

**Next Steps**:
1. Test in 5-node mesh soak test
2. Add peer propagation accounting (track peer discovery paths)
3. Add "port truth" enforcement (P2P port vs HTTP port consistency)
