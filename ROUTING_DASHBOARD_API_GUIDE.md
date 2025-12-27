# Routing Intelligence Dashboard - API Integration Guide

## Overview

This document describes the backend API endpoints required to power the Routing Intelligence Dashboard. These endpoints expose Phase 3.5 (latency-based routing) and Phase 4 (reputation system) metrics for real-time monitoring.

---

## Required API Endpoints

### 1. Cluster Statistics

**Endpoint:** `GET /api/p2p/routing/cluster_stats`

**Description:** Returns current swarm topology and health metrics.

**Response Type:**
```typescript
interface ClusterStats {
  inner_count: number;              // Peers in Inner ring (< 100ms, same region)
  middle_count: number;             // Peers in Middle ring (regional backup)
  outer_count: number;              // Peers in Outer ring (global backbone)
  avg_inner_latency_ms: number | null;   // Average RTT for Inner ring
  avg_middle_latency_ms: number | null;  // Average RTT for Middle ring
  avg_outer_latency_ms: number | null;   // Average RTT for Outer ring
  guardians: number;                // Count of Guardian peers
  anchors: number;                  // Count of Anchor peers
  total_peers: number;              // Total connected peers
  routing_health_score: number;     // Overall health (0-100)
}
```

**Example Response:**
```json
{
  "inner_count": 12,
  "middle_count": 6,
  "outer_count": 4,
  "avg_inner_latency_ms": 45,
  "avg_middle_latency_ms": 120,
  "avg_outer_latency_ms": 250,
  "guardians": 3,
  "anchors": 2,
  "total_peers": 22,
  "routing_health_score": 85
}
```

**Rust Implementation (axum):**
```rust
use axum::{Json, extract::State};
use crate::p2p::{PeerStore, get_cluster_stats, PeerRing};

pub async fn get_cluster_stats_handler(
    State(peer_store): State<Arc<PeerStore>>,
) -> Json<ClusterStats> {
    let stats = get_cluster_stats(&peer_store, None); // Pass local_region if available
    
    // Calculate routing health score
    let health_score = calculate_routing_health(&peer_store, &stats);
    
    Json(ClusterStats {
        inner_count: stats.inner_count,
        middle_count: stats.middle_count,
        outer_count: stats.outer_count,
        avg_inner_latency_ms: stats.avg_inner_latency,
        avg_middle_latency_ms: stats.avg_middle_latency,
        avg_outer_latency_ms: stats.avg_outer_latency,
        guardians: stats.guardians,
        anchors: stats.anchors,
        total_peers: peer_store.all().len(),
        routing_health_score: health_score,
    })
}

fn calculate_routing_health(peer_store: &PeerStore, stats: &ClusterStats) -> f32 {
    let mut score = 100.0;
    
    // Penalty for low peer count
    if stats.total_peers < 10 {
        score -= 20.0;
    }
    
    // Penalty for imbalanced rings
    let inner_ratio = stats.inner_count as f32 / stats.total_peers as f32;
    if inner_ratio < 0.4 {
        score -= 15.0; // Want at least 40% inner ring
    }
    
    // Penalty for high latency
    if let Some(avg_inner) = stats.avg_inner_latency_ms {
        if avg_inner > 100 {
            score -= 10.0;
        }
    }
    
    // Bonus for guardians/anchors
    score += (stats.guardians + stats.anchors) as f32 * 2.0;
    
    score.max(0.0).min(100.0)
}
```

---

### 2. Top Peers

**Endpoint:** `GET /api/p2p/routing/top_peers?limit=20`

**Description:** Returns highest-scoring peers ranked by routing intelligence.

**Query Parameters:**
- `limit` (optional, default: 20): Maximum peers to return

**Response Type:**
```typescript
interface RoutingPeer {
  node_tag: string;            // Human-readable node name
  node_id: string;             // Unique node identifier
  region: string | null;       // Geographic region
  ring: "Inner" | "Middle" | "Outer";
  routing_score: number;       // Composite routing score
  avg_rtt_ms: number | null;   // Average round-trip time
  success_rate: number;        // 0.0-1.0 (route_successes / route_uses)
  trust_level: "Trusted" | "Normal" | "Probation" | "Graylisted" | "Banned";
  reputation: number;          // 0-100 reputation score
  misbehavior_score: number;   // Accumulated violations
}
```

