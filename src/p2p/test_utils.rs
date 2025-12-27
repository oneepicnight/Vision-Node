//! Test utilities for P2P testing with proper isolation
//!
//! This module provides mock implementations and test helpers to avoid
//! global state dependencies in P2P unit tests.

use crate::p2p::connection::HandshakeMessage;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mock chain state for testing
pub struct MockChain {
    pub genesis_hash: [u8; 32],
    pub chain_id: [u8; 32],
    pub height: u64,
}

impl MockChain {
    pub fn new() -> Self {
        Self {
            genesis_hash: [0xAA; 32],
            chain_id: [0xBB; 32],
            height: 0,
        }
    }

    pub fn with_genesis(genesis: [u8; 32]) -> Self {
        Self {
            genesis_hash: genesis,
            chain_id: [0xBB; 32],
            height: 0,
        }
    }

    pub fn with_height(mut self, height: u64) -> Self {
        self.height = height;
        self
    }
}

/// Create a valid test handshake message
pub fn create_test_handshake(genesis: [u8; 32], height: u64, nonce: u64) -> HandshakeMessage {
    HandshakeMessage {
        protocol_version: crate::vision_constants::PROTOCOL_VERSION_LITE,
        chain_id: [0xBB; 32],
        genesis_hash: genesis,
        node_nonce: nonce,
        chain_height: height,
        node_version: 100,
        node_tag: "TEST-NODE-0000".to_string(),
        admission_ticket: "test_ticket_placeholder".to_string(),
        vision_address: "vision://TEST-NODE-0000@testhash".to_string(),
        node_id: "test-node-0000".to_string(),
        public_key: "test_pubkey".to_string(),
        role: "constellation".to_string(),
    }
}

/// Create a handshake with wrong protocol version
pub fn create_invalid_version_handshake(genesis: [u8; 32]) -> HandshakeMessage {
    HandshakeMessage {
        protocol_version: 99999, // Invalid version
        chain_id: [0xBB; 32],
        genesis_hash: genesis,
        node_nonce: 1234,
        chain_height: 0,
        node_version: 100,
        node_tag: "TEST-NODE-INVALID".to_string(),
        admission_ticket: "test_ticket_invalid".to_string(),
        vision_address: "vision://TEST-NODE-INVALID@invalid".to_string(),
        node_id: "test-node-invalid".to_string(),
        public_key: "invalid_pubkey".to_string(),
        role: "constellation".to_string(),
    }
}

/// Create a handshake with wrong genesis
pub fn create_wrong_genesis_handshake() -> HandshakeMessage {
    HandshakeMessage {
        protocol_version: crate::vision_constants::PROTOCOL_VERSION_LITE,
        chain_id: [0xBB; 32],
        genesis_hash: [0xFF; 32], // Wrong genesis
        node_nonce: 1234,
        chain_height: 0,
        node_version: 100,
        node_tag: "TEST-NODE-WRONG".to_string(),
        admission_ticket: "test_ticket_wrong".to_string(),
        vision_address: "vision://TEST-NODE-WRONG@wronghash".to_string(),
        node_id: "test-node-wrong".to_string(),
        public_key: "wrong_pubkey".to_string(),
        role: "constellation".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_chain_creation() {
        let chain = MockChain::new();
        assert_eq!(chain.genesis_hash, [0xAA; 32]);
        assert_eq!(chain.height, 0);
    }

    #[test]
    fn test_mock_chain_with_genesis() {
        let custom_genesis = [0x42; 32];
        let chain = MockChain::with_genesis(custom_genesis);
        assert_eq!(chain.genesis_hash, custom_genesis);
    }

    #[test]
    fn test_mock_chain_with_height() {
        let chain = MockChain::new().with_height(100);
        assert_eq!(chain.height, 100);
    }

    #[test]
    fn test_create_test_handshake() {
        let genesis = [0x11; 32];
        let handshake = create_test_handshake(genesis, 50, 9999);
        assert_eq!(handshake.genesis_hash, genesis);
        assert_eq!(handshake.chain_height, 50);
        assert_eq!(handshake.node_nonce, 9999);
    }

    #[test]
    fn test_invalid_version_handshake() {
        let genesis = [0x22; 32];
        let handshake = create_invalid_version_handshake(genesis);
        assert_eq!(handshake.protocol_version, 99999);
        assert_eq!(handshake.genesis_hash, genesis);
    }

    #[test]
    fn test_wrong_genesis_handshake() {
        let handshake = create_wrong_genesis_handshake();
        assert_eq!(handshake.genesis_hash, [0xFF; 32]);
    }
}
