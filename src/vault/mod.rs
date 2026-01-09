// Vault System - Exchange fee collection and LAND auto-buy
// Crypto fees (BTC/BCH/DOGE): 50% Miners (auto-buy hot wallet), 25% Founder1, 25% Founder2
// On-chain LAND revenue: 50% Miners/Vault, 30% DevOps, 20% split between Founder1/Founder2
//
// Auto-buy: ONLY Miners bucket crypto is converted to LAND (threshold: 100k sats)
// Founder1/Founder2 crypto accumulates and requires withdrawal system to send to their addresses
// Miners LAND is stored as reserve for future staking rewards (post-mining era)

pub mod land_auto_buy;
pub mod miners_multisig;
pub mod router;
pub mod store;
pub mod withdrawals;

pub use router::VaultRouter;
pub use store::{VaultBucket, VaultStore};
