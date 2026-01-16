// Staking Rewards Engine - distributes rewards to LAND stakers post-mining era
// Even-split model: reward_pool divided equally among deed owners, dust returns to Vault
use sled::Db;
use anyhow::Result;
use crate::vision_constants::{STAKING_REWARD_LAND, land_amount};
use crate::land_deeds::all_deed_owners;

const VAULT_BALANCE_KEY: &str = "supply:vault"; // u128 BE

/// Get the base staking reward per block (4.25 LAND in base units)
pub fn staking_base_reward() -> u128 {
    land_amount(STAKING_REWARD_LAND)
}

/// Get vault balance from DB
fn get_vault_balance(db: &Db) -> u128 {
    if let Ok(Some(bytes)) = db.get(VAULT_BALANCE_KEY.as_bytes()) {
        if bytes.len() >= 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes[..16]);
            return u128::from_be_bytes(arr);
        }
    }
    0
}

/// Debit from vault (helper function)
fn debit_vault(db: &Db, amount: u128) -> Result<()> {
    let current = get_vault_balance(db);
    if current < amount {
        return Err(anyhow::anyhow!("Insufficient vault balance: {} < {}", current, amount));
    }
    
    let new_balance = current - amount;
    let bytes = new_balance.to_be_bytes();
    db.insert(VAULT_BALANCE_KEY.as_bytes(), &bytes[..])?;
    
    Ok(())
}

/// Credit to vault (helper function)
fn credit_vault(db: &Db, amount: u128) -> Result<()> {
    let current = get_vault_balance(db);
    let new_balance = current.checked_add(amount)
        .ok_or_else(|| anyhow::anyhow!("Vault balance overflow"))?;
    
    let bytes = new_balance.to_be_bytes();
    db.insert(VAULT_BALANCE_KEY.as_bytes(), &bytes[..])?;
    
    Ok(())
}

/// Add balance to an address (helper)
fn add_balance(db: &Db, addr: &str, amount: u128) -> Result<()> {
    let bal_key = format!("bal:{}", addr);
    
    let current = if let Ok(Some(bytes)) = db.get(bal_key.as_bytes()) {
        if bytes.len() >= 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes[..16]);
            u128::from_be_bytes(arr)
        } else {
            0
        }
    } else {
        0
    };
    
    let new_balance = current.checked_add(amount)
        .ok_or_else(|| anyhow::anyhow!("Balance overflow"))?;
    
    let bytes = new_balance.to_be_bytes();
    db.insert(bal_key.as_bytes(), &bytes[..])?;
    
    Ok(())
}

/// Distribute staking rewards using even-split among deed owners
/// In staking phase:
/// - base_reward (4.25 LAND) comes from Vault
/// - fees_for_stakers are the block's usage fees assigned to stakers
/// - reward_pool is split evenly across all deed-owning wallets (staked nodes)
/// - any dust returns to the Vault
pub fn distribute_staking_rewards(db: &Db, fees_for_stakers: u128) -> Result<()> {
    // 1. Determine all staked nodes (deed owners)
    let stakers = all_deed_owners(db);
    let n = stakers.len();
    
    if n == 0 {
        // No one to pay; keep everything in vault (fees + potential base reward)
        tracing::debug!("No deed owners for staking rewards, skipping distribution");
        return Ok(());
    }
    
    // 2. Compute base_reward and total reward pool
    let mut base_reward = staking_base_reward();
    
    
    // Check vault can cover base_reward
    let vault_balance = get_vault_balance(db);
    if vault_balance < base_reward {
        // If vault can't fully cover, cap base_reward to what's left
        tracing::warn!(
            "⚠️ Vault balance ({}) below base reward ({}), capping to available funds",
            vault_balance,
            base_reward
        );
        base_reward = vault_balance;
    }
    
    let reward_pool = base_reward.saturating_add(fees_for_stakers);
    
    if reward_pool == 0 {
        return Ok(());
    }
    
    // 3. Compute per-node share + dust
    let n_u128 = n as u128;
    let per_node = reward_pool / n_u128;
    let dust = reward_pool - (per_node * n_u128);
    
    if per_node == 0 {
        // reward_pool too small to split fairly; send all to vault
        // to accumulate for future bigger payouts
        tracing::debug!(
            "Reward pool ({}) too small to split among {} nodes, keeping in vault",
            reward_pool,
            n
        );
        return Ok(());
    }
    
    tracing::debug!(
        "staking_rewards: stakers={}, reward_pool={}, per_node={}, dust={}",
        n,
        reward_pool,
        per_node,
        dust
    );
    
    // 4. Debit vault for base_reward only (fees_for_stakers already collected separately)
    if base_reward > 0 {
        debit_vault(db, base_reward)?;
    }
    
    // 5. Credit each staker equally
    for addr in stakers.iter() {
        add_balance(db, addr, per_node)?;
    }
    
    // 6. Send dust back to vault
    if dust > 0 {
        credit_vault(db, dust)?;
    }
    
    tracing::info!(
        "💰 Distributed {} LAND evenly to {} deed owners ({} per node, {} dust to vault)",
        reward_pool - dust,
        n,
        per_node,
        dust
    );
    
    Ok(())
}

