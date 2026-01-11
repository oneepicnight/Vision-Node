#![cfg(test)]

use sled::Db;
use std::collections::BTreeMap;
use std::env;

/// A small RAII helper used in tests to set environment variables and restore them when dropped.
/// Use `let _s = ScopedEnv::with_vars(&[("FOO", "1"), ("BAR", "2")]);` in tests.
pub(crate) struct ScopedEnv {
    prev: Vec<(String, Option<String>)>,
}

impl ScopedEnv {
    pub(crate) fn new(key: &str, value: &str) -> Self {
        let prev_val = env::var(key).ok();
        env::set_var(key, value);
        Self { prev: vec![(key.to_string(), prev_val)] }
    }

    pub(crate) fn with_vars(vars: &[(&str, &str)]) -> Self {
        let mut s = Self { prev: Vec::new() };
        for (k, v) in vars {
            let prev = env::var(k).ok();
            env::set_var(k, v);
            s.prev.push((k.to_string(), prev));
        }
        s
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (k, prev) in &self.prev {
            if let Some(old) = prev {
                env::set_var(k, old);
            } else {
                env::remove_var(k);
            }
        }
    }
}

/// Test-only: emulate the deprecated `apply_vision_tokenomics` wrapper used previously. It computes
/// tokenomics deltas and applies them to the provided balances, persists supply metrics to DB,
/// and returns the miner_reward, fees_distributed and treasury_total.
pub(crate) fn apply_vision_tokenomics_legacy_for_test(
    height: u64,
    miner_addr: &str,
    tx_fees_total: u128,
    mev_revenue: u128,
    balances: &mut BTreeMap<String, u128>,
    db: &Db,
) -> (u128, u128, u128) {
    let tr = crate::compute_vision_tokenomics_deltas(height, miner_addr, tx_fees_total, mev_revenue, &*balances);
    for (addr, d) in tr.deltas.iter() {
        if *d >= 0 {
            *balances.entry(addr.clone()).or_insert(0) += *d as u128;
        } else {
            let sub = (-*d) as u128;
            let entry = balances.entry(addr.clone()).or_insert(0);
            *entry = entry.saturating_sub(sub);
        }
    }
    crate::persist_supply_metrics(db, balances);
    (tr.miner_reward, tr.fees_distributed, tr.treasury_total)
}
