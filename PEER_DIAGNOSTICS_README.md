# Peer Connection Diagnostics

## Overview

The Vision Node now automatically generates peer connection diagnostics reports every ~30 seconds during connection maintainer cycles. These reports help you understand why peers aren't connecting.

## Output Files

Located in `./vision_data_7070/public/`:

### 1. `peer_store_stats.json` (Safe to share)

Simple counts without IP addresses:

```json
{
  "ts": "2026-01-07T23:53:37Z",
  "scope": "mainnet",
  "known_peers": 71,
  "connected": 4,
  "connecting": 0,
  "cooldown": 41,
  "banned": 0,
  "unhealthy": 6,
  "attempted_last_cycle": 2,
  "connected_last_cycle": 2
}
```

### 2. `peer_connect_reasons.json` (Detailed with samples)

Shows WHY peers aren't connecting, with example IPs (redacted by default):

```json
{
  "ts": "2026-01-07T23:53:37Z",
  "scope": "mainnet",
  "cycle": {
    "attempted": 2,
    "connected": 2,
    "final_connected": 5,
    "target": 8
  },
  "top_blockers": {
    "primary": "CooldownActive",
    "secondary": "DialTimeout",
    "third": "AlreadyConnected"
  },
  "reasons": [
    {
      "reason": "CooldownActive",
      "count": 41,
      "samples": [
        "75.128.***:***:7072",
        "69.173.***:***:7072"
      ]
    },
    {
      "reason": "AlreadyConnected",
      "count": 4,
      "samples": [
        "68.142.***:***:7072"
      ]
    },
    {
      "reason": "DialTimeout",
      "count": 9,
      "samples": [
        "91.86.***:***:7072"
      ]
    }
  ]
}
```

## Reason Codes

### Skip Reasons (Not Attempted)
- **AlreadyConnected**: Already have active connection
- **AlreadyConnecting**: Dial in progress
- **CooldownActive**: Recently failed, waiting for backoff timer
- **MaxPeersReached**: Hit configured peer limit
- **PeerBanned**: Quarantined due to repeated failures
- **PeerUnhealthy**: Health score dropped to zero
- **InvalidAddress**: Malformed address
- **FilteredByPolicy**: IP validation failed (localhost, private, etc.)

### Dial Failure Reasons
- **DialRefused**: Connection refused by peer
- **DialTimeout**: Dial timed out (5 seconds)
- **DialError**: Other connection error
- **NoRouteToHost**: Network unreachable

### Handshake Failure Reasons
- **HandshakeTimeout**: Handshake timed out (5 seconds)
- **HandshakeFailed_ChainId**: Wrong network
- **HandshakeFailed_Version**: Incompatible protocol version
- **HandshakeFailed_IncompatibleChain**: Chain mismatch (different genesis/fork)
- **HandshakeFailed_Other**: Other handshake error

## Privacy

**By default**, IP addresses are redacted: `75.128.***:***:7072`

To show **full IP addresses** (for debugging), set:
```bash
$env:VISION_DEBUG_PUBLIC_PEERS="1"
```

Then restart your node.

## Common Scenarios

### "I have 70 peers but only 5 connected!"

Check `peer_connect_reasons.json`:

1. **High CooldownActive count**: Most peers recently failed, waiting for retry
   - This is normal gradual mesh building
   - Cooldown prevents dial storms
   - Peers will retry after backoff expires

2. **High AlreadyConnected count**: Good! Your connections are stable
   - Not actually a problem
   - Maintainer sees them as "candidates" but filters them out

3. **High DialTimeout count**: Peers not responding
   - They may be offline
   - They may be behind NAT
   - They may have connection limits
   - Consider UPnP/port forwarding

4. **High HandshakeFailed_IncompatibleChain**: Network mismatch
   - They're on a different fork
   - Or different network entirely
   - Peer will be quarantined

### "Why does cooldown take so long?"

Exponential backoff prevents dial storms:
- 1st failure: 30 seconds
- 2nd failure: 60 seconds
- 3rd failure: 120 seconds
- ...up to 1 hour max

This protects both you and the remote peer from connection spam.

### "I want full mesh (everyone connected to everyone)"

Set higher peer limit:
```bash
$env:VISION_MAX_PEERS="64"
```

The maintainer will gradually connect to all known peers until:
- `connected >= VISION_MAX_PEERS`, OR
- `connected >= known_peers` (full mesh)

## Monitoring

Watch the files update every ~30 seconds during connection cycles:

```powershell
Get-Content -Path ".\vision_data_7070\public\peer_store_stats.json" -Wait
```

Or in bash:
```bash
tail -f vision_data_7070/public/peer_store_stats.json
```

## Troubleshooting

### Files not appearing?

Check your logs for:
```
[CONN_REPORT] Failed to write diagnostics: <error>
```

Common causes:
- Permissions issue on `vision_data_7070/public/` directory
- Disk full
- Antivirus blocking file writes

### Empty reasons list?

This means:
- All known peers are already connected (full mesh achieved!)
- OR no connection attempts made this cycle

### Very high counts for one reason?

Examples:
- **41 CooldownActive**: Normal - gradual mesh building with backoff
- **50 DialTimeout**: Network issues - peers offline or unreachable
- **30 HandshakeFailed**: Version mismatch - update your node or peers outdated

## Developer Notes

### Adding New Reasons

Edit `src/p2p/peer_connect_report.rs`:
```rust
pub enum PeerConnectReason {
    // Add your new reason
    YourNewReason,
}
```

And update `as_str()` and error classifier.

### Instrumentation Points

1. **Filter decisions**: `connection_maintainer.rs` lines ~270-340
2. **Dial attempts**: Lines ~410-530
3. **Handshake failures**: Classified in `classify_dial_error()`

### File Format

- Atomic writes (`.tmp` → rename) prevent corruption
- Pretty-printed JSON for human readability
- ISO 8601 timestamps
- Samples capped at 5 per reason

## Support

If diagnostics show unexpected patterns, include both JSON files in bug reports:
- `peer_store_stats.json` (always safe to share)
- `peer_connect_reasons.json` (IPs redacted by default)

Set `VISION_DEBUG_PUBLIC_PEERS=1` only if maintainers need full IPs for debugging.
