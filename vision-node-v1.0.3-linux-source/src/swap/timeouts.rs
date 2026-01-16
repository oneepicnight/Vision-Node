//! Swap Timeout + Refund Paths (Non-Custodial Safety Net)
//!
//! If the counterparty disappears, funds are recoverable by design,
//! not by admin intervention. This module provides HTLC timeout logic
//! and refund eligibility tracking.

use serde::{Deserialize, Serialize};

/// Timeout blocks for swap refund eligibility (per coin)
///
/// Defaults:
/// - BTC: 144 blocks (~24 hours at 10min/block)
/// - BCH: 72 blocks (~12 hours at 10min/block)
/// - DOGE: 720 blocks (~12 hours at 1min/block)
pub fn swap_timeout_blocks(coin: &str) -> u64 {
    match coin {
        "BTC" => 144,
        "BCH" => 72,
        "DOGE" => 720,
        _ => 144, // Safe default
    }
}

/// Convert timeout blocks to approximate seconds (for display)
pub fn swap_timeout_seconds(coin: &str) -> u64 {
    let blocks = swap_timeout_blocks(coin);
    match coin {
        "BTC" | "BCH" => blocks * 600, // 10 minutes per block
        "DOGE" => blocks * 60,         // 1 minute per block
        _ => blocks * 600,             // Default to BTC timing
    }
}

/// Check if swap can be refunded based on current height
pub fn can_refund(
    current_height: u64,
    _swap_initiated_height: u64,
    swap_refund_height: u64,
    swap_completed: bool,
) -> bool {
    !swap_completed && current_height >= swap_refund_height
}

/// Calculate refund height for new swap
pub fn calculate_refund_height(coin: &str, current_height: u64) -> u64 {
    current_height + swap_timeout_blocks(coin)
}

/// Refund eligibility status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundStatus {
    pub can_refund: bool,
    pub current_height: u64,
    pub refund_height: u64,
    pub blocks_remaining: i64,
    pub time_remaining_seconds: i64,
}

impl RefundStatus {
    pub fn new(coin: &str, current_height: u64, refund_height: u64, completed: bool) -> Self {
        let can_refund = can_refund(current_height, 0, refund_height, completed);
        let blocks_remaining = (refund_height as i64) - (current_height as i64);

        let time_remaining_seconds = if blocks_remaining > 0 {
            match coin {
                "BTC" | "BCH" => blocks_remaining * 600,
                "DOGE" => blocks_remaining * 60,
                _ => blocks_remaining * 600,
            }
        } else {
            0
        };

        Self {
            can_refund,
            current_height,
            refund_height,
            blocks_remaining,
            time_remaining_seconds,
        }
    }

    pub fn status_message(&self) -> String {
        if self.can_refund {
            format!(
                "✓ Refund available (expired {} blocks ago)",
                -self.blocks_remaining
            )
        } else {
            let hours = self.time_remaining_seconds / 3600;
            let minutes = (self.time_remaining_seconds % 3600) / 60;
            format!(
                "⏳ Refund in {} blocks (~{}h {}m)",
                self.blocks_remaining, hours, minutes
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_blocks() {
        assert_eq!(swap_timeout_blocks("BTC"), 144);
        assert_eq!(swap_timeout_blocks("BCH"), 72);
        assert_eq!(swap_timeout_blocks("DOGE"), 720);
        assert_eq!(swap_timeout_blocks("UNKNOWN"), 144);
    }

    #[test]
    fn test_timeout_seconds() {
        // BTC: 144 blocks * 600 seconds = 86400 seconds (24 hours)
        assert_eq!(swap_timeout_seconds("BTC"), 86400);
        // BCH: 72 blocks * 600 seconds = 43200 seconds (12 hours)
        assert_eq!(swap_timeout_seconds("BCH"), 43200);
        // DOGE: 720 blocks * 60 seconds = 43200 seconds (12 hours)
        assert_eq!(swap_timeout_seconds("DOGE"), 43200);
    }

    #[test]
    fn test_calculate_refund_height() {
        assert_eq!(calculate_refund_height("BTC", 1000), 1144);
        assert_eq!(calculate_refund_height("BCH", 1000), 1072);
        assert_eq!(calculate_refund_height("DOGE", 1000), 1720);
    }

    #[test]
    fn test_can_refund() {
        // Swap at height 1000, refund at height 1144, currently at 1100
        assert!(!can_refund(1100, 1000, 1144, false));

        // Now at height 1144
        assert!(can_refund(1144, 1000, 1144, false));

        // Now at height 1200
        assert!(can_refund(1200, 1000, 1144, false));

        // Completed swaps can never be refunded
        assert!(!can_refund(1200, 1000, 1144, true));
    }

    #[test]
    fn test_refund_status() {
        let status = RefundStatus::new("BTC", 1100, 1144, false);
        assert!(!status.can_refund);
        assert_eq!(status.blocks_remaining, 44);
        assert_eq!(status.time_remaining_seconds, 44 * 600); // 44 blocks * 10 minutes

        let status = RefundStatus::new("BTC", 1150, 1144, false);
        assert!(status.can_refund);
        assert_eq!(status.blocks_remaining, -6);
    }

    #[test]
    fn test_status_message() {
        let status = RefundStatus::new("BTC", 1100, 1144, false);
        let msg = status.status_message();
        assert!(msg.contains("44 blocks"));
        assert!(msg.contains("⏳"));

        let status = RefundStatus::new("BTC", 1150, 1144, false);
        let msg = status.status_message();
        assert!(msg.contains("✓ Refund available"));
    }
}
