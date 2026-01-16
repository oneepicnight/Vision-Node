//! Advanced Tokenomics V2 Module
//!
//! This module contains advanced token economics features including:
//! - Token emissions and burning
//! - Treasury management and proposals
//! - Vesting schedules
//! - Staking rewards calculation

use axum::{
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    Json,
};
use parking_lot::Mutex;
use prometheus::{IntCounter, IntGauge};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::BTreeMap;

use crate::{
    acct_key, mk_int_counter, mk_int_gauge, now_ts, u128_from_be, u64_from_be, ProposalStatus,
    ProposalType, CHAIN, PROPOSALS, TOK_CONFIG_KEY, TOK_SUPPLY_BURNED, TOK_SUPPLY_FUND,
    TOK_SUPPLY_TOTAL, TOK_SUPPLY_TREASURY, TOK_SUPPLY_VAULT,
};

// =================== PHASE 8.4: TOKEN ECONOMICS ENGINE ===================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenomicsConfig {
    initial_supply: u128,
    max_supply: u128,
    inflation_rate: f64, // Annual percentage
    deflation_rate: f64, // Burn rate percentage
    staking_reward_rate: f64,
    emission_per_block: u128,
    halving_interval: u64, // Blocks
    fee_burn_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenomicsState {
    current_supply: u128,
    total_burned: u128,
    total_staked: u128,
    total_rewards_distributed: u128,
    last_emission_block: u64,
    current_epoch: u64,
}

impl Default for TokenomicsConfig {
    fn default() -> Self {
        Self {
            initial_supply: 1_000_000_000 * 10u128.pow(18), // 1B tokens
            max_supply: 21_000_000_000 * 10u128.pow(18),    // 21B max
            inflation_rate: 5.0,                            // 5% annual
            deflation_rate: 0.0,
            staking_reward_rate: 10.0,               // 10% APY
            emission_per_block: 50 * 10u128.pow(18), // 50 tokens per block
            halving_interval: 210_000,               // ~4 years at 10s blocks
            fee_burn_percentage: 10.0,               // Burn 10% of fees
        }
    }
}

// Storage prefix
const TOKENOMICS_CONFIG_KEY: &str = "tokenomics:config";
const TOKENOMICS_STATE_KEY: &str = "tokenomics:state";

// Global tokenomics state
static TOKENOMICS_CONFIG: once_cell::sync::Lazy<Mutex<TokenomicsConfig>> =
    once_cell::sync::Lazy::new(|| Mutex::new(TokenomicsConfig::default()));

// Prometheus metrics
static TOKENOMICS_SUPPLY: once_cell::sync::Lazy<IntGauge> =
    once_cell::sync::Lazy::new(|| mk_int_gauge("vision_tokenomics_supply", "Current token supply"));

static TOKENOMICS_BURNED: once_cell::sync::Lazy<IntCounter> = once_cell::sync::Lazy::new(|| {
    mk_int_counter("vision_tokenomics_burned_total", "Total tokens burned")
});

static TOKENOMICS_STAKED: once_cell::sync::Lazy<IntGauge> =
    once_cell::sync::Lazy::new(|| mk_int_gauge("vision_tokenomics_staked", "Total tokens staked"));

static TOKENOMICS_REWARDS: once_cell::sync::Lazy<IntCounter> = once_cell::sync::Lazy::new(|| {
    mk_int_counter("vision_tokenomics_rewards_total", "Total staking rewards")
});

// Apply block emission
pub fn apply_block_emission(
    db: &Db,
    balances: &mut BTreeMap<String, u128>,
    block_height: u64,
    miner: &str,
) -> Result<u128, String> {
    let config = TOKENOMICS_CONFIG.lock().clone();

    // Calculate emission with halving
    let halvings = block_height / config.halving_interval;
    let emission = config.emission_per_block / 2u128.pow(halvings as u32);

    // Load current state
    let mut state = get_tokenomics_state(db)?;

    // Check max supply
    if state.current_supply + emission > config.max_supply {
        return Ok(0); // No more emissions
    }

    // Mint to miner
    let miner_key = acct_key(miner);
    *balances.entry(miner_key).or_insert(0) += emission;

    state.current_supply += emission;
    state.last_emission_block = block_height;

    // Save state
    db.insert(
        TOKENOMICS_STATE_KEY.as_bytes(),
        serde_json::to_vec(&state).unwrap(),
    )
    .map_err(|e| format!("Failed to save state: {}", e))?;

    TOKENOMICS_SUPPLY.set(state.current_supply as i64);

    Ok(emission)
}

