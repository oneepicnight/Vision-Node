// 2-LAND Block Tithe - siphoned every block and split across foundation addresses
// Ensures Vault grows from block 1 to fund future mining rewards
#![allow(dead_code)]

/// Small, fixed deduction per block (in smallest units)
/// Default: 2 LAND with 9 decimals → 2 * 10^9 = 2_000_000_000
use crate::vision_constants;

pub fn tithe_amount() -> u128 {
    // Fixed protocol fee as defined in vision_constants (2 LAND)
    vision_constants::land_amount(vision_constants::PROTOCOL_FEE_LAND)
}

/// Basis points split for tithe: MINER / VAULT / FUND / TREASURY (sum = 10_000)
/// Default: 0/50/30/20 (miner gets 0 of tithe since they already get emission)
pub fn tithe_split_bps() -> (u16, u16, u16, u16) {
    // Fixed 0/50/30/20 split
    (0u16, 5000u16, 3000u16, 2000u16)
}

/// Get foundation addresses from env vars (matching Tokenomics config)
pub fn vault_addr() -> Result<String, String> {
    Ok(vision_constants::vault_address())
}

pub fn fund_addr() -> Result<String, String> {
    Ok(vision_constants::founder_address())
}

pub fn treasury_addr() -> Result<String, String> {
    Ok(vision_constants::ops_address())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tithe_defaults() {
        // Default tithe is 2 LAND (with 9 decimals)
        std::env::remove_var("VISION_TOK_TITHE_AMOUNT");
        assert_eq!(tithe_amount(), 2_000_000_000u128);

        // Default split is 0/50/30/20
        std::env::remove_var("VISION_TOK_TITHE_MINER_BPS");
        std::env::remove_var("VISION_TOK_TITHE_VAULT_BPS");
        std::env::remove_var("VISION_TOK_TITHE_FUND_BPS");
        std::env::remove_var("VISION_TOK_TITHE_TREASURY_BPS");
        let (m, v, f, t) = tithe_split_bps();
        assert_eq!(m, 0);
        assert_eq!(v, 5000);
        assert_eq!(f, 3000);
        assert_eq!(t, 2000);
        assert_eq!(m + v + f + t, 10_000);
    }

    #[test]
    fn test_custom_tithe() {
        std::env::set_var("VISION_TOK_TITHE_AMOUNT", "500000000");
        assert_eq!(tithe_amount(), 500_000_000u128);

        std::env::set_var("VISION_TOK_TITHE_MINER_BPS", "2000");
        std::env::set_var("VISION_TOK_TITHE_VAULT_BPS", "4000");
        std::env::set_var("VISION_TOK_TITHE_FUND_BPS", "2500");
        std::env::set_var("VISION_TOK_TITHE_TREASURY_BPS", "1500");
        let (m, v, f, t) = tithe_split_bps();
        assert_eq!(m, 2000);
        assert_eq!(v, 4000);
        assert_eq!(f, 2500);
        assert_eq!(t, 1500);
        assert_eq!(m + v + f + t, 10_000);

        // Cleanup
        std::env::remove_var("VISION_TOK_TITHE_AMOUNT");
        std::env::remove_var("VISION_TOK_TITHE_MINER_BPS");
        std::env::remove_var("VISION_TOK_TITHE_VAULT_BPS");
        std::env::remove_var("VISION_TOK_TITHE_FUND_BPS");
        std::env::remove_var("VISION_TOK_TITHE_TREASURY_BPS");
    }
}
