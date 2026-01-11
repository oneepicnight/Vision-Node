//! Beacon module - Network discovery and peer registry
#![allow(dead_code)]

pub mod registry;

pub use registry::{get_peers, register_peer, BeaconPeer};
