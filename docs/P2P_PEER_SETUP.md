# Vision Node - P2P Peer Connection Guide

## You're Correct!

Each miner/node operator runs **their own node** locally. They don't need to point their wallet at your node. They just need to **peer with your public node** so transactions and blocks can gossip between nodes.

## Your Public Node Setup

### Current:
- **Local:** `http://127.0.0.1:7070`
- **TCP Tunnel:** `tcp://4.tcp.us-cal-1.ngrok.io:11805` → localhost:7070

### Problem:
TCP tunnel works for raw TCP, but P2P gossip uses HTTP endpoints like:
- `POST /peer/add`
- `POST /gossip/tx`
- `POST /gossip/block`
- `GET /peers`

### Solution - Use HTTP Tunnel:
```bash
ngrok http 7070
```

This gives you an HTTP URL like:
```
http://abc123.ngrok-free.app
```

## How Other Nodes Connect to Your Public Node

### Step 1: They Start Their Own Node
```bash
cd VisionNode-v0.1.6-testnet2-WIN64
.\START-VISION-NODE.bat
```

Their node runs on: `http://localhost:7070` (or another port if configured)

### Step 2: They Add Your Node as a Peer

**PowerShell:**
```powershell
$yourPublicUrl = "http://YOUR-NGROK-URL"
$theirAdminToken = "THEIR_ADMIN_TOKEN"

$body = @{
    url = $yourPublicUrl
} | ConvertTo-Json

Invoke-WebRequest -Method POST `
  -Uri "http://localhost:7070/peer/add?token=$theirAdminToken" `
  -Body $body `
  -ContentType "application/json"
```

**curl:**
```bash
curl -X POST 'http://localhost:7070/peer/add?token=THEIR_ADMIN_TOKEN' \
  -H 'Content-Type: application/json' \
  -d '{"url": "http://YOUR-NGROK-URL"}'
```

### Step 3: Automatic Gossip!

Once peered:
- ✅ **Transactions** propagate automatically via `/gossip/tx`
- ✅ **Blocks** propagate automatically via `/gossip/block`
- ✅ **Chain sync** happens automatically
- ✅ **Consensus** maintained across network

## P2P Endpoints

Your node exposes these P2P endpoints:

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/peers` | GET | List all connected peers |
| `/peer/add` | POST | Add a new peer (requires admin token) |
| `/gossip/tx` | POST | Receive gossiped transaction |
| `/gossip/block` | POST | Receive gossiped block |

## Testing P2P Connection

### Check Your Peers:
```powershell
Invoke-RestMethod http://127.0.0.1:7070/peers
```

### Add a Test Peer (on your node):
```powershell
$body = @{url = "http://another-node:7070"} | ConvertTo-Json
Invoke-WebRequest -Method POST `
  -Uri "http://127.0.0.1:7070/peer/add?token=YOUR_ADMIN_TOKEN" `
  -Body $body -ContentType "application/json"
```

## What Miners Need from You

**Share with other node operators:**

1. **Your public node URL:** `http://YOUR-NGROK-URL`
2. **Instruction:** Add this peer command to their node:

```powershell
$body = @{url = "http://YOUR-NGROK-URL"} | ConvertTo-Json
Invoke-WebRequest -Method POST `
  -Uri "http://localhost:7070/peer/add?token=THEIR_ADMIN_TOKEN" `
  -Body $body -ContentType "application/json"
```

## Network Topology

```
Your Public Node (ngrok)
         ↕ gossip
    ┌────┼────┬────┐
    ↓    ↓    ↓    ↓
 Miner1 Miner2 Miner3 Miner4
   ↕────────────────↕
      (can peer with each other too)
```

Each node:
- Runs independently
- Has own local database
- Syncs via P2P gossip
- Mines blocks locally
- Broadcasts to peers

## Summary

✅ **Miners run their own nodes** (vision-node.exe)  
✅ **They add your public node as a peer** (one command)  
✅ **Gossip happens automatically** after peering  
✅ **No wallet configuration needed** on their end  
✅ **Each node stays independent** but synchronized  

The Settings page node URL feature is for **if someone wants to use a remote node for their wallet** - but that's optional. For mining, they just need to peer!
