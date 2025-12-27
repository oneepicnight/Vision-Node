# Testnet to Mainnet Migration Guide

## Overview

The Vision Network testnet sunsets at block 1,000,000, triggering an automatic wallet export and requiring manual migration to mainnet. This document guides users through the migration process.

## Testnet Sunset Timeline

### Phase 1: Pre-Sunset (Blocks 900,000 - 999,999)
- **Warning Messages**: Nodes display countdown in logs
- **Wallet Preparation**: Users advised to export keys preemptively
- **Final Testing**: Complete any remaining testnet experiments

### Phase 2: Sunset Block (Block 1,000,000)
- **Automatic Export**: Node exports wallets to `migration-testnet-to-mainnet.json`
- **Mining Disabled**: No new blocks mined after this height
- **P2P Shutdown**: Nodes reject new blocks and stop syncing
- **Graceful Halt**: Node displays migration instructions and exits

### Phase 3: Post-Sunset (After Block 1,000,000)
- **Node Refuses Start**: Testnet nodes won't restart if sunset flag set
- **Data Preserved**: Local blockchain data remains intact for auditing
- **Migration Window**: Users have unlimited time to migrate keys

## Automatic Wallet Export

### Export File Location
```
migration-testnet-to-mainnet.json
```

### Export File Structure
```json
{
  "network": "testnet",
  "export_height": 1000000,
  "timestamp": "2024-12-01T00:00:00Z",
  "keys": {
    "address1": {
      "key": "hex_encoded_private_key",
      "balance": "1234567890"
    },
    "address2": {
      "key": "hex_encoded_private_key",
      "balance": "9876543210"
    }
  }
}
```

### Manual Pre-Sunset Export
If you want to export before sunset:
```bash
# Export current wallet state
curl http://localhost:7070/wallet/export > my-wallet-backup.json

# Or use CLI tool
vision-wallet export --output my-wallet-backup.json
```

## Migration Process

### Step 1: Stop Testnet Node
```powershell
# Kill any running testnet nodes
Get-Process -Name "vision-node" | Stop-Process -Force

# Verify testnet data is backed up
Copy-Item -Recurse vision_data_7070 backups/testnet-final
```

### Step 2: Verify Migration File
```powershell
# Check migration file exists
Get-Content migration-testnet-to-mainnet.json | ConvertFrom-Json

# Verify key count
$migration = Get-Content migration-testnet-to-mainnet.json | ConvertFrom-Json
Write-Host "Keys to migrate: $($migration.keys.Count)"
```

### Step 3: Prepare Mainnet Environment
```bash
# Clean any existing mainnet data (if you had a dev instance)
rm -rf vision_data_7070

# Set mainnet environment
export VISION_NETWORK=mainnet
export VISION_PORT=7070

# Download latest mainnet release
curl -L https://github.com/vision-network/vision-node/releases/latest/download/vision-node-linux-x64.tar.gz | tar xz
```

### Step 4: Import Keys to Mainnet
```bash
# Method 1: Automatic import from migration file
./vision-node --import-migration migration-testnet-to-mainnet.json

# Method 2: Manual key import (for each address)
./vision-wallet import --key <hex_key> --address <address>

# Method 3: Bulk import via script
python3 scripts/import-testnet-keys.py migration-testnet-to-mainnet.json
```

### Step 5: Verify Mainnet Wallet
```bash
# Start mainnet node
VISION_NETWORK=mainnet ./vision-node

# Check wallet addresses
curl http://localhost:7070/wallet/addresses

# Verify balances (initially zero on mainnet)
curl http://localhost:7070/balance/<your_address>
```

## Important Notes

### Testnet Balances Do NOT Transfer
- **Testnet tokens**: Have no economic value
- **Mainnet start**: All native token balances start at zero
- **Mining required**: Earn mainnet tokens through mining
- **Land deeds**: Testnet land deeds do NOT transfer; mainnet has separate genesis distribution

### What DOES Transfer
- **Private keys**: Your cryptographic identity
- **Public addresses**: Same addresses on both networks
- **Wallet software**: Compatible with both networks
- **Experience**: Knowledge of how to use the network

### What Doesn't Transfer
- **Token balances**: Start fresh on mainnet
- **Land deed ownership**: Separate genesis for mainnet
- **Transaction history**: Different blockchain
- **Peer connections**: Different P2P network

## Common Migration Issues

