//! Domain-specific error types for Vision Node
//!
//! Provides structured error handling instead of String/anyhow mix
#![allow(dead_code)]

use thiserror::Error;

/// Blockchain consensus errors
#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Invalid proof of work: {0}")]
    InvalidPoW(String),

    #[error("Block timestamp too far in future: {0}")]
    FutureTimestamp(u64),

    #[error("Block timestamp not greater than median of recent blocks")]
    TimestampNotIncreasing,

    #[error("Difficulty target not met: required {required} bits, got {actual}")]
    DifficultyNotMet { required: u8, actual: u8 },

    #[error("Invalid block height: expected {expected}, got {actual}")]
    InvalidHeight { expected: u64, actual: u64 },

    #[error("Genesis hash mismatch: {0}")]
    GenesisHashMismatch(String),

    #[error("Chain reorganization too deep: {depth} blocks (max: {max})")]
    ReorgTooDeep { depth: u64, max: u64 },
}

/// Transaction validation errors
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },

    #[error("Nonce mismatch: expected {expected}, got {actual}")]
    NonceMismatch { expected: u64, actual: u64 },

    #[error("Transaction too large: {size} bytes (max: {max})")]
    TooLarge { size: usize, max: usize },

    #[error("Fee too low: minimum {required}, provided {actual}")]
    FeeTooLow { required: u64, actual: u64 },

    #[error("Duplicate transaction: {0}")]
    Duplicate(String),

    #[error("Invalid transaction format: {0}")]
    InvalidFormat(String),
}

/// Mempool management errors
#[derive(Error, Debug)]
pub enum MempoolError {
    #[error("Mempool full: {size}/{capacity} transactions")]
    Full { size: usize, capacity: usize },

    #[error("Transaction already in mempool: {0}")]
    AlreadyExists(String),

    #[error("Replace-by-fee tip too low: existing {existing}, new {new}")]
    RbfTipTooLow { existing: u64, new: u64 },

    #[error("Nonce gap too large: expected {expected}, got {actual}")]
    NonceGapTooLarge { expected: u64, actual: u64 },

    #[error("Transaction validation failed: {0}")]
    ValidationFailed(#[from] TransactionError),
}

/// P2P networking errors
#[derive(Error, Debug)]
pub enum P2PError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("Protocol version mismatch: local {local}, remote {remote}")]
    ProtocolMismatch { local: u32, remote: u32 },

    #[error("Message too large: {size} bytes (max: {max})")]
    MessageTooLarge { size: u32, max: u32 },

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Peer timeout: {0}")]
    Timeout(String),

    #[error("Peer banned: {0}")]
    PeerBanned(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] std::io::Error),
}

/// Database operation errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to read from database: {0}")]
    ReadFailed(String),

    #[error("Failed to write to database: {0}")]
    WriteFailed(String),

    #[error("Database corruption detected: {0}")]
    Corrupted(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Sled error: {0}")]
    SledError(#[from] sled::Error),
}

/// Mining and PoW errors
#[derive(Error, Debug)]
pub enum MiningError {
    #[error("Mining disabled: {0}")]
    Disabled(String),

    #[error("No miner address configured")]
    NoMinerAddress,

    #[error("Block template generation failed: {0}")]
    TemplateGenerationFailed(String),

    #[error("Invalid block solution: {0}")]
    InvalidSolution(String),

    #[error("Mining era ended at block {0}")]
    EraEnded(u64),
}

/// API and RPC errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// Governance system errors
#[derive(Error, Debug)]
pub enum GovernanceError {
    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),

    #[error("Voting window closed for proposal: {0}")]
    VotingClosed(String),

    #[error("Already voted on proposal: {0}")]
    AlreadyVoted(String),

    #[error("Insufficient LAND balance: required {required}, available {available}")]
    InsufficientLand { required: u128, available: u128 },

    #[error("Not authorized to vote: {0}")]
    NotAuthorized(String),
}

/// Smart contract errors
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("Invalid WASM bytecode: {0}")]
    InvalidBytecode(String),

    #[error("Contract not found: {0}")]
    NotFound(String),

    #[error("Contract execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Failed to instantiate contract: {0}")]
    InstantiationFailed(String),

    #[error("Failed to store contract: {0}")]
    StorageFailed(String),
}

/// State management errors
#[derive(Error, Debug)]
pub enum StateError {
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },

    #[error("Nonce mismatch: expected {expected}, got {actual}")]
    NonceMismatch { expected: u64, actual: u64 },

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Missing access key: {0}")]
    MissingAccess(String),
}

/// Unified node error type
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Consensus error: {0}")]
    Consensus(#[from] ConsensusError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),

    #[error("Mempool error: {0}")]
    Mempool(#[from] MempoolError),

    #[error("P2P error: {0}")]
    P2P(#[from] P2PError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Mining error: {0}")]
    Mining(#[from] MiningError),

    #[error("Contract error: {0}")]
    Contract(#[from] ContractError),

    #[error("State error: {0}")]
    State(#[from] StateError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Governance error: {0}")]
    Governance(#[from] GovernanceError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<serde_json::Error> for NodeError {
    fn from(e: serde_json::Error) -> Self {
        NodeError::Serialization(e.to_string())
    }
}

impl From<anyhow::Error> for NodeError {
    fn from(e: anyhow::Error) -> Self {
        NodeError::Other(e.to_string())
    }
}

// Allow converting String errors to NodeError for gradual migration
impl From<String> for NodeError {
    fn from(s: String) -> Self {
        NodeError::Other(s)
    }
}

impl From<&str> for NodeError {
    fn from(s: &str) -> Self {
        NodeError::Other(s.to_string())
    }
}

/// Result type alias for node operations
pub type NodeResult<T> = Result<T, NodeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_error_display() {
        let err = ConsensusError::InvalidPoW("hash doesn't meet target".to_string());
        assert!(err.to_string().contains("Invalid proof of work"));
    }

    #[test]
    fn test_transaction_error_display() {
        let err = TransactionError::InsufficientBalance {
            required: 1000,
            available: 500,
        };
        assert!(err.to_string().contains("required 1000"));
        assert!(err.to_string().contains("available 500"));
    }

    #[test]
    fn test_node_error_from_consensus() {
        let consensus_err = ConsensusError::GenesisHashMismatch("test".to_string());
        let node_err: NodeError = consensus_err.into();
        assert!(matches!(node_err, NodeError::Consensus(_)));
    }

    #[test]
    fn test_mempool_error_chain() {
        let tx_err = TransactionError::FeeTooLow {
            required: 100,
            actual: 50,
        };
        let mempool_err: MempoolError = tx_err.into();
        assert!(matches!(mempool_err, MempoolError::ValidationFailed(_)));
    }
}
