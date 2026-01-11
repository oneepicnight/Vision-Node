#![allow(dead_code)]

use sled::{Db, IVec};

use crate::vision_constants::{
    founder_inventory_address_for_deed, LAND_DEED_PREFIX, META_LAND_GENESIS_DONE,
};

// Reverse index: "land:deed:by-owner:<addr>" => deed_id (u64 BE)
const LAND_DEED_OWNER_INDEX: &str = "land:deed:by-owner:";

/// Sled tree for land deed ownership index
pub const LAND_DEEDS_TREE: &str = "land_deeds";
/// Tier state tree
const LAND_TIERS_TREE: &str = "land_tiers";

pub fn genesis_land_deed_total() -> u64 {
    std::env::var("VISION_GENESIS_LAND_DEED_TOTAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1_000_000u64)
}

/// Determine 10 deed price tiers spanning from 100,000 to 5,000,000 LAND.
/// Returns the tier index (0..=9) for a given deed id.
fn deed_tier_for_id(deed_id: u64) -> u8 {
    let idx = (deed_id.saturating_sub(1)) / 100_000; // 100k deeds per tier
    if idx >= 9 {
        9
    } else {
        idx as u8
    }
}

/// Get the LAND price (human units) for a given tier index (0..=9), linearly spaced
/// from 100,000 to 5,000,000 LAND. Uses integer math so final tier equals exactly 5,000,000.
fn deed_price_land_for_tier_human(tier: u8) -> u64 {
    let start: u64 = 100_000;
    let end: u64 = 5_000_000;
    let t = tier as u64;
    start + ((end - start) * t) / 9
}

/// Get the LAND price for a deed id in atomic units (1e8 per coin)
fn deed_price_land_units_for_id(deed_id: u64) -> u128 {
    let tier = deed_tier_for_id(deed_id);
    let human = deed_price_land_for_tier_human(tier) as u128;
    human * 100_000_000u128
}

/// Initialize tier schedule and next-id pointers for stock-by-tier mode
fn init_tier_schedule(db: &Db) -> Result<(), String> {
    let tree = db.open_tree(LAND_TIERS_TREE).map_err(|e| e.to_string())?;
    // Set current tier to 0
    tree.insert("land:tiers:current".as_bytes(), IVec::from(&[0u8][..]))
        .map_err(|e| e.to_string())?;
    // Set range and next-id for each tier
    for t in 0..10u64 {
        let start = t * 100_000 + 1;
        let end = (t + 1) * 100_000;
        let mut buf8 = [0u8; 8];
        buf8.copy_from_slice(&start.to_be_bytes());
        tree.insert(
            format!("land:tiers:range_start:{}", t).as_bytes(),
            IVec::from(&buf8[..]),
        )
        .map_err(|e| e.to_string())?;
        buf8.copy_from_slice(&end.to_be_bytes());
        tree.insert(
            format!("land:tiers:range_end:{}", t).as_bytes(),
            IVec::from(&buf8[..]),
        )
        .map_err(|e| e.to_string())?;
        buf8.copy_from_slice(&start.to_be_bytes());
        tree.insert(
            format!("land:tiers:next_id:{}", t).as_bytes(),
            IVec::from(&buf8[..]),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_u64(tree: &sled::Tree, key: &str) -> Result<u64, String> {
    let Some(val) = tree.get(key.as_bytes()).map_err(|e| e.to_string())? else {
        return Err(format!("missing key {}", key));
    };
    if val.len() < 8 {
        return Err("invalid u64 bytes".to_string());
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&val[..8]);
    Ok(u64::from_be_bytes(arr))
}

/// Stock deed inventory by tier: mint `count` deeds starting from next_id and assign to founder inventory
pub fn stock_deeds_by_tier(db: &Db, tier: u8, count: u64) -> Result<u64, String> {
    let tree = db.open_tree(LAND_TIERS_TREE).map_err(|e| e.to_string())?;
    let t = tier as u64;
    let mut next_id = read_u64(&tree, &format!("land:tiers:next_id:{}", t))?;
    let end = read_u64(&tree, &format!("land:tiers:range_end:{}", t))?;
    let mut minted = 0u64;

    while minted < count && next_id <= end {
        let key = format!("{}{}", LAND_DEED_PREFIX, next_id);
        let owner = founder_inventory_address_for_deed(next_id);
        // Mint: set owner
        db.insert(key.as_bytes(), IVec::from(owner.as_bytes()))
            .map_err(|e| e.to_string())?;
        // Guard: ensure founders
        let f1 = crate::foundation_config::founder1_address();
        let f2 = crate::foundation_config::founder2_address();
        if !(owner == f1 || owner == f2) {
            return Err("deed mints must go to founders".to_string());
        }
        // Index owner (first-deed per owner only)
        let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, owner);
        match db.get(owner_key.as_bytes()) {
            Ok(Some(_)) => {}
            Ok(None) => {
                let bytes = next_id.to_be_bytes();
                db.insert(owner_key.as_bytes(), IVec::from(&bytes[..]))
                    .map_err(|e| e.to_string())?;
            }
            Err(e) => return Err(e.to_string()),
        }
        minted += 1;
        next_id += 1;
    }
    // Persist updated next_id
    let mut buf8 = [0u8; 8];
    buf8.copy_from_slice(&next_id.to_be_bytes());
    tree.insert(
        format!("land:tiers:next_id:{}", t).as_bytes(),
        IVec::from(&buf8[..]),
    )
    .map_err(|e| e.to_string())?;

    tracing::info!(
        tier = (tier + 1),
        minted_count = minted,
        "Stocked deeds for tier"
    );
    Ok(minted)
}

/// If current tier exhausted, advance to next tier and auto-stock 100k deeds
pub fn advance_tier_if_exhausted(db: &Db) -> Result<(), String> {
    let tree = db.open_tree(LAND_TIERS_TREE).map_err(|e| e.to_string())?;
    let mut cur = read_u64(&tree, "land:tiers:current")?;
    let next_id = read_u64(&tree, &format!("land:tiers:next_id:{}", cur))?;
    let end = read_u64(&tree, &format!("land:tiers:range_end:{}", cur))?;
    if next_id > end {
        if cur < 9 {
            cur += 1;
            tree.insert(
                "land:tiers:current".as_bytes(),
                IVec::from(&[cur as u8][..]),
            )
            .map_err(|e| e.to_string())?;
            tracing::info!(
                tier = (cur + 1),
                "Advanced to next deed tier, auto-stocking 100k deeds"
            );
            // Auto-stock 100k deeds in the new tier
            if let Err(e) = stock_deeds_by_tier(db, cur as u8, 100_000) {
                tracing::error!("Failed to auto-stock tier {}: {}", cur + 1, e);
            }
        } else {
            tracing::info!("All deed tiers exhausted");
        }
    }
    Ok(())
}

/// Mint genesis land deeds to founder inventory addresses.
/// Policy: even deed ids -> founder1, odd deed ids -> founder2 (if configured; else founder1).
/// Ids will be `1..=count`.
/// This function will ensure it only runs once by checking/setting the
/// `META_LAND_GENESIS_DONE` DB flag. Returns Ok(minted_count) or Err.
pub fn mint_genesis_deeds(db: &Db) -> Result<u64, String> {
    // If already done, no-op
    match db.get(META_LAND_GENESIS_DONE.as_bytes()) {
        Ok(Some(_)) => return Ok(0),
        Ok(None) => (),
        Err(e) => return Err(format!("db error: {}", e)),
    }
    // Optional: stock-by-tier mode to avoid mass mint at genesis
    if std::env::var("VISION_STOCK_BY_TIER")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        init_tier_schedule(db)?;
        tracing::info!("Initialized tier schedule; skipping mass genesis mint");
        db.insert(META_LAND_GENESIS_DONE.as_bytes(), IVec::from(&[1u8][..]))
            .map_err(|e| e.to_string())?;
        let _ = db.flush();
        return Ok(0);
    }
    let deeds_total = genesis_land_deed_total();
    if deeds_total == 0 {
        // ensure 0 is not allowed
        return Err("deed total cannot be 0".to_string());
    }

    let mut tier_counts: [u64; 10] = [0; 10];
    for i in 1..=deeds_total {
        let key = format!("{}{}", LAND_DEED_PREFIX, i);
        // Determine founder inventory address based on deed id
        let owner = founder_inventory_address_for_deed(i);
        let tier = deed_tier_for_id(i);
        let price_units = deed_price_land_units_for_id(i);
        // Insert owner address bytes
        if let Err(e) = db.insert(key.as_bytes(), IVec::from(owner.as_bytes())) {
            return Err(format!("failed to insert deed {}: {}", i, e));
        }

        // Paranoia guard: ensure mints go only to founder inventory addresses
        let f1 = crate::foundation_config::founder1_address();
        let f2 = crate::foundation_config::founder2_address();
        if !(owner == f1 || owner == f2) {
            return Err("deed mints must go to founders".to_string());
        }

        // Update owner index for the first deed per owner to avoid duplicates
        let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, owner);
        match db.get(owner_key.as_bytes()) {
            Ok(Some(_)) => { /* already indexed, skip */ }
            Ok(None) => {
                let deed_id_bytes = i.to_be_bytes();
                if let Err(e) = db.insert(owner_key.as_bytes(), IVec::from(&deed_id_bytes[..])) {
                    return Err(format!(
                        "failed to update owner index for deed {}: {}",
                        i, e
                    ));
                }
            }
            Err(e) => return Err(format!("failed to read owner index for deed {}: {}", i, e)),
        }

        // Track counts per tier
        tier_counts[tier as usize] += 1;

        // Periodic flush + log
        if i % 100_000 == 0 && i != 0 {
            let _ = db.flush();
            let human_price = price_units / 100_000_000u128;
            tracing::info!(
                minted = i,
                owner = %owner,
                tier = (tier + 1),
                price_land = human_price,
                "genesis deeds minted in current tier"
            );
        } else {
            // Light log for early items to aid genesis verification
            if i <= 20 {
                let human_price = price_units / 100_000_000u128;
                tracing::info!(
                    deed_id = i,
                    owner = %owner,
                    tier = (tier + 1),
                    price_land = human_price,
                    "Minted deed"
                );
            }
        }
    }

    // Final summary: print tier breakdown and prices
    for t in 0..10u8 {
        let price = deed_price_land_for_tier_human(t);
        let count = tier_counts[t as usize];
        tracing::info!(
            tier = (t + 1),
            price_land = price,
            deeds = count,
            "Genesis deed tier summary"
        );
    }

    // Mark as done
    if let Err(e) = db.insert(META_LAND_GENESIS_DONE.as_bytes(), IVec::from(&[1u8][..])) {
        return Err(format!("failed to set genesis done flag: {}", e));
    }
    let _ = db.flush();
    Ok(deeds_total)
}

/// Check if a wallet owns at least one deed
pub fn wallet_has_deed(db: &Db, addr: &str) -> bool {
    // Normal deed check
    let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, addr);
    db.get(owner_key.as_bytes()).ok().flatten().is_some()
}

