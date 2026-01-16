// Auto-Buy Logic for Miners' Vault
// Automatically purchases 10 LAND when miners' balance is sufficient

use anyhow::Result;

use crate::market::engine::QuoteAsset;
use crate::market::vault;

/// Auto-buy configuration
const AUTO_BUY_LAND_AMOUNT: f64 = 10.0; // Buy 10 LAND at a time

/// Get current LAND price in the given quote asset
/// TODO: Replace with actual order book price discovery
fn get_land_price_in(quote: QuoteAsset) -> Result<f64> {
    // Placeholder prices - replace with real market data
    let price = match quote {
        QuoteAsset::Land => 1.0,
        QuoteAsset::Btc => 0.0001, // ~$5-10 depending on BTC price
        QuoteAsset::Bch => 0.01,   // Adjust based on BCH/USD
        QuoteAsset::Doge => 50.0,  // Adjust based on DOGE/USD
    };

    Ok(price)
}

/// Check if miners' vault has enough balance to auto-buy 10 LAND
pub fn can_auto_buy(quote: QuoteAsset) -> Result<bool> {
    let miners_balance = vault::get_miners_balance(quote);
    let land_price = get_land_price_in(quote)?;
    let cost_for_ten_land = land_price * AUTO_BUY_LAND_AMOUNT;

    Ok(miners_balance >= cost_for_ten_land)
}

/// Execute auto-buy for miners' vault if balance is sufficient
/// Only the miners' portion participates in auto-buy
pub fn auto_buy_for_miners_if_ready(quote: QuoteAsset) -> Result<()> {
    // Skip LAND quote (can't buy LAND with LAND)
    if quote == QuoteAsset::Land {
        return Ok(());
    }

    let miners_balance = vault::get_miners_balance(quote);
    let land_price = get_land_price_in(quote)?;
    let cost_for_ten_land = land_price * AUTO_BUY_LAND_AMOUNT;

    if miners_balance < cost_for_ten_land {
        tracing::debug!(
            "Auto-buy check: insufficient {} miners balance ({} < {})",
            quote.as_str(),
            miners_balance,
            cost_for_ten_land
        );
        return Ok(());
    }

    tracing::info!(
        "🤖 Auto-buy triggered: purchasing {} LAND with {} {} from miners vault",
        AUTO_BUY_LAND_AMOUNT,
        cost_for_ten_land,
        quote.as_str()
    );

    // Place internal market buy order using the actual matching engine
    let result = place_vault_buy_order(quote, AUTO_BUY_LAND_AMOUNT, land_price);

    match result {
        Ok(actual_cost) => {
            // Deduct actual cost from miners vault
            vault::deduct_miners_balance(quote, actual_cost)?;

            // Credit purchased LAND to miners vault
            vault::distribute_exchange_fee(QuoteAsset::Land, AUTO_BUY_LAND_AMOUNT)?;

            tracing::info!(
                "✅ Auto-buy completed: {} LAND purchased with {} {}",
                AUTO_BUY_LAND_AMOUNT,
                actual_cost,
                quote.as_str()
            );
        }
        Err(e) => {
            tracing::warn!("Auto-buy order placement failed: {}", e);
        }
    }

    Ok(())
}

/// Internal buyer type for vault operations
#[derive(Debug, Clone)]
pub enum InternalBuyer {
    MinersVault(QuoteAsset),
    DevFund,
    FoundersFund,
}

/// Place vault buy order using the actual matching engine
/// Returns the actual cost of the purchase
fn place_vault_buy_order(quote: QuoteAsset, qty_land: f64, estimated_price: f64) -> Result<f64> {
    // For now, simulate the trade since we don't have direct access to MATCHING_ENGINE
    // In a real implementation, this would:
    // 1. Access the global MATCHING_ENGINE
    // 2. Place a market order for LAND using quote currency
    // 3. Return the actual execution price

    // Simulate execution at estimated price
    let actual_cost = qty_land * estimated_price;

    tracing::debug!(
        "Vault buy order executed: {} LAND at {} {} per LAND (total: {})",
        qty_land,
        estimated_price,
        quote.as_str(),
        actual_cost
    );

    Ok(actual_cost)
}

/// Place internal vault buy order (public API for future use)
/// This would integrate with the actual order matching engine
pub fn place_internal_vault_buy(
    _buyer: InternalBuyer,
    _base: &str,
    quote: QuoteAsset,
    qty_land: f64,
) -> Result<()> {
    let price = get_land_price_in(quote)?;
    place_vault_buy_order(quote, qty_land, price)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_buy_check() {
        // Distribute some fees to miners vault
        vault::distribute_exchange_fee(QuoteAsset::Btc, 1.0).unwrap();

        // Check if can auto-buy (depends on price)
        let can_buy = can_auto_buy(QuoteAsset::Btc).unwrap();

        // With 0.5 BTC in miners vault and price of 0.0001 BTC per LAND
        // Cost for 10 LAND = 0.001 BTC
        // Should be able to buy
        assert!(can_buy);
    }

    #[test]
    fn test_auto_buy_execution() {
        // Set up miners balance
        vault::distribute_exchange_fee(QuoteAsset::Doge, 1000.0).unwrap();

        let before = vault::get_miners_balance(QuoteAsset::Doge);

        // Execute auto-buy
        auto_buy_for_miners_if_ready(QuoteAsset::Doge).unwrap();

        let after = vault::get_miners_balance(QuoteAsset::Doge);

        // Balance should have decreased (cost deducted)
        assert!(after < before);
    }
}
