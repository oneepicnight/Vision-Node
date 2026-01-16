//! Node Identity Module
//!
//! Provides cryptographic node identity based on Ed25519 keypairs.
//! Node ID is deterministically derived from the public key.
#![allow(dead_code)]

pub mod migration;
pub mod node_id;
pub mod nonce_cache;

pub use node_id::{
    init_node_identity, local_fingerprint, local_node_id, local_node_identity, local_pubkey_b64,
    node_id_from_pubkey,
};

pub use migration::check_and_migrate_legacy_identity;
