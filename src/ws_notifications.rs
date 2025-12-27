//! WebSocket Notifications Module
//!
//! Real-time push notifications for wallet events:
//! - Transaction status updates
//! - Balance changes
//! - UTXO updates
//! - Confirmation updates
#![allow(dead_code)]

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::market::engine::QuoteAsset;
use crate::tx_history::{TxStatus, TxType};

/// WebSocket notification event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WalletEvent {
    /// Transaction status changed
    TransactionUpdate {
        txid: String,
        asset: String,
        status: TxStatus,
        confirmations: u32,
        block_height: Option<u64>,
    },
    /// New transaction detected
    NewTransaction {
        txid: String,
        asset: String,
        tx_type: TxType,
        amount: f64,
        from_address: String,
        to_address: String,
    },
    /// Balance changed
    BalanceUpdate {
        asset: String,
        available: f64,
        locked: f64,
        total: f64,
    },
    /// UTXO added
    UtxoAdded {
        txid: String,
        vout: u32,
        asset: String,
        amount: f64,
        confirmations: u32,
    },
    /// UTXO removed (spent)
    UtxoRemoved {
        txid: String,
        vout: u32,
        asset: String,
    },
    /// Confirmation update for transaction
    ConfirmationUpdate {
        txid: String,
        asset: String,
        confirmations: u32,
    },
    /// Governance proposal created
    GovernanceProposalCreated {
        proposal_id: u64,
        proposer: String,
        title: String,
        voting_ends_at: u64,
    },
    /// Governance proposal closed
    GovernanceProposalClosed {
        proposal_id: u64,
        status: String, // "passed", "rejected", "expired"
        yes_votes: u64,
        no_votes: u64,
    },
    /// Heartbeat/keepalive
    Ping { timestamp: i64 },
}

/// WebSocket notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub user_id: String,
    pub event: WalletEvent,
    pub timestamp: i64,
}

/// Broadcast channels per user
type UserChannels = Arc<Mutex<HashMap<String, broadcast::Sender<NotificationMessage>>>>;

static USER_CHANNELS: Lazy<UserChannels> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// WebSocket notification manager
pub struct WsNotificationManager;

impl WsNotificationManager {
    /// Get or create a broadcast channel for a user
    fn get_or_create_channel(user_id: &str) -> broadcast::Sender<NotificationMessage> {
        let mut channels = USER_CHANNELS.lock().unwrap();

        channels
            .entry(user_id.to_string())
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(100); // Buffer 100 messages
                tracing::debug!("Created notification channel for user: {}", user_id);
                tx
            })
            .clone()
    }

    /// Subscribe to notifications for a user
    pub fn subscribe(user_id: &str) -> broadcast::Receiver<NotificationMessage> {
        let sender = Self::get_or_create_channel(user_id);
        sender.subscribe()
    }

    /// Send a notification to a user
    pub fn notify(user_id: &str, event: WalletEvent) {
        let sender = Self::get_or_create_channel(user_id);

        let message = NotificationMessage {
            user_id: user_id.to_string(),
            event,
            timestamp: chrono::Utc::now().timestamp(),
        };

        match sender.send(message) {
            Ok(count) => {
                tracing::debug!(
                    "Sent notification to {} subscribers for user {}",
                    count,
                    user_id
                );
            }
            Err(_) => {
                tracing::debug!("No active subscribers for user {}", user_id);
            }
        }
    }

    /// Notify transaction status update
    pub fn notify_transaction_update(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        status: TxStatus,
        confirmations: u32,
        block_height: Option<u64>,
    ) {
        Self::notify(
            user_id,
            WalletEvent::TransactionUpdate {
                txid: txid.to_string(),
                asset: asset.as_str().to_string(),
                status,
                confirmations,
                block_height,
            },
        );
    }

    /// Notify new transaction
    pub fn notify_new_transaction(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        tx_type: TxType,
        amount: f64,
        from_address: &str,
        to_address: &str,
    ) {
        Self::notify(
            user_id,
            WalletEvent::NewTransaction {
                txid: txid.to_string(),
                asset: asset.as_str().to_string(),
                tx_type,
                amount,
                from_address: from_address.to_string(),
                to_address: to_address.to_string(),
            },
        );
    }

    /// Notify balance update
    pub fn notify_balance_update(user_id: &str, asset: QuoteAsset, available: f64, locked: f64) {
        Self::notify(
            user_id,
            WalletEvent::BalanceUpdate {
                asset: asset.as_str().to_string(),
                available,
                locked,
                total: available + locked,
            },
        );
    }

    /// Notify UTXO added
    pub fn notify_utxo_added(
        user_id: &str,
        txid: &str,
        vout: u32,
        asset: QuoteAsset,
        amount: f64,
        confirmations: u32,
    ) {
        Self::notify(
            user_id,
            WalletEvent::UtxoAdded {
                txid: txid.to_string(),
                vout,
                asset: asset.as_str().to_string(),
                amount,
                confirmations,
            },
        );
    }

    /// Notify UTXO removed
    pub fn notify_utxo_removed(user_id: &str, txid: &str, vout: u32, asset: QuoteAsset) {
        Self::notify(
            user_id,
            WalletEvent::UtxoRemoved {
                txid: txid.to_string(),
                vout,
                asset: asset.as_str().to_string(),
            },
        );
    }

    /// Notify confirmation update
    pub fn notify_confirmation_update(
        user_id: &str,
        txid: &str,
        asset: QuoteAsset,
        confirmations: u32,
    ) {
        Self::notify(
            user_id,
            WalletEvent::ConfirmationUpdate {
                txid: txid.to_string(),
                asset: asset.as_str().to_string(),
                confirmations,
            },
        );
    }

    /// Notify governance proposal created
    pub fn notify_governance_proposal_created(
        user_id: &str,
        proposal_id: u64,
        proposer: &str,
        title: &str,
        voting_ends_at: u64,
    ) {
        Self::notify(
            user_id,
            WalletEvent::GovernanceProposalCreated {
                proposal_id,
                proposer: proposer.to_string(),
                title: title.to_string(),
                voting_ends_at,
            },
        );
    }

    /// Notify governance proposal closed
    pub fn notify_governance_proposal_closed(
        user_id: &str,
        proposal_id: u64,
        status: &str,
        yes_votes: u64,
        no_votes: u64,
    ) {
        Self::notify(
            user_id,
            WalletEvent::GovernanceProposalClosed {
                proposal_id,
                status: status.to_string(),
                yes_votes,
                no_votes,
            },
        );
    }

    /// Get count of active subscribers for a user
    pub fn subscriber_count(user_id: &str) -> usize {
        let channels = USER_CHANNELS.lock().unwrap();
        channels
            .get(user_id)
            .map(|sender| sender.receiver_count())
            .unwrap_or(0)
    }

    /// Clean up channels with no subscribers
    pub fn cleanup_inactive_channels() {
        let mut channels = USER_CHANNELS.lock().unwrap();
        channels.retain(|user_id, sender| {
            let count = sender.receiver_count();
            if count == 0 {
                tracing::debug!("Removing inactive channel for user: {}", user_id);
                false
            } else {
                true
            }
        });
    }
}

