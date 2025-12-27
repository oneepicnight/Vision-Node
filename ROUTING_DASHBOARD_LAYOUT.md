# Routing Intelligence Dashboard - Visual Layout Guide

## Dashboard Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Routing Intelligence Dashboard                      Updated 14:23:10        │
│ Live view of swarm topology, peer quality, and adversarial defense.        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ╔══════════════════════════ CLUSTER HEALTH ═══════════════════════════╗   │
│  ║                                                                       ║   │
│  ║  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                ║   │
│  ║  │ Inner Ring  │  │ Middle Ring │  │ Outer Ring  │                ║   │
│  ║  │   12 / 22   │  │   6 / 22    │  │   4 / 22    │                ║   │
│  ║  │    45 ms    │  │   120 ms    │  │   250 ms    │                ║   │
│  ║  └─────────────┘  └─────────────┘  └─────────────┘                ║   │
│  ║                                                                       ║   │
│  ║  ┌────────────────────────────────────────────────────────────────┐ ║   │
│  ║  │ Guardians & Anchors                                            │ ║   │
│  ║  │ [Guardians: 3] [Anchors: 2]                                   │ ║   │
│  ║  │                                                                 │ ║   │
│  ║  │ Routing Health: 85% [Excellent]                               │ ║   │
│  ║  └────────────────────────────────────────────────────────────────┘ ║   │
│  ╚═══════════════════════════════════════════════════════════════════════╝   │
│                                                                              │
│  ╔══════════════ TOP PEERS ═════════════╦═══ BAD ACTORS & WATCHLIST ═══╗   │
│  ║ Ranked by routing score, latency     ║ Low reputation / trust issues ║   │
│  ║                                       ║                               ║   │
│  ║ Node           Ring  Region  Latency ║ Node         Trust  Rep      ║   │
│  ║────────────────────────────────────  ║──────────────────────────────║   │
│  ║ VNODE-ABC-123  [In]  NA-US   45ms   ║ VNODE-BAD-1  [Gray]  15/100  ║   │
│  ║ Success: 95%  Score: 125.3  [Tru]   ║ Latency: 350ms  Score: 25.1  ║   │
│  ║                                       ║                               ║   │
│  ║ VNODE-DEF-456  [In]  NA-CA   52ms   ║ VNODE-BAD-2  [Prob]  38/100  ║   │
│  ║ Success: 92%  Score: 118.7  [Tru]   ║ Latency: 180ms  Score: 45.8  ║   │
│  ║                                       ║                               ║   │
│  ║ VNODE-GHI-789  [Mid] EU-UK   115ms  ║ No bad actors detected.       ║   │
│  ║ Success: 88%  Score: 102.4  [Nor]   ║ The swarm is calm… for now.  ║   │
│  ║                                       ║                               ║   │
│  ║ (17 more peers...)                   ║ Graylisted/Banned peers decay ║   │
│  ╚═══════════════════════════════════════╩═══════════════════════════════╝   │
│                                                                              │
│  ╔═══════════ NETWORK EVOLUTION TIMELINE ══════════╦═══ TREND SNAPSHOT ═══╗│
│  ║ Most recent routing-related events              ║                      ║│
│  ║                                                  ║ Total Peers: 22     ║│
│  ║ [14:23:10] [WARN] Peer VNODE-BAD-1 graylisted  ║                      ║│
│  ║            (misbehavior: 35.0 >= 30.0)          ║ Inner Ring: 55%     ║│
│  ║                                                  ║                      ║│
│  ║ [14:22:30] [INFO] Cluster balance maintained:  ║ Routing Health: 85% ║│
│  ║            12 inner, 6 middle, 4 outer          ║                      ║│
│  ║                                                  ║ Routing intelligence ║│
│  ║ [14:21:45] [INFO] Peer VNODE-ABC-123 promoted: ║ adapts as peers join,║│
│  ║            95% success rate, 45ms avg delivery  ║ leave, and misbehave.║│
│  ║                                                  ║                      ║│
│  ║ (47 more events, scroll to see...)              ║                      ║│
│  ╚══════════════════════════════════════════════════╩══════════════════════╝│
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Color Scheme (Cyberpunk Theme)

### Trust Level Pills
```
[Trusted]      → Bright Green (#00ff00) with green glow
[Normal]       → Cyan (#00ffff) neutral
[Probation]    → Orange (#ffaa00) warning
[Graylisted]   → Gray (#aaaaaa) muted
[Banned]       → Red (#ff0000) with red glow
```

