# 🌐 Bootstrap Node Setup Guide

**Your Public IP**: `12.74.244.112`  
**Required Port**: `7070` (TCP)  
**Date**: November 4, 2025

---

## 🎯 What This Does

Your laptop will act as the **bootstrap node** for the Vision Blockchain testnet. All testers will connect to your node first, then discover each other through peer exchange.

---

## 📋 Step 1: Configure Windows Firewall

### Option A: Using PowerShell (Recommended)
```powershell
# Allow incoming connections on port 7070
New-NetFirewallRule -DisplayName "Vision Blockchain Node" -Direction Inbound -LocalPort 7070 -Protocol TCP -Action Allow

# Verify the rule was created
Get-NetFirewallRule -DisplayName "Vision Blockchain Node"
```

### Option B: Using Windows Defender Firewall GUI
1. Open **Windows Defender Firewall with Advanced Security**
2. Click **Inbound Rules** → **New Rule**
3. Select **Port** → Next
4. Select **TCP**, enter port **7070** → Next
5. Select **Allow the connection** → Next
6. Check all profiles (Domain, Private, Public) → Next
7. Name: **Vision Blockchain Node** → Finish

---

## 📋 Step 2: Configure Router Port Forwarding

### General Steps (varies by router)
1. Open your router admin panel (usually `192.168.1.1` or `192.168.0.1`)
2. Find **Port Forwarding** or **Virtual Server** section
3. Create new rule:
   - **External Port**: `7070`
   - **Internal Port**: `7070`
   - **Internal IP**: Your laptop's local IP (find with `ipconfig`)
   - **Protocol**: `TCP` or `TCP/UDP`
   - **Name**: `Vision Node`
4. Save and apply changes

### Find Your Local IP
```powershell
# Get your laptop's local IP address
ipconfig | Select-String "IPv4"
```
Look for something like `192.168.1.XXX` or `10.0.0.XXX`

### Common Router Interfaces
- **TP-Link**: Advanced → NAT Forwarding → Virtual Servers
- **Netgear**: Advanced → Advanced Setup → Port Forwarding
- **Asus**: WAN → Virtual Server / Port Forwarding
- **Linksys**: Security → Apps and Gaming → Single Port Forwarding
- **D-Link**: Advanced → Port Forwarding

---

## 📋 Step 3: Start Your Bootstrap Node

### Create Bootstrap Configuration
```powershell
# Navigate to vision-node directory
cd C:\vision-node

# Create config.env for bootstrap mode
@"
VISION_PORT=7070
VISION_ADMIN_TOKEN=your-secure-admin-token-here
VISION_BOOTSTRAP=
VISION_SOLO=false
"@ | Out-File -FilePath config.env -Encoding ASCII
```

### Start the Node
```powershell
# Start the node
.\target\release\vision-node.exe
```

The node will start on port 7070 and be accessible at:
- **Local**: `http://localhost:7070`
- **Internet**: `http://12.74.244.112:7070`

---

## 📋 Step 4: Test Port Forwarding

### From Another Computer/Phone (Not Connected to Your WiFi)
```bash
# Test if port is open (use browser or curl)
curl http://12.74.244.112:7070/api/status

# Or visit in browser:
http://12.74.244.112:7070/panel.html
```

### Expected Response
```json
{
  "height": 123,
  "peers": 0,
  "mining": true,
  "difficulty": "...",
  ...
}
```

If you see this, **port forwarding is working!** ✅

---

## 📋 Step 5: Update Tester Instructions

### What Testers Need to Do

**Add this to their `config.env` file:**
```
VISION_BOOTSTRAP=http://12.74.244.112:7070
```

### Complete Tester Config Example
```env
VISION_PORT=7070
VISION_ADMIN_TOKEN=generated-token-here
VISION_BOOTSTRAP=http://12.74.244.112:7070
VISION_SOLO=false
```

Then restart their node. They should see:
- Peers connecting (check `/api/status` or miner panel)
- Blocks syncing from the network
- Their node contributing to the chain

---

## 🔧 Troubleshooting

### Port Not Accessible from Internet

