// Land Stake Registry - manages LAND staking for post-mining era
#![allow(dead_code)]
use anyhow::Result;
use sled::Db;

const LAND_STAKE_PREFIX: &str = "land:stake:"; // per-address staked amount
const LAND_STAKE_TOTAL_KEY: &str = "land:stake:total";
const LAND_STAKE_OWNERS_KEY: &str = "land:stake:owners"; // comma-separated list of stakers

/// Get the staked LAND amount for an address
pub fn get_stake(db: &Db, addr: &str) -> u128 {
    let key = format!("{}{}", LAND_STAKE_PREFIX, addr);
    if let Ok(Some(bytes)) = db.get(key.as_bytes()) {
        if bytes.len() >= 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes[..16]);
            return u128::from_be_bytes(arr);
        }
    }
    0
}

/// Set the staked LAND amount for an address (internal helper)
fn set_stake_internal(db: &Db, addr: &str, amount: u128) -> Result<()> {
    let key = format!("{}{}", LAND_STAKE_PREFIX, addr);
    let bytes = amount.to_be_bytes();
    db.insert(key.as_bytes(), &bytes[..])?;

    // Update owner index
    rebuild_owner_weights(db)?;

    Ok(())
}

/// Get total staked LAND across all addresses
pub fn total_stake(db: &Db) -> u128 {
    if let Ok(Some(bytes)) = db.get(LAND_STAKE_TOTAL_KEY.as_bytes()) {
        if bytes.len() >= 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes[..16]);
            return u128::from_be_bytes(arr);
        }
    }
    0
}

/// Set total staked LAND (internal helper)
fn set_total_stake(db: &Db, amount: u128) -> Result<()> {
    let bytes = amount.to_be_bytes();
    db.insert(LAND_STAKE_TOTAL_KEY.as_bytes(), &bytes[..])?;
    Ok(())
}

/// Stake LAND tokens (lock them for staking rewards)
pub fn stake_land(db: &Db, addr: &str, amount: u128) -> Result<()> {
    if amount == 0 {
        return Err(anyhow::anyhow!("Cannot stake 0 LAND"));
    }

    // TODO: Check that addr has sufficient LAND balance
    // TODO: Lock the LAND tokens (deduct from tradeable balance)

    let current_stake = get_stake(db, addr);
    let new_stake = current_stake
        .checked_add(amount)
        .ok_or_else(|| anyhow::anyhow!("Stake overflow"))?;

    set_stake_internal(db, addr, new_stake)?;

    let total = total_stake(db);
    let new_total = total
        .checked_add(amount)
        .ok_or_else(|| anyhow::anyhow!("Total stake overflow"))?;
    set_total_stake(db, new_total)?;

    tracing::info!(
        "🔒 Staked {} LAND for address {}, total stake: {}",
        amount,
        addr,
        new_stake
    );

    Ok(())
}

/// Unstake LAND tokens (unlock them)
pub fn unstake_land(db: &Db, addr: &str, amount: u128) -> Result<()> {
    if amount == 0 {
        return Err(anyhow::anyhow!("Cannot unstake 0 LAND"));
    }

    let current_stake = get_stake(db, addr);
    if current_stake < amount {
        return Err(anyhow::anyhow!(
            "Insufficient stake: has {}, trying to unstake {}",
            current_stake,
            amount
        ));
    }

    let new_stake = current_stake - amount;
    set_stake_internal(db, addr, new_stake)?;

    let total = total_stake(db);
    let new_total = total.saturating_sub(amount);
    set_total_stake(db, new_total)?;

    // TODO: Unlock the LAND tokens (return to tradeable balance)

    tracing::info!(
        "🔓 Unstaked {} LAND for address {}, remaining stake: {}",
        amount,
        addr,
        new_stake
    );

    Ok(())
}

/// Get list of all stakers (for reward distribution)
pub fn get_all_stakers(db: &Db) -> Vec<String> {
    let mut stakers = Vec::new();

    // Scan all stake entries
    for (key, value) in db.scan_prefix(LAND_STAKE_PREFIX.as_bytes()).flatten() {
        if value.len() >= 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&value[..16]);
            let stake = u128::from_be_bytes(arr);

            if stake > 0 {
                let key_str = String::from_utf8_lossy(&key);
                let addr = key_str[LAND_STAKE_PREFIX.len()..].to_string();
                stakers.push(addr);
            }
        }
    }

    stakers
}

/// Rebuild the owner weights index (called after stake changes)
pub fn rebuild_owner_weights(db: &Db) -> Result<()> {
    let stakers = get_all_stakers(db);
    let owners_list = stakers.join(",");

    db.insert(LAND_STAKE_OWNERS_KEY.as_bytes(), owners_list.as_bytes())?;

    Ok(())
}

/// Check if an address has any LAND stake
pub fn has_stake(db: &Db, addr: &str) -> bool {
    get_stake(db, addr) > 0
}

/// Legacy compatibility: stake_weight is an alias for get_stake
pub fn stake_weight(db: &Db, addr: &str) -> u128 {
    get_stake(db, addr)
}

/// Legacy compatibility: total_weight is an alias for total_stake
pub fn total_weight(db: &Db) -> u128 {
    total_stake(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_unstake_flow() {
        let db = sled::Config::new().temporary(true).open().unwrap();

        // Initial state
        assert_eq!(get_stake(&db, "alice"), 0);
        assert_eq!(total_stake(&db), 0);

        // Stake some LAND
        stake_land(&db, "alice", 1000).unwrap();
        assert_eq!(get_stake(&db, "alice"), 1000);
        assert_eq!(total_stake(&db), 1000);

        // Stake more
        stake_land(&db, "alice", 500).unwrap();
        assert_eq!(get_stake(&db, "alice"), 1500);
        assert_eq!(total_stake(&db), 1500);

        // Another staker
        stake_land(&db, "bob", 2000).unwrap();
        assert_eq!(get_stake(&db, "bob"), 2000);
        assert_eq!(total_stake(&db), 3500);

        // Unstake
        unstake_land(&db, "alice", 500).unwrap();
        assert_eq!(get_stake(&db, "alice"), 1000);
        assert_eq!(total_stake(&db), 3000);

        // Get all stakers
        let stakers = get_all_stakers(&db);
        assert_eq!(stakers.len(), 2);
        assert!(stakers.contains(&"alice".to_string()));
        assert!(stakers.contains(&"bob".to_string()));
    }
}