// Burn tokens from fees
pub fn burn_fees(db: &Db, fee_amount: u128) -> Result<u128, String> {
    let config = TOKENOMICS_CONFIG.lock().clone();
    let burn_amount = (fee_amount as f64 * config.fee_burn_percentage / 100.0) as u128;

    let mut state = get_tokenomics_state(db)?;
    state.current_supply -= burn_amount;
    state.total_burned += burn_amount;

    db.insert(
        TOKENOMICS_STATE_KEY.as_bytes(),
        serde_json::to_vec(&state).unwrap(),
    )
    .map_err(|e| format!("Failed to save state: {}", e))?;

    TOKENOMICS_SUPPLY.set(state.current_supply as i64);
    TOKENOMICS_BURNED.inc_by(burn_amount as u64);

    Ok(burn_amount)
}

// Calculate staking rewards
pub fn calculate_staking_rewards(staked_amount: u128, duration_seconds: u64) -> u128 {
    let config = TOKENOMICS_CONFIG.lock().clone();
    let apy = config.staking_reward_rate / 100.0;
    let duration_years = duration_seconds as f64 / (365.25 * 24.0 * 3600.0);

    (staked_amount as f64 * apy * duration_years) as u128
}

// Get tokenomics state
pub fn get_tokenomics_state(db: &Db) -> Result<TokenomicsState, String> {
    match db.get(TOKENOMICS_STATE_KEY.as_bytes()) {
        Ok(Some(data)) => {
            serde_json::from_slice(&data).map_err(|e| format!("Deserialization error: {}", e))
        }
        Ok(None) => {
            // Initialize default state
            let config = TOKENOMICS_CONFIG.lock().clone();
            Ok(TokenomicsState {
                current_supply: config.initial_supply,
                total_burned: 0,
                total_staked: 0,
                total_rewards_distributed: 0,
                last_emission_block: 0,
                current_epoch: 0,
            })
        }
        Err(e) => Err(format!("DB error: {}", e)),
    }
}

// =================== PHASE 8.5: TREASURY SYSTEM ===================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    pub balance: u128,
    pub total_collected: u128,
    pub total_distributed: u128,
    pub proposals_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryProposal {
    pub proposal_id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u128,
    pub proposer: String,
    pub votes_for: u64,
    pub votes_against: u64,
    pub status: TreasuryProposalStatus,
    pub created_at: u64,
    pub voting_ends_at: u64,
    pub executed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TreasuryProposalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    pub schedule_id: String,
    pub beneficiary: String,
    pub total_amount: u128,
    pub released_amount: u128,
    pub start_time: u64,
    pub duration_seconds: u64,
    pub cliff_seconds: u64,
}

// Storage prefix
const TREASURY_KEY: &str = "treasury";
const TREASURY_PROPOSAL_PREFIX: &str = "treasury_prop:";
const VESTING_PREFIX: &str = "vesting:";

// Global treasury state
pub static TREASURY: once_cell::sync::Lazy<Mutex<Treasury>> = once_cell::sync::Lazy::new(|| {
    Mutex::new(Treasury {
        balance: 0,
        total_collected: 0,
        total_distributed: 0,
        proposals_count: 0,
    })
});

pub static TREASURY_PROPOSALS: once_cell::sync::Lazy<Mutex<BTreeMap<String, TreasuryProposal>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(BTreeMap::new()));

// Prometheus metrics
static TREASURY_BALANCE: once_cell::sync::Lazy<IntGauge> =
    once_cell::sync::Lazy::new(|| mk_int_gauge("vision_treasury_balance", "Treasury balance"));

