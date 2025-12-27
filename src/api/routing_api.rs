//! Routing Intelligence API
//!
//! Provides HTTP API endpoints for the Routing Intelligence Dashboard:
//! - GET /api/p2p/routing/cluster_stats - Cluster health and ring distribution
//! - GET /api/p2p/routing/top_peers - Highest-scoring peers with metrics
//! - GET /api/p2p/routing/events - Recent routing and reputation events
//! - WS /api/p2p/routing/events_stream - Real-time WebSocket event streaming
//! - GET /metrics - Prometheus metrics for Grafana integration

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    response::IntoResponse,
    Json,
};
use futures_util::future::FutureExt;
use once_cell::sync::Lazy;
use prometheus::{
    register_gauge_with_registry, register_int_gauge_with_registry, Encoder, Gauge, IntGauge,
    Registry, TextEncoder,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

use crate::{
    p2p::peer_store::{PeerStore, PeerTrustLevel},
    CHAIN,
};

// ============================================================================
// EVENT STORE (Ring Buffer for Timeline)
// ============================================================================

/// Routing event for timeline display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingEvent {
    pub timestamp: u64,
    pub level: String, // "info", "warn", "bad"
    pub message: String,
}

/// Global event store with ring buffer (max 500 events)
pub static ROUTING_EVENT_STORE: Lazy<Arc<RwLock<VecDeque<RoutingEvent>>>> =
    Lazy::new(|| Arc::new(RwLock::new(VecDeque::with_capacity(500))));

/// Push a new routing event to the event store
pub fn push_routing_event(level: &str, message: String) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let event = RoutingEvent {
        timestamp,
        level: level.to_string(),
        message,
    };

    if let Ok(mut store) = ROUTING_EVENT_STORE.write() {
        if store.len() >= 500 {
            store.pop_front(); // Remove oldest event
        }
        store.push_back(event);
    }

    // Update Prometheus event counter
    ROUTING_EVENTS_TOTAL.inc();
}

// ============================================================================
// PROMETHEUS METRICS (Grafana Integration)
// ============================================================================

/// Prometheus metrics registry for routing intelligence
pub static METRICS_REGISTRY: Lazy<Registry> =
    Lazy::new(|| Registry::new_custom(Some("vision".to_string()), None).unwrap());

/// Cluster health score (0-100)
pub static CLUSTER_HEALTH_SCORE: Lazy<Gauge> = Lazy::new(|| {
    register_gauge_with_registry!(
        "cluster_health_score",
        "Routing health score (0-100)",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Inner ring peer count
pub static INNER_RING_PEERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "inner_ring_peers",
        "Number of peers in inner ring (low latency)",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Middle ring peer count
pub static MIDDLE_RING_PEERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "middle_ring_peers",
        "Number of peers in middle ring (regional)",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Outer ring peer count
pub static OUTER_RING_PEERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "outer_ring_peers",
        "Number of peers in outer ring (global)",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Guardian count
pub static GUARDIAN_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "guardian_count",
        "Number of guardian nodes",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Anchor count
pub static ANCHOR_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!("anchor_count", "Number of anchor nodes", METRICS_REGISTRY)
        .unwrap()
});

/// Average inner ring latency
pub static INNER_RING_LATENCY: Lazy<Gauge> = Lazy::new(|| {
    register_gauge_with_registry!(
        "inner_ring_latency_ms",
        "Average latency for inner ring peers (milliseconds)",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Total routing events generated
pub static ROUTING_EVENTS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "routing_events_total",
        "Total routing events generated",
        METRICS_REGISTRY
    )
    .unwrap()
});

/// Banned peer count
pub static BANNED_PEERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!("banned_peers", "Number of banned peers", METRICS_REGISTRY)
        .unwrap()
});

/// Graylisted peer count
pub static GRAYLISTED_PEERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge_with_registry!(
        "graylisted_peers",
        "Number of graylisted peers",
        METRICS_REGISTRY
    )
    .unwrap()
});

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Cluster statistics response
#[derive(Serialize, Deserialize)]
pub struct ClusterStats {
    pub inner_count: usize,
    pub middle_count: usize,
    pub outer_count: usize,
    pub total_count: usize,
    pub inner_avg_latency_ms: u32,
    pub middle_avg_latency_ms: u32,
    pub outer_avg_latency_ms: u32,
    pub guardian_count: usize,
    pub anchor_count: usize,
    pub health_score: f32,
}

