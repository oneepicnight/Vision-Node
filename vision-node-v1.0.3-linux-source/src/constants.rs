//! Vision Node Constants
//!
//! Central location for important constants used throughout the node.

// =================== Guardian Launch Constants ===================

/// Number of blocks the Guardian mines during launch sequence (blocks 1, 2, 3)
/// Only applies to mainnet-full with launch_guardian_enabled=true
pub const LAUNCH_BLOCK_COUNT: u64 = 3;

/// Network identifier for full feature mainnet
pub const NETWORK_MAINNET_FULL: &str = "mainnet-full";

/// Network identifier for testnet
pub const NETWORK_TESTNET: &str = "testnet";

// =================== Block Validation Constants ===================

/// Genesis block height
pub const GENESIS_HEIGHT: u64 = 0;

/// First mineable block (after genesis)
pub const FIRST_MINEABLE_BLOCK: u64 = 1;

// =================== Node Version & Protocol ===================

/// Human-readable node version for UI/logs/banners
pub const NODE_VERSION: &str = "v2.0";

/// Wire protocol version used in handshake
pub const PROTOCOL_VERSION: u32 = 2;

/// Minimum required protocol version for new testnet
/// Nodes with protocol < 2 are rejected on testnet
pub const MIN_PROTOCOL_TESTNET: u32 = 2;

// =================== Port Configuration ===================

/// Default HTTP API port - used for web interface, wallet, API endpoints
pub const VISION_DEFAULT_HTTP_PORT: u16 = 7070;

/// Default P2P port - used for mining, handshake, peer connections
/// ⭐ ALL P2P operations (mining, handshake, seed peers, UPnP) use this port
pub const VISION_DEFAULT_P2P_PORT: u16 = 7072;

// =================== Testnet Default Seeds ===================

/// Default seed nodes for testnet bootstrap (v2.0.0 - Port 7072)
/// Fresh installs automatically connect to these peers
/// ⭐ SwarmOnly mode: permanent seeds, never blacklisted
pub const TESTNET_DEFAULT_SEEDS: &[&str] = &[
    "16.163.123.221:7072",
    "69.173.206.211:7072",
    "69.173.207.135:7072",
    "75.128.156.69:7072",
    "98.97.137.74:7072",
    "182.106.66.15:7072",
];

// =================== Testnet Bootstrap Auto-Stamp ===================

/// Number of blocks to auto-stamp on testnet seed nodes
/// Only applies to testnet with is_testnet_seed=true
pub const TESTNET_STAMP_BLOCK_COUNT: u64 = 3;
