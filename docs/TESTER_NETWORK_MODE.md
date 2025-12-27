# 🌐 Network Mode Setup for Testers

## Connecting to the Vision Blockchain Testnet

To join the network and connect with other nodes, follow these simple steps:

---

## 📋 Step 1: Edit Configuration

After installing Vision Blockchain, you need to edit the configuration file:

1. Open this file in Notepad:
   ```
   %LOCALAPPDATA%\VisionBlockchain\config.env
   ```
   
   **Quick way**: Press `Win+R`, paste this, press Enter:
   ```
   notepad %LOCALAPPDATA%\VisionBlockchain\config.env
   ```

2. Find this line:
   ```
   VISION_BOOTSTRAP=
   ```

3. Change it to:
   ```
   VISION_BOOTSTRAP=http://12.74.244.112:7070
   ```

4. Also make sure this line says `false`:
   ```
   VISION_SOLO=false
   ```

5. Save the file (Ctrl+S) and close Notepad

---

## 📋 Step 2: Restart Your Node

1. If the Vision Node is running, close it
2. Double-click the **Vision Node** icon on your desktop
3. Wait 10-20 seconds for it to connect

---

## 📋 Step 3: Verify Connection

### Check if you're connected:

1. Double-click **Vision Miner** icon on desktop
2. Look at the panel - you should see:
   - **Peers**: 1 or more (instead of 0)
   - **Height**: Should sync up with the network
   - **Status**: "Connected to network"

### Alternative: Check via browser
Open this URL in your browser:
```
http://localhost:7070/api/status
```

Look for `"peers"` - should show connected nodes.

---

## ✅ You're Connected When...

- ✅ Peer count is 1 or higher
- ✅ Block height increases automatically
- ✅ You see "Syncing" or "Synchronized" status
- ✅ Your miner is contributing to the network

---

## 🔧 Troubleshooting

### "Peers: 0" - Not Connecting

1. **Check your config**:
   - Open: `%LOCALAPPDATA%\VisionBlockchain\config.env`
   - Verify: `VISION_BOOTSTRAP=http://12.74.244.112:7070`
   - Verify: `VISION_SOLO=false`

2. **Restart the node**:
   - Close Vision Node
   - Wait 5 seconds
   - Start it again from desktop icon

3. **Check your firewall**:
   - Windows Firewall might be blocking outgoing connections
   - Usually not an issue, but check if antivirus is blocking

4. **Test bootstrap node**:
   - Open browser, go to: `http://12.74.244.112:7070/api/status`
   - Should show blockchain status
   - If it doesn't load, the bootstrap node might be down

### Blocks Not Syncing

- Wait 30-60 seconds - initial sync takes time
- Check peer count - need at least 1 peer
- Restart your node if stuck

### "Connection Refused" Error

- Bootstrap node might be offline
- Check if you can access: `http://12.74.244.112:7070/api/status`
- Contact support if bootstrap is down

---

## 🎯 Network Mode vs Solo Mode

| Feature | Solo Mode | Network Mode |
|---------|-----------|--------------|
| Peers | 0 (alone) | 1+ (connected) |
| Blocks | Your own chain | Shared chain |
| Difficulty | Low (easy) | Adjusts with network |
| Mining | All rewards to you | Compete with others |
| Testing | Local only | Real testnet |

**For testnet, use Network Mode!**

---

## 📊 Monitoring Your Connection

### Peer List
```
http://localhost:7070/api/peers
```
Shows all connected peers

### Network Status
```
http://localhost:7070/api/status
```
Shows your node's status and stats

### Miner Panel
```
http://localhost:7070/panel.html
```
Or just click **Vision Miner** desktop icon

---

## 🚀 Complete Config Example

Your `config.env` should look like this:

```env
VISION_PORT=7070
VISION_ADMIN_TOKEN=abc123xyz789
VISION_BOOTSTRAP=http://12.74.244.112:7070
VISION_SOLO=false
```

**Important**:
- Don't change `VISION_PORT` (keep as 7070)
- Your `VISION_ADMIN_TOKEN` will be different (auto-generated)
- Set `VISION_BOOTSTRAP` to the bootstrap node
- Set `VISION_SOLO=false` for network mode

---

## 📞 Need Help?

1. Check if bootstrap node is online: http://12.74.244.112:7070/api/status
2. Verify your config file has correct bootstrap URL
3. Restart your node
4. Check Windows Firewall isn't blocking
5. Contact support with your error message

---

**Bootstrap Node**: `http://12.74.244.112:7070`  
**Updated**: November 4, 2025

🎉 **Happy testing on the Vision Blockchain testnet!**
