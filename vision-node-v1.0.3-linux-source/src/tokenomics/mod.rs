// Tokenomics module - official emission system
pub mod tithe;

// Tokenomics: Central 50/30/20 split utility
// Used for distributing rewards and fees across vault buckets

use serde::{Deserialize, Serialize};

/// Result of 50/30/20 split
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultSplit {
    pub miners: u64,
    pub devops: u64,
    pub founders: u64,
}

/// Split any amount into our 50/30/20 vault buckets.
/// 50% Miners, 30% DevOps, 20% Founders.
pub fn split_50_30_20(total: u64) -> VaultSplit {
    if total == 0 {
        return VaultSplit {
            miners: 0,
            devops: 0,
            founders: 0,
        };
    }

    let miners = total.saturating_mul(50) / 100;
    let devops = total.saturating_mul(30) / 100;
    let mut founders = total.saturating_sub(miners + devops);

    // Safety: avoid accidental overflow or weird rounding
    if miners + devops + founders > total {
        founders = total.saturating_sub(miners + devops);
    }

    VaultSplit {
        miners,
        devops,
        founders,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_50_30_20() {
        let split = split_50_30_20(1000);
        assert_eq!(split.miners, 500);
        assert_eq!(split.devops, 300);
        assert_eq!(split.founders, 200);
        assert_eq!(split.miners + split.devops + split.founders, 1000);
    }

    #[test]
    fn test_split_zero() {
        let split = split_50_30_20(0);
        assert_eq!(split.miners, 0);
        assert_eq!(split.devops, 0);
        assert_eq!(split.founders, 0);
    }

    #[test]
    fn test_split_odd_number() {
        let split = split_50_30_20(999);
        assert_eq!(split.miners, 499);
        assert_eq!(split.devops, 299);
        // Founders gets remainder
        assert!(split.miners + split.devops + split.founders <= 999);
    }
}
