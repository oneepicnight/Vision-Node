#![allow(dead_code)]
//! P2P Protocol Messages for Headers-First Sync
//!
//! Implements efficient block synchronization with:
//! - Headers-first download (separate header/body fetch)
//! - Pipelined block requests (sliding window)
//! - Orphan handling with parent requests
//! - Deduplication and seen filters

use serde::{Deserialize, Serialize};

/// Announce a new block tip to peers (lightweight, no full block)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnounceBlock {
    pub height: u64,
    pub hash: String,
    pub prev: String,
}

/// Request block headers from a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHeaders {
    /// Block locator hashes (exponential backoff from tip)
    pub locator: Vec<String>,
    /// Optional stopping hash
    pub stop: Option<String>,
    /// Maximum number of headers to return
    pub max: usize,
}

/// Response with block headers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Headers {
    pub headers: Vec<LiteHeader>,
}

/// Lightweight block header (no full block data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteHeader {
    pub hash: String,
    pub prev: String,
    pub height: u64,
    pub time: u64,
    pub target: String,
    pub merkle: String,
    pub difficulty: u64,
    pub nonce: u64,
}

/// Request full blocks by hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlocks {
    pub hashes: Vec<String>,
}

/// Response with full blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocks {
    pub blocks: Vec<BlockEnvelope>,
}

/// Full block envelope (base64-encoded)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEnvelope {
    pub hash: String,
    /// Base64-encoded JSON block
    pub raw: String,
}

impl GetHeaders {
    /// Create a new headers request with default max
    pub fn new(locator: Vec<String>) -> Self {
        Self {
            locator,
            stop: None,
            max: 2000,
        }
    }
}

impl LiteHeader {
    /// Convert from full Block to LiteHeader
    pub fn from_block(block: &crate::Block) -> Self {
        // DIAGNOSTIC: Log what pow_hash we're copying from block to LiteHeader
        tracing::debug!(
            "[LITE-FROM-BLOCK] height={} copying pow_hash={} to LiteHeader.hash",
            block.header.number,
            block.header.pow_hash
        );
        
        Self {
            hash: block.header.pow_hash.clone(),
            prev: block.header.parent_hash.clone(),
            height: block.header.number,
            time: block.header.timestamp,
            target: format!("{:016x}", block.header.difficulty),
            merkle: block.header.tx_root.clone(),
            difficulty: block.header.difficulty,
            nonce: block.header.nonce,
        }
    }
}