static TREASURY_PROPOSALS_TOTAL: once_cell::sync::Lazy<IntCounter> =
    once_cell::sync::Lazy::new(|| {
        mk_int_counter(
            "vision_treasury_proposals_total",
            "Total treasury proposals",
        )
    });

static TREASURY_DISTRIBUTED: once_cell::sync::Lazy<IntCounter> = once_cell::sync::Lazy::new(|| {
    mk_int_counter(
        "vision_treasury_distributed_total",
        "Total treasury funds distributed",
    )
});

// Fund treasury from fees
pub fn fund_treasury(db: &Db, amount: u128) -> Result<(), String> {
    let mut treasury = TREASURY.lock();
    treasury.balance += amount;
    treasury.total_collected += amount;

    db.insert(
        TREASURY_KEY.as_bytes(),
        serde_json::to_vec(&*treasury).unwrap(),
    )
    .map_err(|e| format!("Failed to save treasury: {}", e))?;
    db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    TREASURY_BALANCE.set(treasury.balance as i64);

    Ok(())
}

// Create treasury spending proposal
pub fn create_treasury_proposal(
    db: &Db,
    title: &str,
    description: &str,
    recipient: &str,
    amount: u128,
    proposer: &str,
    voting_duration: u64,
) -> Result<String, String> {
    let treasury = TREASURY.lock();
    if amount > treasury.balance {
        return Err("Insufficient treasury balance".to_string());
    }
    drop(treasury);

    let proposal_id = format!(
        "tprop_{}",
        hex::encode(
            &blake3::hash(format!("{}:{}:{}", proposer, recipient, now_ts()).as_bytes()).as_bytes()
                [..16]
        )
    );

    let proposal = TreasuryProposal {
        proposal_id: proposal_id.clone(),
        title: title.to_string(),
        description: description.to_string(),
        recipient: recipient.to_string(),
        amount,
        proposer: proposer.to_string(),
        votes_for: 0,
        votes_against: 0,
        status: TreasuryProposalStatus::Pending,
        created_at: now_ts(),
        voting_ends_at: now_ts() + voting_duration,
        executed_at: None,
    };

    TREASURY_PROPOSALS
        .lock()
        .insert(proposal_id.clone(), proposal.clone());

    let key = format!("{}{}", TREASURY_PROPOSAL_PREFIX, proposal_id);
    db.insert(key.as_bytes(), serde_json::to_vec(&proposal).unwrap())
        .map_err(|e| format!("Failed to store proposal: {}", e))?;
    db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    TREASURY_PROPOSALS_TOTAL.inc();

    Ok(proposal_id)
}

// Execute approved treasury proposal
pub fn execute_treasury_proposal(
    db: &Db,
    balances: &mut BTreeMap<String, u128>,
    proposal_id: &str,
) -> Result<(), String> {
    let mut proposals = TREASURY_PROPOSALS.lock();
    let proposal = proposals
        .get_mut(proposal_id)
        .ok_or_else(|| "Proposal not found".to_string())?;

    if proposal.status != TreasuryProposalStatus::Approved {
        return Err("Proposal is not approved".to_string());
    }

    let mut treasury = TREASURY.lock();
    if proposal.amount > treasury.balance {
        return Err("Insufficient treasury balance".to_string());
    }

    // Transfer funds
    treasury.balance -= proposal.amount;
    treasury.total_distributed += proposal.amount;

    let recipient_key = acct_key(&proposal.recipient);
    *balances.entry(recipient_key).or_insert(0) += proposal.amount;

    proposal.status = TreasuryProposalStatus::Executed;
    proposal.executed_at = Some(now_ts());

    // Save state
    db.insert(
        TREASURY_KEY.as_bytes(),
        serde_json::to_vec(&*treasury).unwrap(),
    )
    .map_err(|e| format!("Failed to save treasury: {}", e))?;

    let key = format!("{}{}", TREASURY_PROPOSAL_PREFIX, proposal_id);
    db.insert(key.as_bytes(), serde_json::to_vec(&*proposal).unwrap())
        .map_err(|e| format!("Failed to update proposal: {}", e))?;
    db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    TREASURY_BALANCE.set(treasury.balance as i64);
    TREASURY_DISTRIBUTED.inc_by(proposal.amount as u64);

    Ok(())
}