/// Peer entry for top peers table
#[derive(Serialize, Deserialize)]
pub struct RoutingPeerJson {
    pub node_tag: String,
    pub vision_address: String,
    pub ring: String, // "inner", "middle", "outer"
    pub region: String,
    pub latency_ms: u32,
    pub routing_score: f32,
    pub trust_level: String, // "trusted", "normal", "probation", "graylisted", "banned"
    pub reputation: f32,
    pub route_uses: u32,
    pub route_successes: u32,
    pub success_rate: f32, // Percentage (0.0 - 100.0)
    pub is_guardian: bool,
    pub is_anchor: bool,
}

/// Query parameters for top peers endpoint
#[derive(Deserialize)]
pub struct TopPeersQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Query parameters for events endpoint
#[derive(Deserialize)]
pub struct EventsQuery {
    #[serde(default = "default_events_limit")]
    pub limit: usize,
}

fn default_events_limit() -> usize {
    50
}

// ============================================================================
// API ENDPOINT HANDLERS
// ============================================================================

/// WS /api/p2p/routing/events_stream
/// WebSocket endpoint for real-time event streaming
pub async fn routing_events_ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_routing_events_stream)
}

/// Handle WebSocket connection for event streaming
async fn handle_routing_events_stream(mut socket: WebSocket) {
    let mut last_event_count = 0;

    loop {
        // Check for new events
        let new_events: Vec<String> = {
            if let Ok(store) = ROUTING_EVENT_STORE.read() {
                let current_count = store.len();

                if current_count > last_event_count {
                    // Collect new events to send
                    let events: Vec<String> = store
                        .iter()
                        .skip(last_event_count)
                        .filter_map(|event| serde_json::to_string(event).ok())
                        .collect();
                    last_event_count = current_count;
                    events
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };

        // Send new events (lock is dropped here)
        for json in new_events {
            if socket.send(Message::Text(json)).await.is_err() {
                // Client disconnected
                return;
            }
        }

        // Check if client is still connected
        if socket.recv().now_or_never().is_some() {
            // Client sent close or disconnected
            return;
        }

        // Wait before checking for new events
        sleep(Duration::from_secs(1)).await;
    }
}

/// GET /metrics
/// Prometheus metrics endpoint for Grafana integration
pub async fn get_metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        )
            .into_response();
    }

    let output = match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to convert metrics to UTF-8: {}", e),
            )
                .into_response();
        }
    };

    (
        axum::http::StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        output,
    )
        .into_response()
}

