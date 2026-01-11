// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

//! Watch-Only Mode Indicators (UX Truthfulness)
//!
//! Make it impossible for users to misunderstand their node's role.
//! Watch-only ≠ signing node.

use serde::{Deserialize, Serialize};

/// Check if node is in watch-only mode (no private keys available)
///
/// A node is watch-only if:
/// - external_master_seed.bin does not exist
/// - Node can observe swaps but cannot sign transactions
pub fn is_watch_only() -> bool {
    !crate::market::real_addresses::seed_file_path_public().exists()
}

/// Can this node sign transactions?
pub fn can_sign() -> bool {
    !is_watch_only()
}

/// Wallet mode enum for API responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletMode {
    Full,
    #[serde(rename = "watch-only")]
    WatchOnly,
}

impl WalletMode {
    pub fn current() -> Self {
        if is_watch_only() {
            WalletMode::WatchOnly
        } else {
            WalletMode::Full
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WalletMode::Full => "full",
            WalletMode::WatchOnly => "watch-only",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WalletMode::Full => "This node can sign transactions and initiate swaps.",
            WalletMode::WatchOnly => "This node can observe swaps but cannot sign transactions. Import a seed to enable signing.",
        }
    }
}

/// Wallet mode status for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletModeStatus {
    pub mode: WalletMode,
    pub can_sign: bool,
    pub message: String,
    pub capabilities: WalletCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletCapabilities {
    pub swap_initiation: bool,
    pub refund_signing: bool,
    pub key_export: bool,
    pub balance_viewing: bool,
    pub swap_monitoring: bool,
    pub confirmation_tracking: bool,
}

impl WalletModeStatus {
    pub fn current() -> Self {
        let mode = WalletMode::current();
        let can_sign = can_sign();

        let capabilities = WalletCapabilities {
            swap_initiation: can_sign,
            refund_signing: can_sign,
            key_export: can_sign,
            balance_viewing: true,
            swap_monitoring: true,
            confirmation_tracking: true,
        };

        Self {
            mode,
            can_sign,
            message: mode.description().to_string(),
            capabilities,
        }
    }
}

/// Error type for watch-only operations
#[derive(Debug, Clone)]
pub struct WatchOnlyError {
    pub operation: String,
}

impl WatchOnlyError {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
        }
    }

    pub fn message(&self) -> String {
        format!(
            "WATCH_ONLY_NODE: {} disabled - import seed to enable signing",
            self.operation
        )
    }
}

impl std::fmt::Display for WatchOnlyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for WatchOnlyError {}

/// Guard function for operations requiring signing capability
///
/// Usage:
/// ```
/// require_signing_capability("swap initiation")?;
/// ```
pub fn require_signing_capability(operation: &str) -> Result<(), WatchOnlyError> {
    if is_watch_only() {
        Err(WatchOnlyError::new(operation))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_mode_current() {
        let mode = WalletMode::current();
        // Test will pass regardless of actual mode
        assert!(mode == WalletMode::Full || mode == WalletMode::WatchOnly);
    }

    #[test]
    fn test_wallet_mode_str() {
        assert_eq!(WalletMode::Full.as_str(), "full");
        assert_eq!(WalletMode::WatchOnly.as_str(), "watch-only");
    }

    #[test]
    fn test_capabilities() {
        let status = WalletModeStatus::current();

        if status.can_sign {
            assert!(status.capabilities.swap_initiation);
            assert!(status.capabilities.refund_signing);
            assert!(status.capabilities.key_export);
        } else {
            assert!(!status.capabilities.swap_initiation);
            assert!(!status.capabilities.refund_signing);
            assert!(!status.capabilities.key_export);
        }

        // Always enabled
        assert!(status.capabilities.balance_viewing);
        assert!(status.capabilities.swap_monitoring);
        assert!(status.capabilities.confirmation_tracking);
    }

    #[test]
    fn test_watch_only_error() {
        let err = WatchOnlyError::new("swap initiation");
        let msg = err.message();
        assert!(msg.contains("WATCH_ONLY_NODE"));
        assert!(msg.contains("swap initiation"));
        assert!(msg.contains("disabled"));
    }

    #[test]
    fn test_require_signing_capability() {
        let result = require_signing_capability("test operation");

        if is_watch_only() {
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.operation, "test operation");
        } else {
            assert!(result.is_ok());
        }
    }
}