1. **Check Windows Firewall**:
   ```powershell
   Get-NetFirewallRule -DisplayName "Vision Blockchain Node" | Select-Object Enabled, Direction, Action
   ```
   Should show: `Enabled: True`, `Direction: Inbound`, `Action: Allow`

2. **Check Router Port Forwarding**:
   - Verify external port: `7070`
   - Verify internal IP is correct (use `ipconfig`)
   - Some routers need a reboot after configuration

3. **Check ISP Restrictions**:
   - Some ISPs block incoming connections on residential plans
   - Some ISPs use CGNAT (Carrier-Grade NAT) - your public IP might be shared
   - Test with: https://www.yougetsignal.com/tools/open-ports/

4. **Check Node is Running**:
   ```powershell
   # Verify node is listening on port 7070
   Get-NetTCPConnection -LocalPort 7070
   ```

5. **Dynamic IP Warning**:
   - Your public IP `12.74.244.112` may change if your ISP uses dynamic IPs
   - Check your IP periodically: `curl ifconfig.me`
   - Consider using a Dynamic DNS service (No-IP, DuckDNS)

### Peers Not Connecting

1. **Check your node's peer list**:
   ```powershell
   curl http://localhost:7070/api/status | ConvertFrom-Json | Select-Object -ExpandProperty peers
   ```

2. **Check tester's config**:
   - Ensure they have `VISION_BOOTSTRAP=http://12.74.244.112:7070`
   - Ensure `VISION_SOLO=false`

3. **Check both firewalls**:
   - Your node: Inbound on 7070
   - Tester's node: Outbound on 7070 (usually allowed by default)

### Node Crashes or Stops

- **Keep laptop plugged in** (prevent sleep mode)
- **Disable sleep mode**:
  ```powershell
  powercfg /change standby-timeout-ac 0
  powercfg /change monitor-timeout-ac 30
  ```

---

## 📊 Monitoring Your Bootstrap Node

### View Connected Peers
```powershell
# Check peer count every 10 seconds
while ($true) {
    $status = curl http://localhost:7070/api/status | ConvertFrom-Json
    Write-Host "Height: $($status.height) | Peers: $($status.peers.Count) | $(Get-Date -Format 'HH:mm:ss')"
    Start-Sleep -Seconds 10
}
```

### View Peer Details
```powershell
# Get peer list with details
curl http://localhost:7070/api/peers | ConvertFrom-Json | Format-Table
```

### View Logs
```powershell
# Monitor logs in real-time (if logging to file)
Get-Content .\logs\vision-node.log -Wait -Tail 20
```

---

## 🎯 Success Criteria

Your bootstrap node is working correctly when:

✅ Port 7070 accessible from internet (`curl http://12.74.244.112:7070/api/status`)  
✅ Node shows connected peers in `/api/status`  
✅ Blocks are being produced (height increases)  
✅ Testers can sync from your node  
✅ Network forms mesh (peers connect to each other)

---

## 🔐 Security Notes

1. **Admin Token**: Change the default token in `config.env` to something secure
2. **Only Port 7070**: Don't forward other ports unnecessarily
3. **Monitor Logs**: Watch for suspicious activity or attack attempts
4. **Backup Keys**: Keep your wallet keys secure (in `keys.json`)
5. **Bandwidth**: Monitor your internet usage - blockchain can be bandwidth-intensive

---

## 📞 Support

If testers can't connect:
1. Share this document with them
2. Verify your public IP hasn't changed: `curl ifconfig.me`
3. Test port yourself from external network
4. Check Windows Firewall and router logs

---

## 🚀 Quick Reference

**Your Bootstrap URL**: `http://12.74.244.112:7070`

**Tester Config Line**: `VISION_BOOTSTRAP=http://12.74.244.112:7070`

**Test Command**: `curl http://12.74.244.112:7070/api/status`

**Firewall Rule**: `New-NetFirewallRule -DisplayName "Vision Blockchain Node" -Direction Inbound -LocalPort 7070 -Protocol TCP -Action Allow`

---

**Status**: Ready for testnet deployment! 🎉
