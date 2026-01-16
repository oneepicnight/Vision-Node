// Guardian Event Webhook Integration
// Sends events to Vision Bot for Discord ceremony automation

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, warn};

/// Event types for Guardian -> Vision Bot communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum GuardianEvent {
    #[serde(rename = "first_block")]
    FirstBlock {
        discord_user_id: Option<String>,
        height: u64,
        hash: String,
        miner_address: String,
    },
    #[serde(rename = "node_status")]
    NodeStatus {
        discord_user_id: Option<String>,
        status: String, // "online" | "offline"
        miner_address: String,
    },
    #[serde(rename = "link_wallet_discord")]
    LinkWalletDiscord {
        wallet_address: String,
        discord_user_id: String,
        discord_username: String,
    },
    #[serde(rename = "guardian_core_status")]
    GuardianCoreStatus {
        status: String, // "online" | "offline"
        miner_address: String,
        discord_user_id: Option<String>,
    },
}

/// Send Guardian event to Vision Bot webhook
///
/// Set VISION_BOT_WEBHOOK_URL environment variable to enable.
/// Example: http://vision-bot-host:3000/guardian
///
/// Silently ignores errors - bot integration is optional.
pub async fn send_guardian_event(event: GuardianEvent) {
    if let Ok(url) = env::var("VISION_BOT_WEBHOOK_URL") {
        let client = Client::new();

        match client
            .post(&url)
            .json(&event)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    info!("[GUARDIAN EVENT] Sent {:?} to Vision Bot", event);
                } else {
                    warn!(
                        "[GUARDIAN EVENT] Vision Bot returned {}: {:?}",
                        response.status(),
                        event
                    );
                }
            }
            Err(e) => {
                warn!(
                    "[GUARDIAN EVENT] Failed to send to Vision Bot: {} - {:?}",
                    e, event
                );
            }
        }
    }
    // Silently skip if VISION_BOT_WEBHOOK_URL not set
}

/// Check if wallet address or Discord ID belongs to Guardian owner
fn is_guardian_owner_node(wallet_address: &str, discord_user_id: Option<&String>) -> bool {
    // Check wallet address
    if let Ok(owner_wallet) = env::var("GUARDIAN_OWNER_WALLET_ADDRESS") {
        if wallet_address == owner_wallet {
            return true;
        }
    }

    // Check Discord ID
    if let Some(discord_id) = discord_user_id {
        if let Ok(owner_discord) = env::var("GUARDIAN_OWNER_DISCORD_ID") {
            if discord_id == &owner_discord {
                return true;
            }
        }
    }

    false
}

/// Helper: Send first block event
pub async fn notify_first_block(
    discord_user_id: Option<String>,
    height: u64,
    hash: String,
    miner_address: String,
) {
    let event = GuardianEvent::FirstBlock {
        discord_user_id,
        height,
        hash,
        miner_address,
    };
    send_guardian_event(event).await;
}

/// Helper: Send node status change event
pub async fn notify_node_status(
    discord_user_id: Option<String>,
    status: String,
    miner_address: String,
) {
    // Check if this is the Guardian owner's node
    let is_guardian_core = is_guardian_owner_node(&miner_address, discord_user_id.as_ref());

    if is_guardian_core {
        // Special logging for Guardian core node
        if status == "online" {
            info!("🛡️  [GUARDIAN CORE] Guardian core node ONLINE – Donnie is watching.");
        } else {
            warn!("⚠️  [GUARDIAN CORE] Guardian core node OFFLINE – Guardian temporarily blind.");
        }

        // Send special Guardian core status event
        let core_event = GuardianEvent::GuardianCoreStatus {
            status: status.clone(),
            miner_address: miner_address.clone(),
            discord_user_id: discord_user_id.clone(),
        };
        send_guardian_event(core_event).await;
    }

    // Also send regular node_status event
    let event = GuardianEvent::NodeStatus {
        discord_user_id,
        status,
        miner_address,
    };
    send_guardian_event(event).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_first_block_event() {
        env::set_var("VISION_BOT_WEBHOOK_URL", "http://localhost:3000/guardian");

        notify_first_block(
            Some("123456789012345678".to_string()),
            1,
            "0xabc123...".to_string(),
            "vision1...".to_string(),
        )
        .await;

        env::remove_var("VISION_BOT_WEBHOOK_URL");
    }

    #[tokio::test]
    async fn test_node_status_event() {
        env::set_var("VISION_BOT_WEBHOOK_URL", "http://localhost:3000/guardian");

        notify_node_status(
            Some("123456789012345678".to_string()),
            "online".to_string(),
            "vision1...".to_string(),
        )
        .await;

        env::remove_var("VISION_BOT_WEBHOOK_URL");
    }
}
