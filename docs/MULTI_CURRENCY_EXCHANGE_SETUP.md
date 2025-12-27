# Multi-Currency Exchange System - Setup Guide

## Overview

The Vision Node now includes a fully functional multi-currency exchange supporting BTC, BCH, and DOGE trading pairs against LAND. This guide covers setup, configuration, and usage.

## Features Implemented

### ✅ Core Infrastructure
- **QuoteAsset System**: Type-safe currency identification (Land, Btc, Bch, Doge)
- **Multi-Currency Vault**: Per-currency fee collection (50% miners, 30% dev, 20% founders)
- **User Wallets**: LAND/BTC/BCH/DOGE balances with available+locked tracking
- **Auto-Buy Logic**: Miners' vault automatically purchases 10 LAND when balance sufficient
- **HD Wallet**: BIP32-based deterministic address generation for deposits

### ✅ Trading Features
- **Balance Locking**: Quote currency locked when buy orders placed
- **Fee Charging**: 0.1% fee charged in quote currency on every trade
- **Fee Distribution**: Fees split 50/30/20 to vault, triggers auto-buy
- **Order Cancellation**: Unlocks funds when orders cancelled

### ✅ External Chain Integration
- **Bitcoin RPC**: Full blockchain scanning for BTC deposits
- **Bitcoin Cash RPC**: BCH deposit support (same RPC interface)
- **Dogecoin RPC**: DOGE deposit support
- **Deposit Scanner**: Background task scanning chains every 30 seconds
- **Address Mapping**: Persistent storage of deposit address → user_id

## Configuration

### Environment Variables

Configure external chain RPC connections:

```bash
# Bitcoin Core RPC
BITCOIN_RPC_URL=http://localhost:8332
BITCOIN_RPC_USER=your_bitcoin_rpc_user
BITCOIN_RPC_PASS=your_bitcoin_rpc_password

# Bitcoin Cash RPC
BCH_RPC_URL=http://localhost:8432
BCH_RPC_USER=your_bch_rpc_user
BCH_RPC_PASS=your_bch_rpc_password

# Dogecoin RPC
DOGE_RPC_URL=http://localhost:22555
DOGE_RPC_USER=your_doge_rpc_user
DOGE_RPC_PASS=your_doge_rpc_password
```

**Note**: If RPC credentials are not provided, the system will run without deposit scanning (trading still works, deposits must be credited manually).

### Running External Chain Nodes

#### Bitcoin Core
```bash
bitcoind -daemon \
  -rpcuser=your_bitcoin_rpc_user \
  -rpcpassword=your_bitcoin_rpc_password \
  -rpcport=8332 \
  -server=1 \
  -txindex=1
```

#### Bitcoin Cash Node
```bash
bitcoind -daemon \
  -rpcuser=your_bch_rpc_user \
  -rpcpassword=your_bch_rpc_password \
  -rpcport=8432 \
  -server=1 \
  -txindex=1
```

#### Dogecoin Core
```bash
dogecoind -daemon \
  -rpcuser=your_doge_rpc_user \
  -rpcpassword=your_doge_rpc_password \
  -rpcport=22555 \
  -server=1 \
  -txindex=1
```

## API Endpoints

### Wallet Endpoints

#### Get Multi-Currency Balances
```bash
GET /wallet/balances?user_id=alice

Response:
{
  "user_id": "alice",
  "balances": {
    "LAND": {
      "available": 1000.0,
      "locked": 100.0,
      "total": 1100.0
    },
    "BTC": {
      "available": 0.5,
      "locked": 0.0,
      "total": 0.5,
      "deposit_address": "bc1q..."
    },
    "BCH": { ... },
    "DOGE": { ... }
  }
}
```

#### Get Deposit Address
```bash
GET /wallet/deposit/BTC?user_id=alice

Response:
{
  "currency": "BTC",
  "user_id": "alice",
  "deposit_address": "bc1q...",
  "network": "BTC Mainnet",
  "confirmations_required": 6,
  "note": "Deposits will be credited after 6 confirmations"
}
```

#### View Vault Status
```bash
GET /vault/status

Response:
{
  "LAND": {
    "miners": 500.0,
    "dev": 300.0,
    "founders": 200.0,
    "total": 1000.0
  },
  "BTC": { ... },
  "BCH": { ... },
  "DOGE": { ... },
  "split": {
    "miners": "50%",
    "dev": "30%",
    "founders": "20%"
  },
  "note": "Exchange fees charged in quote currency, split 50/30/20"
}
```

### Trading Endpoints

#### Place Order
```bash
POST /market/exchange/order
Content-Type: application/json

{
  "owner": "alice",
  "chain": "BTC",
  "price": 0.0001,
  "size": 10,
  "side": "buy",
  "post_only": false,
  "tif": "GTC"
}

# System checks LAND balance, locks funds, places order
```

