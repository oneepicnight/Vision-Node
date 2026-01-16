// Multi-Currency Wallet System
// Supports LAND (on-chain) + BTC, BCH, DOGE (off-chain)

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::market::engine::QuoteAsset;

/// User wallet with multi-currency support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWallet {
    pub user_id: String,

    // LAND balances (native on-chain asset)
    pub land_available: f64,
    pub land_locked: f64,

    // BTC balances (off-chain)
    pub btc_available: f64,
    pub btc_locked: f64,
    pub btc_deposit_address: String,

    // BCH balances (off-chain)
    pub bch_available: f64,
    pub bch_locked: f64,
    pub bch_deposit_address: String,

    // DOGE balances (off-chain)
    pub doge_available: f64,
    pub doge_locked: f64,
    pub doge_deposit_address: String,
}

impl UserWallet {
    pub fn new(user_id: String) -> Self {
        // Derive deterministic deposit addresses for off-chain assets
        let btc_addr = crate::market::deposits::deposit_address_for_user(&user_id, QuoteAsset::Btc)
            .unwrap_or_else(|_| format!("error_{}", user_id));
        let bch_addr = crate::market::deposits::deposit_address_for_user(&user_id, QuoteAsset::Bch)
            .unwrap_or_else(|_| format!("error_{}", user_id));
        let doge_addr =
            crate::market::deposits::deposit_address_for_user(&user_id, QuoteAsset::Doge)
                .unwrap_or_else(|_| format!("error_{}", user_id));

        Self {
            user_id: user_id.clone(),
            land_available: 0.0,
            land_locked: 0.0,
            btc_available: 0.0,
            btc_locked: 0.0,
            btc_deposit_address: btc_addr,
            bch_available: 0.0,
            bch_locked: 0.0,
            bch_deposit_address: bch_addr,
            doge_available: 0.0,
            doge_locked: 0.0,
            doge_deposit_address: doge_addr,
        }
    }

    /// Get available balance for a quote asset
    pub fn get_available(&self, asset: QuoteAsset) -> f64 {
        match asset {
            QuoteAsset::Land => self.land_available,
            QuoteAsset::Btc => self.btc_available,
            QuoteAsset::Bch => self.bch_available,
            QuoteAsset::Doge => self.doge_available,
        }
    }

    /// Get locked balance for a quote asset
    pub fn get_locked(&self, asset: QuoteAsset) -> f64 {
        match asset {
            QuoteAsset::Land => self.land_locked,
            QuoteAsset::Btc => self.btc_locked,
            QuoteAsset::Bch => self.bch_locked,
            QuoteAsset::Doge => self.doge_locked,
        }
    }

    /// Get total balance (available + locked)
    pub fn get_total(&self, asset: QuoteAsset) -> f64 {
        self.get_available(asset) + self.get_locked(asset)
    }

    /// Get deposit address for a quote asset
    pub fn get_deposit_address(&self, asset: QuoteAsset) -> Option<String> {
        match asset {
            QuoteAsset::Land => None, // LAND is native, no deposit address
            QuoteAsset::Btc => Some(self.btc_deposit_address.clone()),
            QuoteAsset::Bch => Some(self.bch_deposit_address.clone()),
            QuoteAsset::Doge => Some(self.doge_deposit_address.clone()),
        }
    }
}

/// Global wallet storage (user_id -> UserWallet)
pub static WALLETS: Lazy<Arc<Mutex<HashMap<String, UserWallet>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Get or create user wallet (auto-funds on testnet on first creation)
pub fn get_or_create_wallet(user_id: &str) -> Result<UserWallet> {
    let mut wallets = WALLETS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));

    Ok(wallet.clone())
}

/// Get available balance for user
pub fn get_quote_balance(user_id: &str, asset: QuoteAsset) -> f64 {
    WALLETS
        .lock()
        .ok()
        .and_then(|w| w.get(user_id).map(|wallet| wallet.get_available(asset)))
        .unwrap_or(0.0)
}

