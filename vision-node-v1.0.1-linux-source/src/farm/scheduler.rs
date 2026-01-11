use crate::farm::profile::{FarmSchedule, RigConfig};
use crate::farm::{FarmCommand, FarmState};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

/// Background task that enforces time-based schedules for farm rigs
pub async fn farm_scheduler_task(farm_state: Arc<RwLock<FarmState>>, db: sled::Db) {
    info!("🕐 Farm scheduler started");
    let mut interval = time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        // Get all rig IDs
        let rig_ids: Vec<String> = {
            let state = farm_state.read();
            state.rigs.keys().cloned().collect()
        };

        for rig_id in rig_ids {
            // Load rig config from sled
            let config_key = format!("farm_rig_config/{}", rig_id);
            let config: Option<RigConfig> = match db.get(&config_key) {
                Ok(Some(bytes)) => match serde_json::from_slice::<RigConfig>(&bytes) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        warn!("Failed to parse config for rig {}: {}", rig_id, e);
                        None
                    }
                },
                Ok(None) => None,
                Err(e) => {
                    warn!("Failed to load config for rig {}: {}", rig_id, e);
                    None
                }
            };

            if let Some(config) = config {
                if let Some(schedule) = config.schedule {
                    process_schedule(&farm_state, &rig_id, &schedule).await;
                }
            }
        }
    }
}

/// Process schedule for a single rig
async fn process_schedule(
    farm_state: &Arc<RwLock<FarmState>>,
    rig_id: &str,
    schedule: &FarmSchedule,
) {
    if !schedule.enabled {
        return;
    }

    let is_active = schedule.is_active_now();

    // Get current rig status
    let current_status = {
        let state = farm_state.read();
        state.get_rig(rig_id).map(|r| r.status.clone())
    };

    let Some(status) = current_status else {
        return;
    };

    if is_active && status != "mining" {
        // Schedule says mining should be active, but rig is not mining
        info!("⏰ Schedule activated for rig {}: starting mining", rig_id);

        // First apply profile, then start mining
        let profile = schedule.profile.clone();

        // Get sender without holding lock across await
        let cmd_sender = farm_state.read().commands.get(rig_id).cloned();
        if let Some(tx) = cmd_sender {
            if let Err(e) = tx.send(FarmCommand::ApplyProfile { config: profile }).await {
                error!("Failed to apply profile for rig {}: {}", rig_id, e);
                return;
            }

            // Give a brief delay for profile to apply
            tokio::time::sleep(Duration::from_millis(500)).await;

            if let Err(e) = tx.send(FarmCommand::StartMining).await {
                error!("Failed to start mining for rig {}: {}", rig_id, e);
            }
        }
    } else if !is_active && status == "mining" {
        // Schedule says mining should be stopped, but rig is mining
        info!(
            "⏰ Schedule deactivated for rig {}: stopping mining",
            rig_id
        );

        // Get sender without holding lock across await
        let cmd_sender = farm_state.read().commands.get(rig_id).cloned();
        if let Some(tx) = cmd_sender {
            if let Err(e) = tx.send(FarmCommand::StopMining).await {
                error!("Failed to stop mining for rig {}: {}", rig_id, e);
            }
        }
    }
}

/// Background task that detects offline rigs and errors
pub async fn farm_health_monitor_task(farm_state: Arc<RwLock<FarmState>>, db: sled::Db) {
    info!("🏥 Farm health monitor started");
    let mut interval = time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Get all rigs
        let rigs: Vec<(String, String, f64, u64)> = {
            let state = farm_state.read();
            state
                .rigs
                .values()
                .map(|r| {
                    (
                        r.rig_id.clone(),
                        r.status.clone(),
                        r.hashrate,
                        r.last_heartbeat,
                    )
                })
                .collect()
        };

        for (rig_id, status, hashrate, last_heartbeat) in rigs {
            // Check if rig is offline (no heartbeat in 60 seconds)
            if now - last_heartbeat > 60 && status != "offline" {
                warn!(
                    "⚠️  Rig {} is offline (no heartbeat for {}s)",
                    rig_id,
                    now - last_heartbeat
                );

                let mut state = farm_state.write();
                if let Some(rig) = state.rigs.get_mut(&rig_id) {
                    rig.status = "offline".to_string();
                    rig.hashrate = 0.0;
                }
            }

            // Check for low hashrate (error detection)
            if status == "mining" {
                let config_key = format!("farm_rig_config/{}", rig_id);
                let config: Option<RigConfig> = match db.get(&config_key) {
                    Ok(Some(bytes)) => serde_json::from_slice(&bytes).ok(),
                    _ => None,
                };

                if let Some(config) = config {
                    if let Some(threshold) = config.min_hashrate_threshold {
                        if hashrate < threshold {
                            warn!(
                                "⚠️  Rig {} hashrate below threshold: {:.2} < {:.2}",
                                rig_id, hashrate, threshold
                            );

                            // Mark as error
                            let mut state = farm_state.write();
                            if let Some(rig) = state.rigs.get_mut(&rig_id) {
                                rig.status = "error".to_string();
                            }
                            drop(state);

                            // Auto-restart if enabled
                            if config.auto_restart_on_error {
                                info!("🔄 Auto-restarting rig {} due to low hashrate", rig_id);

                                // Get command sender before any awaits (don't hold lock across await)
                                let cmd_sender = {
                                    let state = farm_state.read();
                                    state.commands.get(&rig_id).cloned()
                                };

                                if let Some(tx) = cmd_sender {
                                    let rig_id_owned = rig_id.clone();

                                    // Spawn a task to handle restart (don't block the health monitor)
                                    tokio::spawn(async move {
                                        // Stop mining
                                        if let Err(e) = tx.send(FarmCommand::StopMining).await {
                                            error!("Failed to stop rig {}: {}", rig_id_owned, e);
                                            return;
                                        }

                                        // Wait a bit
                                        tokio::time::sleep(Duration::from_secs(2)).await;

                                        // Restart mining
                                        if let Err(e) = tx.send(FarmCommand::StartMining).await {
                                            error!("Failed to restart rig {}: {}", rig_id_owned, e);
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_logic() {
        // Basic smoke test
        let schedule = FarmSchedule::default();
        let _is_active = schedule.is_active_now();
    }
}
