//! Snapshot API Module
//!
//! Provides read-only endpoints for capturing chain state snapshots.
//! Used by the Vision World website to track historical data, generate charts,
//! and maintain a record of network sunsets.

use axum::{routing::get, Json, Router};
use serde::Serialize;

/// Snapshot of the current chain state
#[derive(Debug, Serialize)]
pub struct SnapshotResponse {
    /// Current blockchain height
    pub height: u64,
    /// Snapshot timestamp (RFC 3339 format)
    pub timestamp: String,
    /// Token supply across all currencies
    pub total_supply: TokenSupplySnapshot,
    /// Vault balances for fee distribution
    pub vaults: VaultSnapshot,
}

/// Token supply breakdown by currency
#[derive(Debug, Serialize)]
pub struct TokenSupplySnapshot {
    /// LAND token total supply (string to preserve precision)
    pub land: String,
    /// CASH token total supply (if applicable)
    pub cash: String,
    /// GAME token total supply (if applicable)
    pub game: String,
}

/// Vault balances showing fee distribution
#[derive(Debug, Serialize)]
pub struct VaultSnapshot {
    /// Miners' vault balance (50% of exchange fees)
    pub miners: VaultCurrencySnapshot,
    /// Development vault balance (30% of exchange fees)
    pub dev: VaultCurrencySnapshot,
    /// Founders vault balance (20% of exchange fees)
    pub founders: VaultCurrencySnapshot,
}

/// Vault balances per currency
#[derive(Debug, Serialize)]
pub struct VaultCurrencySnapshot {
    pub land: String,
    pub btc: String,
    pub bch: String,
    pub doge: String,
}

/// Create router for snapshot endpoints
pub fn snapshot_routes() -> Router {
    Router::new().route("/snapshot/current", get(snapshot_current_handler))
}

/// Handler for GET /snapshot/current
///
/// Returns a comprehensive snapshot of the current chain state including:
/// - Current height and timestamp
/// - Total token supply
/// - Vault balances for all currencies
async fn snapshot_current_handler() -> Result<Json<SnapshotResponse>, axum::http::StatusCode> {
    // Get current height and LAND supply from chain state
    let (height, land_supply) = {
        let chain = crate::CHAIN.lock();
        let h = chain.blocks.len() as u64;
        let supply: u128 = chain.balances.values().sum();
        (h, supply)
    };

    // Get DB for real sled data
    let db = {
        let chain = crate::CHAIN.lock();
        chain.db.clone()
    };

    // Helper to read u128 from sled
    let read_u128_sled = |key: &[u8]| -> u128 {
        db.get(key)
            .ok()
            .flatten()
            .map(|v| {
                let mut buf = [0u8; 16];
                let bytes = v.as_ref();
                buf[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
                u128::from_le_bytes(buf)
            })
            .unwrap_or(0)
    };

    // Read real vault totals from sled counters
    let vault_total = read_u128_sled(b"supply:vault");
    let fund_total = read_u128_sled(b"supply:fund");
    let treasury_total = read_u128_sled(b"supply:treasury");

    // Distribute totals according to 50/30/20 split (vault/ops/founder)
    // For now, represent as single totals; future: read actual account balances
    let miners_total = vault_total; // 50% goes to miners vault
    let dev_total = fund_total; // 30% goes to ops/dev
    let founders_total = treasury_total; // 20% goes to founders

    // Build vault snapshot from real sled data
    let vault_snapshot = VaultSnapshot {
        miners: VaultCurrencySnapshot {
            land: miners_total.to_string(),
            btc: "0".to_string(), // Would need separate BTC vault tracking
            bch: "0".to_string(),
            doge: "0".to_string(),
        },
        dev: VaultCurrencySnapshot {
            land: dev_total.to_string(),
            btc: "0".to_string(),
            bch: "0".to_string(),
            doge: "0".to_string(),
        },
        founders: VaultCurrencySnapshot {
            land: founders_total.to_string(),
            btc: "0".to_string(),
            bch: "0".to_string(),
            doge: "0".to_string(),
        },
    };

    // Build response
    let snapshot = SnapshotResponse {
        height,
        timestamp: chrono::Utc::now().to_rfc3339(),
        total_supply: TokenSupplySnapshot {
            land: land_supply.to_string(),
            cash: "0".to_string(), // Placeholder for future CASH token
            game: "0".to_string(), // Placeholder for future GAME token
        },
        vaults: vault_snapshot,
    };

    Ok(Json(snapshot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_response_structure() {
        let snapshot = SnapshotResponse {
            height: 12345,
            timestamp: "2025-11-21T00:00:00Z".to_string(),
            total_supply: TokenSupplySnapshot {
                land: "1000000000".to_string(),
                cash: "0".to_string(),
                game: "0".to_string(),
            },
            vaults: VaultSnapshot {
                miners: VaultCurrencySnapshot {
                    land: "500".to_string(),
                    btc: "100".to_string(),
                    bch: "200".to_string(),
                    doge: "300".to_string(),
                },
                dev: VaultCurrencySnapshot {
                    land: "300".to_string(),
                    btc: "60".to_string(),
                    bch: "120".to_string(),
                    doge: "180".to_string(),
                },
                founders: VaultCurrencySnapshot {
                    land: "200".to_string(),
                    btc: "40".to_string(),
                    bch: "80".to_string(),
                    doge: "120".to_string(),
                },
            },
        };

        // Verify serialization works
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("height"));
        assert!(json.contains("12345"));
        assert!(json.contains("timestamp"));
    }
}
