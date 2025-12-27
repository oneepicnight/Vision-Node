# Vision Node v1.0 - LAND Token Update

**Date:** November 5, 2025  
**Version:** 1.0 (LAND Corrected)

## Changes

### Token Name Correction ✅
All exchange trading pairs now correctly use **LAND** as the quote currency (previously used "VISION").

**Updated Trading Pairs:**
- BTC/LAND (was BTC/VISION)
- BCH/LAND (was BCH/VISION)
- DOGE/LAND (was DOGE/VISION)
- LAND/LAND (was LAND/VISION)

### Files Modified
- `src/main.rs` - 6 instances updated in exchange endpoints:
  - `exchange_book` (line 5631)
  - `exchange_ticker` (line 5656)
  - `exchange_trades` (line 5685)
  - `exchange_my_orders` (line 5710)
  - `exchange_create_order` (line 5776)
  - `exchange_buy` (line 5864)

## Installation

1. Extract `vision-node.exe` from this ZIP
2. Run with: `.\vision-node.exe` or `cargo run --release`
3. Server listens on port 7070

## Exchange Trading

All exchange endpoints now return LAND as the quote currency:
- `/api/market/exchange/book?chain=BTC` - Returns BTC/LAND order book
- `/api/market/exchange/ticker?chain=BCH` - Returns BCH/LAND ticker
- `/api/market/exchange/trades?chain=DOGE` - Returns DOGE/LAND trades

## Compatibility

✅ Fully compatible with Vision Wallet Marketplace  
✅ No breaking changes to API structure  
✅ Only token naming updated (VISION → LAND)

## Testing

```powershell
# Start Vision Node
.\vision-node.exe

# Test exchange endpoints
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/book?chain=BTC&depth=10"
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/ticker?chain=BTC"
```

Expected output now shows "LAND" in trading pair symbols.

---

**Built:** November 5, 2025  
**Rust Version:** cargo 1.x  
**Architecture:** x86_64-pc-windows-msvc
