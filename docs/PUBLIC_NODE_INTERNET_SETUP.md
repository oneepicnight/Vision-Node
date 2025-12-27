# Public Node Setup Guide - Internet Access

## Your Network Information

- **Public IP**: 12.74.244.112
- **Local IP**: 192.168.1.123
- **HTTP Port**: 7070
- **P2P Port**: 7071

## Step 1: Configure Windows Firewall

**Run as Administrator**:
```powershell
.\setup-firewall.ps1
```

Or manually:
```powershell
# Run PowerShell as Administrator, then:
netsh advfirewall firewall add rule name="Vision Node HTTP" dir=in action=allow protocol=TCP localport=7070
netsh advfirewall firewall add rule name="Vision Node P2P" dir=in action=allow protocol=TCP localport=7071
```

## Step 2: Configure Router Port Forwarding

**You need to forward these ports on your router**:

| External Port | Internal IP      | Internal Port | Protocol | Description    |
|---------------|------------------|---------------|----------|----------------|
| 7071          | 192.168.1.123    | 7071          | TCP      | P2P (Required) |
| 7070          | 192.168.1.123    | 7070          | TCP      | HTTP (Optional)|

### How to Access Router Settings

1. Open browser and go to your router's admin page (usually one of these):
   - http://192.168.1.1
   - http://192.168.0.1
   - http://10.0.0.1

2. Log in with your router credentials

3. Find the Port Forwarding section (names vary by router):
   - "Port Forwarding"
   - "Virtual Server"
   - "NAT Forwarding"
   - "Applications & Gaming"

4. Add the port forwarding rules as shown in the table above

### Common Router Configuration Examples

**Netgear**:
- Advanced > Advanced Setup > Port Forwarding/Port Triggering

**TP-Link**:
- Forwarding > Virtual Servers

**Linksys**:
- Applications & Gaming > Port Range Forwarding

**ASUS**:
- WAN > Virtual Server / Port Forwarding

**Ubiquiti**:
- Settings > Routing & Firewall > Port Forwarding

## Step 3: Start Public Node

The node will automatically listen on all interfaces (0.0.0.0):

```powershell
START-VISION-NODE.bat
```

Or manually:
```powershell
.\vision-node.exe
```

**Verify it's listening**:
```powershell
# Check if P2P port is listening
Test-NetConnection -ComputerName localhost -Port 7071
```

Expected output:
```
ComputerName     : localhost
RemoteAddress    : ::1
RemotePort       : 7071
TcpTestSucceeded : True
```

## Step 4: Test External Access

