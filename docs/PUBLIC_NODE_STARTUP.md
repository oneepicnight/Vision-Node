# Public Node Startup Guide (TCP P2P)

## Port Architecture

**IMPORTANT**: The Vision node now uses TWO separate ports:

- **Port 7070**: HTTP API (status, blockchain queries, wallet operations)
- **Port 7071**: TCP P2P (persistent peer connections, block propagation)

## Startup Scripts Updated

All startup scripts have been updated for the new TCP P2P system:

### 1. START-PUBLIC-NODE.bat (Ngrok - Recommended)

**What it does**:
- Starts Vision node with HTTP on :7070, P2P on :7071
- Launches Ngrok to tunnel P2P port 7071 to the internet
- Provides public address for miners to connect

**Usage**:
```batch
START-PUBLIC-NODE.bat
```

**Requirements**:
- Ngrok installed and in PATH
- Ngrok account (free tier works)

**Expected output**:
```
VISION NODE window:
  INFO  Vision node listening listen=127.0.0.1:7070
  INFO  P2P listener started on 0.0.0.0:7071

NGROK window:
  Forwarding: tcp://0.tcp.ngrok.io:17482 -> localhost:7071
```

**Share with miners**: `0.tcp.ngrok.io:17482`

### 2. START-PUBLIC-NODE-LOCAL.bat (Local Network)

**What it does**:
- Starts Vision node accessible on local network
- No internet tunnel required
- Great for home/office testing

**Usage**:
1. Edit the script and set your local IP:
   ```batch
   SET LOCAL_IP=192.168.1.123
   ```
2. Run:
   ```batch
   START-PUBLIC-NODE-LOCAL.bat
   ```

**Requirements**:
- Know your local IP address (run `ipconfig` to find it)
- Miners must be on same network

**Share with miners**: `192.168.1.123:7071`

### 3. START-PUBLIC-NODE-FREE.bat (Free SSH Tunnel)

**What it does**:
- Starts Vision node with HTTP on :7070, P2P on :7071
- Uses localhost.run (free SSH tunnel) to expose P2P port
- No signup required, completely free

**Usage**:
```batch
START-PUBLIC-NODE-FREE.bat
```

**Requirements**:
- OpenSSH Client (built into Windows 10/11)
- Enable in: Settings > Apps > Optional Features > OpenSSH Client

**Expected output**:
```
FREE P2P TUNNEL window:
  Forwarding TCP traffic from serveo.net:12345 -> localhost:7071
```

**Share with miners**: `serveo.net:12345`

## Miner Connection Instructions

### For Miners Using configure-peer.ps1 (Recommended)

**If public node is on Ngrok**:
```powershell
.\configure-peer.ps1 -PeerIP "0.tcp.ngrok.io" -PeerPort 17482
```

**If public node is on local network**:
```powershell
.\configure-peer.ps1 -PeerIP "192.168.1.123" -PeerPort 7070
```
*(Script automatically calculates P2P port as 7071)*

**If public node is on localhost.run/serveo**:
```powershell
.\configure-peer.ps1 -PeerIP "serveo.net" -PeerPort 12345
```

### For Miners Using Manual Config

Edit `config/node_peer_config.toml`:
```toml
p2p_peer = "0.tcp.ngrok.io:17482"  # Or your public node's P2P address
```

## Verifying Connection

### On Public Node

**Check connected peers**:
```powershell
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json
```

**Expected output**:
```json
{
  "peers": [
    {
      "address": "192.168.1.50:54321",
      "peer_id": "peer-1234567890abcdef",
      "height": 42,
      "direction": "Inbound",
      "last_activity_secs": 5
    }
  ],
  "count": 1
}
```

**Check logs**:
```powershell
Get-Content "logs\vision-node-*.log" | Select-String "Handshake|peer"
```

**Look for**:
```
INFO  Accepted inbound connection peer=192.168.1.50:54321
INFO  Received handshake length prefix received_length=88
INFO  Handshake deserialized protocol_version=1 chain_height=42
INFO  Handshake validation successful
INFO  Peer registered, starting message loop
```