### Ring Chips
```
[Inner]   → Green (#00ff00) - fast, local
[Middle]  → Cyan (#00ffff) - regional backup
[Outer]   → Magenta (#ff00ff) - global backbone
```

### Health Badges
```
[Excellent]  → Green bg, >= 80%
[Healthy]    → Cyan bg, 60-79%
[Degraded]   → Yellow bg, 40-59%
[Critical]   → Red bg, < 40%
```

### Event Level Indicators
```
[INFO]  → Cyan border, normal events
[WARN]  → Orange border, attention needed
[BAD]   → Red border, problems detected
```

---

## Responsive Breakpoints

### Desktop (> 1200px)
```
┌────────────────────────────────────┐
│ [Cluster Health - Full Width]     │
├──────────────────┬─────────────────┤
│ [Top Peers 50%]  │ [Bad Actors 50%]│
├──────────────────┴─────────────────┤
│ [Events 66%]     │ [Trends 33%]    │
└──────────────────┴─────────────────┘
```

### Tablet (768px - 1200px)
```
┌────────────────────────────────────┐
│ [Cluster Health - Full Width]     │
├────────────────────────────────────┤
│ [Top Peers - Full Width]           │
├────────────────────────────────────┤
│ [Bad Actors - Full Width]          │
├────────────────────────────────────┤
│ [Events - Full Width]              │
├────────────────────────────────────┤
│ [Trends - Full Width]              │
└────────────────────────────────────┘
```

### Mobile (< 768px)
- Same stacked layout
- Reduced padding
- Smaller fonts
- Scrollable tables

---

## Component Hierarchy

```
RoutingIntelligenceDashboard
├── Panel Header
│   ├── Title + Subtitle
│   └── Last Updated Timestamp
│
├── Top Row: Cluster Health Card
│   ├── Inner Ring Metric (count, latency)
│   ├── Middle Ring Metric
│   ├── Outer Ring Metric
│   └── Guardian/Anchor Summary + Health Score
│
├── Middle Row
│   ├── Top Peers Card
│   │   ├── Table Header
│   │   └── 20 Peer Rows (node, ring, region, latency, success, score, trust)
│   │
│   └── Bad Actors Card
│       ├── Table Header
│       └── 20 Bad Actor Rows (node, trust, reputation, latency, score)
│
└── Bottom Row
    ├── Network Evolution Card
    │   └── Scrollable Event Timeline (50 events)
    │
    └── Trend Snapshot Card
        ├── Total Peers Metric
        ├── Inner Ring Share Metric
        └── Routing Health Metric
```

---

## Interactive States

### Loading State
```
┌─────────────────────────────────────┐
│ Routing Intelligence Dashboard      │
│ [Syncing…] ← Pulsing animation      │
├─────────────────────────────────────┤
│ [Loading cluster stats...]          │
└─────────────────────────────────────┘
```

### Empty State
```
┌─────────────────────────────────────┐
│ Top Peers                           │
├─────────────────────────────────────┤
│ No peers ranked yet.                │
│ The constellation is warming up.    │
└─────────────────────────────────────┘
```

### Error State (with Fallback)
```
┌─────────────────────────────────────┐
│ Routing Intelligence Dashboard      │
│ Updated 14:23:10                    │
├─────────────────────────────────────┤
│ [Showing mock data - API offline]  │
│ Inner: 12, Middle: 6, Outer: 4     │
└─────────────────────────────────────┘
```

### Hover State (Table Rows)
```
┌─────────────────────────────────────┐
│ VNODE-ABC-123  [In]  NA-US  45ms   │ ← Highlighted row
│ ──────────────────────────────────  │   (cyan glow)
│ Success: 95%  Score: 125.3  [Tru]  │
└─────────────────────────────────────┘
```

---

## Animation Effects

### Pulse Animation (Loading)
```css
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
```

### Fade In (New Events)
```css
@keyframes fadeIn {
  from { opacity: 0; transform: translateY(-10px); }
  to { opacity: 1; transform: translateY(0); }
}
```

### Glow Effect (Trust Pills)
```css
.ri-pill--trusted {
  box-shadow: 0 0 10px rgba(0, 255, 0, 0.5);
}

.ri-pill--banned {
  box-shadow: 0 0 10px rgba(255, 0, 0, 0.5);
}
```

---

## Example Data States

### Healthy Network (85% Health)
```
Inner Ring: 12/22 (55%) - avg 45ms
Middle Ring: 6/22 (27%) - avg 120ms
Outer Ring: 4/22 (18%) - avg 250ms
Guardians: 3, Anchors: 2
Bad Actors: 0

Recent Event: "Cluster balance maintained"
```

