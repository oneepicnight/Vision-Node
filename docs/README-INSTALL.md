# Vision Node v1.0 - Installation Guide

## Quick Start (Easiest)

1. **Extract this ZIP** to a folder (e.g., `C:\VisionNode\`)
2. **Double-click** `START-VISION-NODE.bat`
3. **Wait** for server to start
4. Server runs on: `http://127.0.0.1:7070`

That's it! 🎉

---

## What This Is

Vision Node is the blockchain and exchange backend for the Vision Wallet Marketplace.

**Provides:**
- ✅ Exchange trading engine (BTC/LAND, BCH/LAND, DOGE/LAND pairs)
- ✅ Order matching and execution
- ✅ Blockchain operations
- ✅ Wallet API (send/receive LAND tokens)
- ✅ Vault system (deposits/rewards)

---

## How to Use

### Method 1: Double-Click (Easiest)
```
Double-click: START-VISION-NODE.bat
```

### Method 2: PowerShell
```powershell
.\START-VISION-NODE.ps1
```

### Method 3: Direct
```powershell
.\vision-node.exe
```

---

## Server Info

- **Port:** 7070
- **Exchange API:** `http://127.0.0.1:7070/api/market/exchange/*`
- **Wallet API:** `http://127.0.0.1:7070/wallet/*`
- **Vault API:** `http://127.0.0.1:7070/vault/*`

---

## Testing Endpoints

### Check if running
```powershell
Invoke-RestMethod "http://127.0.0.1:7070/status"
```

### Get BTC/LAND order book
```powershell
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/book?chain=BTC&depth=10"
```

### Get BCH/LAND ticker
```powershell
Invoke-RestMethod "http://127.0.0.1:7070/api/market/exchange/ticker?chain=BCH"
```

---

## Trading Pairs

All pairs use **LAND** as the quote currency:
- **BTC/LAND** - Bitcoin to LAND
- **BCH/LAND** - Bitcoin Cash to LAND
- **DOGE/LAND** - Dogecoin to LAND
- **LAND/LAND** - LAND to LAND

---

## Stopping the Server

Press `Ctrl+C` in the terminal window

---

## Troubleshooting

### "Port 7070 already in use"
Another instance is running. Close it first:
```powershell
Get-Process | Where-Object { $_.ProcessName -like "*vision*" } | Stop-Process
```

### "File not found"
Make sure `vision-node.exe` is in the same folder as the starter scripts.

### Windows Security Warning
Click "More info" → "Run anyway" (Windows SmartScreen protection)

---

## Files Included

- `vision-node.exe` - Main executable (~6 MB)
- `START-VISION-NODE.bat` - Double-click launcher
- `START-VISION-NODE.ps1` - PowerShell launcher
- `README-INSTALL.md` - This file
- `Cargo.toml` - Project configuration
- `VERSION` - Version number

---

## Use with Vision Wallet

The Vision Wallet Marketplace requires this node to be running for:
- Exchange trading
- Wallet operations
- Vault features

**Start both:**
1. Start Vision Node (this) on port 7070
2. Start Vision Wallet on port 4173

---

## System Requirements

- Windows 10/11
- 2GB RAM
- 50MB disk space
- Port 7070 available

---

## Updates

**Version:** 1.0 (LAND Token Corrected)  
**Date:** November 5, 2025  
**Token:** LAND (not VISION)

---

## Support

For issues:
1. Check port 7070 is not in use
2. Run as Administrator if needed
3. Check Windows Firewall settings
4. Review terminal output for errors

---

**Quick Start:** Just double-click `START-VISION-NODE.bat` and you're done! 🚀
