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
    pub fn route_exchange_fee(&self, asset: QuoteAsset, amount: f64) -> Result<()> {
        if amount == 0.0 {
            return Ok(());
        }

        // Convert to u128 for split (assuming amounts in smallest units)
        let amount_units = (amount * 100_000_000.0) as u128;
        let split = split_50_30_20(amount_units as u64);

        // Convert u64 split results to u128
        let miners_amount = split.miners as u128;
        let devops_amount = split.devops as u128;
        let founders_amount = split.founders as u128;

        self.store
            .credit_vault(VaultBucket::Miners, asset, miners_amount)?;
        self.store
            .credit_vault(VaultBucket::DevOps, asset, devops_amount)?;
        self.store
            .credit_vault(VaultBucket::Founders, asset, founders_amount)?;

        tracing::info!(
            "💰 Vault fee routed: asset={} total={:.8} miners={} devops={} founders={}",
            asset.as_str(),
            amount,
            miners_amount,
            devops_amount,
            founders_amount
        );

        Ok(())
    }
}
