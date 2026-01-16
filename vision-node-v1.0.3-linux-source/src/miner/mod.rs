//! Active mining module with worker threads and VisionX PoW
//!
//! Provides the ActiveMiner struct for managing mining operations

#[cfg(feature = "miner-tuning")]
pub mod auto_tuner;
pub mod hint_manager;
#[cfg(feature = "miner-tuning")]
pub mod intelligent_tuner;
pub mod manager;
#[cfg(feature = "miner-tuning")]
pub mod numa;
#[cfg(feature = "miner-tuning")]
pub mod perf_store;
#[cfg(feature = "miner-tuning")]
pub mod power;
#[cfg(feature = "miner-tuning")]
pub mod telemetry;
#[cfg(feature = "miner-tuning")]
pub mod thermal;
pub mod tuning_hint;

pub use manager::ActiveMiner;
