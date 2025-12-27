# Peer Discovery System

## Overview

Vision Node uses a decentralized peer discovery system where **no single node is required** for the network to function. All nodes are equal peers, and seed peers are only used for initial bootstrap.

## Key Principles

### 1. **No Beacon Node Dependency**
- There is NO special "beacon" node that the protocol depends on
- Seed peers are **bootstrap helpers only**, not authorities
- Consensus rules and chain validation are completely independent of any specific peer
- The network continues operating even if all seed peers go offline

### 2. **Peer Discovery Flow**

#### First Startup (Empty Peer Database)
1. Node loads `config/seed_peers.toml`
2. Attempts to connect to seed peers in order
3. On successful connection:
   - Performs handshake (validates genesis, protocol version)
   - Requests peer list via `/p2p/peers` or `GetPeers` message
   - Saves discovered peers to local database (`peer:address` keys)

#### Subsequent Startups
1. Node loads persisted peers from database first
2. Tries stored peers before falling back to seed peers
3. Only uses seed peers if no stored peers connect successfully
4. Continuously discovers new peers through peer exchange

### 3. **Peer Persistence**

Peers are automatically persisted to the local sled database:
- **Key format**: `peer:host:port` → `"1"`
- **Trigger**: Successfully established outbound TCP connection
- **Cleanup**: Failed connections are removed from database
- **Location**: `./vision_data_<port>/` directory

### 4. **Connection Maintenance**

The P2P connection manager automatically:
- Maintains minimum outbound connections (default: 8)
- Respects maximum connection limit (default: 16)
- Reconnects to peers at regular intervals (default: 30s)
- Prioritizes persisted peers over seed peers
- Removes dead connections and updates peer database

## Configuration

### Seed Peers Configuration

Edit `config/seed_peers.toml`:

```toml
# Bootstrap peers (not authorities!)
seed_peers = [
    "node1.example.com:7071",
    "node2.example.com:7071",
    "192.168.1.100:7071",
]

# Connection parameters
min_outbound_connections = 8
max_outbound_connections = 16
connection_timeout_seconds = 10
reconnection_interval_seconds = 30
```

### CLI Peer Flag

Connect directly to any working node without seed configuration:

```bash
# Connect to a specific peer
./vision-node --peer node.example.com:7071

# The peer will be persisted for future connections
```

**Use cases:**
- New users bootstrapping from a friend's node
- Private networks without public seed peers
- Emergency recovery if seed peers are down
- Testing and development

## Peer Exchange Protocol

### TCP P2P Messages

When nodes connect via TCP P2P, they exchange peer lists:

```rust
// Request peer list
P2PMessage::GetPeers

// Response with known peers
P2PMessage::PeerList {
    peers: Vec<String>,  // ["host1:port1", "host2:port2", ...]
}
```

### HTTP Endpoint

Query a node's known peers via HTTP:

```bash
curl http://localhost:7070/p2p/peers
```

Response:
```json
{
  "peers": [
    "node1.example.com:7071",
    "192.168.1.50:7071",
    "node3.example.com:7071"
  ],
  "count": 3
}
```

## Network Security

### Handshake Validation

All peer connections validate:
- **Protocol version**: Must match exactly
- **Genesis hash**: Must match local chain
- **Chain ID**: Must match configured network (testnet/mainnet)
- **Network enforcement**: Nodes reject mismatched genesis/chain identity

This prevents:
- Cross-network contamination
- Protocol version mismatches
- Connecting to wrong chain forks

### No Trust Required

Peers are NOT trusted for:
- Block validation (full PoW verification)
- Transaction verification (signature checks)
- Consensus rules (independent validation)
- Chain selection (cumulative work verification)

Peers only provide:
- Block announcements (triggers validation)
- Transaction relay (triggers verification)
- Peer discovery (verified independently)

## Monitoring

### Metrics

Track peer health via Prometheus metrics:
- `vision_p2p_peers_connected` - Current peer count
- `vision_p2p_announces_received` - Block announcements
- `vision_p2p_dupes_dropped` - Duplicate blocks filtered

### Logs

Peer discovery events are logged:
```
INFO Loaded 5 persisted peers from database
INFO Attempting to establish outbound connections attempting_connections=3
INFO Successfully established outbound connection peer="node1.example.com:7071"
INFO Received peer list from peer peer="node1.example.com:7071" peer_count=12
```

## Best Practices

### For Seed Peer Operators

If you run a public seed peer:
1. Keep it online and accessible (but failures are tolerated)
2. Ensure proper port forwarding (P2P port = HTTP port + 1)
3. Monitor connection limits and bandwidth
4. **Remember**: Your node is a helper, not a requirement

### For Node Operators

To ensure good peer connectivity:
1. Configure multiple seed peers from different operators
2. Open your P2P port (7071) for inbound connections
3. Let the node run to build up persisted peer database
4. Use `--peer` flag to manually add reliable peers
5. Check `/p2p/peers` endpoint to verify peer discovery

### For Private Networks

To run isolated network:
1. Clear seed_peers.toml (empty array)
2. Bootstrap all nodes using `--peer` pointing to each other
3. Nodes will discover each other and persist connections
4. Configure firewall to only allow your network's peers

## Troubleshooting

### "No peers connected"

Check:
1. Seed peers are accessible (try `telnet seed.example.com 7071`)
2. Firewall allows outbound connections to P2P port
3. Genesis hash matches seed peers' chain
4. Protocol version matches (same Vision Node version)

### "Peer connects then disconnects"

Causes:
- Genesis mismatch (different chain)
- Protocol version mismatch (upgrade needed)
- Network enforcement (testnet vs mainnet)
- TCP connection timeout

Check handshake logs for specific error.

### "No seed peers configured"

Use CLI flag to bootstrap:
```bash
./vision-node --peer working-node.example.com:7071
```

## Architecture Diagrams

### Startup Flow
```
┌─────────────┐
│ Node Starts │
└──────┬──────┘
       │
       ├─→ Load persisted peers from DB
       │   └─→ Try connecting (highest priority)
       │
       ├─→ Check CLI --peer flag
       │   └─→ Connect immediately & persist
       │
       └─→ Load seed_peers.toml (fallback)
           └─→ Connect if needed
```

### Peer Exchange
```
Node A                    Node B
  │                         │
  ├──→ TCP Connect ────────→│
  │                         │
  ├──→ Handshake ──────────→│
  │←── Handshake ←──────────┤
  │                         │
  ├──→ GetPeers ───────────→│
  │←── PeerList [X,Y,Z] ←───┤
  │                         │
  └──→ Save to DB           │
       Connect to X,Y,Z     │
```

## Migration from Old System

If migrating from a beacon-based system:

1. **No code changes needed** - New system is backward compatible
2. **Persisted peers carry over** - Existing peer database still works
3. **Update seed_peers.toml** - Replace beacon with multiple seeds
4. **No consensus impact** - Block validation was always independent

The key difference:
- **Before**: "Connect to THE beacon node"
- **After**: "Bootstrap from ANY seed, then discover peers"

## Future Enhancements

Potential improvements (not yet implemented):
- Peer reputation scoring
- Geographic diversity optimization
- DHT-based peer discovery
- Peer gossip protocol improvements
- Connection quality metrics
