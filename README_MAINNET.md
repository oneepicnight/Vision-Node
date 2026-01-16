# Vision Node v1.0.3 - Fort Knox Mainnet Edition

## 🔒 SECURITY NOTICE: HARDENED FOR PRODUCTION

This build is **fully hardcoded** for mainnet security. All consensus-critical parameters are locked down and cannot be changed via environment variables or configuration files.

---

## ⚡ Quick Start

### 1. Run the Node
```powershell
.\vision-node.exe
```

### 2. Access the Wallet UI
Open in browser: http://localhost:7070

### 3. (Optional) Set Admin Token
```powershell
$env:VISION_ADMIN_TOKEN="your-secret-token"
.\vision-node.exe
```

---

## 📊 What's Included

- **Binary**: `vision-node.exe` (Windows 64-bit)
- **Data**: Stored in `./vision_data` (created automatically)
- **Ports**: 
  - HTTP API: `127.0.0.1:7070` (localhost only)
  - P2P Network: `0.0.0.0:7072` (public)

---

## 🔒 Fort Knox Security Features

✅ **All consensus rules hardcoded** - No runtime tampering  
✅ **HTTP API localhost-only** - No accidental internet exposure  
✅ **P2P seed peers hardcoded** - Connect to trusted mainnet nodes  
✅ **Archival mode always on** - Keep full blockchain history  
✅ **No .env files needed** - Simplest possible deployment  

See [FORT_KNOX_LOCKDOWN.md](FORT_KNOX_LOCKDOWN.md) for complete technical details.

---

## 🌐 Network Configuration

### Mainnet Settings (Hardcoded)
- Block Time: 2 seconds
- Max Reorg: 36 blocks
- P2P Port: 7072
- HTTP Port: 7070 (localhost only)
- Max Peers: 50
- Data Directory: `./vision_data`

### Seed Peers
Automatically connects to hardcoded mainnet seed peers. No configuration needed.

---

## 📝 Environment Variables (Optional)

**Only 2 environment variables are supported:**

1. **`VISION_ADMIN_TOKEN`** - Admin API authentication
   ```powershell
   $env:VISION_ADMIN_TOKEN="your-secret-token"
   ```

2. **`VISION_LOG`** - Logging level
   ```powershell
   $env:VISION_LOG="info"  # Options: error, warn, info, debug, trace
   ```

**All other environment variables are IGNORED for security.**

---

## 🎯 What's Changed in v1.0.3

### Fort Knox Lockdown
- ✅ Hardcoded all consensus parameters
- ✅ Hardcoded P2P configuration
- ✅ HTTP API localhost-only (no public mode)
- ✅ Removed 50+ environment variables
- ✅ Archived all .env files (no longer used)

### Swarm Intelligence (v1.0.2)
- ✅ Increased max peers: 8 → 50
- ✅ Added reputation tier logging
- ✅ P2P auto-restart on crash
- ✅ Periodic reputation summaries

### Sync Improvements (v1.0.1)
- ✅ Increased sync timeouts: 8s/5s → 20s/15s
- ✅ Better handling of large block gaps

---

## 📂 Directory Structure

```
vision-node-v1.0.3-windows-mainnet/
├── vision-node.exe              # Main executable
├── README_MAINNET.md            # This file
├── FORT_KNOX_LOCKDOWN.md        # Technical documentation
├── CHANGELOG.md                 # Full version history
└── archived-env-files/          # Old .env files (not used)
    ├── .env.ARCHIVED
    ├── .env.example.ARCHIVED
    └── ... (reference only)
```

**Data created at runtime:**
```
./vision_data/                   # Blockchain database (auto-created)
```

---

## 🔧 Advanced Usage

### Check Node Status
```powershell
curl http://localhost:7070/status
```

### View Peer Connections
```powershell
curl http://localhost:7070/peers
```

### Check Blockchain Height
```powershell
curl http://localhost:7070/height
```

### Start Mining (if configured)
```powershell
curl -X POST http://localhost:7070/api/miner/start
```

---

## 🛡️ Security Best Practices

1. **Keep Admin Token Secret**
   - Don't share your `VISION_ADMIN_TOKEN`
   - Use a strong, random token (32+ characters)

2. **Protect Wallet Keys**
   - `keys.json` contains your wallet private key
   - Keep backups in secure location
   - Never share or commit to git

3. **Firewall Configuration**
   - HTTP port 7070 is localhost-only (safe)
   - P2P port 7072 should be open for network connectivity

4. **Regular Backups**
   - Backup `./vision_data` directory periodically
   - Especially important if running a miner

---

## 📊 System Requirements

- **OS**: Windows 10/11 (64-bit)
- **RAM**: 8 GB minimum, 16 GB recommended
- **Disk**: 100 GB+ free space (blockchain grows over time)
- **Network**: Stable internet connection (P2P requires port 7072 open)

---

## 🆘 Troubleshooting

### Node Won't Start
- Check if port 7070 or 7072 is already in use
- Check Windows Firewall settings
- Review logs with: `$env:VISION_LOG="debug"; .\vision-node.exe`

### Can't Access UI
- Verify URL is `http://localhost:7070` (not 127.0.0.1)
- Check browser isn't blocking localhost connections
- Try incognito/private browsing mode

### Sync Issues
- Wait 1-2 minutes for P2P connections
- Check firewall allows port 7072 outbound
- Hardcoded seed peers will auto-connect

### Mining Not Working
- Ensure mining is started via `/api/miner/start`
- Check you're synced to network height
- Review mining gate logs: `$env:VISION_LOG="info"`

---

## 🔄 Updating

To update to a newer version:

1. Stop the current node (Ctrl+C)
2. Backup `./vision_data` directory
3. Replace `vision-node.exe` with new version
4. Start node: `.\vision-node.exe`

**Data directory is compatible across versions** (unless noted in changelog).

---

## 📚 Documentation

- **Technical Details**: [FORT_KNOX_LOCKDOWN.md](FORT_KNOX_LOCKDOWN.md)
- **Change History**: [CHANGELOG.md](CHANGELOG.md)
- **Network Status**: https://visionworld.tech

---

## 🤝 Support

- **Website**: https://visionworld.tech
- **Email**: support@visionworld.tech
- **Discord**: [Join our community]

---

## ⚖️ License

Vision Node is open source software. See LICENSE file for details.

---

**🔒 Fort Knox Edition - Hardened for Mainnet Production 🔒**

*Last Updated: January 15, 2026*
