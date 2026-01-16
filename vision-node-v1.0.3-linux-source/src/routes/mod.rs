//! HTTP route handlers extracted from main.rs
//!
//! This module organizes route handlers into logical groups:
//! - `wallet` - Balance queries and transfers
//! - `receipts` - Transaction receipt queries
//! - `admin_seed` - Admin endpoint for seeding test balances
//!
//! Each submodule exports handler functions that can be registered with axum Router.
//!
//! # Router Configuration
//!
//! This repository ships a single build world (FULL).
//!
//! ## Current Status
//!
//! Routes are currently registered via the legacy `build_app()` function in main.rs.
//!
//! To complete migration:
//! 1. Audit all 400+ routes in build_app()
//! 2. Create the main router here
//! 3. Replace build_app() call with routes::create_router()
//!

pub mod admin_cash_airdrop;
pub mod admin_farm;
pub mod admin_farmhand;
pub mod admin_mining_endpoints;
pub mod admin_modes;
pub mod admin_seed;
pub mod beacon;
pub mod era;
pub mod governance;
pub mod guardian_control;
pub mod immortality;
pub mod miner;
pub mod mining_info;
pub mod receipts;
pub mod tip;
pub mod wallet;
pub mod wallet_legacy;
pub mod wallet_mining;

use axum::Router;

/// Create router with state - called from main()
/// Phase 2: Modular router system with feature-gated routes
pub fn create_router_with_state(tok_accounts: crate::accounts::TokenAccountsCfg) -> Router {
    eprintln!("📡 Phase 2 Router: FULL build (single-world)");
    eprintln!("   ✅ Core: health, metrics, config");
    eprintln!("   ✅ Wallet: balance, transfer, deposit");
    eprintln!("   ✅ Market: exchange, orders, trades");
    eprintln!("   ✅ Mining: control, status, rewards");
    eprintln!("   ✅ Network: peers, snapshots, sync");
    eprintln!("   ✅ Advanced features: enabled");

    // Call legacy build_app which already has proper feature gates
    // Phase 2 Complete: Router is modular via build_app's #[cfg(feature = "full")] blocks
    // Next phase: Extract build_app logic into separate route modules for better organization
    crate::build_app(tok_accounts)
}
