// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Vision Contributors

// Vision Chain Constants
// Import serde for derive macros used in enums
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
// Hard-coded values for deterministic chain behavior

// ============================
// Chain / Network Identity
// ============================

/// Canonical network identifier for this binary (MAINNET).
///
/// NOTE: This is telemetry / labeling only. Consensus identity is enforced via
/// `expected_chain_id()` and the baked-in genesis/bootstrap constants.
pub const VISION_NETWORK_ID: &str = "mainnet";

/// Canonical software version for this build (MAINNET).
/// Single source of truth for version across HTTP status, banners, and P2P handshake.
pub const VISION_VERSION: &str = "v1.0.3";

/// Chain ID derivation version. Bump only if the derivation algorithm changes.
pub const CHAIN_ID_VERSION: u32 = 1;

/// Bootstrap prefix used as an additional quarantine label.
///
/// CRITICAL: This must remain stable for the canonical chain.
/// v1.0.1: network reset for miner identity compatibility
pub const VISION_BOOTSTRAP_PREFIX: &str = "vision-constellation-v1.0.1";

/// Deterministically compute the expected chain id (hex string) for this build/drop.
///
/// CRITICAL RULE: This must depend ONLY on deterministic constants.
/// It must NOT depend on system time, RNG, machine info, file paths, or local IP.
pub fn expected_chain_id() -> String {
    // Hash("Vision|CHAIN_ID_VERSION|DROP_PREFIX|GENESIS_HASH|PARAMS")
    let material = format!(
        "Vision|chain_id_v={}|genesis={}|block_time_secs={}|bootstrap_ckpt_h={}|bootstrap_ckpt_hash={}",
        CHAIN_ID_VERSION,
        crate::genesis::GENESIS_HASH,
        BLOCK_TIME_SECS,
        VISION_BOOTSTRAP_HEIGHT,
        VISION_BOOTSTRAP_HASH,
    );
    hex::encode(blake3::hash(material.as_bytes()).as_bytes())
}

/// Same as `expected_chain_id()` but as raw 32-byte hash.
pub fn expected_chain_id_bytes() -> [u8; 32] {
    let hex_id = expected_chain_id();
    let bytes = hex::decode(hex_id).expect("expected_chain_id is valid hex");
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    arr
}

/// Canonical genesis hash for this network (hex, 32 bytes).
pub fn expected_genesis_hash_hex() -> &'static str {
    crate::genesis::GENESIS_HASH
}

/// Canonical bootstrap checkpoint for this network.
/// These should match whatever bootstrap.rs uses/checks.
pub const VISION_BOOTSTRAP_HEIGHT: u64 = 9; // Matches BOOTSTRAP_CHECKPOINT_HEIGHT
pub const VISION_BOOTSTRAP_HASH: &str =
    "e5bddd1e4081d8021937c3bf04a3abb95d2e449770672e6c5c17235ee142a945";

/// Protocol version window we accept on this network.
/// Any node outside this range will be treated as incompatible.
/// Protocol 2 = Constellation protocol with Ed25519 + Genesis launch
pub const VISION_MIN_PROTOCOL_VERSION: u32 = 2;
pub const VISION_MAX_PROTOCOL_VERSION: u32 = 2;

/// Minimum node binary version string that is allowed to talk to this network.
/// Compare lexicographically or with semver parsing where you already do.
pub const VISION_MIN_NODE_VERSION: &str = "v1.0.3";

// ============================
// Node Role & Mining Eligibility
// ============================

use crate::foundation_config;
use once_cell::sync::Lazy;

/// Helper to check if an environment flag is set (accepts "1", "true", "yes")
pub fn is_env_flag_set(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
        .unwrap_or(false)
}

/// Hard-wired pure swarm default (guardian-less operation).
/// Release builds ignore env overrides; debug builds may opt-out for local testing.
pub const PURE_SWARM_DEFAULT: bool = true;