**Example Response:**
```json
[
  {
    "node_tag": "VNODE-ABC-123",
    "node_id": "abc123xyz789",
    "region": "North America > United States",
    "ring": "Inner",
    "routing_score": 125.3,
    "avg_rtt_ms": 45,
    "success_rate": 0.95,
    "trust_level": "Trusted",
    "reputation": 92.0,
    "misbehavior_score": 0.0
  }
]
```

**Rust Implementation:**
```rust
use axum::{Json, extract::{State, Query}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TopPeersQuery {
    limit: Option<usize>,
}

pub async fn get_top_peers_handler(
    State(peer_store): State<Arc<PeerStore>>,
    Query(params): Query<TopPeersQuery>,
) -> Json<Vec<RoutingPeerJson>> {
    let limit = params.limit.unwrap_or(20);
    let local_region = None; // Get from config or detection
    
    let classified = peer_store.classify_peers_for_routing(local_region);
    
    // Sort by routing score descending
    let mut sorted: Vec<_> = classified.into_iter()
        .take(limit)
        .map(|cp| RoutingPeerJson::from_peer(cp.peer, cp.ring, cp.score))
        .collect();
    
    sorted.sort_by(|a, b| b.routing_score.partial_cmp(&a.routing_score).unwrap());
    
    Json(sorted)
}

#[derive(Serialize)]
pub struct RoutingPeerJson {
    node_tag: String,
    node_id: String,
    region: Option<String>,
    ring: String,
    routing_score: f32,
    avg_rtt_ms: Option<u32>,
    success_rate: f32,
    trust_level: String,
    reputation: f32,
    misbehavior_score: f32,
}

impl RoutingPeerJson {
    fn from_peer(peer: VisionPeer, ring: PeerRing, score: f32) -> Self {
        let success_rate = if peer.route_uses > 0 {
            peer.route_successes as f32 / peer.route_uses as f32
        } else {
            0.0
        };
        
        Self {
            node_tag: peer.node_tag,
            node_id: peer.node_id,
            region: peer.region,
            ring: format!("{:?}", ring),
            routing_score: score,
            avg_rtt_ms: peer.avg_rtt_ms,
            success_rate,
            trust_level: format!("{:?}", peer.trust_level),
            reputation: peer.reputation,
            misbehavior_score: peer.misbehavior_score,
        }
    }
}
```

---

### 3. Routing Events

**Endpoint:** `GET /api/p2p/routing/events?limit=50`

**Description:** Returns recent routing-related events (cluster changes, reputation changes, bans, etc.)

**Query Parameters:**
- `limit` (optional, default: 50): Maximum events to return

**Response Type:**
```typescript
interface RoutingEvent {
  id: string;              // Unique event ID
  timestamp: string;       // ISO 8601 timestamp
  level: "info" | "warn" | "bad";
  message: string;         // Human-readable event description
}
```

**Example Response:**
```json
[
  {
    "id": "evt_1702342890_001",
    "timestamp": "2025-12-08T14:23:10.000Z",
    "level": "warn",
    "message": "Peer VNODE-XYZ-456 graylisted (misbehavior: 35.0 >= 30.0)"
  },
  {
    "id": "evt_1702342850_002",
    "timestamp": "2025-12-08T14:22:30.000Z",
    "level": "info",
    "message": "Cluster balance maintained: 12 inner, 6 middle, 4 outer"
  }
]
```

