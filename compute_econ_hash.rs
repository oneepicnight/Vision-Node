// Temporary script to compute canonical ECON_HASH
// Run with: rustc compute_econ_hash.rs && ./compute_econ_hash

use std::io::Write;

fn main() {
    // Mainnet vault addresses from token_accounts.toml
    let staking_vault = "0xb977c16e539670ddfecc0ac902fcb916ec4b944e";
    let ecosystem_fund = "0x8bb8edcd4cdbcb132cc5e88ff90ba48cebf11cbd";
    let founder1 = "0xdf7a79291bb96e9dd1c77da089933767999eabf0";
    let founder2 = "0x083f95edd48e3e9da396891b704994b86e7790e7";
    
    // Split percentages in basis points (10000 BPS = 100%)
    let vault_bps: u16 = 5000;   // 50%
    let fund_bps: u16 = 3000;    // 30%
    let founder1_bps: u16 = 1000; // 10%
    let founder2_bps: u16 = 1000; // 10%
    
    // Validate splits sum to 100%
    let total = vault_bps + fund_bps + founder1_bps + founder2_bps;
    assert_eq!(total, 10000, "Splits must sum to 10000 BPS (100%)");
    
    // Build deterministic hash input (fixed order)
    let mut hasher = blake3::Hasher::new();
    
    // Addresses (deterministic order)
    hasher.update(staking_vault.as_bytes());
    hasher.update(ecosystem_fund.as_bytes());
    hasher.update(founder1.as_bytes());
    hasher.update(founder2.as_bytes());
    
    // Splits (same order)
    hasher.update(&vault_bps.to_le_bytes());
    hasher.update(&fund_bps.to_le_bytes());
    hasher.update(&founder1_bps.to_le_bytes());
    hasher.update(&founder2_bps.to_le_bytes());
    
    let hash = hasher.finalize();
    let hash_hex = format!("{}", hash.to_hex());
    
    println!("ECON_HASH (canonical): {}", hash_hex);
    println!("\nCopy this value into src/genesis.rs ECON_HASH constant");
    println!("\nInputs:");
    println!("  Staking vault (50%): {}", staking_vault);
    println!("  Ecosystem fund (30%): {}", ecosystem_fund);
    println!("  Founder1 (10%): {}", founder1);
    println!("  Founder2 (10%): {}", founder2);
}