/// Unified accessor for pure swarm mode.
/// In debug builds, allows opt-out via env for developers; in release builds, always true.
pub fn pure_swarm_mode() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var("VISION_PURE_SWARM")
            .ok()
            .or_else(|| std::env::var("VISION_PURE_SWARM_MODE").ok())
            .or_else(|| std::env::var("PURE_SWARM").ok())
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(PURE_SWARM_DEFAULT)
    }

    #[cfg(not(debug_assertions))]
    {
        PURE_SWARM_DEFAULT
    }
}

/// Seed export/import security flags (MAINNET LOCKDOWN)
/// Default OFF in release builds to prevent remote key theft
pub fn allow_seed_export() -> bool {
    #[cfg(debug_assertions)]
    {
        is_env_flag_set("VISION_ALLOW_SEED_EXPORT")
    }

    #[cfg(not(debug_assertions))]
    {
        // Release builds: require explicit opt-in
        is_env_flag_set("VISION_ALLOW_SEED_EXPORT")
    }
}

pub fn allow_seed_import() -> bool {
    #[cfg(debug_assertions)]
    {
        is_env_flag_set("VISION_ALLOW_SEED_IMPORT")
    }

    #[cfg(not(debug_assertions))]
    {
        // Release builds: require explicit opt-in
        is_env_flag_set("VISION_ALLOW_SEED_IMPORT")
    }
}

/// VISION_ANCHOR_NODE=1: This node is an anchor/backbone node (public, always healthy)
/// Anchors are truth keepers with strict eligibility requirements
pub static VISION_ANCHOR_NODE: Lazy<bool> = Lazy::new(|| is_env_flag_set("VISION_ANCHOR_NODE"));

/// VISION_OUTBOUND_ONLY=1: This node operates in outbound-only mode (home miner)
/// Outbound-only nodes can still mine if synced and healthy
pub static VISION_OUTBOUND_ONLY: Lazy<bool> = Lazy::new(|| is_env_flag_set("VISION_OUTBOUND_ONLY"));

/// VISION_EXCHANGE_NODE=1: This node is optimized for exchange/wallet backend use
/// Exchange nodes should be anchors with lag=0 for maximum reliability
pub static VISION_EXCHANGE_NODE: Lazy<bool> = Lazy::new(|| is_env_flag_set("VISION_EXCHANGE_NODE"));

/// How far behind the network tip a node can be and still mine.
pub const MAX_MINING_LAG_BLOCKS: i64 = 2;

// ============================
// Mining Timing & Safety
// ============================

/// Warmup window: disable rewards for the first N blocks
pub const WARMUP_BLOCKS: u64 = 1000;

/// Ship-safe consensus rule: do not produce blocks unless we have this many
/// validated connected peers (does not count self).
pub const MIN_VALIDATED_PEERS_FOR_BLOCKS: usize = 2;

// ============================
// Block & Emission Constants
// ============================

/// Block time in seconds
pub const BLOCK_TIME_SECS: u64 = 2;

/// Blocks per day (86400 seconds / 2 seconds per block)
pub const BLOCKS_PER_DAY: u64 = 43_200;

/// Blocks per era (1 year worth of blocks)
pub const BLOCKS_PER_ERA: u64 = 15_768_000;

/// Maximum mining block height (4 years total)
pub const MAX_MINING_BLOCK: u64 = 63_072_000;

/// Cash first mint height
pub const CASH_FIRST_MINT_HEIGHT: u64 = 1_000_000;

/// Foundation addresses (loaded from foundation_config)
pub fn vault_address() -> String {
    foundation_config::vault_address()
}

pub fn founder_address() -> String {
    foundation_config::founder1_address()
}

/// Selects the founder inventory address for a given deed id.
/// Policy: even deed ids -> founder1, odd deed ids -> founder2.
/// If founder2 is not configured, defaults to founder1.
pub fn founder_inventory_address_for_deed(deed_id: u64) -> String {
    let f1 = foundation_config::founder1_address();
    let f2 = foundation_config::founder2_address();
    // If founder2 is the same as founder1 or appears unset, prefer founder1
    if f2.is_empty() || f2 == f1 {
        return f1;
    }
    if deed_id.is_multiple_of(2) {
        f1
    } else {
        f2
    }
}

