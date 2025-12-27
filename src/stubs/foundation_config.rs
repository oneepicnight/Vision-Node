//! Stub for foundation_config module when staging is disabled
//! Non-custodial: no real vault addresses

pub fn vault_address() -> String {
    "vault_stub".to_string()
}

pub fn fund_address() -> String {
    "fund_stub".to_string()
}

pub fn founder1_address() -> String {
    "founder1_stub".to_string()
}

pub fn founder2_address() -> String {
    "founder2_stub".to_string()
}

pub fn miners_btc_address() -> Option<String> {
    None
}

pub fn miners_bch_address() -> Option<String> {
    None
}

pub fn miners_doge_address() -> Option<String> {
    None
}
