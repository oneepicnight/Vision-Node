//! types.rs — shared helpers for Vision node
use blake3::Hasher;

/// Count leading zero bits in a 32-byte hash.
pub fn leading_zero_bits(hash: &[u8; 32]) -> u8 {
    let mut bits: u8 = 0;
    for b in hash.iter() {
        if *b == 0 {
            bits += 8;
        } else {
            bits += b.leading_zeros() as u8;
            break;
        }
    }
    bits
}

/// Compute a simple "work" score from a hash: 2^(leading_zero_bits).
/// Capped at 2^127 to avoid overflow.
pub fn work_from_hash(hash: &[u8; 32]) -> u128 {
    let lz = leading_zero_bits(hash).min(127);
    1u128 << lz
}

/// Hash arbitrary bytes with BLAKE3, returning 32 bytes.
pub fn blake3_hash(bytes: &[u8]) -> [u8; 32] {
    let mut h = Hasher::new();
    h.update(bytes);
    *h.finalize().as_bytes()
}