pub fn ops_address() -> String {
    foundation_config::fund_address()
}

/// Pending rewards tree name for sled database
/// Stores rewards banked for nodes that don't have a payout address set
pub const PENDING_REWARDS_TREE: &str = "pending_rewards";

/// Gross per-block LAND emission in each era (human units).
///
/// Updated tokenomics: Era 1 starts at 34 LAND per block and halves each era
/// resulting in 34 / 17 / 8.5 / 4.25 emissions for eras 1-4 respectively. The
/// protocol fee remains 2 LAND per block consumed from the emission (so miner
/// gets emission - protocol_fee).
pub const ERA1_REWARD_LAND: f64 = 34.0;
pub const ERA2_REWARD_LAND: f64 = 17.0;
pub const ERA3_REWARD_LAND: f64 = 8.5;
pub const ERA4_REWARD_LAND: f64 = 4.25;

/// Protocol fee in LAND tokens
pub const PROTOCOL_FEE_LAND: f64 = 2.0;

/// Staking reward (same as ERA4 emission) paid from Vault after mining ends
pub const STAKING_REWARD_LAND: f64 = ERA4_REWARD_LAND;

/// Number of decimal places for the LAND asset
pub const LAND_DECIMALS: u32 = 8;

/// Chain phase enum - mining vs staking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainPhase {
    Mining,
    Staking,
}

impl ChainPhase {
    pub fn from_height(height: u64) -> Self {
        if height >= MAX_MINING_BLOCK {
            ChainPhase::Staking
        } else {
            ChainPhase::Mining
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ChainPhase::Mining => "mining",
            ChainPhase::Staking => "staking",
        }
    }
}

/// Helper function to convert human LAND amounts (f64) into integer base units
pub fn land_amount(units: f64) -> u128 {
    let factor: u128 = 10u128.pow(LAND_DECIMALS);
    // Multiply using float then round to nearest integer to handle fractional LAND
    (units * factor as f64).round() as u128
}

