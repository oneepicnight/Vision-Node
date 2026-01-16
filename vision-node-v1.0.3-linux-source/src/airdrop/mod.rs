//! Airdrop system module
//!
//! Handles CASH distribution to wallet addresses.

pub mod cash;

pub use cash::{
    execute_cash_airdrop, get_cash_total_supply, validate_airdrop_request, CashAirdropLimits,
    CashAirdropRequest,
};