### Degraded Network (55% Health)
```
Inner Ring: 5/22 (23%) - avg 180ms  ← Too few!
Middle Ring: 10/22 (45%) - avg 200ms
Outer Ring: 7/22 (32%) - avg 350ms
Guardians: 1, Anchors: 0
Bad Actors: 3 graylisted

Recent Event: "Inner ring under target (5 < 8)"
```

### Under Attack (35% Health)
```
Inner Ring: 8/22 (36%) - avg 90ms
Middle Ring: 6/22 (27%) - avg 150ms
Outer Ring: 8/22 (36%) - avg 280ms
Guardians: 2, Anchors: 1
Bad Actors: 5 graylisted, 2 banned  ← Attack detected!

Recent Events:
[BAD] "Peer XYZ-123 banned (misbehavior: 85.0)"
[BAD] "Peer ABC-456 banned (misbehavior: 90.0)"
[WARN] "Peer DEF-789 graylisted (spam +15.0)"
```

---

## Real-World Example Timeline

**T+0:00** - Dashboard opens
```
[INFO] Fetching routing stats...
[INFO] Connected to 22 peers
[INFO] Cluster health: 85%
```

**T+0:20** - First auto-refresh
```
[INFO] Cluster balance maintained: 12 inner, 6 middle, 4 outer
[INFO] Top peer: VNODE-ABC-123 (score: 125.3)
```

**T+2:15** - Peer misbehaves
```
[WARN] Peer VNODE-BAD-1 sent invalid block
[WARN] Misbehavior +25.0 (total: 25.0, reputation: 25.0)
[WARN] Trust level: Normal → Probation
```

**T+2:45** - More violations
```
[WARN] Peer VNODE-BAD-1 spam +15.0 (total: 40.0)
[BAD] Peer VNODE-BAD-1 GRAYLISTED (misbehavior: 40.0 >= 30.0)
[INFO] Excluding graylisted peer from relay targets
```

**T+1:02:45** - Ban expires
```
[INFO] Graylist expired for peer VNODE-BAD-1
[INFO] Trust level: Graylisted → Probation
[INFO] Reputation decayed: 40.0 → 35.0
```

**T+1:05:00** - Network adapts
```
[INFO] Peer VNODE-ABC-123 promoted: 98% success rate
[INFO] Route learning: VNODE-ABC-123 avg delivery 42ms
[INFO] Routing score updated: 125.3 → 132.8
```

---

## CSS Class Reference (Quick Lookup)

### Layout
- `.routing-intel-panel` - Main container
- `.ri-grid`, `.ri-grid--top/middle/bottom` - Grid sections
- `.ri-card`, `.ri-card--health/top-peers/bad-actors/evolution/trend` - Card types

### Components
- `.ri-badge`, `.ri-badge--good/ok/warn/bad/neutral` - Health badges
- `.ri-pill`, `.ri-pill--trusted/normal/probation/gray/banned` - Trust pills
- `.ri-chip`, `.ri-chip--inner/middle/outer/guardian/anchor` - Ring/role chips

### Tables
- `.ri-table`, `.ri-table--compact` - Table styles
- `.ri-row--banned/gray/probation` - Highlighted rows
- `.ri-node`, `.ri-node__tag`, `.ri-node__id` - Node display

### Events
- `.ri-event`, `.ri-event--info/warn/bad` - Event items
- `.ri-event__meta`, `.ri-event__time`, `.ri-event__level` - Event parts

### Misc
- `.ri-empty` - Empty state message
- `.ri-meta`, `.ri-meta--loading` - Metadata displays
- `.ri-value`, `.ri-label`, `.ri-sub` - Metric displays

---

## Summary

**Visual Identity:**
- Dark cyberpunk theme with neon accents
- Cyan/magenta/green color scheme
- Glassmorphism effects
- Color-coded trust levels and event severity

**Layout Philosophy:**
- Information density (operator dashboard, not consumer app)
- Responsive grid (desktop → tablet → mobile)
- Scannable tables with visual hierarchy
- Real-time updates without page jumps

**User Experience:**
- Auto-refresh every 20s (configurable)
- Graceful fallback to mock data
- Clear empty states
- Color-coded severity for quick assessment

**Accessibility:**
- High contrast colors
- Clear typography
- Semantic HTML structure
- Keyboard-navigable tables

---

**Result:** Tactical command center for monitoring your adaptive, self-defending swarm in real time. 🚀🔥
