# External Blockchain RPC Configuration

Vision Node can monitor Bitcoin, Bitcoin Cash, and Dogecoin blockchains for incoming deposits to user addresses.

## Configuration Methods

### Option 1: Environment Variables (Recommended for Docker/Production)

```bash
# Bitcoin
export VISION_RPC_BTC_URL="http://localhost:8332"
export VISION_RPC_BTC_USER="bitcoinrpc"
export VISION_RPC_BTC_PASS="your_password"

# Bitcoin Cash
export VISION_RPC_BCH_URL="http://localhost:8442"
export VISION_RPC_BCH_USER="bitcoincashrpc"
export VISION_RPC_BCH_PASS="your_password"

# Dogecoin
export VISION_RPC_DOGE_URL="http://localhost:22555"
export VISION_RPC_DOGE_USER="dogecoinrpc"
export VISION_RPC_DOGE_PASS="your_password"
```

### Option 2: JSON Configuration File

Copy `external_rpc.json.example` to `external_rpc.json` and edit:

```json
{
  "btc": {
    "rpc_url": "http://localhost:8332",
    "username": "bitcoinrpc",
    "password": "your_password_here"
  }
}
```

Place the file in:
- `./external_rpc.json` (same directory as executable)
- `./config/external_rpc.json`

## Requirements

You need to run full nodes or connect to RPC endpoints for each blockchain you want to monitor:

- **Bitcoin Core** (bitcoind) - Default port: 8332
- **Bitcoin Cash Node** (bitcoincashd) - Default port: 8442  
- **Dogecoin Core** (dogecoind) - Default port: 22555

## Confirmation Requirements

- BTC: 3 confirmations
- BCH: 6 confirmations
- DOGE: 20 confirmations

## API Endpoints

Once configured, these endpoints become available:

- `GET /api/deposits/status` - Check if deposit monitoring is enabled
- `GET /api/wallet/deposit/{currency}?user_id={address}` - Get deposit address

## Security Notes

⚠️ **IMPORTANT**: 
- Keep RPC credentials secure
- Do NOT expose RPC ports to the internet
- Use localhost/127.0.0.1 bindings for RPC servers
- If running VISION_PUBLIC_NODE=true, ensure RPC endpoints are firewalled

## Disabling Deposit Monitoring

Simply don't configure any RPC endpoints. The node will work normally without deposit monitoring.

Logs will show:
```
[DEPOSITS] Deposit scanner disabled (no external RPC config found)
```
