//! Creator/Guardian identity configuration
//!
//! Defines the special "creator" wallet that has god-tier access to Guardian controls.

use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::{error, info};

/// Sled tree for creator configuration
pub const CREATOR_CONFIG_TREE: &str = "creator_config";
pub const CREATOR_CONFIG_KEY: &[u8] = b"creator";

/// Creator-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatorConfig {
    /// The creator's wallet address - only this address can access Guardian page
    pub creator_address: String,

    /// Creator's display name
    pub name: String,

    /// Creator's title/role
    pub title: String,
}

impl Default for CreatorConfig {
    fn default() -> Self {
        Self {
            creator_address: "".to_string(), // Must be set during initialization
            name: "Donald Etcher".to_string(),
            title: "Architect of Chaos & Second Chances".to_string(),
        }
    }
}

/// Extended founders configuration with creator identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundersConfig {
    /// List of founder wallet addresses (for founder checks in governance etc.)
    pub founders: Vec<String>,

    /// Special creator identity for Guardian page access
    pub creator: CreatorConfig,
}

#[allow(clippy::derivable_impls)]
impl Default for FoundersConfig {
    fn default() -> Self {
        Self {
            founders: vec![],
            creator: CreatorConfig::default(),
        }
    }
}

/// Load creator config from database
pub fn load_creator_config(db: &Db) -> Result<CreatorConfig, String> {
    let tree = db
        .open_tree(CREATOR_CONFIG_TREE)
        .map_err(|e| format!("Failed to open creator config tree: {}", e))?;

    match tree.get(CREATOR_CONFIG_KEY) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to deserialize creator config: {}", e)),
        Ok(None) => {
            info!("[CREATOR CONFIG] No creator config found, using default");
            Ok(CreatorConfig::default())
        }
        Err(e) => Err(format!("Failed to read creator config: {}", e)),
    }
}

/// Save creator config to database
pub fn save_creator_config(db: &Db, config: &CreatorConfig) -> Result<(), String> {
    let tree = db
        .open_tree(CREATOR_CONFIG_TREE)
        .map_err(|e| format!("Failed to open creator config tree: {}", e))?;

    let bytes = serde_json::to_vec(config)
        .map_err(|e| format!("Failed to serialize creator config: {}", e))?;

    tree.insert(CREATOR_CONFIG_KEY, bytes.as_slice())
        .map_err(|e| format!("Failed to save creator config: {}", e))?;

    tree.flush()
        .map_err(|e| format!("Failed to flush creator config: {}", e))?;

    info!(
        "[CREATOR CONFIG] Saved creator address: {}",
        config.creator_address
    );

    Ok(())
}

/// Check if a given wallet address is the creator
pub fn is_creator_address(db: &Db, address: &str) -> bool {
    match load_creator_config(db) {
        Ok(config) => {
            if config.creator_address.is_empty() {
                error!("[CREATOR AUTH] Creator address not configured!");
                false
            } else {
                config.creator_address.eq_ignore_ascii_case(address)
            }
        }
        Err(e) => {
            error!("[CREATOR AUTH] Failed to load creator config: {}", e);
            false
        }
    }
}

/// Initialize creator config with a specific address (call once during setup)
pub fn initialize_creator_config(db: &Db, creator_address: &str) -> Result<(), String> {
    let config = CreatorConfig {
        creator_address: creator_address.to_string(),
        ..CreatorConfig::default()
    };

    save_creator_config(db, &config)?;

    info!(
        "[CREATOR CONFIG] Initialized with address: {}",
        creator_address
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creator_config_default() {
        let config = CreatorConfig::default();
        assert_eq!(config.name, "Donald Etcher");
        assert_eq!(config.title, "Architect of Chaos & Second Chances");
    }

    #[test]
    fn test_is_creator_address() {
        let db = sled::Config::new().temporary(true).open().unwrap();

        // No config yet
        assert!(!is_creator_address(&db, "0x123"));

        // Initialize
        initialize_creator_config(&db, "0xCREATOR").unwrap();

        // Check
        assert!(is_creator_address(&db, "0xCREATOR"));
        assert!(is_creator_address(&db, "0xcreator")); // case insensitive
        assert!(!is_creator_address(&db, "0x123"));
    }
}