// Create vesting schedule
pub fn create_vesting_schedule(
    db: &Db,
    beneficiary: &str,
    total_amount: u128,
    duration_seconds: u64,
    cliff_seconds: u64,
) -> Result<String, String> {
    let schedule_id = format!(
        "vest_{}",
        hex::encode(
            &blake3::hash(format!("{}:{}", beneficiary, now_ts()).as_bytes()).as_bytes()[..16]
        )
    );

    let schedule = VestingSchedule {
        schedule_id: schedule_id.clone(),
        beneficiary: beneficiary.to_string(),
        total_amount,
        released_amount: 0,
        start_time: now_ts(),
        duration_seconds,
        cliff_seconds,
    };

    let key = format!("{}{}", VESTING_PREFIX, schedule_id);
    db.insert(key.as_bytes(), serde_json::to_vec(&schedule).unwrap())
        .map_err(|e| format!("Failed to store vesting schedule: {}", e))?;
    db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    Ok(schedule_id)
}

// Calculate vested amount
fn calculate_vested_amount(schedule: &VestingSchedule) -> u128 {
    let elapsed = now_ts().saturating_sub(schedule.start_time);

    // Check cliff period
    if elapsed < schedule.cliff_seconds {
        return 0;
    }

    // Linear vesting after cliff
    if elapsed >= schedule.duration_seconds {
        return schedule.total_amount;
    }

    let vested =
        (schedule.total_amount as f64 * elapsed as f64 / schedule.duration_seconds as f64) as u128;
    vested.min(schedule.total_amount)
}

// Release vested tokens
pub fn release_vested_tokens(
    db: &Db,
    balances: &mut BTreeMap<String, u128>,
    schedule_id: &str,
) -> Result<u128, String> {
    let key = format!("{}{}", VESTING_PREFIX, schedule_id);
    let data = db
        .get(key.as_bytes())
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| "Vesting schedule not found".to_string())?;

    let mut schedule: VestingSchedule =
        serde_json::from_slice(&data).map_err(|e| format!("Deserialization error: {}", e))?;

    let vested = calculate_vested_amount(&schedule);
    let releasable = vested.saturating_sub(schedule.released_amount);

    if releasable == 0 {
        return Err("No tokens available to release".to_string());
    }

    // Release tokens
    let beneficiary_key = acct_key(&schedule.beneficiary);
    *balances.entry(beneficiary_key).or_insert(0) += releasable;

    schedule.released_amount += releasable;

    db.insert(key.as_bytes(), serde_json::to_vec(&schedule).unwrap())
        .map_err(|e| format!("Failed to update vesting schedule: {}", e))?;
    db.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    Ok(releasable)
}

// Get treasury stats
pub fn get_treasury_stats() -> serde_json::Value {
    let treasury = TREASURY.lock();
    serde_json::json!({
        "balance": treasury.balance,
        "total_collected": treasury.total_collected,
        "total_distributed": treasury.total_distributed,
        "proposals_count": treasury.proposals_count,
    })
}

// =================== API HANDLERS ===================

// Phase 8.4: Token Economics Handlers

pub async fn tokenomics_state_handler_old() -> (StatusCode, Json<serde_json::Value>) {
    let chain = CHAIN.lock();
    let db = chain.db.clone();
    drop(chain);

    match get_tokenomics_state(&db) {
        Ok(state) => {
            let config = TOKENOMICS_CONFIG.lock().clone();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "ok": true,
                    "config": config,
                    "state": state
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

pub async fn tokenomics_calculate_rewards_handler(
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let staked_amount = req["staked_amount"].as_u64().unwrap_or(0) as u128;
    let duration_seconds = req["duration_seconds"].as_u64().unwrap_or(0);

    let rewards = calculate_staking_rewards(staked_amount, duration_seconds);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "rewards": rewards,
            "staked_amount": staked_amount,
            "duration_seconds": duration_seconds
        })),
    )
}

