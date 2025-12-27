# Sync Promotion Quick Reference

## Peer Tiers
```
Hot    → Default (new peer)
Warm   → Reliable sync provider
Anchor → Public server (serves swarm)
```

## Promotion Logic
```rust
// On sync completion
if local_height >= network_height {
    promote_to_warm(provider_id);     // 🌡
    
    if public_reachable {
        promote_to_anchor(provider_id); // 🚀
    }
    
    auto_elect_warm_peer();            // Fallback
}
```

## Peer Selection Priority
1. **Tier**: Anchor > Warm > Hot
2. **Health**: Higher score = better
3. **Latency**: Lower RTT = better

## Log Messages
```
🌡 Peer VNODE-abc promoted to WARM (reliable sync provider)
🚀 Peer VNODE-xyz promoted to ANCHOR — now serving the swarm
[PEER HIERARCHY] No warm/anchor peers found - auto-electing: VNODE-123
```

## Genesis Seeds
- Start as **Warm** tier (immediate stability)
- `public_reachable: true` (assumed)
- Protected from eviction

## Beacon Peers
- Start as **Hot** tier (untested)
- `public_reachable: false` (unknown)
- Can be promoted via sync

## Struct Fields
```rust
// VisionPeer additions (peer_store.rs)
pub peer_tier: PeerTier,
pub last_promotion: Option<i64>,
pub public_reachable: bool,
```

## Key Methods
```rust
// PeerStore (peer_store.rs)
promote_to_warm(peer_id: &str)         // Hot → Warm
promote_to_anchor(peer_id: &str)       // Warm → Anchor (if reachable)
get_best_sync_peer() -> Option<Peer>   // Sort by tier+health+latency
auto_elect_warm_peer()                 // Fallback election
```

## Testing
```powershell
# Start node and watch for promotions
.\vision-node.exe | Select-String "promoted"

# Check peer database
# Look for peer_tier: "Warm" or "Anchor"
```

## Version
- **v2.2.0-CONSTELLATION**
- Protocol: 2
- Build: v2.2-constellation
- Date: Dec 9, 2025
