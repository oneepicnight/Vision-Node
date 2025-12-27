//! Era Status API Routes
//!
//! Provides visibility into the current chain era (Mining vs Staking)
//! and user-specific information about eligibility and rewards.

use axum::{response::IntoResponse, Json};
use serde_json::json;

/// GET /status/era
/// Returns current era, emission progress, and user guidance
pub async fn get_era_status() -> impl IntoResponse {
    let chain = crate::CHAIN.lock();

    let current_era = chain.current_era();
    let total_supply = chain.total_supply();
    let emission_progress = chain.emission_progress();
    let emissions_remaining = chain.emissions_remaining();
    let is_mining_era = current_era.is_mining();
    let is_staking_era = current_era.is_staking();

    // Get staking info
    let stakers = crate::land_stake::get_all_stakers(&chain.db);
    let total_staked = crate::land_stake::total_stake(&chain.db);
    let staker_count = stakers.len();

    // Era-specific message
    let message = if is_mining_era {
        if emission_progress < 0.9 {
            format!(
                "Mining era active. {:.1}% complete. {} LAND remaining to mint.",
                emission_progress * 100.0,
                emissions_remaining as f64 / 1_000_000_000.0
            )
        } else {
            "Mining era nearing completion. Prepare for staking era transition.".to_string()
        }
    } else {
        format!(
            "Staking era active. {} guardian nodes earning rewards. Mining disabled.",
            staker_count
        )
    };

    // User guidance based on era
    let guidance = if is_mining_era {
        json!({
            "mining": "Mine LAND tokens by solving blocks",
            "preparing": "Consider staking LAND to prepare for staking era",
            "next_era": "When emissions complete, all stakers become guardians"
        })
    } else {
        json!({
            "staking": "Stake LAND to join the guardian mesh and earn rewards",
            "rewards": format!("{} LAND base reward + fees per block", crate::chain_era::STAKING_BASE_REWARD as f64 / 1_000_000_000.0),
            "mining": "Mining is disabled in staking era"
        })
    };

    Json(json!({
        "era": current_era.as_str(),
        "is_mining": is_mining_era,
        "is_staking": is_staking_era,
        "message": message,
        "supply": {
            "total": total_supply,
            "total_land": format!("{:.2}", total_supply as f64 / 1_000_000_000.0),
            "max_supply": crate::chain_era::MAX_SUPPLY,
            "max_supply_land": format!("{:.2}", crate::chain_era::MAX_SUPPLY as f64 / 1_000_000_000.0),
            "progress": format!("{:.2}%", emission_progress * 100.0),
            "remaining": emissions_remaining,
            "remaining_land": format!("{:.2}", emissions_remaining as f64 / 1_000_000_000.0),
        },
        "staking": {
            "active_stakers": staker_count,
            "total_staked": total_staked,
            "total_staked_land": format!("{:.2}", total_staked as f64 / 1_000_000_000.0),
            "base_reward_per_block": crate::chain_era::STAKING_BASE_REWARD,
            "base_reward_land": format!("{:.2}", crate::chain_era::STAKING_BASE_REWARD as f64 / 1_000_000_000.0),
        },
        "guidance": guidance,
    }))
}

/// GET /my/era
/// Returns era status for a specific wallet address (pass ?addr=xxx)
pub async fn get_my_era_status(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let chain = crate::CHAIN.lock();

    // Get wallet address from query parameter
    let wallet_addr = match params.get("addr") {
        Some(addr) => addr.clone(),
        None => {
            return Json(json!({
                "error": "Wallet address required. Pass ?addr=xxx to see personalized status.",
                "example": "/my/era?addr=abc123"
            }));
        }
    };

    let current_era = chain.current_era();
    let my_stake = crate::land_stake::get_stake(&chain.db, &wallet_addr);
    let has_stake = my_stake > 0;
    let can_stake = true; // Anyone can stake (just need LAND balance)

    let is_mining_era = current_era.is_mining();
    let is_staking_era = current_era.is_staking();

    // Personalized message
    let message = if is_mining_era {
        if has_stake {
            format!(
                "You're mining and have {} LAND staked. When emissions complete, you'll automatically join the guardian mesh.",
                my_stake as f64 / 1_000_000_000.0
            )
        } else {
            "You're mining LAND. Consider staking to prepare for the staking era.".to_string()
        }
    } else {
        // Staking era
        if has_stake {
            format!(
                "🎉 Welcome to the future! Your node is a guardian with {} LAND staked. You're earning staking rewards.",
                my_stake as f64 / 1_000_000_000.0
            )
        } else {
            "Mining has ended. Stake LAND to join the guardian mesh and earn rewards.".to_string()
        }
    };

    // Calculate estimated rewards per block (in staking era)
    let estimated_reward_per_block = if is_staking_era && has_stake {
        let stakers = crate::land_stake::get_all_stakers(&chain.db);
        if !stakers.is_empty() {
            crate::chain_era::STAKING_BASE_REWARD / stakers.len() as u128
        } else {
            0
        }
    } else {
        0
    };

    Json(json!({
        "wallet_address": wallet_addr,
        "era": current_era.as_str(),
        "message": message,
        "my_stake": {
            "has_stake": has_stake,
            "can_stake": can_stake,
            "amount": my_stake,
            "amount_land": format!("{:.2}", my_stake as f64 / 1_000_000_000.0),
        },
        "rewards": {
            "is_earning": is_staking_era && has_stake,
            "estimated_per_block": estimated_reward_per_block,
            "estimated_per_block_land": format!("{:.4}", estimated_reward_per_block as f64 / 1_000_000_000.0),
        },
        "actions": if !has_stake && is_staking_era {
            vec!["Stake LAND to join the guardian mesh"]
        } else if !has_stake && is_mining_era {
            vec!["Stake LAND to prepare for staking era"]
        } else {
            vec![]
        },
    }))
}