// Phase 8.5: Treasury Handlers

pub async fn treasury_stats_handler() -> (StatusCode, Json<serde_json::Value>) {
    let stats = get_treasury_stats();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "treasury": stats
        })),
    )
}

pub async fn treasury_proposal_create_handler(
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let title = req["title"].as_str().unwrap_or_default();
    let description = req["description"].as_str().unwrap_or_default();
    let recipient = req["recipient"].as_str().unwrap_or_default();
    let amount = req["amount"].as_u64().unwrap_or(0) as u128;
    let proposer = req["proposer"].as_str().unwrap_or_default();
    let voting_duration = req["voting_duration_seconds"].as_u64().unwrap_or(86400);

    if title.is_empty() || recipient.is_empty() || proposer.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "title, recipient, and proposer are required"
            })),
        );
    }

    let chain = CHAIN.lock();
    let db = chain.db.clone();
    drop(chain);

    match create_treasury_proposal(
        &db,
        title,
        description,
        recipient,
        amount,
        proposer,
        voting_duration,
    ) {
        Ok(proposal_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "proposal_id": proposal_id
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

pub async fn treasury_proposal_get_handler(
    Path((id,)): Path<(String,)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let proposals = TREASURY_PROPOSALS.lock();
    match proposals.get(&id) {
        Some(proposal) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "proposal": proposal
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "ok": false,
                "error": "Proposal not found"
            })),
        ),
    }
}

pub async fn treasury_proposal_execute_handler(
    Path((id,)): Path<(String,)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut chain = CHAIN.lock();
    let db = chain.db.clone();

    match execute_treasury_proposal(&db, &mut chain.balances, &id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "message": "Treasury proposal executed"
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

pub async fn vesting_create_handler(
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    let beneficiary = req["beneficiary"].as_str().unwrap_or_default();
    let total_amount = req["total_amount"].as_u64().unwrap_or(0) as u128;
    let duration_seconds = req["duration_seconds"].as_u64().unwrap_or(0);
    let cliff_seconds = req["cliff_seconds"].as_u64().unwrap_or(0);

    if beneficiary.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": "beneficiary is required"
            })),
        );
    }

    let chain = CHAIN.lock();
    let db = chain.db.clone();
    drop(chain);

    match create_vesting_schedule(
        &db,
        beneficiary,
        total_amount,
        duration_seconds,
        cliff_seconds,
    ) {
        Ok(schedule_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "schedule_id": schedule_id
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

pub async fn vesting_release_handler(
    Path((schedule_id,)): Path<(String,)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let mut chain = CHAIN.lock();
    let db = chain.db.clone();

    match release_vested_tokens(&db, &mut chain.balances, &schedule_id) {
        Ok(amount) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "released_amount": amount
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "error": e
            })),
        ),
    }
}

