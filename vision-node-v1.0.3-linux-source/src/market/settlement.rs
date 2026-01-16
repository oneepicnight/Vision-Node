use crate::market::engine::QuoteAsset;
use crate::vault::land_auto_buy::LandAutoBuyer;
use crate::vault::store::VaultStore;
use crate::vision_constants;
use anyhow::Result;

/// Route exchange fee using new vault system
/// Fee is charged in the quote asset and split 50/25/25 (Miners/Founder1/Founder2)
/// DevOps gets ZERO crypto fees (they only participate in on-chain LAND revenue)
/// Miners address is the AUTO-BUY HOT WALLET - triggers land auto-buy cycle if enabled.
pub fn route_exchange_fee(quote: QuoteAsset, fee_amount: f64) -> Result<()> {
    // Get database from global chain context
    let db = {
        let chain = crate::CHAIN.lock();
        chain.db.clone()
    };

    // Route fee through new vault system (50/30/20 split)
    let vault_router = crate::vault::VaultRouter::new(db.clone());
    if let Err(e) = vault_router.route_exchange_fee(quote, fee_amount) {
        tracing::error!("Failed to route exchange fee: {}", e);
        return Err(e);
    }

    // Log the complete routing: this proves the fee hit VaultStore
    let store = VaultStore::new(db.clone());
    let miners_bal = store
        .get_bucket_balance(crate::vault::store::VaultBucket::Miners, quote)
        .unwrap_or(0);
    let founder1_bal = store
        .get_bucket_balance(crate::vault::store::VaultBucket::Founder1, quote)
        .unwrap_or(0);
    let founder2_bal = store
        .get_bucket_balance(crate::vault::store::VaultBucket::Founder2, quote)
        .unwrap_or(0);

    tracing::info!(
        "[VAULT] routed fee: asset={} amount={:.8} -> buckets: miners={} (50% auto-buy) founder1={} (25%) founder2={} (25%)",
        quote.as_str(),
        fee_amount,
        miners_bal,
        founder1_bal,
        founder2_bal
    );

    // Trigger vault auto-buy cycle to convert external assets to LAND
    if vision_constants::is_env_flag_set("VISION_ENABLE_VAULT_AUTO_BUY") {
        let store = VaultStore::new(db.clone());
        let auto_buyer = LandAutoBuyer::new(store, db);
        if let Err(e) = auto_buyer.run_conversion_cycle() {
            tracing::warn!("Auto-buy conversion cycle failed: {}", e);
            // Don't fail the fee routing if auto-buy fails
        }
    }

    Ok(())
}

/// Route sale proceeds according to 50/30/20 split (Vault/Ops/Founder)
/// Clean split without double-crediting any account.
/// [STAGED] Uses old treasury::vault module - not in launch-core builds
#[cfg(feature = "staged")]
pub fn route_proceeds(db: &Db, total: u128) -> Result<()> {
    // Get addresses from foundation config
    let vault_addr = foundation_config::vault_address();
    let fund_addr = foundation_config::fund_address();
    let founder_addr = foundation_config::founder1_address();

    // Route through treasury once using canonical 50/30/20 split
    let evt =
        crate::treasury::vault::route_inflow("CASH", total, format!("market_sale total={}", total))
            .map_err(|e| anyhow!(e))?;

    tracing::info!(
        "Routing {} total: vault={} (50%), fund={} (30%), founder={} (20%)",
        total,
        evt.to_vault,
        evt.to_ops,
        evt.to_founders
    );

    // Write receipts for each distribution (best-effort, don't fail on receipt errors)
    let write_settlement_receipt = |to: &str, amount: u128, label: &str| {
        let rec = Receipt {
            id: String::new(),
            ts_ms: 0,
            kind: "market_settle".to_string(),
            from: "market_proceeds".to_string(),
            to: to.to_string(),
            amount: amount.to_string(),
            fee: "0".to_string(),
            memo: Some(format!("{} settlement from market sale", label)),
            txid: None,
            ok: true,
            note: None,
        };
        if let Err(e) = write_receipt(db, None, rec) {
            tracing::warn!("Failed to write {} settlement receipt: {}", label, e);
        }
    };

    write_settlement_receipt(&vault_addr, evt.to_vault, "Vault (50%)");
    write_settlement_receipt(&fund_addr, evt.to_ops, "Fund (30%)");
    write_settlement_receipt(&founder_addr, evt.to_founders, "Founder (20%)");

    Ok(())
}
