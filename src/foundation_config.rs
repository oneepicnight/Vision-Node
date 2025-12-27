//! Canonical Foundation Configuration
//! Single source of truth for vault/fund/founder addresses and distribution percentages.
//!
//! Replaces the fragmented system of:
//! - config/foundation.rs (placeholder garbage)
//! - vision_constants.rs (hardcoded addresses)
//! - accounts.rs TokenAccountsCfg (loaded from TOML)
//!
//! This module loads TokenAccountsCfg once and exposes it globally.
#![allow(dead_code)]

use crate::accounts::TokenAccountsCfg;
use crate::market::address_validate::{validate_address, Asset};
use anyhow::Result;
use once_cell::sync::Lazy;

/// Global canonical foundation configuration
/// Loaded once from TOKEN_ACCOUNTS_TOML_PATH or defaults
pub static FOUNDATION_CONFIG: Lazy<Result<TokenAccountsCfg>> = Lazy::new(|| {
    let path = std::env::var("TOKEN_ACCOUNTS_TOML_PATH")
        .unwrap_or_else(|_| "config/token_accounts.toml".to_string());

    let mut cfg = crate::accounts::load_token_accounts(&path).or_else(|e| {
        tracing::warn!("Failed to load token accounts from {}: {}", path, e);
        // Return default if file missing (for tests/dev)
        Ok::<TokenAccountsCfg, anyhow::Error>(TokenAccountsCfg {
            vault_address: "vault_default_addr".to_string(),
            fund_address: "fund_default_addr".to_string(),
            founder1_address: "founder1_default_addr".to_string(),
            founder2_address: "founder2_default_addr".to_string(),
            vault_pct: 50,
            fund_pct: 30,
            treasury_pct: 20,
            founder1_pct: 50,
            founder2_pct: 50,
            miners_btc_address: None,
            miners_bch_address: None,
            miners_doge_address: None,
        })
    })?;

    // Load miners vault addresses from environment, overriding TOML if present
    if let Ok(btc_addr) = std::env::var("VISION_MINERS_BTC_ADDRESS") {
        cfg.miners_btc_address = Some(btc_addr);
    }
    if let Ok(bch_addr) = std::env::var("VISION_MINERS_BCH_ADDRESS") {
        cfg.miners_bch_address = Some(bch_addr);
    }
    if let Ok(doge_addr) = std::env::var("VISION_MINERS_DOGE_ADDRESS") {
        cfg.miners_doge_address = Some(doge_addr);
    }

    Ok(cfg)
});

/// Get vault address from canonical config
pub fn vault_address() -> String {
    FOUNDATION_CONFIG
        .as_ref()
        .map(|c| c.vault_address.clone())
        .unwrap_or_else(|_| "unknown_vault".to_string())
}

/// Get fund address from canonical config
pub fn fund_address() -> String {
    FOUNDATION_CONFIG
        .as_ref()
        .map(|c| c.fund_address.clone())
        .unwrap_or_else(|_| "unknown_fund".to_string())
}

/// Get founder1 address from canonical config
pub fn founder1_address() -> String {
    FOUNDATION_CONFIG
        .as_ref()
        .map(|c| c.founder1_address.clone())
        .unwrap_or_else(|_| "unknown_founder1".to_string())
}

/// Get founder2 address from canonical config
pub fn founder2_address() -> String {
    FOUNDATION_CONFIG
        .as_ref()
        .map(|c| c.founder2_address.clone())
        .unwrap_or_else(|_| "unknown_founder2".to_string())
}

/// Get miners BTC vault deposit address
pub fn miners_btc_address() -> Option<String> {
    FOUNDATION_CONFIG
        .as_ref()
        .ok()
        .and_then(|c| c.miners_btc_address.clone())
}

/// Get miners BCH vault deposit address
pub fn miners_bch_address() -> Option<String> {
    FOUNDATION_CONFIG
        .as_ref()
        .ok()
        .and_then(|c| c.miners_bch_address.clone())
}

/// Get miners DOGE vault deposit address
pub fn miners_doge_address() -> Option<String> {
    FOUNDATION_CONFIG
        .as_ref()
        .ok()
        .and_then(|c| c.miners_doge_address.clone())
}

/// Validate miners addresses are properly formatted
pub fn validate_miners_addresses() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if let Some(addr) = miners_btc_address() {
        if let Err(e) = validate_address(Asset::BTC, &addr) {
            errors.push(format!("Invalid BTC miners address: {}", e));
        }
    }

    if let Some(addr) = miners_bch_address() {
        if let Err(e) = validate_address(Asset::BCH, &addr) {
            errors.push(format!("Invalid BCH miners address: {}", e));
        }
    }

    if let Some(addr) = miners_doge_address() {
        if let Err(e) = validate_address(Asset::DOGE, &addr) {
            errors.push(format!("Invalid DOGE miners address: {}", e));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Get full config
pub fn config() -> Result<TokenAccountsCfg> {
    FOUNDATION_CONFIG
        .as_ref()
        .map(|c| c.clone())
        .map_err(|e| anyhow::anyhow!("{}", e))
}