// Tokenomics stats handler
pub async fn tokenomics_stats_handler() -> (StatusCode, Json<serde_json::Value>) {
    let chain = CHAIN.lock();
    let db = chain.db.clone();
    let cfg = chain.tokenomics_cfg.clone();
    let height = chain.blocks.last().map(|b| b.header.number).unwrap_or(0);

    // Read counters from sled
    let supply_total = db
        .get(TOK_SUPPLY_TOTAL.as_bytes())
        .ok()
        .and_then(|opt| opt.map(|v| u128_from_be(&v)))
        .unwrap_or(0);
    let supply_burned = db
        .get(TOK_SUPPLY_BURNED.as_bytes())
        .ok()
        .and_then(|opt| opt.map(|v| u128_from_be(&v)))
        .unwrap_or(0);
    let supply_treasury = db
        .get(TOK_SUPPLY_TREASURY.as_bytes())
        .ok()
        .and_then(|opt| opt.map(|v| u128_from_be(&v)))
        .unwrap_or(0);
    let supply_vault = db
        .get(TOK_SUPPLY_VAULT.as_bytes())
        .ok()
        .and_then(|opt| opt.map(|v| u128_from_be(&v)))
        .unwrap_or(0);
    let supply_fund = db
        .get(TOK_SUPPLY_FUND.as_bytes())
        .ok()
        .and_then(|opt| opt.map(|v| u128_from_be(&v)))
        .unwrap_or(0);

    // Calculate next halving height
    let next_halving_height = if cfg.halving_interval_blocks > 0 {
        ((height / cfg.halving_interval_blocks) + 1) * cfg.halving_interval_blocks
    } else {
        0
    };

    drop(chain);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "config": {
                "enable_emission": cfg.enable_emission,
                "emission_per_block": cfg.emission_per_block.to_string(),
                "halving_interval_blocks": cfg.halving_interval_blocks,
                "fee_distribution_bps": cfg.fee_burn_bps,
                "treasury_bps": cfg.treasury_bps,
                "staking_epoch_blocks": cfg.staking_epoch_blocks,
                "decimals": cfg.decimals,
                "vault_addr": cfg.vault_addr,
                "fund_addr": cfg.fund_addr,
                "treasury_addr": cfg.treasury_addr
            },
            "state": {
                "current_height": height,
                "total_supply": supply_total.to_string(),
                "fees_distributed": supply_burned.to_string(),
                "treasury_total": supply_treasury.to_string(),
                "vault_total": supply_vault.to_string(),
                "fund_total": supply_fund.to_string(),
                "next_halving_height": next_halving_height
            }
        })),
    )
}

// Tokenomics emission handler
pub async fn tokenomics_emission_handler(
    Path((height,)): Path<(u64,)>,
) -> (StatusCode, Json<serde_json::Value>) {
    let chain = CHAIN.lock();
    let cfg = &chain.tokenomics_cfg;

    // Calculate emission using official Tokenomics halving schedule
    let halvings = height / cfg.halving_interval_blocks;
    let halving_divisor = 2u128.saturating_pow(halvings as u32);
    let block_emission = if cfg.enable_emission {
        cfg.emission_per_block / halving_divisor
    } else {
        0
    };

    // Calculate tithe splits
    let tithe_amt = crate::tokenomics::tithe::tithe_amount();
    let (bp_miner, bp_vault, bp_fund, bp_tres) = crate::tokenomics::tithe::tithe_split_bps();
    let tithe_vault = tithe_amt.saturating_mul(bp_vault as u128) / 10_000;
    let tithe_fund = tithe_amt.saturating_mul(bp_fund as u128) / 10_000;
    let tithe_tres = tithe_amt.saturating_sub(
        tithe_amt.saturating_mul(bp_miner as u128) / 10_000 + tithe_vault + tithe_fund,
    );

    drop(chain);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "height": height,
            "halvings": halvings,
            "halving_divisor": halving_divisor,
            "block_emission": block_emission.to_string(),
            "tithe": {
                "amount": tithe_amt.to_string(),
                "vault_share": tithe_vault.to_string(),
                "fund_share": tithe_fund.to_string(),
                "treasury_share": tithe_tres.to_string(),
                "split_bps": {
                    "miner": bp_miner,
                    "vault": bp_vault,
                    "fund": bp_fund,
                    "treasury": bp_tres
                }
            }
        })),
    )
}

// Foundation addresses handler
pub async fn foundation_addresses_handler() -> (StatusCode, Json<serde_json::Value>) {
    let chain = CHAIN.lock();
    let vault_addr = chain.tokenomics_cfg.vault_addr.clone();
    let fund_addr = chain.tokenomics_cfg.fund_addr.clone();
    let treasury_addr = chain.tokenomics_cfg.treasury_addr.clone();
    drop(chain);

    let tithe_amt = crate::tokenomics::tithe::tithe_amount();
    let (bp_miner, bp_vault, bp_fund, bp_tres) = crate::tokenomics::tithe::tithe_split_bps();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "addresses": {
                "vault": vault_addr,
                "fund": fund_addr,
                "treasury": treasury_addr
            },
            "tithe": {
                "amount": tithe_amt.to_string(),
                "split_bps": {
                    "miner": bp_miner,
                    "vault": bp_vault,
                    "fund": bp_fund,
                    "treasury": bp_tres
                }
            },
            "note": "Tithe is applied every block and split across foundation addresses to ensure Vault growth from block 1"
        })),
    )
}