/// GET /api/p2p/routing/cluster_stats
/// Returns cluster health metrics and ring distribution
pub async fn get_cluster_stats_handler() -> Json<ClusterStats> {
    let chain = CHAIN.lock();

    let stats = match PeerStore::new(&chain.db) {
        Ok(peer_store) => {
            let all_peers = peer_store.all();

            // Classify by ring
            let mut inner_peers = Vec::new();
            let mut middle_peers = Vec::new();
            let mut outer_peers = Vec::new();
            let mut guardian_count = 0;
            let mut anchor_count = 0;

            for peer in &all_peers {
                // Count guardians and anchors
                if peer.role == "guardian" {
                    guardian_count += 1;
                }
                if peer.role == "anchor" {
                    anchor_count += 1;
                }

                // Classify by ring (latency-based)
                let ring = classify_ring_simple(peer);
                match ring {
                    "inner" => inner_peers.push(peer),
                    "middle" => middle_peers.push(peer),
                    "outer" => outer_peers.push(peer),
                    _ => {} // Unknown ring
                }
            }

            // Calculate average latencies
            let inner_avg = if !inner_peers.is_empty() {
                inner_peers
                    .iter()
                    .map(|p| p.avg_rtt_ms.unwrap_or(0))
                    .sum::<u32>()
                    / inner_peers.len() as u32
            } else {
                0
            };

            let middle_avg = if !middle_peers.is_empty() {
                middle_peers
                    .iter()
                    .map(|p| p.avg_rtt_ms.unwrap_or(0))
                    .sum::<u32>()
                    / middle_peers.len() as u32
            } else {
                0
            };

            let outer_avg = if !outer_peers.is_empty() {
                outer_peers
                    .iter()
                    .map(|p| p.avg_rtt_ms.unwrap_or(0))
                    .sum::<u32>()
                    / outer_peers.len() as u32
            } else {
                0
            };

            // Calculate health score
            let health_score = calculate_routing_health(
                all_peers.len(),
                inner_peers.len(),
                middle_peers.len(),
                outer_peers.len(),
                guardian_count,
            );

            // Update Prometheus metrics
            CLUSTER_HEALTH_SCORE.set(health_score as f64);
            INNER_RING_PEERS.set(inner_peers.len() as i64);
            MIDDLE_RING_PEERS.set(middle_peers.len() as i64);
            OUTER_RING_PEERS.set(outer_peers.len() as i64);
            GUARDIAN_COUNT.set(guardian_count as i64);
            ANCHOR_COUNT.set(anchor_count as i64);
            INNER_RING_LATENCY.set(inner_avg as f64);

            ClusterStats {
                inner_count: inner_peers.len(),
                middle_count: middle_peers.len(),
                outer_count: outer_peers.len(),
                total_count: all_peers.len(),
                inner_avg_latency_ms: inner_avg,
                middle_avg_latency_ms: middle_avg,
                outer_avg_latency_ms: outer_avg,
                guardian_count,
                anchor_count,
                health_score,
            }
        }
        Err(_) => {
            // Return empty stats on error
            ClusterStats {
                inner_count: 0,
                middle_count: 0,
                outer_count: 0,
                total_count: 0,
                inner_avg_latency_ms: 0,
                middle_avg_latency_ms: 0,
                outer_avg_latency_ms: 0,
                guardian_count: 0,
                anchor_count: 0,
                health_score: 0.0,
            }
        }
    };

    drop(chain);
    Json(stats)
}

/// GET /api/p2p/routing/top_peers?limit=20
/// Returns top-scoring peers with routing intelligence metrics
pub async fn get_top_peers_handler(
    Query(params): Query<TopPeersQuery>,
) -> Json<Vec<RoutingPeerJson>> {
    let chain = CHAIN.lock();

    let peers = match PeerStore::new(&chain.db) {
        Ok(peer_store) => {
            let classified = peer_store.classify_peers_for_routing(None);

            // Map to JSON format (classified already contains all peers sorted by score)
            let peer_jsons: Vec<RoutingPeerJson> = classified
                .into_iter()
                .map(|classified_peer| {
                    let peer = classified_peer.peer;
                    let ring_str = match classified_peer.ring {
                        crate::p2p::peer_store::PeerRing::Inner => "inner",
                        crate::p2p::peer_store::PeerRing::Middle => "middle",
                        crate::p2p::peer_store::PeerRing::Outer => "outer",
                    };

                    let success_rate = if peer.route_uses > 0 {
                        (peer.route_successes as f32 / peer.route_uses as f32) * 100.0
                    } else {
                        0.0
                    };

                    let trust_level_str = match peer.trust_level {
                        PeerTrustLevel::Trusted => "trusted",
                        PeerTrustLevel::Normal => "normal",
                        PeerTrustLevel::Probation => "probation",
                        PeerTrustLevel::Graylisted => "graylisted",
                        PeerTrustLevel::Banned => "banned",
                    };

                    let region = peer.region.clone().unwrap_or_else(|| "unknown".to_string());

                    RoutingPeerJson {
                        node_tag: peer.node_tag,
                        vision_address: peer.vision_address,
                        ring: ring_str.to_string(),
                        region,
                        latency_ms: peer.avg_rtt_ms.unwrap_or(0),
                        routing_score: classified_peer.score,
                        trust_level: trust_level_str.to_string(),
                        reputation: peer.reputation,
                        route_uses: peer.route_uses,
                        route_successes: peer.route_successes,
                        success_rate,
                        is_guardian: peer.role == "guardian",
                        is_anchor: peer.role == "anchor",
                    }
                })
                .collect();

            // Count banned and graylisted for Prometheus metrics
            let banned = peer_jsons
                .iter()
                .filter(|p| p.trust_level == "banned")
                .count();
            let graylisted = peer_jsons
                .iter()
                .filter(|p| p.trust_level == "graylisted")
                .count();
            BANNED_PEERS.set(banned as i64);
            GRAYLISTED_PEERS.set(graylisted as i64);

            // Take top N
            peer_jsons.into_iter().take(params.limit).collect()
        }
        Err(_) => vec![],
    };

    drop(chain);
    Json(peers)
}