/// WebSocket handler
pub async fn wallet_ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let user_id = params
        .get("user_id")
        .cloned()
        .unwrap_or_else(|| "anonymous".to_string());

    tracing::info!("WebSocket connection requested for user: {}", user_id);

    ws.on_upgrade(move |socket| handle_socket(socket, user_id))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, user_id: String) {
    tracing::info!("WebSocket connected: user={}", user_id);

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to notifications
    let mut rx = WsNotificationManager::subscribe(&user_id);

    // Spawn a task to forward notifications to WebSocket
    let user_id_clone = user_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(notification) = rx.recv().await {
            let json = match serde_json::to_string(&notification) {
                Ok(j) => j,
                Err(e) => {
                    tracing::error!("Failed to serialize notification: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(json)).await.is_err() {
                tracing::debug!("WebSocket send failed for user {}", user_id_clone);
                break;
            }
        }
    });

    // Spawn a task to handle incoming messages (ping/pong)
    let user_id_clone = user_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("Received text message from {}: {}", user_id_clone, text);

                    // Handle ping requests
                    if text.trim() == "ping" {
                        // Will be handled by sending pong via notification system
                    }
                }
                Message::Close(_) => {
                    tracing::info!("WebSocket close received from user {}", user_id_clone);
                    break;
                }
                Message::Ping(_) => {
                    // Axum handles pong automatically
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            tracing::debug!("Send task completed for user {}", user_id);
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            tracing::debug!("Receive task completed for user {}", user_id);
            send_task.abort();
        }
    }

    tracing::info!("WebSocket disconnected: user={}", user_id);
}

/// Background task to send periodic pings
pub async fn start_ping_task() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            // Get all active users
            let users: Vec<String> = {
                let channels = USER_CHANNELS.lock().unwrap();
                channels
                    .iter()
                    .filter(|(_, sender)| sender.receiver_count() > 0)
                    .map(|(user_id, _)| user_id.clone())
                    .collect()
            };

            // Send ping to each user
            for user_id in users {
                WsNotificationManager::notify(
                    &user_id,
                    WalletEvent::Ping {
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                );
            }

            // Cleanup inactive channels
            WsNotificationManager::cleanup_inactive_channels();
        }
    });

    tracing::info!("✅ WebSocket ping task started");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let sender = WsNotificationManager::get_or_create_channel("test_user");
        assert_eq!(sender.receiver_count(), 0);

        let _rx = sender.subscribe();
        assert_eq!(sender.receiver_count(), 1);
    }

    #[test]
    fn test_notification_serialization() {
        let event = WalletEvent::BalanceUpdate {
            asset: "btc".to_string(),
            available: 1.5,
            locked: 0.5,
            total: 2.0,
        };

        let msg = NotificationMessage {
            user_id: "alice".to_string(),
            event,
            timestamp: 1700000000,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("balance_update"));
        assert!(json.contains("btc"));
    }
}