### Issue: "Migration file not found"
**Cause**: Node crashed before sunset or file was deleted
**Solution**: 
```bash
# Re-export from sled database if node won't start
vision-node --export-keys-from-db vision_data_7070 > migration.json
```

### Issue: "Invalid key format"
**Cause**: Corrupted migration file
**Solution**:
```bash
# Validate JSON structure
jq . migration-testnet-to-mainnet.json

# If corrupted, extract keys manually from sled DB
cargo run --bin extract-keys -- vision_data_7070/db
```

### Issue: "Node refuses to start after sunset"
**Cause**: Testnet sunset flag set in database
**Expected Behavior**: This is correct; testnet is permanently sunset
**Solution**: Use mainnet instead (`VISION_NETWORK=mainnet`)

### Issue: "Can't import keys to mainnet"
**Cause**: Mainnet node already has different keys
**Solution**:
```bash
# Backup existing mainnet keys
mv vision_data_7070/keys vision_data_7070/keys.backup

# Import testnet keys
./vision-node --import-migration migration-testnet-to-mainnet.json
```

## Security Best Practices

### Protect Your Keys
1. **Secure Storage**: Store migration file on encrypted disk
2. **Backup Multiple Locations**: Cloud + local + USB drive
3. **Never Share**: Private keys are sensitive; never post publicly
4. **Verify Integrity**: Check file hash before and after transfer

### Validate Mainnet Connection
```bash
# Check genesis hash matches official mainnet
curl http://localhost:7070/status | jq '.genesis_hash'

# Official mainnet genesis: [to be published at mainnet launch]
# If mismatch, you're on wrong network!
```

### Phishing Protection
- **Official Sources Only**: Download node from github.com/vision-network only
- **Verify Signatures**: Check GPG signatures on releases
- **No "Migration Services"**: Migration is self-service; no one should offer to "migrate for you"

## Post-Migration Checklist

- [ ] Testnet node stopped and data backed up
- [ ] Migration file saved in 3+ secure locations
- [ ] Mainnet node installed and configured
- [ ] Keys imported successfully
- [ ] Wallet addresses verified
- [ ] Mainnet genesis hash validated
- [ ] Node syncing with mainnet peers
- [ ] Mining configured (if applicable)

## Mainnet Activation

### CASH Token Genesis (Block 1,000,000)
Unlike testnet sunset, mainnet block 1,000,000 triggers CASH token genesis:
- **Initial Supply**: Minted based on formula
- **Distribution**: Pro-rata to land deed holders
- **No Migration**: CASH is new; doesn't exist on testnet

### Mining on Mainnet
```bash
# Configure miner
export VISION_MINER_ADDRESS=your_mainnet_address
export VISION_MINER_THREADS=4

# Start mining
./vision-node --mine
```

## Support and Resources

### Documentation
- [GENESIS.md](./GENESIS.md) - Genesis block details
- [TOKENOMICS.md](./TOKENOMICS.md) - Mainnet token economics
- [CASH_SYSTEM.md](./CASH_SYSTEM.md) - CASH token details

### Community
- Discord: https://discord.gg/vision-network
- Forum: https://forum.vision-network.io
- GitHub Issues: https://github.com/vision-network/vision-node/issues

### Emergency Contact
- Security issues: security@vision-network.io
- Lost keys: Self-custodial; we cannot recover lost keys
- Bug reports: GitHub Issues

## Timeline Reference

| Event | Block Height | Date (Estimated) |
|-------|--------------|------------------|
| Testnet Launch | 0 | 2024-Q1 |
| Testnet Sunset | 1,000,000 | 2024-Q4 |
| Mainnet Launch | 0 | 2025-Q1 |
| CASH Genesis | 1,000,000 | 2025-Q3 |

*Dates are estimates; actual timeline depends on block production rate*

## FAQ

**Q: Can I run both testnet and mainnet simultaneously?**  
A: Not on same port. Use different ports: `VISION_PORT=7070` (mainnet) and `VISION_PORT=7071` (testnet archive)

**Q: What happens to my land deeds after testnet sunset?**  
A: Testnet land deeds have no value. Mainnet land deeds are distributed at mainnet genesis.

**Q: Can I delay migration?**  
A: Yes. Your keys remain valid indefinitely. Import whenever you're ready.

**Q: Do I lose anything by migrating late?**  
A: You miss out on early mainnet mining rewards, but keys never expire.

**Q: Can I revert back to testnet after migration?**  
A: Testnet is permanently sunset at block 1,000,000. No revival planned.