**From another network** (use your phone's mobile data or ask someone else):

```powershell
# Test P2P port
Test-NetConnection -ComputerName 12.74.244.112 -Port 7071
```

**Or use an online port checker**:
- https://www.yougetsignal.com/tools/open-ports/
- Enter: 12.74.244.112 and port 7071
- Should show "Open"

## Step 5: Configure Miners to Connect

### Option A: Using configure-peer.ps1 (Recommended)

Miners run this command:
```powershell
.\configure-peer.ps1 -PeerIP "12.74.244.112" -PeerPort 7070
```

This automatically:
- Calculates P2P port as 7071
- Creates config/node_peer_config.toml
- Sets p2p_peer = "12.74.244.112:7071"

### Option B: Manual Configuration

Miners edit `config/node_peer_config.toml`:
```toml
p2p_peer = "12.74.244.112:7071"
```

Then start the miner:
```powershell
START-VISION-NODE.bat
```

## Step 6: Verify Connections

### On Public Node

**Check connected peers**:
```powershell
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json
```

**Check logs**:
```powershell
Get-Content "logs\vision-node-*.log" -Tail 50 | Select-String "Handshake|peer|Accepted"
```

Look for:
```
INFO  Accepted inbound connection peer=X.X.X.X:12345
INFO  Handshake validation successful
INFO  Registered new peer connection
```

### On Miner

**Check connection status**:
```powershell
Invoke-WebRequest -Uri "http://localhost:7070/api/tcp_peers" | ConvertFrom-Json
```

Should show:
```json
{
  "peers": [
    {
      "address": "12.74.244.112:7071",
      "peer_id": "peer-...",
      "height": 50,
      "direction": "Outbound",
      "last_activity_secs": 3
    }
  ],
  "count": 1
}
```

## Troubleshooting

### Issue: Port forwarding not working

**Test from outside your network**:
1. Use mobile data (not WiFi)
2. Visit: https://www.yougetsignal.com/tools/open-ports/
3. Enter: 12.74.244.112, port: 7071
4. Should show "Open"

**Common fixes**:
- Reboot router after setting port forwarding
- Check if router has UPnP enabled (disable it, use manual forwarding)
- Some ISPs block inbound connections (call ISP if needed)
- Double-check internal IP is 192.168.1.123

### Issue: Firewall blocking connections

**Temporarily disable firewall to test**:
```powershell
# Run as Administrator
netsh advfirewall set allprofiles state off
```

**Test connection, then re-enable**:
```powershell
netsh advfirewall set allprofiles state on
```

**If that worked, add specific rules**:
```powershell
.\setup-firewall.ps1
```

### Issue: Dynamic IP changes

**Check if your public IP changed**:
```powershell
Invoke-RestMethod -Uri "https://api.ipify.org?format=json"
```

**Solutions**:
- Use Dynamic DNS (DDNS) service
  - No-IP (free): https://www.noip.com
  - DuckDNS (free): https://www.duckdns.org
- Check router for built-in DDNS support
- Update miners with new IP if it changes

### Issue: No peers connecting

**Checklist**:
1. ✅ Windows Firewall rules added
2. ✅ Router port forwarding configured
3. ✅ Node running and listening on 0.0.0.0:7071
4. ✅ External port test shows "Open"
5. ✅ Miners have correct IP (12.74.244.112:7071)
6. ✅ Miners using updated binary (same version as public node)

### Issue: Handshake fails

**Check logs on both sides**:
```powershell
Get-Content "logs\vision-node-*.log" | Select-String "Handshake"
```

**Common causes**:
- Different binary versions (old vs new handshake)
- Different genesis blocks
- Protocol version mismatch

**Fix**: Ensure both nodes built from same source code

## Alternative: Using Ngrok (Easier Setup)

If port forwarding is too complicated, use Ngrok:

```powershell
# Start node
START-VISION-NODE.bat

# In another terminal
ngrok tcp 7071
```

Ngrok will give you an address like:
```
tcp://0.tcp.ngrok.io:17482
```

**Miners connect to**:
```powershell
.\configure-peer.ps1 -PeerIP "0.tcp.ngrok.io" -PeerPort 17482
```

**Pros**:
- No router configuration needed
- No firewall issues
- Works from anywhere

**Cons**:
- Requires ngrok running
- Free tier has session limits
- Slightly higher latency

## Security Considerations

**Exposing your node to the internet**:

1. **Keep software updated**: Always run latest version
2. **Monitor connections**: Check `/api/tcp_peers` regularly
3. **Watch logs**: Look for suspicious connection patterns
4. **Consider firewall rules**: Only allow specific IPs if possible
5. **Backup regularly**: Keep blockchain data backed up

**Optional: Whitelist known miners**:
If you know miner IPs, you can restrict access:
```powershell
# Only allow specific IP
netsh advfirewall firewall set rule name="Vision Node P2P" new remoteip=X.X.X.X
```

## Summary

✅ **Your public node address**: 12.74.244.112:7071  
✅ **Miners connect with**: `.\configure-peer.ps1 -PeerIP "12.74.244.112" -PeerPort 7070`  
✅ **Check peers**: `http://localhost:7070/api/tcp_peers`  
✅ **Monitor logs**: Look for "Handshake validation successful"  

**Share with miners**: "Connect to 12.74.244.112:7071"
