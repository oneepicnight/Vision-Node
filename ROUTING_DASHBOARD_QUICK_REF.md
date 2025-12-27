# Routing Intelligence Dashboard - Developer Quick Reference

## API Endpoints (Live & Ready)

```
GET  /api/p2p/routing/cluster_stats        # Cluster health + ring distribution
GET  /api/p2p/routing/top_peers?limit=20   # Top-scoring peers with metrics
GET  /api/p2p/routing/events?limit=50      # Recent routing events timeline
```

## Testing Commands

```powershell
# Test cluster stats
curl http://localhost:3030/api/p2p/routing/cluster_stats

# Test top 5 peers
curl http://localhost:3030/api/p2p/routing/top_peers?limit=5

# Test recent 10 events
curl http://localhost:3030/api/p2p/routing/events?limit=10
```

## Event Store Usage

```rust
use crate::api::routing_api;

// Log an info event
routing_api::push_routing_event("info", "Cluster balanced".to_string());

// Log a warning
routing_api::push_routing_event("warn", format!("Peer {} on probation", node_tag));

// Log critical issue
routing_api::push_routing_event("bad", format!("Peer {} banned", node_tag));
```

## Helper Functions (Available)

```rust
// Already integrated in reputation.rs
routing_api::log_misbehavior_event(node_tag, kind, score, trust_level);
routing_api::log_ban_expiry_event(node_tag, trust_level);

// Ready to use (not yet wired)
routing_api::log_cluster_balance_event(inner, middle, outer);
routing_api::log_peer_promotion_event(node_tag, success_rate, avg_ms);
routing_api::log_decay_event(node_tag, new_reputation);
```

## React Dashboard Integration

```tsx
// In Command Center component
import RoutingIntelligenceDashboard from './components/command-center/RoutingIntelligenceDashboard';
import './styles/routing-intelligence.css';

<RoutingIntelligenceDashboard />
```

## Health Score Interpretation

- **80-100:** Excellent (green badge)
- **60-79:** Healthy (cyan badge)
- **40-59:** Degraded (yellow badge)
- **0-39:** Critical (red badge)

## Trust Level Colors

- **Trusted:** Green (#00ff00)
- **Normal:** Cyan (#00ffff)
- **Probation:** Orange (#ffaa00)
- **Graylisted:** Gray (#aaaaaa)
- **Banned:** Red (#ff0000)

## Event Levels

- **"info":** Normal operations (cyan border)
- **"warn":** Minor issues (orange border)
- **"bad":** Severe issues (red border)

## File Locations

```
src/api/routing_api.rs                        # Backend API implementation
src/api/mod.rs                                # API module exports (routing_api added)
src/api/website_api.rs                        # Route registration (3 new routes)
src/p2p/reputation.rs                         # Event integration (misbehavior + expiry)
src/components/command-center/RoutingIntelligenceDashboard.tsx  # React UI
src/styles/routing-intelligence.css           # Cyberpunk theme
```

## Key Functions

### Backend API
- `get_cluster_stats_handler()` - Returns health metrics
- `get_top_peers_handler()` - Returns ranked peers
- `get_routing_events_handler()` - Returns event timeline
- `calculate_routing_health()` - Computes health score (0-100)

### Event Store
- `push_routing_event()` - Add event to ring buffer
- `ROUTING_EVENT_STORE` - Global VecDeque (500 capacity)

### Event Helpers
- `log_misbehavior_event()` - Protocol violations, spam, etc.
- `log_ban_expiry_event()` - Graylist/ban expiration
- `log_cluster_balance_event()` - Ring distribution changes
- `log_peer_promotion_event()` - High-performing peers
- `log_decay_event()` - Reputation forgiveness

## Build & Deploy

```powershell
# Build
cargo build --release

# Binary location
target\release\vision-node.exe

# Size
27.02 MB (27,016,704 bytes)
```

## Common Tasks

### Add a new event type
```rust
// In your code
use crate::api::routing_api;

routing_api::push_routing_event("info", 
    format!("New guardian joined: {}", node_tag)
);
```

### Check cluster health programmatically
```rust
use crate::api::routing_api::calculate_routing_health;

let health = calculate_routing_health(total, inner, middle, outer, guardians);
if health < 40.0 {
    warn!("Cluster health critical: {:.1}%", health);
}
```

### Query top peers in Rust code
```rust
let chain = CHAIN.lock();
let peer_store = PeerStore::new(&chain.db)?;
let classified = peer_store.classify_peers_for_routing(None);

// Already sorted by score (best first)
let top_peer = classified.first();
```

## Troubleshooting

### No events showing
- Check if reputation system is running (`start_reputation_maintenance()`)
- Verify peers are connected and generating events
- Test with manual event: `routing_api::push_routing_event("info", "Test".to_string())`

### Health score is 0
- Ensure peers are connected (`total_count` should be > 0)
- Check if peers have latency measurements (`avg_rtt_ms`)
- Verify ring classification is working (check `classify_ring_simple()`)

### Dashboard shows mock data
- API endpoints must be running on `localhost:3030`
- Check browser console for fetch errors
- Verify CORS headers if accessing from external domain

## Performance Notes

- **Event Store:** O(1) append, 40 KB memory
- **Health Calculation:** O(n) where n = peer count
- **Top Peers Query:** O(n log n) sorting
- **Events Query:** O(k) where k = limit
- **Dashboard Polling:** 12 KB/20 seconds = negligible

## Documentation

- `PHASE4_REPUTATION_COMPLETE.md` - Phase 4 implementation details
- `ROUTING_DASHBOARD_API_GUIDE.md` - Complete API specification
- `ROUTING_DASHBOARD_QUICKSTART.md` - 5-minute integration
- `ROUTING_DASHBOARD_LAYOUT.md` - Visual mockups + CSS reference
- `ROUTING_DASHBOARD_IMPLEMENTATION.md` - Full implementation summary
- `ROUTING_DASHBOARD_QUICK_REF.md` (this file) - Developer cheat sheet

## Status

✅ Backend API - IMPLEMENTED  
✅ Event Store - IMPLEMENTED  
✅ Event Integration - IMPLEMENTED  
✅ React Dashboard - CREATED  
✅ CSS Theme - CREATED  
✅ Documentation - COMPLETE  
⏳ Frontend Integration - READY (5 minutes)  

**Ready for production deployment.**