/// GET /api/p2p/routing/events?limit=50
/// Returns recent routing and reputation events
pub async fn get_routing_events_handler(
    Query(params): Query<EventsQuery>,
) -> Json<Vec<RoutingEvent>> {
    let events = if let Ok(store) = ROUTING_EVENT_STORE.read() {
        // Get last N events (most recent first)
        store.iter().rev().take(params.limit).cloned().collect()
    } else {
        vec![]
    };

    Json(events)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Simple ring classification based on latency only (for stats calculation)
fn classify_ring_simple(peer: &crate::p2p::peer_store::VisionPeer) -> &'static str {
    let avg = peer.avg_rtt_ms.unwrap_or(200);

    if avg <= 100 {
        "inner"
    } else if avg <= 200 {
        "middle"
    } else {
        "outer"
    }
}

/// Calculate overall routing health score (0.0 - 100.0)
///
/// Health factors:
/// - Ring distribution balance (ideal: 60% inner, 30% middle, 10% outer)
/// - Guardian presence (bonus points)
/// - Total peer count (penalties if too low)
fn calculate_routing_health(
    total: usize,
    inner: usize,
    middle: usize,
    outer: usize,
    guardians: usize,
) -> f32 {
    if total == 0 {
        return 0.0;
    }

    let mut health = 50.0; // Base score

    // Ring distribution scoring (max +30 points)
    let inner_pct = (inner as f32 / total as f32) * 100.0;
    let middle_pct = (middle as f32 / total as f32) * 100.0;
    let outer_pct = (outer as f32 / total as f32) * 100.0;

    // Ideal distribution: 60% inner, 30% middle, 10% outer
    let inner_delta = (inner_pct - 60.0).abs();
    let middle_delta = (middle_pct - 30.0).abs();
    let outer_delta = (outer_pct - 10.0).abs();

    let distribution_score = 30.0 - (inner_delta + middle_delta + outer_delta) / 10.0;
    health += distribution_score.max(0.0);

    // Guardian bonus (max +10 points)
    health += (guardians as f32 * 3.0).min(10.0);

    // Peer count scoring (max +10 points)
    if total >= 20 {
        health += 10.0;
    } else if total >= 10 {
        health += 5.0;
    }

    // Clamp to 0-100 range
    health.max(0.0).min(100.0)
}

// ============================================================================
// EVENT GENERATION HELPERS (called from reputation system)
// ============================================================================

/// Log a misbehavior event to the routing timeline
pub fn log_misbehavior_event(node_tag: &str, kind: &str, new_score: f32, trust_level: &str) {
    let message = format!(
        "Peer {} misbehavior: {} (score: {:.1}, trust: {})",
        node_tag, kind, new_score, trust_level
    );

    let level = if trust_level == "banned" || trust_level == "graylisted" {
        "bad"
    } else {
        "warn"
    };

    push_routing_event(level, message);
}

/// Log a reputation decay event
pub fn log_decay_event(node_tag: &str, new_reputation: f32) {
    let message = format!(
        "Peer {} reputation decayed: {:.1}/100.0",
        node_tag, new_reputation
    );
    push_routing_event("info", message);
}

/// Log a cluster balance event
pub fn log_cluster_balance_event(inner: usize, middle: usize, outer: usize) {
    let message = format!(
        "Cluster balance: {} inner, {} middle, {} outer",
        inner, middle, outer
    );
    push_routing_event("info", message);
}

/// Log a peer promotion event (high success rate)
pub fn log_peer_promotion_event(node_tag: &str, success_rate: f32, avg_delivery_ms: u32) {
    let message = format!(
        "Peer {} performing well: {:.1}% success, {}ms avg delivery",
        node_tag, success_rate, avg_delivery_ms
    );
    push_routing_event("info", message);
}

/// Log a ban expiry event
pub fn log_ban_expiry_event(node_tag: &str, trust_level: &str) {
    let message = format!("Peer {} ban expired, now: {}", node_tag, trust_level);
    push_routing_event("info", message);
}
