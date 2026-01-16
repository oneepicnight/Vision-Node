// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

//! Atomic Swap Hardening Module
//!
//! This module provides production-grade safety mechanisms for atomic swaps:
//! - Confirmation depth enforcement (per-coin finality profiles)
//! - Swap timeouts with trustless refund paths
//! - Watch-only mode detection and honest UX
//! - Swap state machine for tracking lifecycle
//! - SHA256 hash lock primitives (cross-chain compatible)

pub mod confirmations;
pub mod hashlock;
pub mod timeouts;
pub mod watch_only;

// Re-exports for main.rs usage (marked to avoid unused warnings in release builds)
#[allow(unused_imports)]
pub use confirmations::{confirmations_met, required_confirmations};
#[allow(unused_imports)]
pub use timeouts::{calculate_refund_height, swap_timeout_blocks, swap_timeout_seconds};
#[allow(unused_imports)]
pub use watch_only::{require_signing_capability, WalletMode, WalletModeStatus};

// Hash lock primitives (SHA256 for cross-chain atomic swaps)
#[allow(unused_imports)]
pub use hashlock::{htlc_hash_lock, htlc_hash_lock_hex, verify_hash_lock, verify_hash_lock_hex};

use serde::{Deserialize, Serialize};

/// MAINNET: Swap lifecycle state machine
///
/// State progression:
/// Created → Funded → Confirmed → Claimable → Claimed (success path)
///                                          ↓
///                                      Refunding → Refunded (timeout path)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapState {
    /// Swap initiated but no funds deposited yet
    Created,
    /// Funds detected on-chain but not yet confirmed
    Funded,
    /// Deposit confirmed according to coin's confirmation requirements
    Confirmed,
    /// Ready to claim (counterparty can claim the secret)
    Claimable,
    /// Successfully claimed
    Claimed,
    /// Refund initiated (swap expired)
    Refunding,
    /// Refund completed
    Refunded,
}

impl SwapState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SwapState::Created => "created",
            SwapState::Funded => "funded",
            SwapState::Confirmed => "confirmed",
            SwapState::Claimable => "claimable",
            SwapState::Claimed => "claimed",
            SwapState::Refunding => "refunding",
            SwapState::Refunded => "refunded",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, SwapState::Claimed | SwapState::Refunded)
    }

    pub fn can_claim(&self) -> bool {
        matches!(self, SwapState::Claimable)
    }

    pub fn can_refund(&self) -> bool {
        matches!(self, SwapState::Confirmed | SwapState::Claimable)
    }
}

/// Swap error types
#[derive(Debug, Clone)]
pub enum SwapError {
    InsufficientConfirmations {
        observed: u32,
        required: u32,
        coin: String,
    },
    SwapNotExpired {
        current_height: u64,
        refund_height: u64,
    },
    WatchOnlyNode {
        operation: String,
    },
    AlreadyCompleted,
    InvalidState,
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::InsufficientConfirmations {
                observed,
                required,
                coin,
            } => {
                write!(
                    f,
                    "Insufficient confirmations for {}: {}/{} (waiting)",
                    coin, observed, required
                )
            }
            SwapError::SwapNotExpired {
                current_height,
                refund_height,
            } => {
                let blocks_remaining = refund_height.saturating_sub(*current_height);
                write!(
                    f,
                    "Swap has not expired yet: {} blocks remaining",
                    blocks_remaining
                )
            }
            SwapError::WatchOnlyNode { operation } => {
                write!(
                    f,
                    "WATCH_ONLY_NODE: {} disabled - import seed to enable signing",
                    operation
                )
            }
            SwapError::AlreadyCompleted => {
                write!(f, "Swap already completed")
            }
            SwapError::InvalidState => {
                write!(f, "Invalid swap state")
            }
        }
    }
}

impl std::error::Error for SwapError {}

/// Verify swap can progress based on confirmations
pub fn verify_confirmations(coin: &str, observed: u32) -> Result<(), SwapError> {
    if confirmations_met(coin, observed) {
        Ok(())
    } else {
        Err(SwapError::InsufficientConfirmations {
            observed,
            required: required_confirmations(coin),
            coin: coin.to_string(),
        })
    }
}

/// Verify refund is allowed based on timeout
pub fn verify_refund_allowed(
    current_height: u64,
    refund_height: u64,
    completed: bool,
) -> Result<(), SwapError> {
    if completed {
        return Err(SwapError::AlreadyCompleted);
    }

    if current_height >= refund_height {
        Ok(())
    } else {
        Err(SwapError::SwapNotExpired {
            current_height,
            refund_height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_confirmations() {
        assert!(verify_confirmations("BTC", 3).is_ok());
        assert!(verify_confirmations("BTC", 2).is_err());

        match verify_confirmations("BTC", 2) {
            Err(SwapError::InsufficientConfirmations {
                observed,
                required,
                coin,
            }) => {
                assert_eq!(observed, 2);
                assert_eq!(required, 3);
                assert_eq!(coin, "BTC");
            }
            _ => panic!("Expected InsufficientConfirmations error"),
        }
    }

    #[test]
    fn test_verify_refund_allowed() {
        // Refund available
        assert!(verify_refund_allowed(1150, 1144, false).is_ok());

        // Not yet expired
        assert!(verify_refund_allowed(1100, 1144, false).is_err());

        // Already completed
        assert!(verify_refund_allowed(1150, 1144, true).is_err());
    }

    #[test]
    fn test_error_messages() {
        let err = SwapError::InsufficientConfirmations {
            observed: 2,
            required: 3,
            coin: "BTC".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("2/3"));
        assert!(msg.contains("BTC"));

        let err = SwapError::SwapNotExpired {
            current_height: 1100,
            refund_height: 1144,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("44 blocks"));
    }
}
