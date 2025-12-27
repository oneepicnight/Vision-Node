//! Immortality Status API Endpoints
//!
//! Provides visibility into Phase 6 features: constellation memory,
//! guardian rotation, and peer recovery status.

use axum::{extract::Json, response::IntoResponse};

/// GET /constellation/memory
/// Returns constellation memory summary
pub async fn get_constellation_memory() -> impl IntoResponse {
    let summary = {
        let memory = crate::CONSTELLATION_MEMORY.lock();
        memory.get_summary()
    };

    let top_peers = {
        let memory = crate::CONSTELLATION_MEMORY.lock();
        let peers = memory.get_best_peers(5);

        peers
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "ebid": p.ebid,
                    "peer_id": p.peer_id,
                    "last_seen": p.last_seen,
                    "uptime_score": p.uptime_score,
                    "is_guardian_candidate": p.is_guardian_candidate,
                    "connection_count": p.connection_count,
                    "last_ip": p.last_ip,
                })
            })
            .collect::<Vec<_>>()
    };

    Json(serde_json::json!({
        "summary": summary,
        "top_peers": top_peers,
    }))
}

/// GET /guardian/role
/// Returns current guardian role information
pub async fn get_guardian_role() -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let role_info = {
        let role = crate::GUARDIAN_ROLE.lock();

        if let Some(state) = role.get_state() {
            serde_json::json!({
                "current_guardian_ebid": state.current_guardian_ebid,
                "last_guardian_change": state.last_guardian_change,
                "last_guardian_ping": state.last_guardian_ping,
                "time_since_change_secs": now.saturating_sub(state.last_guardian_change),
                "time_since_ping_secs": now.saturating_sub(state.last_guardian_ping),
                "is_reachable": role.is_guardian_reachable(now),
            })
        } else {
            serde_json::json!({
                "current_guardian_ebid": null,
                "message": "No guardian elected yet",
            })
        }
    };

    Json(role_info)
}

/// GET /status/immortality
/// Returns overall immortality health snapshot
pub async fn get_immortality_status() -> impl IntoResponse {
    let local_ebid = {
        let mgr = crate::EBID_MANAGER.lock();
        mgr.get_ebid().to_string()
    };

    let guardian_ebid = {
        let role = crate::GUARDIAN_ROLE.lock();
        role.get_current_guardian()
    };

    let is_local_guardian = crate::guardian::is_local_guardian();

    let memory_peer_count = {
        let memory = crate::CONSTELLATION_MEMORY.lock();
        memory.peer_count()
    };

    let live_peer_count = crate::PEER_MANAGER.connected_validated_count().await;

    let has_known_peers = memory_peer_count > 0;

    // Determine overall immortality status
    let immortality_active = has_known_peers && (live_peer_count > 0 || memory_peer_count >= 3);

    Json(serde_json::json!({
        "immortality_active": immortality_active,
        "local_ebid": local_ebid,
        "has_known_peers": has_known_peers,
        "memory_peer_count": memory_peer_count,
        "live_peer_count": live_peer_count,
        "guardian": {
            "current_ebid": guardian_ebid,
            "is_local_guardian": is_local_guardian,
        },
        "features": {
            "constellation_memory": true,
            "peer_recovery": true,
            "guardian_rotation": true,
            "ebid_system": true,
        },
        "message": if immortality_active {
            "Constellation is self-healing. Guardian rotation ready. Immortality: ON."
        } else {
            "Building constellation memory. Connect to peers to activate immortality."
        },
    }))
}

/// GET /ebid
/// Returns local node's Eternal Broadcast ID
pub async fn get_local_ebid() -> impl IntoResponse {
    let mgr = crate::EBID_MANAGER.lock();
    let full = mgr.get_full();

    Json(serde_json::json!({
        "ebid": full.ebid,
        "created_at": full.created_at,
        "age_seconds": mgr.age_seconds(),
        "node_tag": full.node_tag,
    }))
}
