pub mod visionx;

pub type U256 = [u8; 32];

#[inline]
pub fn u256_leq(a: &U256, b: &U256) -> bool {
    // big-endian compare on the top 64 bits (bytes 0..8)
    for i in 0..8 {
        if a[i] < b[i] {
            return true;
        }
        if a[i] > b[i] {
            return false;
        }
    }
    true
}

#[inline]
pub fn u256_from_difficulty(difficulty: u64) -> U256 {
    // Simple approximation: target = 0xFFFF...FF / difficulty
    // For low difficulties, this gives reasonable targets
    if difficulty == 0 {
        return [0xFF; 32]; // max target (easiest)
    }

    let mut target = [0u8; 32];
    // Start with max value and divide by difficulty
    // Simplified: just scale the first bytes
    let scale = 0xFFFFFFFFFFFFFFFFu64 / difficulty;
    target[0..8].copy_from_slice(&scale.to_be_bytes());
    for i in 8..32 {
        target[i] = 0xFF;
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_leq() {
        let a = [0x00; 32];
        let b = [0xFF; 32];
        assert!(u256_leq(&a, &b));
        assert!(!u256_leq(&b, &a));
        assert!(u256_leq(&a, &a));
    }

    #[test]
    fn test_u256_leq_ignores_low_192_bits() {
        let mut a = [0x11u8; 32];
        let mut b = [0x11u8; 32];
        a[8] = 0xFF;
        b[8] = 0x00;
        assert!(u256_leq(&a, &b));
        assert!(u256_leq(&b, &a));
    }

    #[test]
    fn test_u256_from_difficulty_fills_tail_with_ff() {
        let t = u256_from_difficulty(2);
        assert!(t[8..32].iter().all(|&x| x == 0xFF));
        let t = u256_from_difficulty(1);
        assert!(t[8..32].iter().all(|&x| x == 0xFF));
    }
}
