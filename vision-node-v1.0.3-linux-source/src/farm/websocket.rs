use crate::farm::{
    FarmCommand, FarmRig, FarmState, RigHeartbeat, RigMessage, RigRegistration, RigUpdate,
};
use axum::{
    extract::{ws::Message, ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// WebSocket handler for farm rig connections
pub async fn farm_ws_handler(
    ws: WebSocketUpgrade,
    State(farm_state): State<Arc<RwLock<FarmState>>>,
) -> Response {
    ws.on_upgrade(|socket| handle_farm_websocket(socket, farm_state))
}

/// Handle individual WebSocket connection from a farm rig
async fn handle_farm_websocket(mut socket: WebSocket, farm_state: Arc<RwLock<FarmState>>) {
    // Wait for registration message
    let registration = match socket.recv().await {
        Some(Ok(msg)) => {
            if let axum::extract::ws::Message::Text(text) = msg {
                match serde_json::from_str::<RigMessage>(&text) {
                    Ok(msg) if msg.msg_type == "register" => {
                        match serde_json::from_value::<RigRegistration>(msg.data) {
                            Ok(reg) => reg,
                            Err(e) => {
                                error!("Invalid registration format: {}", e);
                                let _ = socket
                                    .send(Message::Text(
                                        serde_json::json!({
                                            "type": "error",
                                            "message": format!("Invalid registration: {}", e)
                                        })
                                        .to_string(),
                                    ))
                                    .await;
                                return;
                            }
                        }
                    }
                    _ => {
                        error!("First message must be registration");
                        let _ = socket
                            .send(Message::Text(
                                serde_json::json!({
                                    "type": "error",
                                    "message": "First message must be registration"
                                })
                                .to_string(),
                            ))
                            .await;
                        return;
                    }
                }
            } else {
                error!("Expected text message for registration");
                return;
            }
        }
        Some(Err(e)) => {
            error!("WebSocket error during registration: {}", e);
            return;
        }
        None => {
            warn!("Connection closed before registration");
            return;
        }
    };

    let rig_id = registration.rig_id.clone();
    info!("🚜 Farm rig registered: {} ({})", registration.name, rig_id);

    // Create command channel
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<FarmCommand>(32);

    // Register rig
    let rig = FarmRig {
        rig_id: rig_id.clone(),
        name: registration.name,
        os: registration.os,
        cpu_threads: registration.cpu_threads,
        status: "online".to_string(),
        hashrate: 0.0,
        last_heartbeat: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        profile: None,
        endpoint_mode: None,
    };

    {
        let mut state = farm_state.write();
        state.register_rig(rig, cmd_tx);
    }

    // Send registration success
    let _ = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "registered",
                "rig_id": rig_id,
                "message": "Successfully registered with farm controller"
            })
            .to_string(),
        ))
        .await;

    // Handle messages
    loop {
        tokio::select! {
            // Incoming message from rig
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_rig_message(&farm_state, &rig_id, &text).await {
                            error!("Error handling message from {}: {}", rig_id, e);
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("🚜 Rig {} disconnected", rig_id);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error for rig {}: {}", rig_id, e);
                        break;
                    }
                    _ => {}
                }
            }

            // Outgoing command to rig
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(command) => {
                        let json = match serde_json::to_string(&command) {
                            Ok(j) => j,
                            Err(e) => {
                                error!("Failed to serialize command: {}", e);
                                continue;
                            }
                        };

                        if let Err(e) = socket.send(Message::Text(json)).await {
                            error!("Failed to send command to rig {}: {}", rig_id, e);
                            break;
                        }
                    }
                    None => {
                        warn!("Command channel closed for rig {}", rig_id);
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: remove rig from state
    {
        let mut state = farm_state.write();
        state.remove_rig(&rig_id);
    }

    info!("🚜 Rig {} removed from farm", rig_id);
}

/// Handle incoming message from a rig
async fn handle_rig_message(
    farm_state: &Arc<RwLock<FarmState>>,
    rig_id: &str,
    text: &str,
) -> Result<(), String> {
    let msg: RigMessage = serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {}", e))?;

    match msg.msg_type.as_str() {
        "heartbeat" => {
            let heartbeat: RigHeartbeat = serde_json::from_value(msg.data)
                .map_err(|e| format!("Invalid heartbeat: {}", e))?;

            let update = RigUpdate {
                status: Some(heartbeat.status),
                hashrate: Some(heartbeat.hashrate),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            let mut state = farm_state.write();
            state.update_rig(rig_id, update);
        }
        _ => {
            warn!("Unknown message type from rig {}: {}", rig_id, msg.msg_type);
        }
    }

    Ok(())
}
