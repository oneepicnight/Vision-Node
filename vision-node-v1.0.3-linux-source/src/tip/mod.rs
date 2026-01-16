//! Tip System - "Buy Me a Drink" Feature
//!
//! One-time $3 tip system that guilt-trips users until they tip.
//! Once tipped, the button disappears forever for that wallet.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sled::Db;
use std::fmt;

const TIP_STATE_TREE: &str = "tip_state";

/// Errors that can occur in the tip system
#[derive(Debug)]
pub enum TipError {
    AlreadyTipped,
    UnsupportedCoin(String),
    InsufficientBalance,
    DatabaseError(String),
    PriceError(String),
    TransferFailed(String),
}

impl fmt::Display for TipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TipError::AlreadyTipped => {
                write!(f, "You already tipped. One drink per wallet, cheapskate.")
            }
            TipError::UnsupportedCoin(coin) => write!(f, "Coin {} not supported for tips", coin),
            TipError::InsufficientBalance => {
                write!(f, "Not enough balance. You're broke, I get it.")
            }
            TipError::DatabaseError(e) => write!(f, "Database error: {}", e),
            TipError::PriceError(e) => write!(f, "Price oracle error: {}", e),
            TipError::TransferFailed(e) => write!(f, "Transfer failed: {}", e),
        }
    }
}

impl std::error::Error for TipError {}

/// State tracking whether a wallet has tipped
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipState {
    pub wallet_address: String,
    pub has_tipped: bool,
    pub last_tip_at: Option<u64>,
    pub coin: Option<String>,
    pub amount: Option<u128>, // in smallest units
}

impl TipState {
    /// Create a new untipped state for a wallet
    pub fn new(wallet_address: String) -> Self {
        Self {
            wallet_address,
            has_tipped: false,
            last_tip_at: None,
            coin: None,
            amount: None,
        }
    }

    /// Mark this wallet as having tipped
    pub fn tipped(&mut self, coin: &str, amount: u128, now: u64) {
        self.has_tipped = true;
        self.last_tip_at = Some(now);
        self.coin = Some(coin.to_string());
        self.amount = Some(amount);
    }
}

/// Tip configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipConfig {
    /// Address to receive tips
    pub tip_address: String,
    /// Allowed coins for tipping (BTC, BCH, DOGE, etc.)
    pub tip_allowed_coins: Vec<String>,
    /// USD amount for a tip (default $3)
    pub tip_usd_amount: f64,
}

impl Default for TipConfig {
    fn default() -> Self {
        Self {
            tip_address: std::env::var("VISION_TIP_ADDRESS")
                .unwrap_or_else(|_| "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string()),
            tip_allowed_coins: vec!["BTC".to_string(), "BCH".to_string(), "DOGE".to_string()],
            tip_usd_amount: 3.0,
        }
    }
}

/// Load tip state for a wallet
pub fn load_tip_state(db: &Db, wallet: &str) -> Result<TipState, TipError> {
    let tip_tree = db
        .open_tree(TIP_STATE_TREE)
        .map_err(|e| TipError::DatabaseError(e.to_string()))?;

    match tip_tree.get(wallet.as_bytes()) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes)
            .map_err(|e| TipError::DatabaseError(format!("Deserialization error: {}", e))),
        Ok(None) => Ok(TipState::new(wallet.to_string())),
        Err(e) => Err(TipError::DatabaseError(e.to_string())),
    }
}

/// Save tip state for a wallet
pub fn save_tip_state(db: &Db, state: &TipState) -> Result<(), TipError> {
    let tip_tree = db
        .open_tree(TIP_STATE_TREE)
        .map_err(|e| TipError::DatabaseError(e.to_string()))?;

    let bytes = serde_json::to_vec(state)
        .map_err(|e| TipError::DatabaseError(format!("Serialization error: {}", e)))?;

    tip_tree
        .insert(state.wallet_address.as_bytes(), bytes)
        .map_err(|e| TipError::DatabaseError(e.to_string()))?;

    tip_tree
        .flush()
        .map_err(|e| TipError::DatabaseError(e.to_string()))?;

    Ok(())
}

/// Convert USD amount to coin amount in smallest units (satoshis, etc.)
///
/// For now, uses hardcoded prices for testing.
/// In production, integrate with external price API.
pub fn usd_to_coin_amount(coin: &str, usd_amount: f64) -> Result<u128, TipError> {
    // Hardcoded prices for testing (update these or integrate with price oracle)
    let price_usd_per_coin = match coin {
        "BTC" => 95000.0, // $95k per BTC
        "BCH" => 450.0,   // $450 per BCH
        "DOGE" => 0.35,   // $0.35 per DOGE
        _ => return Err(TipError::UnsupportedCoin(coin.to_string())),
    };

    // Calculate raw coin amount
    let raw_amount = usd_amount / price_usd_per_coin;

    // Convert to smallest units
    let factor: u128 = match coin {
        "BTC" | "BCH" => 100_000_000, // 1 coin = 100M satoshis
        "DOGE" => 100_000_000,        // 1 DOGE = 100M koinus
        _ => return Err(TipError::UnsupportedCoin(coin.to_string())),
    };

    let smallest_units = (raw_amount * factor as f64).ceil() as u128;

    Ok(smallest_units)
}

/// Check if wallet has enough balance to tip
pub fn check_balance_for_tip(
    wallet_balance: u128,
    tip_amount: u128,
    coin: &str,
) -> Result<(), TipError> {
    // Minimum dust amounts per coin
    let min_balance = match coin {
        "BTC" => 1000u128,     // 1000 sats min
        "BCH" => 546u128,      // BCH dust limit
        "DOGE" => 100_000u128, // 0.001 DOGE min
        _ => 1000u128,
    };

    if wallet_balance < tip_amount + min_balance {
        return Err(TipError::InsufficientBalance);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usd_to_coin_amount() {
        // $3 in BTC at $95k = ~0.00003158 BTC = 3158 sats
        let btc_sats = usd_to_coin_amount("BTC", 3.0).unwrap();
        assert!(btc_sats > 3000 && btc_sats < 4000);

        // $3 in DOGE at $0.35 = ~8.57 DOGE = 857M koinus
        let doge_koinus = usd_to_coin_amount("DOGE", 3.0).unwrap();
        assert!(doge_koinus > 800_000_000 && doge_koinus < 900_000_000);
    }

    #[test]
    fn test_tip_state() {
        let mut state = TipState::new("0x123".to_string());
        assert!(!state.has_tipped);

        state.tipped("BTC", 5000, 1700000000);
        assert!(state.has_tipped);
        assert_eq!(state.coin, Some("BTC".to_string()));
        assert_eq!(state.amount, Some(5000));
    }
}
