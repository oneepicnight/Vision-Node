use once_cell::sync::Lazy;
/// Simple checkpoint type used to pin historic heights to a known hash.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub height: u64,
    /// hex-encoded 32-byte pow_hash (64 hex chars)
    pub hash: String,
}

/// STATIC_CHECKPOINTS: keep a small in-code list for now. The genesis checkpoint
/// is represented as 64 zeros which matches the `genesis_block()` used in the
/// node (the testnet genesis uses a zero-hash placeholder).
pub static STATIC_CHECKPOINTS: Lazy<Vec<Checkpoint>> = Lazy::new(|| {
    vec![Checkpoint {
        height: 0,
        hash: "0".repeat(64),
    }]
});
