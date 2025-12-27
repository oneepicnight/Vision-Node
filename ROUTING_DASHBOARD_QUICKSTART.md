# Routing Intelligence Dashboard - Quick Integration Checklist

## ✅ Files Created

```
src/components/command-center/RoutingIntelligenceDashboard.tsx  (550 lines)
src/styles/routing-intelligence.css                            (450 lines)
ROUTING_DASHBOARD_API_GUIDE.md                                 (Full API spec)
```

## 🚀 5-Minute Integration

### 1. Import the Dashboard Component

**File:** Your main Command Center component (e.g., `CommandCenter.tsx`)

```tsx
import RoutingIntelligenceDashboard from './components/command-center/RoutingIntelligenceDashboard';
import '../styles/routing-intelligence.css';
```

### 2. Add to Layout

```tsx
<div className="command-center-grid">
  {/* Existing panels */}
  <MiningPanel />
  <WalletPanel />
  <NetworkPanel />
  
  {/* NEW: Routing Intelligence Dashboard */}
  <RoutingIntelligenceDashboard />
</div>
```

### 3. Backend API Endpoints (Required)

Add these three endpoints to your Rust backend:

```rust
// In src/api/p2p_api.rs or similar

GET /api/p2p/routing/cluster_stats
GET /api/p2p/routing/top_peers?limit=20
GET /api/p2p/routing/events?limit=50
```

See `ROUTING_DASHBOARD_API_GUIDE.md` for full implementation details.

---

## 📊 What You Get

### Live Metrics Displayed:

1. **Cluster Health (Top Row)**
   - Inner/Middle/Outer ring distribution
   - Average latency per ring
   - Guardian/Anchor counts
   - Overall routing health score (0-100)

2. **Top Peers (Middle Left)**
   - 20 highest-scoring peers
   - Sorted by routing score
   - Shows: node, ring, region, latency, success rate, trust level

3. **Bad Actors (Middle Right)**
   - Peers with low reputation or trust issues
   - Shows: trust level, reputation, misbehavior score
   - Color-coded rows (banned=red, graylisted=gray, probation=orange)

4. **Network Evolution (Bottom Left)**
   - Real-time event timeline
   - Shows: reputation changes, cluster rebalancing, bans/unbans
   - Scrollable log with color-coded severity

5. **Trend Snapshot (Bottom Right)**
   - Total peer count
   - Inner ring share percentage
   - Routing health trend

---

## 🎨 Styling

The CSS is **cyberpunk-themed** with:
- Neon cyan/magenta accents
- Glassmorphism effects
- Dark backgrounds with transparency
- Color-coded trust levels (green=trusted, red=banned, etc.)
- Responsive grid layout

**Customization:** All colors use CSS variables, so you can easily theme it to match your existing Command Center palette.

---

## 🔄 Auto-Refresh

Dashboard automatically refreshes every **20 seconds** by default.

To change refresh rate, edit `RoutingIntelligenceDashboard.tsx`:

```tsx
// Line 135
const id = setInterval(fetchData, 20_000); // Change to 10_000 for 10s, etc.
```

---

## 🧪 Testing (Development Mode)

If backend APIs aren't ready yet, the dashboard has **fallback mock data** in the catch block:

```tsx
// Automatically shows sample data if fetch fails
setClusterStats({
  inner_count: 12,
  middle_count: 6,
  outer_count: 4,
  // ... etc
});
```

This lets you see the UI immediately without backend wiring.

---

## 🔌 Backend Integration Priority

**Phase 1 (MVP):** Implement just `cluster_stats` endpoint
- Dashboard will show cluster health panel
- Other panels will show "No data yet" messages

**Phase 2 (Full):** Add `top_peers` and `events` endpoints
- All panels fully functional
- Live monitoring of entire swarm

**Phase 3 (Real-Time):** Add WebSocket streaming (see API guide)
- Replace 20s polling with instant updates
- Zero-latency event notifications

---

## 📈 Backend Implementation Hints

### Fastest Path (30 minutes):

1. **Cluster Stats Endpoint:**
   ```rust
   // Use existing get_cluster_stats() from routing.rs
   let stats = crate::p2p::get_cluster_stats(&peer_store, None);
   Json(stats) // Convert to JSON response
   ```

2. **Top Peers Endpoint:**
   ```rust
   // Use classify_peers_for_routing() from peer_store.rs
   let classified = peer_store.classify_peers_for_routing(None);
   // Map to JSON, sort by score, return top 20
   ```

3. **Events Endpoint:**
   ```rust
   // Create simple in-memory VecDeque<RoutingEvent>
   // Push events from reputation system and routing logic
   // Return most recent N events
   ```

All the Rust logic already exists from Phase 3.5 + Phase 4! You just need to wrap it in HTTP handlers.

---

## 🎯 Expected User Experience

**On Load:**
```
[Dashboard appears]
Cluster Health: 12 inner, 6 middle, 4 outer | Health: 85%
Top Peers: VNODE-ABC (score: 125.3), VNODE-DEF (score: 118.7)...
Bad Actors: 2 peers on probation, 0 banned
Recent Events: "Cluster balance maintained", "Peer XYZ promoted"
```

**When Peer Misbehaves:**
```
[Event appears in timeline - RED]
"Peer VNODE-BAD-123 graylisted (misbehavior: 35.0 >= 30.0)"

[Peer appears in Bad Actors table]
Trust: Graylisted | Reputation: 15 / 100

[Routing score drops, removed from Top Peers]
```

**When Network Adapts:**
```
[Event appears - YELLOW]
"Inner ring under target (8 < 12), need more local peers"

[Cluster stats update]
Inner Ring: 8/22 peers (was 12/22)

[Health score adjusts]
Routing Health: 72% (was 85%)
```

---

## 🔥 Power User Features

### Click-to-Detail (Future Enhancement)
Add onClick handlers to peer rows:
```tsx
<tr onClick={() => openPeerDetailDrawer(peer.node_id)}>
```

### Micro-Sparklines (Future Enhancement)
Add tiny trend charts next to metrics:
```tsx
<Sparkline data={innerRingHistory} />
```

### Export Events (Future Enhancement)
Add button to download event log as CSV:
```tsx
<button onClick={exportEventsToCSV}>Export Log</button>
```

---

## 🐛 Troubleshooting

**Problem:** Dashboard shows "Syncing…" forever

**Solution:** Check browser console for fetch errors. Verify API endpoints are accessible.

---

**Problem:** No data in Top Peers table

**Solution:** Ensure `/api/p2p/routing/top_peers` returns array, not single object.

---

**Problem:** Styling looks broken

**Solution:** Verify `routing-intelligence.css` is imported before component renders.

---

## 📦 Summary

**What's Done:**
✅ React component (full TypeScript)
✅ Cyberpunk CSS theme
✅ Auto-refresh (20s polling)
✅ Fallback mock data
✅ API specification guide

**What's Needed:**
⚠️ 3 backend API endpoints (see ROUTING_DASHBOARD_API_GUIDE.md)
⚠️ Wire event store into reputation system
⚠️ Add to Command Center layout

**Time to Deploy:** ~1 hour (30 min backend, 30 min frontend integration)

---

## 🚀 Next Steps

1. Read `ROUTING_DASHBOARD_API_GUIDE.md` for backend implementation
2. Add API endpoints to your Axum router
3. Import dashboard component in Command Center
4. Test with mock data first, then wire real APIs
5. Deploy and watch your swarm think in real time! 🧠⚡

---

**Status:** Ready to integrate ✊  
**Difficulty:** Medium (requires backend API wiring)  
**Impact:** High (gives operators god-mode visibility into network intelligence)
