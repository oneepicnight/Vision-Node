#![cfg(feature = "staged")]
#![allow(dead_code)]

use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::bank::credit;
use crate::foundation_config;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Split {
    pub vault: u8,
    pub ops: u8,
    pub founders: u8,
}

// *** Your plan: 50 / 30 / 20 ***
pub const VAULT_SPLIT: Split = Split {
    vault: 50,
    ops: 30,
    founders: 20,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VaultEvent {
    pub ts: u64,
    pub ccy: String, // "LAND" or "CASH"
    pub gross: u128, // full amount before splitting
    pub to_vault: u128,
    pub to_ops: u128,
    pub to_founders: u128,
    pub memo: String, // e.g., "land_sale parcel=42"
}

#[derive(Default)]
pub struct VaultLedger {
    events: Vec<VaultEvent>,
    total_land: (u128, u128, u128), // (vault, ops, founders)
    total_cash: (u128, u128, u128),
}

lazy_static! {
    static ref LEDGER: RwLock<VaultLedger> = RwLock::new(VaultLedger::default());
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Split amount per VAULT_SPLIT
pub fn split_amount(amount: u128) -> (u128, u128, u128) {
    let v = amount * (VAULT_SPLIT.vault as u128) / 100;
    let o = amount * (VAULT_SPLIT.ops as u128) / 100;
    let f = amount - v - o; // rounding dust goes to founders
    (v, o, f)
}

/// Route inflow to accounts AND record to ledger
pub fn route_inflow(
    ccy: &str,
    amount: u128,
    memo: impl Into<String>,
) -> Result<VaultEvent, String> {
    let (v, o, f) = split_amount(amount);

    // credit three accounts from foundation_config
    credit(&foundation_config::vault_address(), ccy, v)?;
    credit(&foundation_config::fund_address(), ccy, o)?;
    credit(&foundation_config::founder1_address(), ccy, f)?;

    // record ledger
    let mut lg = LEDGER.write();
    let evt = VaultEvent {
        ts: now(),
        ccy: ccy.into(),
        gross: amount,
        to_vault: v,
        to_ops: o,
        to_founders: f,
        memo: memo.into(),
    };
    lg.events.push(evt.clone());
    match ccy {
        "LAND" => {
            lg.total_land.0 += v;
            lg.total_land.1 += o;
            lg.total_land.2 += f;
        }
        "CASH" => {
            lg.total_cash.0 += v;
            lg.total_cash.1 += o;
            lg.total_cash.2 += f;
        }
        _ => {}
    }
    Ok(evt)
}

#[derive(Serialize)]
pub struct VaultStats {
    pub split: Split,
    pub totals_land: (u128, u128, u128),
    pub totals_cash: (u128, u128, u128),
    pub last_10: Vec<VaultEvent>,
}

pub fn stats() -> VaultStats {
    let lg = LEDGER.read();
    let n = lg.events.len();
    let start = n.saturating_sub(10);
    VaultStats {
        split: VAULT_SPLIT,
        totals_land: lg.total_land,
        totals_cash: lg.total_cash,
        last_10: lg.events[start..].to_vec(),
    }
}
