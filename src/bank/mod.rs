#![allow(dead_code)]

use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::collections::BTreeMap;

lazy_static! {
    static ref BAL: RwLock<BTreeMap<(String, String), u128>> = RwLock::new(BTreeMap::new());
}

fn key(addr: &str, ccy: &str) -> (String, String) {
    (addr.into(), ccy.into())
}

pub fn credit(addr: &str, ccy: &str, amt: u128) -> Result<(), String> {
    let mut b = BAL.write();
    *b.entry(key(addr, ccy)).or_default() += amt;
    Ok(())
}
pub fn debit(addr: &str, ccy: &str, amt: u128) -> Result<(), String> {
    let mut b = BAL.write();
    let e = b.entry(key(addr, ccy)).or_default();
    if *e < amt {
        return Err("insufficient funds".into());
    }
    *e -= amt;
    Ok(())
}
pub fn balance(addr: &str, ccy: &str) -> u128 {
    let b = BAL.read();
    *b.get(&key(addr, ccy)).unwrap_or(&0)
}
