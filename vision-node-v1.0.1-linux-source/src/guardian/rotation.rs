//! Guardian Rotation Loop
//!
//! Monitors guardian health and automatically elects a new guardian
//! when the current one becomes unreachable.

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

/// Spawn the guardian rotation monitoring loop
///
/// This background task checks if the current guardian is still reachable,
/// and if not, elects a new guardian from available candidates.
pub async fn spawn_guardian_rotation_loop(check_interval_secs: u64) {
    info!(
        target: "vision_node::guardian::rotation",
        "[GUARDIAN_ROTATION] Starting rotation monitoring loop (interval: {}s)",
        check_interval_secs
    );

    loop {
        sleep(Duration::from_secs(check_interval_secs)).await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if rotation is needed
        let should_rotate = {
            let role = crate::GUARDIAN_ROLE.lock();
            role.should_rotate(now)
        };

        if should_rotate {
            warn!(
                target: "vision_node::guardian::rotation",
                "[GUARDIAN_ROTATION] Current guardian unreachable. Initiating election..."
            );

            // Get top candidates from constellation memory
            let candidates = {
                let memory = crate::CONSTELLATION_MEMORY.lock();
                memory.get_top_guardian_candidates(5)
            };

            if candidates.is_empty() {
                warn!(
                    target: "vision_node::guardian::rotation",
                    "[GUARDIAN_ROTATION] No guardian candidates available. Network running without guardian."
                );
                continue;
            }

            // Select best candidate (highest uptime score + most recent)
            let best_candidate = &candidates[0];

            info!(
                target: "vision_node::guardian::rotation",
                "[GUARDIAN_ROTATION] 👑 Electing new guardian: EBID {} (uptime: {:.2})",
                best_candidate.ebid,
                best_candidate.uptime_score
            );

            // Set new guardian
            {
                let mut role = crate::GUARDIAN_ROLE.lock();
                if let Err(e) = role.set_current_guardian(best_candidate.ebid.clone()) {
                    warn!(
                        target: "vision_node::guardian::rotation",
                        "[GUARDIAN_ROTATION] Failed to set guardian: {}",
                        e
                    );
                }
            }

            // Check if local node is the new guardian
            let local_ebid = {
                let mgr = crate::EBID_MANAGER.lock();
                mgr.get_ebid().to_string()
            };

            if local_ebid == best_candidate.ebid {
                info!(
                    target: "vision_node::guardian::rotation",
                    "[GUARDIAN_ROTATION] 🌟 This node has been elected as guardian by constellation consensus!"
                );

                // TODO: Activate guardian mode features
                // - Start beacon endpoint
                // - Enable peer registry
                // - Activate mood broadcasting
            }
        } else {
            // Guardian is healthy, ping it
            let guardian_ebid = {
                let role = crate::GUARDIAN_ROLE.lock();
                role.get_current_guardian()
            };

            if let Some(ebid) = guardian_ebid {
                // Check if guardian is in our peer list
                let guardian_connected = {
                    let memory = crate::CONSTELLATION_MEMORY.lock();
                    memory.get_by_ebid(&ebid).is_some()
                };

                if guardian_connected {
                    // Update ping time
                    let mut role = crate::GUARDIAN_ROLE.lock();
                    if let Err(e) = role.ping_guardian() {
                        warn!(
                            target: "vision_node::guardian::rotation",
                            "[GUARDIAN_ROTATION] Failed to ping guardian: {}",
                            e
                        );
                    }
                }
            }
        }
    }
}

/// Check if the local node is currently the guardian
pub fn is_local_guardian() -> bool {
    let local_ebid = {
        let mgr = crate::EBID_MANAGER.lock();
        mgr.get_ebid().to_string()
    };

    let current_guardian = {
        let role = crate::GUARDIAN_ROLE.lock();
        role.get_current_guardian()
    };

    if let Some(guardian_ebid) = current_guardian {
        guardian_ebid == local_ebid
    } else {
        // Fallback to environment variable if no guardian elected
        std::env::var("VISION_GUARDIAN_MODE")
            .unwrap_or_default()
            .to_lowercase()
            == "true"
    }
}