### On Miner

**Check connected peers**:
```powershell
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json
```

**Expected output**:
```json
{
  "peers": [
    {
      "address": "0.tcp.ngrok.io:17482",
      "peer_id": "peer-fedcba0987654321",
      "height": 50,
      "direction": "Outbound",
      "last_activity_secs": 3
    }
  ],
  "count": 1
}
```

**Check logs**:
```
INFO  Connecting to peer peer=0.tcp.ngrok.io:17482
INFO  Sending handshake
INFO  Handshake serialized serialized_length=88
INFO  Received handshake length prefix received_length=88
INFO  Peer handshake received and validated chain_height=50
INFO  Successfully connected to peer height=50
```

## Troubleshooting

### Issue: "Invalid handshake length: 121348160 bytes"

**Cause**: Old node connecting to new node (incompatible versions)

**Fix**: Ensure BOTH nodes are running the latest binary with TCP P2P

### Issue: "Handshake failed: protocol mismatch"

**Cause**: Different protocol versions

**Fix**: Rebuild both nodes from same source code

### Issue: "Handshake failed: genesis mismatch"

**Cause**: Nodes have different genesis blocks

**Fix**: Delete `data/` directory and restart with clean chain

### Enforcing network genesis hashes (opportunity for operators)
If you want to ensure a node only joins a specific network (e.g. testnet vs mainnet), set the allowed genesis hash at runtime:

```powershell
SET VISION_GENESIS_HASH_TESTNET=000000...64hexchars
SET VISION_GENESIS_HASH_MAINNET=ffffffff...64hexchars
```

When set, nodes will refuse to start if the runtime genesis does not match the compiled chain identity.

### Issue: No peers connecting

**Checks**:
1. Verify P2P port 7071 is accessible
   ```powershell
   Test-NetConnection -ComputerName localhost -Port 7071
   ```

2. Check firewall rules
   ```powershell
   netsh advfirewall firewall add rule name="Vision P2P" dir=in action=allow protocol=TCP localport=7071
   ```

3. Verify ngrok tunnel is active (check ngrok window)

4. Confirm peer address is correct in miner's config

### Issue: Peers connect but blocks don't propagate

**Checks**:
1. Verify handshake succeeded in logs
2. Check both nodes have same genesis hash
3. Restart mining to trigger block announcement
4. Watch logs for "Received compact block"

## Quick Start Summary

**For Public Node Operator**:
1. Run `START-PUBLIC-NODE.bat` (or LOCAL/FREE variant)
2. Note the P2P address from ngrok/tunnel window
3. Share P2P address with miners
4. Monitor `/api/tcp_peers` for connections

**For Miner**:
1. Get P2P address from public node operator
2. Run `.\configure-peer.ps1 -PeerIP "address" -PeerPort port`
3. Start mining: `START-VISION-NODE.bat`
4. Check `/api/tcp_peers` shows connection
5. Mine blocks and watch propagation

**Success Indicators**:
- ✅ Both nodes show peer in `/api/tcp_peers`
- ✅ Logs show "Handshake validation successful"
- ✅ Miner finds blocks (~2 second intervals)
- ✅ Public node receives "Received compact block"
- ✅ Heights stay synchronized

## Additional Resources

- **Handshake Protocol Details**: See `docs/HANDSHAKE_FRAMING_FIX.md`
- **TCP P2P Architecture**: See `docs/HANDSHAKE_PROTOCOL_FIX.md`
- **Peer Configuration**: Run `Get-Help .\configure-peer.ps1`

## Port Reference

| Service    | Port | Environment Variable | Description                          |
|------------|------|----------------------|--------------------------------------|
| HTTP API   | 7070 | VISION_PORT          | Status, queries, wallet operations   |
| TCP P2P    | 7071 | VISION_P2P_BIND      | Peer connections, block propagation  |
| Web UI     | 7070 | (same as HTTP)       | Dashboard, admin panel               |

**Note**: If you change VISION_PORT, P2P port automatically becomes PORT+1 unless VISION_P2P_BIND is explicitly set.