/// Get the deed ID owned by an address (if any)
pub fn get_owned_deed_id(db: &Db, addr: &str) -> Option<u64> {
    let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, addr);
    if let Ok(Some(bytes)) = db.get(owner_key.as_bytes()) {
        if bytes.len() >= 8 {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[..8]);
            return Some(u64::from_be_bytes(arr));
        }
    }
    None
}

/// List all addresses that appear in the deed owner index.
pub fn list_deed_owners(db: &Db) -> Vec<String> {
    let mut owners = Vec::new();

    for (key, _value) in db.scan_prefix(LAND_DEED_OWNER_INDEX.as_bytes()).flatten() {
        let key_str = String::from_utf8_lossy(&key);
        // Extract address from key: "land:deed:by-owner:<addr>"
        if let Some(addr) = key_str.strip_prefix(LAND_DEED_OWNER_INDEX) {
            owners.push(addr.to_string());
        }
    }

    owners
}

/// Public entry point: check tier exhaustion and auto-print next tier if needed
/// Called after a deed is purchased/transferred out of the market inventory
pub fn check_and_auto_stock_next_tier(db: &Db) -> Result<(), String> {
    advance_tier_if_exhausted(db)
}

/// Back-compat alias used by older modules.
pub fn all_deed_owners(db: &Db) -> Vec<String> {
    list_deed_owners(db)
}

