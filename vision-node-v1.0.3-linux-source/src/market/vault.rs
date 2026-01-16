// Multi-Currency Vault for Exchange Fees
// Manages vault balances for LAND, BTC, BCH, DOGE with 50/30/20 split

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::market::engine::QuoteAsset;

/// Vault wallet for a single currency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultWallet {
    pub miners: f64,   // 50% - used for auto-buy
    pub dev: f64,      // 30% - held for development
    pub founders: f64, // 20% - held for founders
}

impl VaultWallet {
    pub fn new() -> Self {
        Self {
            miners: 0.0,
            dev: 0.0,
            founders: 0.0,
        }
    }

    pub fn total(&self) -> f64 {
        self.miners + self.dev + self.founders
    }
}

/// Multi-currency vault balances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultBalances {
    pub land: VaultWallet,
    pub btc: VaultWallet,
    pub bch: VaultWallet,
    pub doge: VaultWallet,
}

impl VaultBalances {
    pub fn new() -> Self {
        Self {
            land: VaultWallet::new(),
            btc: VaultWallet::new(),
            bch: VaultWallet::new(),
            doge: VaultWallet::new(),
        }
    }

    pub fn get_wallet(&self, asset: QuoteAsset) -> &VaultWallet {
        match asset {
            QuoteAsset::Land => &self.land,
            QuoteAsset::Btc => &self.btc,
            QuoteAsset::Bch => &self.bch,
            QuoteAsset::Doge => &self.doge,
        }
    }

    pub fn get_wallet_mut(&mut self, asset: QuoteAsset) -> &mut VaultWallet {
        match asset {
            QuoteAsset::Land => &mut self.land,
            QuoteAsset::Btc => &mut self.btc,
            QuoteAsset::Bch => &mut self.bch,
            QuoteAsset::Doge => &mut self.doge,
        }
    }
}

/// Global vault instance
pub static EXCHANGE_VAULT: Lazy<Arc<Mutex<VaultBalances>>> =
    Lazy::new(|| Arc::new(Mutex::new(VaultBalances::new())));

/// Distribute exchange fee using 50/30/20 split
/// - 50% to miners (used for auto-buy)
/// - 30% to dev fund
/// - 20% to founders
pub fn distribute_exchange_fee(quote: QuoteAsset, fee_amount: f64) -> Result<()> {
    let miners_cut = fee_amount * 0.50;
    let dev_cut = fee_amount * 0.30;
    let founders_cut = fee_amount * 0.20;

    let mut vault = EXCHANGE_VAULT
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock vault: {}", e))?;

    let wallet = vault.get_wallet_mut(quote);

    wallet.miners += miners_cut;
    wallet.dev += dev_cut;
    wallet.founders += founders_cut;

    tracing::debug!(
        "💰 Exchange fee distributed: {} {} -> miners={}, dev={}, founders={}",
        fee_amount,
        quote.as_str(),
        miners_cut,
        dev_cut,
        founders_cut
    );

    Ok(())
}

/// Get miners balance for a specific asset
pub fn get_miners_balance(quote: QuoteAsset) -> f64 {
    EXCHANGE_VAULT
        .lock()
        .map(|v| v.get_wallet(quote).miners)
        .unwrap_or(0.0)
}

/// Deduct from miners balance (used after auto-buy)
pub fn deduct_miners_balance(quote: QuoteAsset, amount: f64) -> Result<()> {
    let mut vault = EXCHANGE_VAULT
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock vault: {}", e))?;

    let wallet = vault.get_wallet_mut(quote);

    if wallet.miners < amount {
        return Err(anyhow::anyhow!(
            "Insufficient miners balance: {} < {}",
            wallet.miners,
            amount
        ));
    }

    wallet.miners -= amount;

    tracing::debug!(
        "💸 Deducted {} {} from miners vault (remaining: {})",
        amount,
        quote.as_str(),
        wallet.miners
    );

    Ok(())
}

/// Get current vault snapshot (for monitoring/debugging)
pub fn get_vault_snapshot() -> VaultBalances {
    EXCHANGE_VAULT
        .lock()
        .map(|v| v.clone())
        .unwrap_or_else(|_| VaultBalances::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_distribution() {
        let fee = 100.0;
        distribute_exchange_fee(QuoteAsset::Btc, fee).unwrap();

        let snapshot = get_vault_snapshot();
        assert_eq!(snapshot.btc.miners, 50.0);
        assert_eq!(snapshot.btc.dev, 30.0);
        assert_eq!(snapshot.btc.founders, 20.0);
    }

    #[test]
    fn test_deduct_miners_balance() {
        distribute_exchange_fee(QuoteAsset::Doge, 200.0).unwrap();

        let before = get_miners_balance(QuoteAsset::Doge);
        deduct_miners_balance(QuoteAsset::Doge, 50.0).unwrap();
        let after = get_miners_balance(QuoteAsset::Doge);

        assert_eq!(before - after, 50.0);
    }
}
