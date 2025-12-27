// Vault System - Exchange fee collection and LAND auto-buy
// Manages 50/30/20 split across Miners/DevOps/Founders buckets

pub mod land_auto_buy;
pub mod miners_multisig;
pub mod router;
pub mod store;

pub use router::VaultRouter;
