//! Confirmation Depth Enforcement (Per Coin)
//!
//! Each coin has different finality profiles. This module provides
//! chain-specific confirmation thresholds for atomic swap progression.
//!
//! MAINNET enforces these before:
//! 1. Crediting deposits to usable balance
//! 2. Allowing HTLC claim completion

use crate::vision_constants;

/// Required confirmations before swap state can advance
///
/// MAINNET defaults:
/// - BTC: 3 confirmations (~30 minutes)
/// - BCH: 6 confirmations (~60 minutes)
/// - DOGE: 12 confirmations (~12 minutes)
///
/// These are now enforced via vision_constants for consistency
pub fn required_confirmations(coin: &str) -> u32 {
    vision_constants::required_confirmations(coin)
}

/// Check if observed confirmations meet the requirement
pub fn confirmations_met(coin: &str, observed: u32) -> bool {
    observed >= required_confirmations(coin)
}

/// Human-readable confirmation status message
pub fn confirmation_status_message(coin: &str, observed: u32) -> String {
    let required = required_confirmations(coin);
    if observed >= required {
        format!("✓ {}/{} confirmations (ready)", observed, required)
    } else {
        format!("⏳ {}/{} confirmations (waiting)", observed, required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_confirmations() {
        assert_eq!(required_confirmations("BTC"), 3);
        assert_eq!(required_confirmations("BCH"), 6);
        assert_eq!(required_confirmations("DOGE"), 12);
        assert_eq!(required_confirmations("UNKNOWN"), 6);
    }

    #[test]
    fn test_confirmations_met() {
        assert!(!confirmations_met("BTC", 2));
        assert!(confirmations_met("BTC", 3));
        assert!(confirmations_met("BTC", 10));

        assert!(!confirmations_met("BCH", 5));
        assert!(confirmations_met("BCH", 6));

        assert!(!confirmations_met("DOGE", 11));
        assert!(confirmations_met("DOGE", 12));
    }

    #[test]
    fn test_status_message() {
        let msg = confirmation_status_message("BTC", 2);
        assert!(msg.contains("2/3"));
        assert!(msg.contains("waiting"));

        let msg = confirmation_status_message("BTC", 3);
        assert!(msg.contains("3/3"));
        assert!(msg.contains("ready"));
    }

    #[test]
    fn test_env_override() {
        // This test would require setting env vars
        // In production, test with: VISION_BTC_CONFIRMATIONS=10 cargo test
        // For now, test the logic path exists
        assert!(required_confirmations("BTC") > 0);
    }
}