/// Check if vault has sufficient balance for staking rewards
pub fn check_vault_sustainability(db: &Db, blocks_remaining: u64) -> bool {
    let vault_balance = get_vault_balance(db);
    let reward_per_block = staking_base_reward();
    let required = reward_per_block.saturating_mul(blocks_remaining as u128);
    
    vault_balance >= required
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::land_deeds;
    
    #[test]
    fn test_even_split_staking_rewards() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        
        // Set up vault with 10,000 LAND
        let vault_init = land_amount(10000.0);
        let bytes = vault_init.to_be_bytes();
        db.insert(VAULT_BALANCE_KEY.as_bytes(), &bytes[..]).unwrap();
        
        // Set up deed owners (3 wallets each own a deed)
        land_deeds::update_owner_index(&db, 1, "alice").unwrap();
        land_deeds::update_owner_index(&db, 2, "bob").unwrap();
        land_deeds::update_owner_index(&db, 3, "carol").unwrap();
        
        // Verify 3 deed owners
        let owners = land_deeds::all_deed_owners(&db);
        assert_eq!(owners.len(), 3);
        
        // Distribute rewards (base 4.25 LAND + 0 fees)
        // reward_pool = 4.25 LAND
        // per_node = 4.25 / 3 = 1.4166666... (integer division)
        // dust = 4.25 - (1.4166666 * 3)
        distribute_staking_rewards(&db, 0).unwrap();
        
        // Check balances (should receive equal shares)
        let base_reward = staking_base_reward();
        let per_node = base_reward / 3;
        let dust = base_reward - (per_node * 3);
        
        for addr in &["alice", "bob", "carol"] {
            let bal_key = format!("bal:{}", addr);
            let bal = if let Ok(Some(b)) = db.get(bal_key.as_bytes()) {
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&b[..16]);
                u128::from_be_bytes(arr)
            } else { 0 };
            
            // Each should get exactly per_node amount
            assert_eq!(bal, per_node, "{} should receive per_node reward", addr);
        }
        
        // Check vault: should have initial - base_reward + dust
        let vault_bal = get_vault_balance(&db);
        let expected_vault = vault_init - base_reward + dust;
        assert_eq!(vault_bal, expected_vault, "Vault should have initial - base + dust");
    }
    
    #[test]
    fn test_no_deed_owners() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        
        // Set up vault
        let vault_init = land_amount(1000.0);
        let bytes = vault_init.to_be_bytes();
        db.insert(VAULT_BALANCE_KEY.as_bytes(), &bytes[..]).unwrap();
        
        // No deed owners
        let owners = land_deeds::all_deed_owners(&db);
        assert_eq!(owners.len(), 0);
        
        // Distribute should be no-op
        distribute_staking_rewards(&db, 0).unwrap();
        
        // Vault should be unchanged
        let vault_bal = get_vault_balance(&db);
        assert_eq!(vault_bal, vault_init);
    }
}
