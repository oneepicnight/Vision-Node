//! Wallet Configuration
//!
//! Manages wallet addresses for LAND rewards and multi-chain deposits.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    /// Primary LAND address for mining rewards (auto-generated if missing)
    pub land_reward_address: Option<String>,

    /// Optional deposit addresses for supported chains
    pub btc_deposit_address: Option<String>,
    pub bch_deposit_address: Option<String>,
    pub doge_deposit_address: Option<String>,
}

impl WalletConfig {
    /// Load wallet config from file, creating default if missing
    pub fn load_or_create<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();

        if path.exists() {
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read wallet config: {}", e))?;
            let config: WalletConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse wallet config: {}", e))?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Save wallet config to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize wallet config: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write wallet config: {}", e))?;
        Ok(())
    }

    /// Get or generate LAND reward address
    pub fn get_or_generate_land_address(&mut self) -> Result<String, String> {
        if let Some(addr) = &self.land_reward_address {
            if is_valid_land_address(addr) {
                return Ok(addr.clone());
            }
        }

        // Generate new LAND address
        let new_addr = generate_land_address()?;
        self.land_reward_address = Some(new_addr.clone());
        Ok(new_addr)
    }
}

/// Validate LAND address format
pub fn is_valid_land_address(addr: &str) -> bool {
    // LAND addresses start with "land1" and are bech32 encoded
    // Typical length: 42-44 characters
    if !addr.starts_with("land1") {
        return false;
    }

    if addr.len() < 38 || addr.len() > 50 {
        return false;
    }

    // Check bech32 character set (lowercase alphanumeric except 1, b, i, o)
    addr.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Generate a new LAND address (placeholder - implement proper key generation)
fn generate_land_address() -> Result<String, String> {
    // TODO: Implement proper key generation with secp256k1
    // For now, generate a placeholder format
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars = "023456789acdefghjklmnpqrstuvwxyz";
    let random_suffix: String = (0..38)
        .map(|_| {
            let idx = rng.gen_range(0, chars.len());
            chars.chars().nth(idx).unwrap()
        })
        .collect();

    Ok(format!("land1{}", random_suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_land_address_validation() {
        assert!(is_valid_land_address(
            "land1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszq"
        ));
        assert!(!is_valid_land_address(
            "btc1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszq"
        ));
        assert!(!is_valid_land_address("land"));
        assert!(!is_valid_land_address(
            "LAND1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszq"
        ));
    }
}
