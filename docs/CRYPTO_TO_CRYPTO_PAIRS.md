# Crypto-to-Crypto Trading Pairs

## Overview

The exchange now supports direct crypto-to-crypto trading pairs in addition to LAND-based pairs. This allows users to trade between BTC, BCH, and DOGE without going through LAND as an intermediary.

## Supported Trading Pairs

### LAND-Based Pairs (Original)
- **LAND/BTC** - Trade LAND for Bitcoin
- **LAND/BCH** - Trade LAND for Bitcoin Cash
- **LAND/DOGE** - Trade LAND for Dogecoin
- **CASH/LAND** - Trade CASH for LAND

### Crypto-to-Crypto Pairs (New)
- **BTC/BCH** - Trade Bitcoin for Bitcoin Cash
- **BTC/DOGE** - Trade Bitcoin for Dogecoin
- **BCH/DOGE** - Trade Bitcoin Cash for Dogecoin

And the inverse:
- **BCH/BTC** - Trade Bitcoin Cash for Bitcoin
- **DOGE/BTC** - Trade Dogecoin for Bitcoin
- **DOGE/BCH** - Trade Dogecoin for Bitcoin Cash

## API Usage

### Chain Parameter Format

The `chain` parameter now accepts two formats:

1. **Single currency** (defaults to LAND as base):
   - `chain=BTC` → LAND/BTC pair
   - `chain=BCH` → LAND/BCH pair
   - `chain=DOGE` → LAND/DOGE pair

2. **Pair format** (explicit base/quote):
   - `chain=BTC/BCH` → BTC/BCH pair (BTC is base, BCH is quote)
   - `chain=BTC/DOGE` → BTC/DOGE pair
   - `chain=BCH/DOGE` → BCH/DOGE pair
   - `chain=BCH/BTC` → BCH/BTC pair

### Examples

#### Place Order on BTC/BCH Market
```json
POST /api/market/exchange/order
{
  "owner": "user123",
  "chain": "BTC/BCH",
  "side": "sell",
  "price": 15.5,
  "size": 0.1,
  "post_only": false,
  "tif": "GTC"
}
```

**What this does:**
- Sells 0.1 BTC
- Wants to receive 1.55 BCH (0.1 × 15.5)
- Fee charged in BCH (0.1% of 1.55 = 0.00155 BCH)

#### Get Order Book for BTC/DOGE
```
GET /api/market/exchange/book?chain=BTC/DOGE&depth=50
```

#### Get Ticker for BCH/DOGE
```
GET /api/market/exchange/ticker?chain=BCH/DOGE
```

## Fee Structure

Fees are always charged in the **quote currency**:

| Trading Pair | Fee Currency |
|--------------|--------------|
| LAND/BTC | BTC |
| LAND/BCH | BCH |
| LAND/DOGE | DOGE |
| BTC/BCH | BCH |
| BTC/DOGE | DOGE |
| BCH/DOGE | DOGE |
| BCH/BTC | BTC |
| DOGE/BTC | BTC |

### Example: Trading BTC/BCH
- **Buy Order:** User buys BTC with BCH
  - Locks: BCH (quote currency)
  - Fee: 0.1% in BCH
  
- **Sell Order:** User sells BTC for BCH
  - Locks: BTC (base currency)
  - Receives: BCH (quote currency)
  - Fee: 0.1% in BCH (deducted from proceeds)

## Balance Locking

### Buy Orders
When placing a buy order, the system locks the **quote currency**:
- `BTC/BCH` buy → Locks BCH
- `BTC/DOGE` buy → Locks DOGE
- `BCH/DOGE` buy → Locks DOGE

### Sell Orders
When placing a sell order, the system locks the **base currency**:
- `BTC/BCH` sell → Locks BTC
- `BTC/DOGE` sell → Locks BTC
- `BCH/DOGE` sell → Locks BCH

## Order Cancellation

When canceling an order, the system automatically unlocks the correct currency:
- Canceled buy order → Unlocks quote currency
- Canceled sell order → Unlocks base currency

## Vault Distribution

Fees from crypto-to-crypto pairs are distributed to the corresponding currency vault:

**BTC/BCH pair fees (in BCH):**
- 50% → BCH miners bucket
- 30% → BCH dev bucket
- 20% → BCH founders bucket

