# Vision Node v2.2.0 - Sync Promotion Implementation

## Overview
Implementation of automatic peer hierarchy promotion to fix sync stagnation. Nodes now establish a stable tier system (Hot → Warm → Anchor) for efficient chain synchronization.

## Peer Tier System

### Three-Tier Hierarchy
```
Hot (Default)   → Normal peer, no special status
Warm (Promoted) → Reliable sync provider, successful history
Anchor (Elite)  → Publicly reachable warm peer, serves the swarm
```

### Promotion Path
```
Hot Peer
  ↓ (successful sync completion)
Warm Peer
  ↓ (if public_reachable == true)
Anchor Peer
```

## Implementation Details

### VisionPeer Struct (peer_store.rs)
Added three new fields:
```rust
pub peer_tier: PeerTier,           // Hot/Warm/Anchor
pub last_promotion: Option<i64>,    // Unix timestamp
pub public_reachable: bool,         // Can serve other nodes
```

### PeerTier Enum
```rust
pub enum PeerTier {
    Hot,    // Default tier - normal peer
    Warm,   // Promoted sync provider - reliable source  
    Anchor, // Publicly reachable warm peer - serves the swarm
}
```

### Promotion Methods (PeerStore)

#### `promote_to_warm(peer_id: &str)`
- Promotes Hot peers to Warm tier
- Records promotion timestamp
- Logs: `🌡 Peer {tag} promoted to WARM (reliable sync provider)`

#### `promote_to_anchor(peer_id: &str)`
- Promotes Warm peers to Anchor tier
- Requires `public_reachable == true`
- Logs: `🚀 Peer {tag} promoted to ANCHOR — now serving the swarm`

#### `get_best_sync_peer() -> Option<VisionPeer>`
Priority sorting:
1. Tier (Anchor > Warm > Hot)
2. Health score (higher = better)
3. Latency (lower = better)

#### `auto_elect_warm_peer()`
Fallback election when no warm/anchor peers exist:
- Finds best Hot peer by health + latency
- Auto-promotes to Warm tier
- Logs: `[PEER HIERARCHY] No warm/anchor peers found - auto-electing: {tag}`

## Auto-Sync Integration (auto_sync.rs)

### Sync Completion Detection
```rust
// Track sync progress
let mut successful_provider_id: Option<String> = None;
let mut network_estimated_height = local_height;

// After sync attempt
if !synced_from_any && local_height >= network_estimated_height {
    // We caught up!
    if let Some(provider_id) = successful_provider_id {
        promote_to_warm(provider_id);
        
        if peer.public_reachable {
            promote_to_anchor(provider_id);
        }
        
        auto_elect_warm_peer(); // Fallback
    }
}
```

### Requirements
Sync provider response MUST include:
```json
{
    "pulled": 42,
    "provider_node_id": "abc123...",
    "peer_height": 150000
}
```

## Genesis Seeds (seed_peers.rs)
Genesis seeds start as **Warm tier** for immediate stability:
```rust
peer_tier: PeerTier::Warm,
last_promotion: Some(now),
public_reachable: true,  // Assumed reachable
```

## Beacon Bootstrap (beacon_bootstrap.rs)
Beacon-discovered peers start as **Hot tier**:
```rust
peer_tier: PeerTier::Hot,
last_promotion: None,
public_reachable: false,  // Unknown until tested
```

## Version Info
- **Node Version**: v2.2.0-CONSTELLATION
- **Protocol Version**: 2
- **Build Tag**: v2.2-constellation
- **Numeric Version**: 220
- **Binary Size**: 27.6 MB (Windows x64)
- **Build Date**: December 9, 2025

## Files Modified
1. `src/p2p/peer_store.rs` - Added PeerTier enum, fields, promotion methods (Lines 193-210, 920+)
2. `src/auto_sync.rs` - Added sync completion detection and promotion triggers (Lines 115-185)
3. `src/p2p/seed_peers.rs` - Seeds start as Warm tier (Line 156)
4. `src/p2p/beacon_bootstrap.rs` - Beacon peers start as Hot tier (Line 486)

## Expected Behavior

### On First Sync
1. Node connects to genesis seeds (Warm tier)
2. Syncs blocks successfully
3. Genesis seed promoted to Anchor (if reachable)

### On Subsequent Syncs
1. Prefers Anchor > Warm > Hot peers
2. Successful sync promotes Hot → Warm
3. Publicly reachable Warm → Anchor
4. Auto-elects best Hot peer if no elevated peers

### Stagnation Prevention
- Multiple nodes form stable Anchor tier
- New nodes always find reliable sync sources
- Network forms natural hierarchy without centralization

## Testing Checklist
- [ ] Start two fresh nodes on same network
- [ ] Verify first sync completes
- [ ] Check logs for "🌡 promoted to WARM" message
- [ ] Check peer_store database shows tier progression
- [ ] Verify subsequent syncs prefer warm/anchor peers
- [ ] Restart node, confirm tiers persist

## Troubleshooting

### No promotions occurring
- Check sync provider includes `provider_node_id` in response
- Verify `network_estimated_height` is being tracked
- Check auto_sync logs for sync completion detection

### Peers stuck at Hot tier
- Verify sync completion condition: `!synced_from_any && local_height >= network_height`
- Check PeerStore::promote_to_warm() is being called
- Review auto_sync.rs line 165-185

### No Anchor peers
- Verify `public_reachable` flag is set correctly
- Check promote_to_anchor() requires Warm tier first
- Confirm genesis seeds set `public_reachable: true`

## Protocol Compatibility
- ✅ Protocol v2 nodes with sync promotion (v2.2.0+)
- ❌ Protocol v1 nodes rejected by handshake
- ❌ Protocol v2 without sync promotion (v2.0.0-v2.1.0) - will work but no promotions

## Future Enhancements
- [ ] Public reachability detection (test inbound connections)
- [ ] Demotion on failure (Anchor → Warm → Hot)
- [ ] Tier-based connection limits (more slots for Anchor peers)
- [ ] Metrics dashboard showing tier distribution
- [ ] Manual tier override via admin API

---
*Implementation complete: December 9, 2025*