/// Remove owner index entry for a given address
fn remove_owner_index(db: &Db, addr: &str) -> Result<(), String> {
    let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, addr);
    db.remove(owner_key.as_bytes())
        .map_err(|e| format!("failed to remove owner index: {}", e))?;
    Ok(())
}

/// Update owner index when deed ownership changes (atomic: remove old, add new)
/// Enforces one-deed-per-wallet: rejects if new_owner already owns a different deed
/// Call this whenever a deed is transferred
pub fn update_owner_index(
    db: &Db,
    deed_id: u64,
    old_owner: &str,
    new_owner: &str,
) -> Result<(), String> {
    // Check if new owner already has a deed
    if let Some(existing_deed) = get_owned_deed_id(db, new_owner) {
        if existing_deed != deed_id {
            return Err(format!(
                "Wallet {} already owns deed {}. One deed per wallet enforced.",
                new_owner, existing_deed
            ));
        }
    }

    // Remove old owner's index
    remove_owner_index(db, old_owner)?;

    // Insert new owner's index
    let owner_key = format!("{}{}", LAND_DEED_OWNER_INDEX, new_owner);
    let deed_id_bytes = deed_id.to_be_bytes();
    db.insert(owner_key.as_bytes(), IVec::from(&deed_id_bytes[..]))
        .map_err(|e| format!("failed to update owner index: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sled::Config;
    use tempfile::TempDir;

    #[test]
    #[ignore]
    fn heavy_mint_1m() {
        // WARNING: heavy test - inserts 1,000,000 deeds
        let tmp = TempDir::new().unwrap();
        let db = Config::new().temporary(true).open().unwrap();
        std::env::set_var("VISION_GENESIS_LAND_DEED_TOTAL", "1000000");
        let minted = mint_genesis_deeds(&db).expect("mint should succeed");
        assert_eq!(minted, 1_000_000);
    }

    #[test]
    fn mint_small() {
        let db = Config::new().temporary(true).open().unwrap();
        std::env::set_var("VISION_GENESIS_LAND_DEED_TOTAL", "10");
        let minted = mint_genesis_deeds(&db).expect("mint should succeed");
        assert_eq!(minted, 10);
        // verify keys exist
        for i in 1..=10u64 {
            let k = format!("{}{}", LAND_DEED_PREFIX, i);
            let val = db.get(k.as_bytes()).unwrap();
            assert!(val.is_some());
        }
        // second call should be no-op
        let minted2 = mint_genesis_deeds(&db).unwrap();
        assert_eq!(minted2, 0);
    }

    #[test]
    fn default_total_is_one_million() {
        std::env::remove_var("VISION_GENESIS_LAND_DEED_TOTAL");
        assert_eq!(genesis_land_deed_total(), 1_000_000u64);
    }
}
