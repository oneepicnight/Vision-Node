#![allow(dead_code)]

use anyhow::Result;
use sled::Db;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::land_stake;
use crate::receipts::{write_receipt, Receipt};

/// Trees / keys
const TREE_VAULT: &str = "vault_state"; // last_payout_height, last_payout_at_ms, last_snapshot_total
const KEY_LAST_HEIGHT: &[u8] = b"last_payout_height";
const KEY_LAST_AT_MS: &[u8] = b"last_payout_at_ms";
const KEY_LAST_SNAPSHOT: &[u8] = b"last_snapshot_total";

const TREE_BALANCES: &str = "balances"; // address balances

/// Real supply counter key written by main.rs
const KEY_SUPPLY_VAULT: &[u8] = b"supply:vault";

/// Configure epoch length via env. Default: 180 blocks (~30min if 10s/blk)
fn epoch_blocks() -> u64 {
    std::env::var("VISION_EPOCH_BLOCKS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}

/// Called on each *accepted* block. If an epoch boundary passed, run payout.
///
/// Rules:
/// - vault_delta = vault_total - last_snapshot_total
/// - if total_weight == 0: carry forward (no payout)
/// - distribute pro-rata by weight; rounding dust stays in vault
pub fn pay_epoch_if_due(db: &Db, best_height: u64) -> Result<Option<PayoutSummary>> {
    let vault = db.open_tree(TREE_VAULT)?;

    let last_h = read_u64(&vault, KEY_LAST_HEIGHT).unwrap_or(0);
    let blocks = epoch_blocks();

    // Not due yet
    if best_height < last_h.saturating_add(blocks) {
        return Ok(None);
    }

    // Compute delta
    let vault_total = read_u128_db(db, KEY_SUPPLY_VAULT).unwrap_or(0);
    let last_snap = read_u128(&vault, KEY_LAST_SNAPSHOT).unwrap_or(0);
    if vault_total <= last_snap {
        // no growth since last payout
        write_epoch_bookkeeping(&vault, best_height, now_ms(), vault_total)?;
        return Ok(Some(PayoutSummary {
            epoch_index: epoch_index(best_height),
            distributed: 0,
            total_weight: 0,
            recipients: 0,
        }));
    }
    let vault_delta = vault_total - last_snap;

    // Staking weights
    let total_w = land_stake::total_weight(db);
    if total_w == 0 {
        // nobody to pay — just advance epoch snapshot
        write_epoch_bookkeeping(&vault, best_height, now_ms(), vault_total)?;
        return Ok(Some(PayoutSummary {
            epoch_index: epoch_index(best_height),
            distributed: 0,
            total_weight: 0,
            recipients: 0,
        }));
    }

    // Iterate all owner weights and distribute proportionally
    let weights = db.open_tree("owner_weights")?;
    let balances = db.open_tree(TREE_BALANCES)?;
    let mut distributed: u128 = 0;
    let mut recipients: u64 = 0;

    // Calculate payouts (read-only pass)
    let mut payouts: Vec<(Vec<u8>, u128)> = Vec::new();
    for kv in weights.iter() {
        let (k, v) = kv?;
        let w = decode_u128(v.as_ref());
        if w == 0 {
            continue;
        }

        let amt = mul_div_floor(vault_delta, w, total_w);
        if amt == 0 {
            continue;
        }

        payouts.push((k.to_vec(), amt));
        distributed = distributed.saturating_add(amt);
        recipients += 1;
    }

    // Apply payouts to balances
    for (addr_bytes, amt) in &payouts {
        let cur = read_u128_tree(&balances, addr_bytes)?;
        write_u128(&balances, addr_bytes, cur.saturating_add(*amt))?;
    }

    // Update vault and epoch bookkeeping
    let new_vault = vault_total.saturating_sub(distributed);
    // Update supply counter in root db
    db.insert(KEY_SUPPLY_VAULT, new_vault.to_be_bytes().to_vec())?;
    write_epoch_bookkeeping(&vault, best_height, now_ms(), new_vault)?;

    // Write receipts (best-effort)
    for (addr_bytes, amt) in &payouts {
        let addr = String::from_utf8_lossy(addr_bytes).to_string();
        let _ = write_receipt(
            db,
            None,
            Receipt {
                id: String::new(),
                ts_ms: 0,
                kind: "vault_payout".to_string(),
                from: "vault".to_string(),
                to: addr,
                amount: amt.to_string(),
                fee: "0".to_string(),
                memo: None,
                txid: None,
                ok: true,
                note: Some(format!("epoch={}", epoch_index(best_height))),
            },
        );
    }

    Ok(Some(PayoutSummary {
        epoch_index: epoch_index(best_height),
        distributed,
        total_weight: total_w,
        recipients,
    }))
}

/// Status used by the /vault/epoch route
#[derive(Debug, Clone, serde::Serialize)]
pub struct EpochStatus {
    pub epoch_index: u64,
    pub last_payout_height: u64,
    pub next_payout_height: u64,
    pub last_payout_at_ms: u64,
    pub vault_balance: String,
    pub total_weight: String,
    pub due: bool,
}

pub fn get_epoch_status(db: &Db, best_height: u64) -> Result<EpochStatus> {
    let vault = db.open_tree(TREE_VAULT)?;

    let last_h = read_u64(&vault, KEY_LAST_HEIGHT).unwrap_or(0);
    let last_at = read_u64(&vault, KEY_LAST_AT_MS).unwrap_or(0);
    let vtot = read_u128_db(db, KEY_SUPPLY_VAULT).unwrap_or(0);
    let tw = land_stake::total_weight(db);
    let blocks = epoch_blocks();
    let next_h = last_h.saturating_add(blocks);

    Ok(EpochStatus {
        epoch_index: epoch_index(best_height),
        last_payout_height: last_h,
        next_payout_height: next_h,
        last_payout_at_ms: last_at,
        vault_balance: vtot.to_string(),
        total_weight: tw.to_string(),
        due: best_height >= next_h,
    })
}

/// Optional: call once during init to make sure snapshot aligns with vault_total
pub fn ensure_snapshot_coherent(db: &Db) -> Result<()> {
    let vault = db.open_tree(TREE_VAULT)?;
    let vtot = read_u128_db(db, KEY_SUPPLY_VAULT).unwrap_or(0);
    if read_u128(&vault, KEY_LAST_SNAPSHOT).unwrap_or(u128::MAX) == u128::MAX {
        write_u128(&vault, KEY_LAST_SNAPSHOT, vtot)?;
        write_u64(&vault, KEY_LAST_HEIGHT, 0)?;
        write_u64(&vault, KEY_LAST_AT_MS, now_ms())?;
    }
    Ok(())
}

/// Call whenever you recalc owner weights (e.g., after land transfers)
pub fn rebuild_weights_and_resnap(db: &Db) -> Result<()> {
    land_stake::rebuild_owner_weights(db)?;
    ensure_snapshot_coherent(db)
}

/// Summary returned by pay_epoch_if_due
#[derive(Debug, Clone, serde::Serialize)]
pub struct PayoutSummary {
    pub epoch_index: u64,
    pub distributed: u128,
    pub total_weight: u128,
    pub recipients: u64,
}

// ---------- helpers ----------

fn epoch_index(best_height: u64) -> u64 {
    let b = epoch_blocks();
    if b == 0 {
        0
    } else {
        best_height / b
    }
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn mul_div_floor(a: u128, b: u128, d: u128) -> u128 {
    if d == 0 {
        return 0;
    }
    (a.saturating_mul(b)) / d
}

fn read_u128(tree: &sled::Tree, key: &[u8]) -> Result<u128> {
    if let Some(ivec) = tree.get(key)? {
        let mut buf = [0u8; 16];
        let bytes = ivec.as_ref();
        buf[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
        Ok(u128::from_be_bytes(buf))
    } else {
        Ok(0)
    }
}
fn read_u128_db(db: &Db, key: &[u8]) -> Result<u128> {
    if let Some(ivec) = db.get(key)? {
        let mut buf = [0u8; 16];
        let bytes = ivec.as_ref();
        buf[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
        Ok(u128::from_be_bytes(buf))
    } else {
        Ok(0)
    }
}
fn read_u128_tree(tree: &sled::Tree, key: &[u8]) -> Result<u128> {
    if let Some(ivec) = tree.get(key)? {
        let mut buf = [0u8; 16];
        let bytes = ivec.as_ref();
        buf[..bytes.len().min(16)].copy_from_slice(&bytes[..bytes.len().min(16)]);
        Ok(u128::from_be_bytes(buf))
    } else {
        Ok(0)
    }
}
fn write_u128(tree: &sled::Tree, key: &[u8], v: u128) -> Result<()> {
    tree.insert(key, v.to_be_bytes().to_vec())?;
    Ok(())
}
fn read_u64(tree: &sled::Tree, key: &[u8]) -> Result<u64> {
    if let Some(ivec) = tree.get(key)? {
        let mut buf = [0u8; 8];
        let bytes = ivec.as_ref();
        buf[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
        Ok(u64::from_le_bytes(buf))
    } else {
        Ok(0)
    }
}
fn write_u64(tree: &sled::Tree, key: &[u8], v: u64) -> Result<()> {
    tree.insert(key, v.to_le_bytes().to_vec())?;
    Ok(())
}
fn write_epoch_bookkeeping(vault: &sled::Tree, h: u64, at_ms: u64, snapshot: u128) -> Result<()> {
    vault.insert(KEY_LAST_HEIGHT, h.to_le_bytes().to_vec())?;
    vault.insert(KEY_LAST_AT_MS, at_ms.to_le_bytes().to_vec())?;
    vault.insert(KEY_LAST_SNAPSHOT, snapshot.to_le_bytes().to_vec())?;
    Ok(())
}
fn decode_u128(bytes: &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    let take = bytes.len().min(16);
    buf[..take].copy_from_slice(&bytes[..take]);
    u128::from_be_bytes(buf)
}