**Rust Implementation:**
```rust
use std::sync::Arc;
use parking_lot::RwLock;

// Event store (in-memory ring buffer)
pub struct RoutingEventStore {
    events: RwLock<VecDeque<RoutingEvent>>,
    max_events: usize,
}

impl RoutingEventStore {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
        }
    }
    
    pub fn push(&self, level: EventLevel, message: String) {
        let mut events = self.events.write();
        let event = RoutingEvent {
            id: format!("evt_{}_{}", 
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                events.len()
            ),
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            message,
        };
        
        events.push_front(event);
        
        if events.len() > self.max_events {
            events.pop_back();
        }
    }
    
    pub fn get_recent(&self, limit: usize) -> Vec<RoutingEvent> {
        self.events.read().iter().take(limit).cloned().collect()
    }
}

pub async fn get_routing_events_handler(
    State(event_store): State<Arc<RoutingEventStore>>,
    Query(params): Query<EventsQuery>,
) -> Json<Vec<RoutingEvent>> {
    let limit = params.limit.unwrap_or(50);
    Json(event_store.get_recent(limit))
}

// Hook into reputation system and routing logic:
// - When apply_misbehavior() is called → push event
// - When decay_reputation() promotes trust level → push event
// - When maintain_cluster_balance() detects imbalance → push event
```

---

## Axum Router Setup

Add these routes to your existing P2P API router:

```rust
use axum::{Router, routing::get};

pub fn routing_intelligence_api() -> Router {
    Router::new()
        .route("/api/p2p/routing/cluster_stats", get(get_cluster_stats_handler))
        .route("/api/p2p/routing/top_peers", get(get_top_peers_handler))
        .route("/api/p2p/routing/events", get(get_routing_events_handler))
}
```

---

## Event Generation Examples

### From Reputation System

```rust
// In reputation.rs apply_misbehavior()
if peer.misbehavior_score >= config.ban_threshold {
    peer.trust_level = PeerTrustLevel::Banned;
    
    // Push event
    if let Some(event_store) = ROUTING_EVENT_STORE.get() {
        event_store.push(
            EventLevel::Bad,
            format!("Peer {} BANNED: {} (misbehavior: {:.1})",
                peer.node_tag, kind.description(), peer.misbehavior_score
            )
        );
    }
}
```

### From Cluster Balancing

```rust
// In routing.rs maintain_cluster_balance()
if inner_count < targets.inner {
    if let Some(event_store) = ROUTING_EVENT_STORE.get() {
        event_store.push(
            EventLevel::Warn,
            format!("Inner ring under target ({} < {}), need more local peers",
                inner_count, targets.inner
            )
        );
    }
}
```

### From Route Learning

```rust
// In routing.rs mark_route_success()
let success_rate = route_success_rate(&peer);
if success_rate >= 0.95 && peer.route_uses >= 10 {
    if let Some(event_store) = ROUTING_EVENT_STORE.get() {
        event_store.push(
            EventLevel::Info,
            format!("Peer {} promoted: {:.0}% success rate, {} uses",
                peer.node_tag, success_rate * 100.0, peer.route_uses
            )
        );
    }
}
```

---

## Frontend Integration

Import and add to your Command Center layout:

```tsx
import RoutingIntelligenceDashboard from './components/command-center/RoutingIntelligenceDashboard';
import './styles/routing-intelligence.css';

function CommandCenter() {
  return (
    <div className="command-center">
      {/* Other panels */}
      
      <RoutingIntelligenceDashboard />
    </div>
  );
}
```

---

## WebSocket Enhancement (Optional)

For real-time updates without polling, add WebSocket support:

```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

pub async fn routing_events_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_routing_events_socket)
}

async fn handle_routing_events_socket(mut socket: WebSocket) {
    let mut rx = ROUTING_EVENT_STORE.subscribe();
    
    while let Ok(event) = rx.recv().await {
        let json = serde_json::to_string(&event).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}
```

Then update the React component to use WebSocket instead of polling.

---

## Summary

**Endpoints Required:**
1. `GET /api/p2p/routing/cluster_stats` - Swarm topology
2. `GET /api/p2p/routing/top_peers?limit=20` - Best performers
3. `GET /api/p2p/routing/events?limit=50` - Event timeline

**Integration Points:**
- Wire event store into reputation system (apply_misbehavior, decay_reputation)
- Wire event store into routing logic (cluster balance, route learning)
- Add routes to Axum router
- Import dashboard component in Command Center

**Result:** Live tactical view of Vision Network's adaptive swarm intelligence and self-defending capabilities. 🔥
