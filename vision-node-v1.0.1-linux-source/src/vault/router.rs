// Vault Router: Route exchange fees into vault using 50/30/20 split

use anyhow::Result;
use sled::Db;

use crate::market::engine::QuoteAsset;
use crate::tokenomics::split_50_30_20;
use crate::vault::store::{VaultBucket, VaultStore};

pub struct VaultRouter {
    store: VaultStore,
}

impl VaultRouter {
    pub fn new(db: Db) -> Self {
        Self {
            store: VaultStore::new(db),
        }
    }

    /// Route an EXCHANGE fee into the vault balances (in the quote asset)
    /// Split: 50% Miners (auto-buy hot wallet), 25% Founder1, 25% Founder2
    /// DevOps gets ZERO crypto fees (they only participate in on-chain LAND revenue)
    pub fn route_exchange_fee(&self, asset: QuoteAsset, amount: f64) -> Result<()> {
        if amount == 0.0 {
            return Ok(());
        }

        // Convert to u128 for split (assuming amounts in smallest units)
        let amount_units = (amount * 100_000_000.0) as u128;
        
        // Split 50/25/25 (Miners/Founder1/Founder2)
        let miners_amount = amount_units.saturating_mul(50) / 100;
        let founder1_amount = amount_units.saturating_mul(25) / 100;
        let founder2_amount = amount_units.saturating_sub(miners_amount + founder1_amount); // Remaining to avoid dust

        self.store
            .credit_vault(VaultBucket::Miners, asset, miners_amount)?;
        self.store
            .credit_vault(VaultBucket::Founder1, asset, founder1_amount)?;
        self.store
            .credit_vault(VaultBucket::Founder2, asset, founder2_amount)?;

        tracing::info!(
            "💰 Vault fee routed: asset={} total={:.8} → miners={} (50% auto-buy) founder1={} (25%) founder2={} (25%)",
            asset.as_str(),
            amount,
            miners_amount,
            founder1_amount,
            founder2_amount
        );

        Ok(())
    }
}
