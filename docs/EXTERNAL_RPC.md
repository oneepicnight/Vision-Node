# External RPC Configuration

Vision Node supports integration with external blockchain networks (Bitcoin, Bitcoin Cash, Dogecoin) for cross-chain deposits, withdrawals, and price feeds.

## Configuration Methods

### 1. TOML Configuration File (Recommended)

Edit `config/external_rpc.toml`:

```toml
[external_rpc]

  # Bitcoin configuration
  [external_rpc.btc]
  rpc_url = "https://btc.example.com:8332"
  username = "your_rpc_user"     # Optional for public endpoints
  password = "your_rpc_password"  # Optional for public endpoints
  timeout_ms = 8000
  max_retries = 3
  fallback_urls = [
    "https://btc-backup1.example.com:8332",
    "https://btc-backup2.example.com:8332"
  ]

  # Bitcoin Cash configuration
  [external_rpc.bch]
  rpc_url = "https://bch.example.com:8332"
  username = "your_rpc_user"
  password = "your_rpc_password"
  timeout_ms = 8000
  max_retries = 3

  # Dogecoin configuration
  [external_rpc.doge]
  rpc_url = "https://doge.example.com:22555"
  username = "your_rpc_user"
  password = "your_rpc_password"
  timeout_ms = 10000
  max_retries = 5
```

### 2. Environment Variables (Production)

For production deployments, override sensitive credentials using environment variables:

**Bitcoin:**
- `VISION_RPC_BTC_URL` - Override RPC endpoint
- `VISION_RPC_BTC_USER` - Override username
- `VISION_RPC_BTC_PASS` - Override password

**Bitcoin Cash:**
- `VISION_RPC_BCH_URL`
- `VISION_RPC_BCH_USER`
- `VISION_RPC_BCH_PASS`

**Dogecoin:**
- `VISION_RPC_DOGE_URL`
- `VISION_RPC_DOGE_USER`
- `VISION_RPC_DOGE_PASS`

Example (PowerShell):
```powershell
$env:VISION_RPC_BTC_URL = "https://secure-endpoint.example.com:8332"
$env:VISION_RPC_BTC_USER = "production_user"
$env:VISION_RPC_BTC_PASS = "secure_password_123"
```

Example (Linux/macOS):
```bash
export VISION_RPC_BTC_URL="https://secure-endpoint.example.com:8332"
export VISION_RPC_BTC_USER="production_user"
export VISION_RPC_BTC_PASS="secure_password_123"
```

## RPC Provider Options

### Self-Hosted Nodes

**Bitcoin Core:**
```bash
bitcoind -server -rpcuser=vision -rpcpassword=your_password -rpcport=8332
```

**Bitcoin Cash Node:**
```bash
bitcoind -server -rpcuser=vision -rpcpassword=your_password -rpcport=8332
```

**Dogecoin Core:**
```bash
dogecoind -server -rpcuser=vision -rpcpassword=your_password -rpcport=22555
```

### Third-Party Providers

Several providers offer Bitcoin-compatible RPC endpoints:
- BlockCypher (limited free tier)
- QuickNode (paid, reliable)
- Infura (Bitcoin support via RPC)
- GetBlock (multi-chain support)

**Note:** When using public endpoints, you may not need username/password. Simply configure the `rpc_url` and leave `username`/`password` empty.

## Failover Configuration

The system automatically tries fallback URLs if the primary endpoint fails:

```toml
[external_rpc.btc]
rpc_url = "https://primary.example.com:8332"
fallback_urls = [
  "https://backup1.example.com:8332",
  "https://backup2.example.com:8332"
]
max_retries = 3  # Will retry primary 3 times before trying fallback
```

## Required RPC Methods

The Vision Node requires these standard JSON-RPC 2.0 methods:

### Essential (Deposit Scanning):
- `getblockcount` - Get current blockchain height
- `getblockhash` - Get block hash at specific height
- `getblock` - Get block details with transactions

### Optional (Future Features):
- `sendrawtransaction` - Submit withdrawal transactions
- `estimatefee` - Fee estimation for withdrawals
- `gettransaction` - Transaction details lookup
- `listunspent` - UTXO management for hot wallet

## Testing Configuration

After configuring RPC endpoints, check logs on node startup:

```
✅ Bitcoin RPC configured via external_rpc system
✅ Bitcoin Cash RPC configured via external_rpc system
✅ Dogecoin RPC configured via external_rpc system
External RPC clients initialized: 3 chains configured
```

If RPC fails to connect, you'll see:
```
⚠️  Bitcoin RPC not configured - deposits disabled
```

## Security Best Practices

1. **Never commit credentials to git** - Use environment variables for production
2. **Use read-only RPC users** - Limit permissions to essential methods
3. **Enable TLS/SSL** - Use `https://` endpoints whenever possible
4. **Firewall rules** - Restrict RPC access to trusted IPs only
5. **Monitor RPC usage** - Set up alerts for unusual activity

## Troubleshooting

### "Failed to initialize RPC clients" Error
- Check TOML syntax in `config/external_rpc.toml`
- Verify URLs are reachable (try `curl` or browser test)
- Ensure firewall allows outbound connections

### "Invalid block count response" Error
- RPC endpoint may not support standard Bitcoin RPC methods
- Try different endpoint or check provider documentation
- Verify credentials are correct

### Deposits Not Detected
- Ensure RPC endpoint has indexed blockchain data
- Check `getblockcount` returns expected height
- Verify user addresses are generated correctly
- Enable debug logging: `RUST_LOG=vision_node=debug`

## Advanced: Custom Chain Integration

To add support for additional blockchains:

1. Add chain to `src/external_rpc.rs`:
```rust
pub enum ExternalChain {
    Btc,
    Bch,
    Doge,
    Ltc,  // New chain
}
```

2. Add configuration section to `config/external_rpc.toml`
3. Implement backend in `src/market/deposits.rs`
4. Update QuoteAsset enum in `src/market/engine.rs`

## Support

For issues or questions:
- Check logs: `logs/vision-node.log`
- Enable debug mode: `RUST_LOG=vision_node::external_rpc=debug`
- Review blockchain RPC documentation for your provider