/// Returns the per-block LAND emission for a given height, handling eras
///
/// Emission schedule:
/// - Era 1 (blocks 0-15,767,999): 34 LAND/block
/// - Era 2 (blocks 15,768,000-31,535,999): 17 LAND/block  
/// - Era 3 (blocks 31,536,000-47,303,999): 8.5 LAND/block
/// - Era 4 (blocks 47,304,000-63,071,999): 4.25 LAND/block
/// - After block 63,072,000: 0 LAND/block (mining ended)
pub fn land_block_reward(height: u64) -> u128 {
    // After max mining block, emission is 0 forever
    if height >= MAX_MINING_BLOCK {
        return 0;
    }

    // Calculate which era we're in (0-indexed)
    let era = height / BLOCKS_PER_ERA;

    match era {
        0 => land_amount(ERA1_REWARD_LAND), // 34 LAND
        1 => land_amount(ERA2_REWARD_LAND), // 17 LAND
        2 => land_amount(ERA3_REWARD_LAND), // 8.5 LAND
        3 => land_amount(ERA4_REWARD_LAND), // 4.25 LAND
        _ => 0, // Should never reach here given MAX_MINING_BLOCK check above
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test Era 1: blocks 0 to 15,767,999 = 34 LAND
    #[test]
    fn test_era1_emission_34_land() {
        assert_eq!(land_block_reward(0), land_amount(34.0));
        assert_eq!(land_block_reward(1), land_amount(34.0));
        assert_eq!(land_block_reward(BLOCKS_PER_ERA - 1), land_amount(34.0));
    }

    // Test Era 2: blocks 15,768,000 to 31,535,999 = 17 LAND
    #[test]
    fn test_era2_emission_17_land() {
        assert_eq!(land_block_reward(BLOCKS_PER_ERA), land_amount(17.0));
        assert_eq!(land_block_reward(BLOCKS_PER_ERA + 1), land_amount(17.0));
        assert_eq!(land_block_reward(2 * BLOCKS_PER_ERA - 1), land_amount(17.0));
    }

    // Test Era 3: blocks 31,536,000 to 47,303,999 = 8.5 LAND
    #[test]
    fn test_era3_emission_8_5_land() {
        assert_eq!(land_block_reward(2 * BLOCKS_PER_ERA), land_amount(8.5));
        assert_eq!(land_block_reward(2 * BLOCKS_PER_ERA + 1), land_amount(8.5));
        assert_eq!(land_block_reward(3 * BLOCKS_PER_ERA - 1), land_amount(8.5));
    }

    // Test Era 4: blocks 47,304,000 to 63,071,999 = 4.25 LAND
    #[test]
    fn test_era4_emission_4_25_land() {
        assert_eq!(land_block_reward(3 * BLOCKS_PER_ERA), land_amount(4.25));
        assert_eq!(land_block_reward(3 * BLOCKS_PER_ERA + 1), land_amount(4.25));
        assert_eq!(land_block_reward(MAX_MINING_BLOCK - 1), land_amount(4.25));
    }

    // Test emission ends at MAX_MINING_BLOCK = 63,072,000
    #[test]
    fn test_emission_ends_at_max_mining_block() {
        assert_eq!(land_block_reward(MAX_MINING_BLOCK), 0);
        assert_eq!(land_block_reward(MAX_MINING_BLOCK + 1), 0);
        assert_eq!(land_block_reward(MAX_MINING_BLOCK + 1000000), 0);
    }

    // Test protocol fee constant
    #[test]
    fn test_protocol_fee_is_2_land() {
        assert_eq!(land_amount(PROTOCOL_FEE_LAND), land_amount(2.0));
    }

    // Test total supply calculation over all 4 eras
    #[test]
    fn test_total_emission_calculation() {
        // Era 1: 15,768,000 blocks * 34 LAND = 536,112,000 LAND
        let era1_total = BLOCKS_PER_ERA * 34;
        assert_eq!(era1_total, 536_112_000);

        // Era 2: 15,768,000 blocks * 17 LAND = 268,056,000 LAND
        let era2_total = BLOCKS_PER_ERA * 17;
        assert_eq!(era2_total, 268_056_000);

        // Era 3: 15,768,000 blocks * 8.5 LAND = 134,028,000 LAND
        // Using integer math: 15,768,000 * 85 / 10
        let era3_total = (BLOCKS_PER_ERA * 85) / 10;
        assert_eq!(era3_total, 134_028_000);

        // Era 4: 15,768,000 blocks * 4.25 LAND = 67,014,000 LAND
        // Using integer math: 15,768,000 * 425 / 100
        let era4_total = (BLOCKS_PER_ERA * 425) / 100;
        assert_eq!(era4_total, 67_014_000);

        // Total emission: 1,005,210,000 LAND
        let total_emission = era1_total + era2_total + era3_total + era4_total;
        assert_eq!(total_emission, 1_005_210_000);
    }

    // Test land_amount helper precision
    #[test]
    fn test_land_amount_precision() {
        // 34 LAND = 3,400,000,000 base units (8 decimals)
        assert_eq!(land_amount(34.0), 3_400_000_000);
        // 17 LAND = 1,700,000,000 base units
        assert_eq!(land_amount(17.0), 1_700_000_000);
        // 8.5 LAND = 850,000,000 base units
        assert_eq!(land_amount(8.5), 850_000_000);
        // 4.25 LAND = 425,000,000 base units
        assert_eq!(land_amount(4.25), 425_000_000);
        // 2.0 LAND (protocol fee) = 200,000,000 base units
        assert_eq!(land_amount(2.0), 200_000_000);
    }

    // Test chain phase detection
    #[test]
    fn test_chain_phase_mining_vs_staking() {
        assert_eq!(ChainPhase::from_height(0), ChainPhase::Mining);
        assert_eq!(
            ChainPhase::from_height(MAX_MINING_BLOCK - 1),
            ChainPhase::Mining
        );
        assert_eq!(
            ChainPhase::from_height(MAX_MINING_BLOCK),
            ChainPhase::Staking
        );
        assert_eq!(
            ChainPhase::from_height(MAX_MINING_BLOCK + 1),
            ChainPhase::Staking
        );
    }
}

pub const PROTOCOL_VERSION_LITE: u32 = 1;
pub const PROTOCOL_VERSION_FULL: u32 = 2;

// ================================================================================
// BOOTSTRAP CHECKPOINT - Baked-in prefix for network quarantine
// ================================================================================
// Everyone ships with the same first 10 blocks (heights 0-9).
// These blocks are NEVER reorged, NEVER paid out, NEVER changed.
// New builds refuse to talk to nodes that don't have these 10 blocks OR nodes
// on different ports/network tags/versions.
//
// EFFECT: Incompatible builds are automatically quarantined.
// Nodes for this mainnet will all converge on one chain tip or they won't connect.
// ================================================================================

/// Height of the last baked-in bootstrap block (0-based, so 9 = 10 blocks)
pub const BOOTSTRAP_CHECKPOINT_HEIGHT: u64 = 9;

/// Hash of the last baked-in block at height 9.
/// Generated: 2025-12-09
pub const BOOTSTRAP_CHECKPOINT_HASH: &str =
    "e5bddd1e4081d8021937c3bf04a3abb95d2e449770672e6c5c17235ee142a945";

/// All 10 bootstrap block hashes (heights 0-9)
/// These define the canonical start of the mainnet chain.
/// Generated: 2025-12-09 from freshly mined blocks
pub const BOOTSTRAP_BLOCK_HASHES: [&str; 10] = [
    "d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13", // h=0 (genesis)
    "ad9345709e08ab6efbfa0849f5b1e2253f48f4a88a8db872ff087346401e2052", // h=1
    "aff9baa526f1c429e5129c97959519bb2375c08f5e1eba93c8bc4c49639dce45", // h=2
    "ce07f8d59cceaec2cab87fd6fa1a26c75a9f37fb1faa06c9649cab35df4c6d3b", // h=3
    "0cb2ff3986ea62f86610ef7c22d65890c4827d88dff8c8c0ebe73af5d81e9b2c", // h=4
    "5b1ec5d4d6fe6b8e5cd155cf8fda47aeeefbaf92994ba56ca783804ea7181508", // h=5
    "8c92b0931195f519e222a3a9c15a3d3c717046e8962db8fa5d55c6c1f23b8026", // h=6
    "37b58f1193092904404ebabda9e51ab414949100e53da1238c8b7abb4f23087f", // h=7
    "717fec45fc9317a643312c305e65c2873e3a244dcf7bbc7268464106ccf54730", // h=8
    "e5bddd1e4081d8021937c3bf04a3abb95d2e449770672e6c5c17235ee142a945", // h=9 (checkpoint)
];

// ================================================================================
// MINING HEIGHT QUORUM - Network convergence gate for mining eligibility
// ================================================================================
// Prevents isolated/desynced nodes from mining before they converge with network.
// Ensures most miners are building on the same chain tip, reducing orphan rate.
// ================================================================================

/// Minimum number of peers that must agree on similar height before mining is allowed
pub const MIN_SAME_HEIGHT_PEERS_FOR_MINING: usize = 2;

/// Maximum block height difference allowed for peers to be considered "in quorum"
pub const MAX_HEIGHT_DELTA_FOR_QUORUM: u64 = 2;

/// Timeout in seconds after which solo mining is allowed even without quorum
/// After this grace period, node can mine alone with a warning (prevents permanent lockout)
pub const MINING_QUORUM_TIMEOUT_SECS: u64 = 300;

// ================================================================================
// AUTO-SYNC - Background chain synchronization independent of mining
// ================================================================================
// Auto-sync runs continuously to keep the node synchronized with the best known
// chain, regardless of whether mining is enabled or eligible. This ensures nodes
// stay up-to-date even when not mining.
// ================================================================================

/// Interval in seconds between auto-sync checks
pub const AUTO_SYNC_INTERVAL_SECS: u64 = 10;

/// How many blocks behind the best known height before triggering sync
/// BOOTSTRAP: Lowered to 1 for immediate sync from height 0
pub const AUTO_SYNC_MAX_LAG_BLOCKS: u64 = 1;

/// Maximum blocks ahead of network consensus before mining stops
/// Prevents mining in isolation when local chain has diverged too far
pub const MAX_BLOCKS_AHEAD_OF_CONSENSUS: u64 = 50;

// ============================
// Slow Peer Tracking & Auto-Boost
// ============================

/// Threshold (in blocks) for considering a peer "slow" and needing help
/// Peers lagging this far behind network height get boosted gossip weight
pub const SLOW_PEER_LAG_BLOCKS: u64 = 64;

/// Maximum desync (in blocks) allowed for mining eligibility
/// Node must be within this many blocks of network height to mine
pub const MAX_DESYNC_FOR_MINING: u64 = 2;

// ============================
// Testnet Faucet Auto-Funding
// ============================

/// Testnet fake BTC balance (in BTC) - 2 BTC
pub const TESTNET_FAUCET_BTC: f64 = 2.0;

/// Testnet fake BCH balance (in BCH) - 10,000 BCH  
pub const TESTNET_FAUCET_BCH: f64 = 10000.0;

/// Testnet fake DOGE balance (in DOGE) - 1,000,000 DOGE
pub const TESTNET_FAUCET_DOGE: f64 = 1000000.0;

// ============================
// Deposit Confirmation Requirements (MAINNET)
// ============================

/// BTC requires 3 confirmations before credit/claim
pub const BTC_REQUIRED_CONFIRMATIONS: u32 = 3;

/// BCH requires 6 confirmations before credit/claim
pub const BCH_REQUIRED_CONFIRMATIONS: u32 = 6;

/// DOGE requires 12 confirmations before credit/claim
pub const DOGE_REQUIRED_CONFIRMATIONS: u32 = 12;

/// Returns required confirmations for a given coin
pub fn required_confirmations(coin: &str) -> u32 {
    match coin.to_uppercase().as_str() {
        "BTC" => BTC_REQUIRED_CONFIRMATIONS,
        "BCH" => BCH_REQUIRED_CONFIRMATIONS,
        "DOGE" => DOGE_REQUIRED_CONFIRMATIONS,
        _ => 6, // Conservative default
    }
}

// ============================
// Testnet LAND Conversion Rates
// ============================

/// Testnet conversion: 1 BTC = 1,000,000 LAND
pub const TESTNET_LAND_PER_BTC: f64 = 1_000_000.0;

/// Testnet conversion: 1 BCH = 500,000 LAND
pub const TESTNET_LAND_PER_BCH: f64 = 500_000.0;

/// Testnet conversion: 1 DOGE = 100 LAND
pub const TESTNET_LAND_PER_DOGE: f64 = 100.0;

/// Minimum vault balance (in satoshis) before triggering auto-convert
pub const VAULT_MIN_CONVERT_SATS: u64 = 100_000;

// Prefix/key constants moved from main.rs to be available across modules
pub const LAND_DEED_PREFIX: &str = "land:deed:"; // land:deed:<id> -> owner address
pub const META_LAND_GENESIS_DONE: &str = "meta:land_genesis_done"; // bool flag when genesis minted
/// DB keys for supply metrics
pub const SUPPLY_TOTAL_KEY: &str = "supply:total"; // u128 BE
pub const SUPPLY_VAULT_KEY: &str = "supply:vault"; // u128 BE
pub const SUPPLY_FOUNDER_KEY: &str = "supply:founder"; // u128 BE
pub const SUPPLY_OPS_KEY: &str = "supply:ops"; // u128 BE

// ============================
// Deposit Persistence Keys
// ============================

/// DB tree for deposit address mappings (MAINNET: prevents deposit loss on restart)
pub const DEPOSIT_MAPPING_TREE: &str = "deposit_mappings";

/// Key prefix: wallet_address_normalized -> deposit_index
pub const DEPOSIT_WALLET_TO_INDEX_PREFIX: &str = "w2i:";

/// Key prefix: deposit_address -> wallet_address_normalized
pub const DEPOSIT_ADDR_TO_WALLET_PREFIX: &str = "a2w:";
