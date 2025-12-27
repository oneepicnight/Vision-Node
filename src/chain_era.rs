//! Chain Era Management
//!
//! Vision Node operates in two distinct eras:
//! 1. **Mining Era**: Block rewards via emissions (with halving), single rotating guardian
//! 2. **Staking Era**: Fixed staking rewards (4.25 LAND + fees), guardian mesh of land deed holders
//!
//! The transition happens automatically when total supply reaches MAX_SUPPLY.
//! This is the "sunset moment" where mining ends and the guardian mesh takes over.

use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::{info, warn};

/// Maximum supply cap for LAND token (in base units: nanoLAND = 10^9)
/// When reached, emissions end and staking era begins
pub const MAX_SUPPLY: u128 = 100_000_000 * 1_000_000_000; // 100 million LAND

/// Staking era base reward per block (in nanoLAND)
/// This is split among all active stakers (land deed holders)
pub const STAKING_BASE_REWARD: u128 = 4_250_000_000; // 4.25 LAND

/// Database key for storing current era
const ERA_DB_KEY: &[u8] = b"chain_era";

/// Represents the two eras of Vision Node operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainEra {
    /// Mining Era: Traditional PoW mining with block emissions
    Mining,

    /// Staking Era: Guardian mesh with fixed staking rewards
    /// All land deed holders participate in consensus and earn rewards
    Staking,
}

impl ChainEra {
    /// Returns human-readable name
    pub fn as_str(&self) -> &'static str {
        match self {
            ChainEra::Mining => "mining",
            ChainEra::Staking => "staking",
        }
    }

    /// Check if currently in mining era
    pub fn is_mining(&self) -> bool {
        matches!(self, ChainEra::Mining)
    }

    /// Check if currently in staking era
    pub fn is_staking(&self) -> bool {
        matches!(self, ChainEra::Staking)
    }
}

/// Era management with persistence
#[derive(Debug)]
pub struct EraManager {
    db: Db,
    current_era: ChainEra,
}

impl EraManager {
    /// Load or initialize era manager
    /// Defaults to Mining era on genesis
    pub fn new(db: &Db) -> Self {
        let current_era = Self::load_from_db(db).unwrap_or(ChainEra::Mining);

        info!(
            "[CHAIN_ERA] Initialized - Current era: {} ({})",
            current_era.as_str().to_uppercase(),
            match current_era {
                ChainEra::Mining => "Block emissions active",
                ChainEra::Staking => "Guardian mesh active",
            }
        );

        Self {
            db: db.clone(),
            current_era,
        }
    }

    /// Get current era
    pub fn current_era(&self) -> ChainEra {
        self.current_era
    }

    /// Check if total supply has reached MAX_SUPPLY and trigger era flip if needed
    /// Returns true if era flipped
    pub fn check_and_flip_era(&mut self, total_supply: u128, block_height: u64) -> bool {
        // Only flip from Mining → Staking
        if self.current_era != ChainEra::Mining {
            return false;
        }

        // Check if we've reached max supply
        if total_supply < MAX_SUPPLY {
            return false;
        }

        // 🎉 ERA FLIP! Mining → Staking
        info!("╔═══════════════════════════════════════════════════════════════╗");
        info!("║           🎉 EMISSIONS COMPLETE - ERA TRANSITION 🎉          ║");
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║  Block Height: {:<48}║", block_height);
        info!(
            "║  Total Supply: {:<48}║",
            format!("{:.2} LAND", total_supply as f64 / 1_000_000_000.0)
        );
        info!(
            "║  Max Supply:   {:<48}║",
            format!("{:.2} LAND", MAX_SUPPLY as f64 / 1_000_000_000.0)
        );
        info!("╠═══════════════════════════════════════════════════════════════╣");
        info!("║  Mining Era: COMPLETE                                        ║");
        info!("║  Staking Era: ENGAGED                                        ║");
        info!("║                                                              ║");
        info!("║  → Mining disabled                                           ║");
        info!("║  → Guardian mesh activated                                   ║");
        info!("║  → Land deed holders now earn staking rewards                ║");
        info!("║  → Base reward: 4.25 LAND + fees per block                   ║");
        info!("╚═══════════════════════════════════════════════════════════════╝");

        self.current_era = ChainEra::Staking;
        self.save_to_db();

        true
    }

    /// Force era transition (for testing or manual intervention)
    pub fn set_era(&mut self, era: ChainEra) {
        if self.current_era == era {
            return;
        }

        warn!(
            "[CHAIN_ERA] Manual era transition: {} → {}",
            self.current_era.as_str(),
            era.as_str()
        );

        self.current_era = era;
        self.save_to_db();
    }

    /// Load era from database
    fn load_from_db(db: &Db) -> Option<ChainEra> {
        db.get(ERA_DB_KEY)
            .ok()
            .flatten()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }

    /// Save current era to database
    fn save_to_db(&self) {
        if let Ok(json) = serde_json::to_vec(&self.current_era) {
            let _ = self.db.insert(ERA_DB_KEY, json.as_slice());
            let _ = self.db.flush();
        }
    }
}

/// Helper to check if emissions should still be active
pub fn emissions_active(total_supply: u128, current_era: ChainEra) -> bool {
    current_era.is_mining() && total_supply < MAX_SUPPLY
}

/// Calculate remaining emissions before era flip
pub fn emissions_remaining(total_supply: u128) -> u128 {
    MAX_SUPPLY.saturating_sub(total_supply)
}

/// Calculate progress to max supply (0.0 - 1.0)
pub fn emission_progress(total_supply: u128) -> f64 {
    if MAX_SUPPLY == 0 {
        return 1.0;
    }
    (total_supply as f64 / MAX_SUPPLY as f64).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_era_enum() {
        assert!(ChainEra::Mining.is_mining());
        assert!(!ChainEra::Mining.is_staking());
        assert!(ChainEra::Staking.is_staking());
        assert!(!ChainEra::Staking.is_mining());
    }

    #[test]
    fn test_emissions_active() {
        assert!(emissions_active(0, ChainEra::Mining));
        assert!(emissions_active(MAX_SUPPLY - 1, ChainEra::Mining));
        assert!(!emissions_active(MAX_SUPPLY, ChainEra::Mining));
        assert!(!emissions_active(0, ChainEra::Staking));
    }

    #[test]
    fn test_emission_progress() {
        assert_eq!(emission_progress(0), 0.0);
        assert_eq!(emission_progress(MAX_SUPPLY / 2), 0.5);
        assert_eq!(emission_progress(MAX_SUPPLY), 1.0);
        assert_eq!(emission_progress(MAX_SUPPLY * 2), 1.0); // Capped at 1.0
    }
}
