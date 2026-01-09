#![allow(dead_code)]
//! Pool payout calculation logic

use super::state::PoolState;

/// Payout entry: (wallet_address, amount_in_base_units)
pub type PayoutEntry = (String, u128);

/// Distribute payouts via proper transactions (future implementation)
///
/// TODO: This should create a multi-output transaction from the pool's wallet
/// For now, we use direct balance updates for simplicity
pub async fn distribute_pool_payouts_via_tx(
    payouts: Vec<PayoutEntry>,
    _pool_wallet: &str,
) -> Result<(), String> {
    // TODO: Implement proper transaction-based payouts
    // Steps:
    // 1. Build a multi-output LAND transfer transaction
    // 2. Sign with pool operator's key
    // 3. Submit to mempool
    // 4. Wait for confirmation

    // For now, fall back to direct balance updates
    distribute_pool_payouts_direct(payouts)
}

/// Distribute payouts by directly updating balances (current implementation)
///
/// NOTE: This bypasses the transaction system for speed, but loses auditability.
/// Should be replaced with proper transactions in production.
pub fn distribute_pool_payouts_direct(payouts: Vec<PayoutEntry>) -> Result<(), String> {
    // Access DB through the global CHAIN
    let chain = crate::CHAIN.lock();
    let db = &chain.db;

    let balances = db
        .open_tree("balances")
        .map_err(|e| format!("Failed to open balances tree: {}", e))?;

    // Update each recipient's balance
    for (address, amount) in payouts {
        // Read current balance
        let current = balances
            .get(address.as_bytes())
            .map_err(|e| format!("Failed to read balance: {}", e))?
            .map(|v| {
                let mut buf = [0u8; 16];
                let take = v.len().min(16);
                buf[..take].copy_from_slice(&v[..take]);
                u128::from_le_bytes(buf)
            })
            .unwrap_or(0);

        // Write new balance
        let new_balance = current + amount;
        balances
            .insert(address.as_bytes(), new_balance.to_le_bytes().to_vec())
            .map_err(|e| format!("Failed to write balance: {}", e))?;

        tracing::debug!(
            "💰 Payout: {} -> {} LAND",
            address,
            amount as f64 / 100_000_000.0
        );
    }

    // Flush to disk
    balances
        .flush()
        .map_err(|e| format!("Failed to flush balances: {}", e))?;

    Ok(())
}

/// Compute pool payouts after a block is found
///
/// Takes the total miner reward (after protocol fee has been deducted),
/// deducts pool and foundation fees, then splits remaining among workers
/// proportionally based on their share contributions.
pub fn compute_pool_payouts(
    pool_state: &PoolState,
    total_miner_reward: u128,
) -> Result<Vec<PayoutEntry>, String> {
    let mut payouts = Vec::new();

    // Get all workers and total shares
    let workers = pool_state.get_workers();
    let total_shares = pool_state.get_total_shares();

    if total_shares == 0 {
        return Err("No shares recorded".to_string());
    }

    // Calculate fees
    let foundation_cut =
        (total_miner_reward * pool_state.config.foundation_fee_bps as u128) / 10_000u128;
    let pool_cut = (total_miner_reward * pool_state.config.pool_fee_bps as u128) / 10_000u128;

    // Remaining reward pool to split among workers
    let payout_pool = total_miner_reward
        .saturating_sub(foundation_cut)
        .saturating_sub(pool_cut);

    // Add foundation fee payout
    if foundation_cut > 0 {
        payouts.push((pool_state.config.foundation_address.clone(), foundation_cut));
    }

    // Add pool fee payout to host
    if pool_cut > 0 && !pool_state.config.host_address.is_empty() {
        payouts.push((pool_state.config.host_address.clone(), pool_cut));
    }

    // Calculate worker payouts proportionally
    let mut total_distributed = foundation_cut + pool_cut;

    for worker in &workers {
        if worker.total_shares == 0 {
            continue;
        }

        // Proportional share: (worker_shares / total_shares) * payout_pool
        let worker_payout = (worker.total_shares as u128 * payout_pool) / total_shares as u128;

        if worker_payout > 0 {
            payouts.push((worker.wallet_address.clone(), worker_payout));
            total_distributed += worker_payout;
        }
    }

    // Handle any rounding dust - send it to foundation
    let dust = total_miner_reward.saturating_sub(total_distributed);
    if dust > 0 {
        // Add dust to foundation's payout
        if let Some(foundation_entry) = payouts
            .iter_mut()
            .find(|(addr, _)| addr == &pool_state.config.foundation_address)
        {
            foundation_entry.1 += dust;
        } else {
            payouts.push((pool_state.config.foundation_address.clone(), dust));
        }
    }

    Ok(payouts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::{PoolConfig, PoolState};

    #[test]
    fn test_payout_calculation() {
        let mut config = PoolConfig::default();
        config.host_address = "host123".to_string();
        config.pool_fee_bps = 200; // 2%
        config.foundation_fee_bps = 100; // 1%

        let pool = PoolState::new(config);

        // Register workers
        pool.register_worker("worker1".to_string(), "addr1".to_string(), None)
            .unwrap();
        pool.register_worker("worker2".to_string(), "addr2".to_string(), None)
            .unwrap();

        // Record shares: worker1 has 70%, worker2 has 30%
        pool.record_share("worker1", 70).unwrap();
        pool.record_share("worker2", 30).unwrap();

        // Total miner reward: 32 LAND = 3,200,000,000 base units
        let total_reward = 3_200_000_000u128;

        let payouts = compute_pool_payouts(&pool, total_reward).unwrap();

        // Verify fees
        let foundation_fee = (total_reward * 100) / 10_000; // 1%
        let pool_fee = (total_reward * 200) / 10_000; // 2%
        let payout_pool = total_reward - foundation_fee - pool_fee;

        // Verify worker payouts
        let worker1_expected = (70 * payout_pool) / 100;
        let worker2_expected = (30 * payout_pool) / 100;

        // Check that all payouts add up to total reward
        let total_paid: u128 = payouts.iter().map(|(_, amount)| amount).sum();
        assert_eq!(
            total_paid, total_reward,
            "Total payouts should equal total reward"
        );

        // Foundation should get 1% + dust
        let foundation_payout = payouts
            .iter()
            .find(|(addr, _)| addr == &crate::vision_constants::vault_address())
            .map(|(_, amt)| *amt)
            .unwrap_or(0);
        assert!(
            foundation_payout >= foundation_fee,
            "Foundation should get at least 1%"
        );

        // Host should get 2%
        let host_payout = payouts
            .iter()
            .find(|(addr, _)| addr == "host123")
            .map(|(_, amt)| *amt)
            .unwrap_or(0);
        assert_eq!(host_payout, pool_fee, "Host should get 2%");
    }
}
