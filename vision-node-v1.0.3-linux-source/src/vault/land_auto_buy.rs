// Land Auto-Buyer: Convert BTC/BCH/DOGE vault balances to LAND

use anyhow::Result;
use sled::Db;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::market::engine::QuoteAsset;
use crate::receipts::{write_receipt, Receipt};
use crate::tokenomics::split_50_30_20;
use crate::vault::store::{VaultBucket, VaultStore};
use crate::vision_constants::{
    is_env_flag_set, TESTNET_LAND_PER_BCH, TESTNET_LAND_PER_BTC, TESTNET_LAND_PER_DOGE,
    VAULT_MIN_CONVERT_SATS,
};

// Rate-limit threshold logs to once every 60 seconds
static LAST_THRESHOLD_LOG: AtomicU64 = AtomicU64::new(0);
const LOG_RATE_LIMIT_SECS: u64 = 60;

fn should_log_threshold() -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let last = LAST_THRESHOLD_LOG.load(Ordering::Relaxed);
    if now - last >= LOG_RATE_LIMIT_SECS {
        LAST_THRESHOLD_LOG.store(now, Ordering::Relaxed);
        true
    } else {
        false
    }
}

pub struct LandAutoBuyer {
    store: VaultStore,
    db: Db,
}

impl LandAutoBuyer {
    pub fn new(store: VaultStore, db: Db) -> Self {
        Self { store, db }
    }

    /// Run one conversion cycle for all supported external assets
    pub fn run_conversion_cycle(&self) -> Result<()> {
        let auto_buy_enabled = is_env_flag_set("VISION_ENABLE_VAULT_AUTO_BUY");

        if !auto_buy_enabled {
            tracing::debug!(
                "Land auto-buy disabled (set VISION_ENABLE_VAULT_AUTO_BUY=1 to enable)"
            );
            return Ok(());
        }

        tracing::debug!("🔄 Land auto-buy: enabled=true, starting conversion cycle");
        self.convert_asset_to_land(QuoteAsset::Btc, TESTNET_LAND_PER_BTC)?;
        self.convert_asset_to_land(QuoteAsset::Bch, TESTNET_LAND_PER_BCH)?;
        self.convert_asset_to_land(QuoteAsset::Doge, TESTNET_LAND_PER_DOGE)?;

        Ok(())
    }

    fn convert_asset_to_land(&self, asset: QuoteAsset, land_per_unit: f64) -> Result<()> {
        // ONLY convert Miners bucket - Founder1/Founder2 crypto should NOT be auto-converted
        // Their crypto accumulates for future withdrawal to their addresses
        let miners_balance = self.store.get_bucket_balance(VaultBucket::Miners, asset)?;

        // Convert to satoshis for threshold check
        let total_sats = miners_balance.min(u64::MAX as u128) as u64;

        if total_sats < VAULT_MIN_CONVERT_SATS {
            // Rate-limited: show why we're not converting yet
            if should_log_threshold() {
                tracing::info!(
                    "⏳ Auto-buy threshold not met: asset={} current={} sats < required={} sats (rate-limited log)",
                    asset.as_str(),
                    total_sats,
                    VAULT_MIN_CONVERT_SATS
                );
            }
            return Ok(());
        }

        // Calculate LAND amount: convert balance to f64, multiply by rate, then back to u128
        let balance_f64 = miners_balance as f64 / 100_000_000.0;
        let land_amount = balance_f64 * land_per_unit;

        if land_amount < 0.00000001 {
            if should_log_threshold() {
                tracing::info!(
                    "⏳ Auto-buy: {} balance too small for conversion: {:.8} {} -> {:.8} LAND (rate-limited)",
                    asset.as_str(),
                    balance_f64,
                    asset.as_str(),
                    land_amount
                );
            }
            return Ok(());
        }

        tracing::info!(
            "🔄 VAULT AUTO-BUY: Converting {:.8} {} → {:.2} LAND (rate: {:.2} per unit) [Miners bucket only]",
            balance_f64,
            asset.as_str(),
            land_amount,
            land_per_unit
        );

        // Burn ONLY Miners bucket - Founder1/Founder2 crypto is NOT converted (goes to their wallets)
        self.store.debit_vault(VaultBucket::Miners, asset, miners_balance)?;

        // Convert LAND amount to u128 for split
        let land_units = (land_amount * 100_000_000.0) as u128;
        let split = split_50_30_20(land_units as u64);

        // Convert u64 split results to u128 and credit LAND
        let miners_land = split.miners as u128;
        let devops_land = split.devops as u128;
        let founders_land = split.founders as u128;
        
        // Split founders amount 50/50 between Founder1 and Founder2
        let founder1_land = founders_land / 2;
        let founder2_land = founders_land - founder1_land;

        self.store
            .credit_vault(VaultBucket::Miners, QuoteAsset::Land, miners_land)?;
        self.store
            .credit_vault(VaultBucket::DevOps, QuoteAsset::Land, devops_land)?;
        self.store
            .credit_vault(VaultBucket::Founder1, QuoteAsset::Land, founder1_land)?;
        self.store
            .credit_vault(VaultBucket::Founder2, QuoteAsset::Land, founder2_land)?;

        tracing::info!(
            "✅ VAULT AUTO-BUY COMPLETE: Converted {:.8} {} → {:.2} LAND | Distributed: miners={} devops={} founder1={} founder2={}",
            balance_f64,
            asset.as_str(),
            land_amount,
            miners_land,
            devops_land,
            founder1_land,
            founder2_land
        );

        // Write receipt for auditability (best effort - don't fail conversion if receipt fails)
        let memo = format!(
            "Auto-buy: {} {} ({} sats) → {} LAND [Miners bucket only] | Split: miners={}, devops={}, founder1={}, founder2={} | Rate: {:.2} LAND per unit",
            balance_f64,
            asset.as_str(),
            total_sats,
            land_amount,
            miners_land,
            devops_land,
            founder1_land,
            founder2_land,
            land_per_unit
        );

        let receipt = Receipt {
            id: String::new(), // will be filled by write_receipt
            ts_ms: 0,          // will be filled by write_receipt
            kind: "vault_autobuy".to_string(),
            from: format!("vault:miners:{}", asset.as_str()),
            to: "vault:LAND".to_string(),
            amount: land_units.to_string(),
            fee: miners_balance.to_string(), // Only Miners bucket crypto burned
            memo: Some(memo),
            txid: None,
            ok: true,
            note: Some(format!("Converted {} to LAND", asset.as_str())),
        };

        if let Err(e) = write_receipt(&self.db, None, receipt) {
            tracing::warn!("Failed to write auto-buy receipt: {}", e);
        }

        Ok(())
    }
}