/// Ensure user has sufficient available balance
pub fn ensure_quote_available(user_id: &str, asset: QuoteAsset, required: f64) -> Result<()> {
    let available = get_quote_balance(user_id, asset);

    if available < required {
        return Err(anyhow::anyhow!(
            "Insufficient {} balance: {} < {}",
            asset.as_str(),
            available,
            required
        ));
    }

    Ok(())
}

/// Lock balance for an order
pub fn lock_quote_balance(user_id: &str, asset: QuoteAsset, amount: f64) -> Result<()> {
    let mut wallets = WALLETS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));

    let available = wallet.get_available(asset);
    if available < amount {
        return Err(anyhow::anyhow!(
            "Insufficient available balance to lock: {} < {}",
            available,
            amount
        ));
    }

    match asset {
        QuoteAsset::Land => {
            wallet.land_available -= amount;
            wallet.land_locked += amount;
        }
        QuoteAsset::Btc => {
            wallet.btc_available -= amount;
            wallet.btc_locked += amount;
        }
        QuoteAsset::Bch => {
            wallet.bch_available -= amount;
            wallet.bch_locked += amount;
        }
        QuoteAsset::Doge => {
            wallet.doge_available -= amount;
            wallet.doge_locked += amount;
        }
    }

    tracing::debug!(
        "🔒 Locked {} {} for user {}",
        amount,
        asset.as_str(),
        user_id
    );

    Ok(())
}

/// Lock base currency for sell orders (BTC/BCH/DOGE being sold)
pub fn lock_base_balance(user_id: &str, base_asset: QuoteAsset, amount: f64) -> Result<()> {
    // Base asset is what's being sold (BTC, BCH, DOGE)
    // Same as lock_quote_balance but semantically different
    lock_quote_balance(user_id, base_asset, amount)
}

/// Unlock base currency when sell order is cancelled
pub fn unlock_base_balance(user_id: &str, base_asset: QuoteAsset, amount: f64) -> Result<()> {
    unlock_quote_balance(user_id, base_asset, amount)
}

/// Unlock balance after order cancellation
pub fn unlock_quote_balance(user_id: &str, asset: QuoteAsset, amount: f64) -> Result<()> {
    let mut wallets = WALLETS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));

    match asset {
        QuoteAsset::Land => {
            wallet.land_locked -= amount;
            wallet.land_available += amount;
        }
        QuoteAsset::Btc => {
            wallet.btc_locked -= amount;
            wallet.btc_available += amount;
        }
        QuoteAsset::Bch => {
            wallet.bch_locked -= amount;
            wallet.bch_available += amount;
        }
        QuoteAsset::Doge => {
            wallet.doge_locked -= amount;
            wallet.doge_available += amount;
        }
    }

    tracing::debug!(
        "🔓 Unlocked {} {} for user {}",
        amount,
        asset.as_str(),
        user_id
    );

    Ok(())
}

/// Deduct from available balance (for fees, settlements)
pub fn deduct_quote(user_id: &str, asset: QuoteAsset, amount: f64) -> Result<()> {
    let mut wallets = WALLETS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));

    let available = wallet.get_available(asset);
    if available < amount {
        return Err(anyhow::anyhow!(
            "Insufficient balance to deduct: {} < {}",
            available,
            amount
        ));
    }

    match asset {
        QuoteAsset::Land => wallet.land_available -= amount,
        QuoteAsset::Btc => wallet.btc_available -= amount,
        QuoteAsset::Bch => wallet.bch_available -= amount,
        QuoteAsset::Doge => wallet.doge_available -= amount,
    }

    Ok(())
}