**BTC/DOGE pair fees (in DOGE):**
- 50% → DOGE miners bucket
- 30% → DOGE dev bucket
- 20% → DOGE founders bucket

**BCH/DOGE pair fees (in DOGE):**
- 50% → DOGE miners bucket
- 30% → DOGE dev bucket
- 20% → DOGE founders bucket

## Frontend Integration

Update the currency selector in the Exchange UI to include crypto-to-crypto pairs:

```typescript
const pairs = [
  { key: 'BTC', label: 'LAND/BTC' },
  { key: 'BCH', label: 'LAND/BCH' },
  { key: 'DOGE', label: 'LAND/DOGE' },
  { key: 'CASH', label: 'CASH/LAND' },
  // New crypto-to-crypto pairs
  { key: 'BTC/BCH', label: 'BTC/BCH' },
  { key: 'BTC/DOGE', label: 'BTC/DOGE' },
  { key: 'BCH/DOGE', label: 'BCH/DOGE' },
]
```

## Testing Scenarios

### Scenario 1: Trade BTC for BCH
1. Alice has 1 BTC, wants BCH
2. Places sell order: `chain=BTC/BCH`, `side=sell`, `price=15.0`, `size=1.0`
3. System locks 1 BTC from Alice's balance
4. Bob places buy order matching Alice's sell
5. Trade executes:
   - Alice receives: 15.0 BCH (minus 0.015 BCH fee = 14.985 BCH)
   - Bob receives: 1.0 BTC
   - Fee: 0.015 BCH distributed to BCH vault (50/30/20)

### Scenario 2: Trade BCH for DOGE
1. Charlie has 10 BCH, wants DOGE
2. Places sell order: `chain=BCH/DOGE`, `side=sell`, `price=5000`, `size=10`
3. System locks 10 BCH
4. Trade executes at 5000 DOGE per BCH
5. Charlie receives: 50,000 DOGE (minus 50 DOGE fee = 49,950 DOGE)
6. Fee: 50 DOGE to DOGE vault

### Scenario 3: Cancel Order
1. Dave places buy order: `chain=BTC/DOGE`, `size=0.5`
2. System locks required DOGE from Dave's balance
3. Dave cancels order
4. System unlocks DOGE back to available balance

## Implementation Details

### Helper Function
```rust
fn parse_trading_pair(chain: &str) -> (String, QuoteAsset) {
    if chain.contains('/') {
        // Parse explicit pair: "BTC/BCH" → (BTC, BCH)
        let parts: Vec<&str> = chain.split('/').collect();
        let base = parts[0].to_string();
        let quote = QuoteAsset::from_str(parts[1]);
        return (base, quote);
    }
    
    // Default: LAND/{chain}
    (String::from("LAND"), QuoteAsset::from_str(chain))
}
```

### Updated Endpoints
All exchange endpoints now use `parse_trading_pair()`:
- `exchange_create_order()` - Place orders
- `exchange_buy()` - Market buy
- `exchange_cancel_order()` - Cancel orders
- `exchange_book()` - Order book
- `exchange_ticker()` - Ticker data
- `exchange_trades()` - Recent trades
- `exchange_my_orders()` - User's orders

## Benefits

1. **Direct Trading:** No need to go through LAND as intermediary
2. **Lower Friction:** One trade instead of two (e.g., BTC→LAND→BCH becomes BTC→BCH)
3. **Better Liquidity:** Separate order books for each pair
4. **Fee Efficiency:** Single fee instead of double (two trades)
5. **Arbitrage Opportunities:** Price differences between pairs can be exploited

## Migration Notes

- **Backward Compatible:** Existing LAND-based pairs work exactly as before
- **New Parameter Format:** Frontend can use slash notation for new pairs
- **No Database Changes:** Uses existing order book infrastructure
- **Fee System:** Already supports multi-currency fees per vault

## Summary

The exchange now supports 10+ trading pairs:
- 4 LAND-based pairs (LAND/BTC, LAND/BCH, LAND/DOGE, CASH/LAND)
- 6+ crypto-to-crypto pairs (BTC/BCH, BTC/DOGE, BCH/DOGE and inverses)

All pairs maintain the same 0.1% fee structure with proper quote currency detection and vault distribution.