// Admin tokenomics config handler
pub async fn admin_tokenomics_config_handler(
    headers: HeaderMap,
    Query(q): Query<std::collections::HashMap<String, String>>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<serde_json::Value>) {
    if !crate::check_admin(headers.clone(), &q) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "invalid or missing admin token"
            })),
        );
    }

    // NEW: Check if governance approval is required
    let require_governance = std::env::var("VISION_TOK_GOVERNANCE_REQUIRED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);

    if require_governance {
        // Verify that a passed governance proposal exists for this change
        let proposal_id = req["governance_proposal_id"].as_str();

        if proposal_id.is_none() {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "governance_proposal_id required when VISION_TOK_GOVERNANCE_REQUIRED=true",
                    "hint": "Create a TokenomicsConfig proposal first and wait for it to pass"
                })),
            );
        }

        let prop_id = proposal_id.unwrap();
        let proposals = PROPOSALS.lock();
        let proposal = proposals.get(prop_id);

        match proposal {
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "ok": false,
                        "error": "governance proposal not found"
                    })),
                );
            }
            Some(p) => {
                // Verify proposal is for tokenomics and has passed
                if p.proposal_type != ProposalType::TokenomicsConfig {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "ok": false,
                            "error": "proposal must be of type TokenomicsConfig"
                        })),
                    );
                }

                if !matches!(p.status, ProposalStatus::Passed | ProposalStatus::Executed) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(serde_json::json!({
                            "ok": false,
                            "error": format!("proposal must be Passed or Executed, current status: {:?}", p.status),
                            "proposal_status": format!("{:?}", p.status)
                        })),
                    );
                }

                // Mark proposal as executed if not already
                drop(proposals);
                let mut proposals_mut = PROPOSALS.lock();
                if let Some(p_mut) = proposals_mut.get_mut(prop_id) {
                    if p_mut.status == ProposalStatus::Passed {
                        p_mut.status = ProposalStatus::Executed;
                        p_mut.executed_at = Some(now_ts());
                        p_mut.execution_result = Some("Tokenomics config updated".to_string());
                    }
                }
                drop(proposals_mut);
            }
        }
    }

    let mut chain = CHAIN.lock();

    // Update config fields if provided
    if let Some(val) = req["fee_burn_bps"].as_u64() {
        // Validate: max 50% burn
        if val > 5000 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "fee_burn_bps cannot exceed 5000 (50%)"
                })),
            );
        }
        chain.tokenomics_cfg.fee_burn_bps = val as u32;
    }
    if let Some(val) = req["treasury_bps"].as_u64() {
        // Validate: max 25% treasury cut
        if val > 2500 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "error": "treasury_bps cannot exceed 2500 (25%)"
                })),
            );
        }
        chain.tokenomics_cfg.treasury_bps = val as u32;
    }
    if let Some(val) = req["emission_per_block"].as_str() {
        if let Ok(v) = val.parse::<u128>() {
            // Validate: cannot increase emission by more than 2x
            let current = chain.tokenomics_cfg.emission_per_block;
            if v > current * 2 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "ok": false,
                        "error": format!("emission_per_block cannot exceed 2x current value ({})", current * 2)
                    })),
                );
            }
            chain.tokenomics_cfg.emission_per_block = v;
        }
    }
    if let Some(val) = req["enable_emission"].as_bool() {
        chain.tokenomics_cfg.enable_emission = val;
    }

    // Persist updated config to sled
    let cfg_bytes = serde_json::to_vec(&chain.tokenomics_cfg).unwrap();
    let _ = chain.db.insert(TOK_CONFIG_KEY.as_bytes(), cfg_bytes);
    let _ = chain.db.flush();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "ok": true,
            "config": chain.tokenomics_cfg,
            "governance_enforced": require_governance
        })),
    )
}