/// Credit available balance (for deposits, trade proceeds)
pub fn credit_quote(user_id: &str, asset: QuoteAsset, amount: f64) -> Result<()> {
    let mut wallets = WALLETS
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock wallets: {}", e))?;

    let wallet = wallets
        .entry(user_id.to_string())
        .or_insert_with(|| UserWallet::new(user_id.to_string()));

    match asset {
        QuoteAsset::Land => wallet.land_available += amount,
        QuoteAsset::Btc => wallet.btc_available += amount,
        QuoteAsset::Bch => wallet.bch_available += amount,
        QuoteAsset::Doge => wallet.doge_available += amount,
    }

    tracing::debug!(
        "💳 Credited {} {} to user {}",
        amount,
        asset.as_str(),
        user_id
    );

    // Send WebSocket notification for balance update
    let (available, locked) = match asset {
        QuoteAsset::Land => (wallet.land_available, wallet.land_locked),
        QuoteAsset::Btc => (wallet.btc_available, wallet.btc_locked),
        QuoteAsset::Bch => (wallet.bch_available, wallet.bch_locked),
        QuoteAsset::Doge => (wallet.doge_available, wallet.doge_locked),
    };

    crate::ws_notifications::WsNotificationManager::notify_balance_update(
        user_id, asset, available, locked,
    );

    Ok(())
}

/// Deposit event from external chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositEvent {
    pub user_id: String,
    pub asset: QuoteAsset,
    pub amount: f64,
    pub txid: String,
    pub confirmations: u32,
}

/// Process a deposit (credits user's available balance)
///
/// MAINNET: Enforces confirmation depth requirements before crediting
pub fn process_deposit(deposit: DepositEvent) -> Result<()> {
    // MAINNET: Check confirmation depth before crediting
    let coin = deposit.asset.as_str().to_uppercase();
    let required_confirmations = crate::vision_constants::required_confirmations(&coin);

    if deposit.confirmations < required_confirmations {
        tracing::debug!(
            "⏳ Deposit pending confirmations: {} {}/{} for user {} (txid: {})",
            deposit.amount,
            deposit.confirmations,
            required_confirmations,
            deposit.user_id,
            deposit.txid
        );
        return Err(anyhow!(
            "Insufficient confirmations: {}/{} (waiting)",
            deposit.confirmations,
            required_confirmations
        ));
    }

    credit_quote(&deposit.user_id, deposit.asset, deposit.amount)?;

    // Record in transaction history
    let wallet = get_or_create_wallet(&deposit.user_id)?;
    let to_address = match deposit.asset {
        QuoteAsset::Btc => wallet.btc_deposit_address.clone(),
        QuoteAsset::Bch => wallet.bch_deposit_address.clone(),
        QuoteAsset::Doge => wallet.doge_deposit_address.clone(),
        QuoteAsset::Land => String::from("N/A"),
    };

    let tx_record = crate::tx_history::TransactionRecord::new(
        &deposit.user_id,
        &deposit.txid,
        deposit.asset,
        crate::tx_history::TxType::Receive,
        deposit.amount,
        0.0,        // no fee for deposits
        "external", // from external blockchain
        &to_address,
    );

    if let Err(e) = crate::tx_history::TxHistoryManager::add_transaction(tx_record) {
        tracing::warn!("Failed to record deposit in tx history: {}", e);
    }

    tracing::info!(
        "💰 Deposit processed: {} {} for user {} (txid: {}, confirmations: {}/{})",
        deposit.amount,
        deposit.asset.as_str(),
        deposit.user_id,
        deposit.txid,
        deposit.confirmations,
        required_confirmations
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_creation() {
        let wallet = UserWallet::new("alice".to_string());
        assert_eq!(wallet.get_available(QuoteAsset::Btc), 0.0);
        assert!(wallet.btc_deposit_address.starts_with("btc_"));
    }

    #[test]
    fn test_lock_unlock_balance() {
        let user_id = "bob";
        credit_quote(user_id, QuoteAsset::Doge, 100.0).unwrap();

        lock_quote_balance(user_id, QuoteAsset::Doge, 30.0).unwrap();
        let available = get_quote_balance(user_id, QuoteAsset::Doge);
        assert_eq!(available, 70.0);

        unlock_quote_balance(user_id, QuoteAsset::Doge, 30.0).unwrap();
        let available = get_quote_balance(user_id, QuoteAsset::Doge);
        assert_eq!(available, 100.0);
    }
}
