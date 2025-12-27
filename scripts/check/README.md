# Vision Network Validator Script

## Overview
The `check-vision-network.ps1` script validates the entire Vision Network stack from Guardian to Beacon to Constellation peer connectivity.

## What It Checks

### 1️⃣ Guardian Online?
- Verifies local Guardian node is running on port 7070
- **Failure means:** Guardian isn't running or wrong port

### 2️⃣ Website Upstream Connected?
- Checks connection to upstream website via `VISION_UPSTREAM_HTTP_BASE`
- **Failure means:** Environment variable not set, Cloudflare tunnel down, or website offline

### 3️⃣ Beacon Responding?
- Pings the beacon service to verify it's operational
- **Failure means:** Beacon routing broken, wrong URL, or upstream server not running

### 4️⃣ Peers Registered?
- Fetches peer list from beacon to verify network formation
- **Failure means:** Bootstrap handshake broken, beacon register not being called, or peers not saving IPs

### 5️⃣ Identity Loaded?
- Verifies node identity and Vision address are properly loaded
- **Failure means:** Bootstrap handshake didn't issue a ticket, identity failed to save to sled, or corrupt cache

## How to Run

### VS Code Terminal
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force
./check-vision-network.ps1
```

### PowerShell
```powershell
powershell -ExecutionPolicy Bypass -File check-vision-network.ps1
```

### From scripts/check folder
```powershell
cd scripts/check
Set-ExecutionPolicy Bypass -Scope Process -Force
./check-vision-network.ps1
```

## Prerequisites
- Guardian node must be running on port 7070
- `VISION_UPSTREAM_HTTP_BASE` environment variable must be set
- Network connectivity to upstream website

## Output
The script provides colored output:
- ✔ **Green:** Check passed
- ✖ **Red:** Check failed (script exits)
- **Cyan:** Section headers and success message

## Success Message
```
=== ⭐ ALL SYSTEMS GO — CONSTELLATION CAN FORM ===
```

This confirms the entire interstellar plumbing is operational, top to bottom.
