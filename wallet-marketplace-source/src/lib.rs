// ---- Clippy/lints: keep signals high, noise low ----
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![cfg_attr(not(any(test, feature = "dev")), allow(dead_code))]

// Library crate entry to allow integration tests to access internal modules
pub mod config;
pub mod crypto;
pub mod ledger;
pub mod market;
pub mod util;