#### Cancel Order
```bash
POST /market/exchange/cancel
Content-Type: application/json

{
  "owner": "alice",
  "chain": "BTC",
  "order_id": "ord-1234567890-1234"
}

# System unlocks any locked funds
```

#### Get Order Book
```bash
GET /market/exchange/book?chain=BTC&depth=50

Response:
{
  "bids": [[0.0001, 100.0], [0.00009, 200.0]],
  "asks": [[0.00011, 150.0], [0.00012, 250.0]],
  "chain": "BTC"
}
```

## How It Works

### 1. User Deposits External Currency

User sends BTC to their deposit address:
- Vision node scans Bitcoin blockchain every 30 seconds
- Detects transaction to user's address
- Waits for 6 confirmations
- Credits BTC to user's wallet balance

### 2. User Places Buy Order

User places order to buy BTC with LAND:
```
BUY 10 BTC @ 0.0001 LAND per BTC
Total cost: 10 * 0.0001 = 0.001 LAND + 0.1% fee
```

System:
1. Checks user has ≥ 0.001001 LAND available
2. Locks 0.001001 LAND from user's balance
3. Places order in order book

### 3. Order Matches

When a matching sell order arrives:
1. Trade executes at agreed price
2. 0.1% fee charged to taker in LAND
3. Fee distributed to vault: 50% miners, 30% dev, 20% founders
4. Auto-buy check: If miners' LAND ≥ cost of 10 LAND, purchase executes
5. BTC credited to buyer, LAND credited to seller

### 4. User Cancels Order

If order is cancelled before matching:
1. Order removed from book
2. Locked LAND returned to available balance

## Security Considerations

### HD Wallet Seed
- Currently uses deterministic seed from config
- **PRODUCTION**: Store seed in secure hardware (HSM) or encrypted storage
- Seed controls all deposit addresses - losing it means losing access to funds

### RPC Security
- Use strong RPC passwords
- Restrict RPC access to localhost or trusted networks
- Consider using SSL/TLS for RPC connections
- Enable `rpcauth` in Bitcoin Core for hashed passwords

### Deposit Confirmations
- Default: 6 confirmations before crediting deposits
- Adjustable per chain via `confirmations_required()` method
- Higher confirmations = more security, longer wait times

### Address Reuse
- System generates unique address per user
- No address reuse between users
- Address derivation is deterministic and reproducible

## Monitoring

### Logs

Watch for deposit scanning:
```bash
# Successful deposit
✅ Processed deposit: 0.5 BTC to alice (txid: abc123:0)

# Pending deposit
Deposit pending: 0.5 BTC to alice (3/6 confirmations)

# RPC connection issues
⚠️  Bitcoin RPC not configured - deposits disabled
```

Watch for auto-buy:
```bash
🤖 Auto-buy triggered: purchasing 10.0 LAND with 0.001 BTC from miners vault
✅ Auto-buy completed: 10.0 LAND purchased with 0.001 BTC
```

### Metrics

Check vault balances periodically:
```bash
curl http://localhost:7070/vault/status | jq
```

Check user balances:
```bash
curl "http://localhost:7070/wallet/balances?user_id=alice" | jq
```

## Troubleshooting

### Deposits Not Appearing

1. **Check RPC connection:**
   ```bash
   bitcoin-cli -rpcuser=... -rpcpassword=... getblockcount
   ```

2. **Check logs for scanning errors:**
   ```bash
   grep "deposit" logs/vision-node.log
   ```

3. **Verify address mapping:**
   - Address generated correctly?
   - User sent to correct address?
   - Transaction has enough confirmations?

### Orders Failing

1. **Insufficient balance:**
   ```bash
   curl "http://localhost:7070/wallet/balances?user_id=alice"
   ```

2. **Check locked balance:**
   - May have funds locked in other orders
   - Cancel old orders to free up balance

### RPC Connection Issues

1. **Check node is running:**
   ```bash
   bitcoin-cli getblockchaininfo
   ```

2. **Verify credentials in .env**

3. **Check firewall/network:**
   ```bash
   telnet localhost 8332
   ```

## Future Enhancements

### Planned
- [ ] Persistent storage of last scanned block heights
- [ ] Persistent storage of address mappings
- [ ] Withdrawal support (send BTC/BCH/DOGE to external addresses)
- [ ] Multi-signature vault security
- [ ] Cold storage integration
- [ ] Automated market maker (AMM) liquidity pools
- [ ] Cross-chain atomic swaps

### Community Requests
- Trading fee discounts for stakers
- Referral rewards system
- API rate limiting per user
- WebSocket price feed
- Advanced order types (stop-loss, take-profit)

## Support

For issues or questions:
- GitHub: https://github.com/oneepicnight/Vision-Node
- Discord: [Your Discord]
- Email: [Your Email]

## License

Vision Node Multi-Currency Exchange
Copyright (c) 2025 Vision Network
