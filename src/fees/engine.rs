#![cfg(feature = "staged")]
#![allow(dead_code)]

// Send part of the "burn" back into the Vault as sustainable inflow.
pub fn handle_burn_redirect(
    ccy: &str,
    total_burn: u128,
    redirect_bp: u16,
    memo: &str,
) -> Vec<serde_json::Value> {
    let redirect = total_burn * (redirect_bp as u128) / 10_000; // basis points
    if redirect == 0 {
        return vec![];
    }
    if let Ok(evt) =
        crate::treasury::vault::route_inflow(ccy, redirect, format!("burn_redirect {memo}"))
    {
        return vec![serde_json::json!({"ok": true, "redirected": redirect, "event": evt})];
    }
    vec![serde_json::json!({"ok": false})]
}
