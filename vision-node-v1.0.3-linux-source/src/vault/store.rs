// Vault Store: DB-backed persistent storage for vault bucket balances
//
// Storage format:
//   Tree: "vault_balances"
//   Key: "vault:{bucket}:{asset}" (e.g., "vault:miners:BTC")
//   Value: u128 in atomic units (no float rounding)
//     - BTC/BCH/DOGE: 1 satoshi base (1e8 per coin)
//     - LAND/CASH: 1 smallest unit (1e8 per coin for consistency)
#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sled::Db;

use crate::market::engine::QuoteAsset;

const VAULT_TREE: &str = "vault_balances";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VaultBucket {
    Miners,
    DevOps,
    Founder1,  // 25% of exchange fees
    Founder2,  // 25% of exchange fees
}

impl VaultBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            VaultBucket::Miners => "miners",
            VaultBucket::DevOps => "devops",
            VaultBucket::Founder1 => "founder1",
            VaultBucket::Founder2 => "founder2",
        }
    }
}

/// Asset balances for a single bucket (in-memory representation)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketBalances {
    pub land: u128, // Changed to u128 (atomic units)
    pub btc: u128,  // Changed to u128 (atomic units)
    pub bch: u128,  // Changed to u128 (atomic units)
    pub doge: u128, // Changed to u128 (atomic units)
}

impl BucketBalances {
    pub fn get(&self, asset: QuoteAsset) -> u128 {
        match asset {
            QuoteAsset::Land => self.land,
            QuoteAsset::Btc => self.btc,
            QuoteAsset::Bch => self.bch,
            QuoteAsset::Doge => self.doge,
        }
    }

    pub fn set(&mut self, asset: QuoteAsset, value: u128) {
        match asset {
            QuoteAsset::Land => self.land = value,
            QuoteAsset::Btc => self.btc = value,
            QuoteAsset::Bch => self.bch = value,
            QuoteAsset::Doge => self.doge = value,
        }
    }

    pub fn add(&mut self, asset: QuoteAsset, amount: u128) {
        let current = self.get(asset);
        self.set(asset, current.saturating_add(amount));
    }
}

/// All vault bucket balances (in-memory representation)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AllBucketBalances {
    pub miners: BucketBalances,
    pub devops: BucketBalances,
    pub founder1: BucketBalances,
    pub founder2: BucketBalances,
}

/// Vault store backed by sled database
pub struct VaultStore {
    db: Db,
}

impl VaultStore {
    /// Create a new VaultStore with the given sled database
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Generate the key for a vault bucket balance
    fn make_key(bucket: VaultBucket, asset: QuoteAsset) -> String {
        format!("vault:{}:{}", bucket.as_str(), asset.as_str())
    }

    /// Read a balance from the vault tree
    fn read_balance(&self, bucket: VaultBucket, asset: QuoteAsset) -> Result<u128> {
        let tree = self.db.open_tree(VAULT_TREE)?;
        let key = Self::make_key(bucket, asset);

        match tree.get(key.as_bytes())? {
            Some(value) => {
                let bytes: [u8; 16] = value
                    .as_ref()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid vault balance bytes"))?;
                Ok(u128::from_be_bytes(bytes))
            }
            None => Ok(0),
        }
    }

    /// Write a balance to the vault tree
    fn write_balance(&self, bucket: VaultBucket, asset: QuoteAsset, balance: u128) -> Result<()> {
        let tree = self.db.open_tree(VAULT_TREE)?;
        let key = Self::make_key(bucket, asset);
        tree.insert(key.as_bytes(), &balance.to_be_bytes())?;
        Ok(())
    }

    /// Credit a vault bucket with the given asset and amount
    pub fn credit_vault(&self, bucket: VaultBucket, asset: QuoteAsset, amount: u128) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        let current = self.read_balance(bucket, asset)?;
        let new_balance = current.saturating_add(amount);
        self.write_balance(bucket, asset, new_balance)?;

        tracing::debug!(
            "Vault credit: bucket={} asset={} amount={} new_balance={}",
            bucket.as_str(),
            asset.as_str(),
            amount,
            new_balance
        );

        Ok(())
    }

    /// Debit a vault bucket with the given asset and amount
    pub fn debit_vault(&self, bucket: VaultBucket, asset: QuoteAsset, amount: u128) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }

        let current = self.read_balance(bucket, asset)?;
        let new_balance = current.saturating_sub(amount);
        self.write_balance(bucket, asset, new_balance)?;

        tracing::debug!(
            "Vault debit: bucket={} asset={} amount={} new_balance={}",
            bucket.as_str(),
            asset.as_str(),
            amount,
            new_balance
        );

        Ok(())
    }

    /// Return the total vault balance for a specific asset across all buckets
    pub fn total_vault_balance(&self, asset: QuoteAsset) -> Result<u128> {
        let miners = self.read_balance(VaultBucket::Miners, asset)?;
        let devops = self.read_balance(VaultBucket::DevOps, asset)?;
        let founder1 = self.read_balance(VaultBucket::Founder1, asset)?;
        let founder2 = self.read_balance(VaultBucket::Founder2, asset)?;

        Ok(miners.saturating_add(devops).saturating_add(founder1).saturating_add(founder2))
    }

    /// Get balance for a specific bucket and asset
    pub fn get_bucket_balance(&self, bucket: VaultBucket, asset: QuoteAsset) -> Result<u128> {
        self.read_balance(bucket, asset)
    }

    /// Burn all vault balances for a given asset across all buckets (used before auto-buying LAND)
    pub fn burn_all_vault_balances_for_asset(&self, asset: QuoteAsset) -> Result<u128> {
        let total = self.total_vault_balance(asset)?;

        self.write_balance(VaultBucket::Miners, asset, 0)?;
        self.write_balance(VaultBucket::DevOps, asset, 0)?;
        self.write_balance(VaultBucket::Founder1, asset, 0)?;
        self.write_balance(VaultBucket::Founder2, asset, 0)?;

        tracing::info!("Vault burn: asset={} total={}", asset.as_str(), total);

        Ok(total)
    }

    /// Get all balances (for API)
    pub fn get_all_balances(&self) -> Result<AllBucketBalances> {
        let mut balances = AllBucketBalances::default();

        for bucket in &[
            VaultBucket::Miners,
            VaultBucket::DevOps,
            VaultBucket::Founder1,
            VaultBucket::Founder2,
        ] {
            for asset in &[
                QuoteAsset::Land,
                QuoteAsset::Btc,
                QuoteAsset::Bch,
                QuoteAsset::Doge,
            ] {
                let balance = self.read_balance(*bucket, *asset)?;
                match bucket {
                    VaultBucket::Miners => balances.miners.set(*asset, balance),
                    VaultBucket::DevOps => balances.devops.set(*asset, balance),
                    VaultBucket::Founder1 => balances.founder1.set(*asset, balance),
                    VaultBucket::Founder2 => balances.founder2.set(*asset, balance),
                }
            }
        }

        Ok(balances)
    }
}
