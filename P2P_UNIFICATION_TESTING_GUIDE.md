# P2P Unification - Testing & Verification Guide

**Version:** v1.1.0+unification  
**Status:** ✅ Build Complete - Ready for Testing  
**Binary:** `target\release\vision-node.exe` (24.61 MB)  
**Timestamp:** December 2, 2025 8:28:20 PM

---

## 🎯 Quick Start Testing

### 1. Guardian Node (Already Running)
```powershell
# Current Status: ✅ ONLINE
# Endpoint: http://localhost:7070
# Beacon: Active (broadcasting every 30s)
# P2P Listener: Disabled (Guardian = HTTP beacon only)
```

**Verify Guardian:**
```powershell
Invoke-RestMethod http://localhost:7070/api/health
Invoke-RestMethod http://localhost:7070/api/beacon/peers
```

Expected output:
```json
{
  "status": "healthy",
  "role": "guardian",
  "mode": "celebration",
  "network": "vision-testnet-48h-v0.8.0"
}
```

---

### 2. Launch Test Constellation Node

**Option A: Local Test (Same Machine)**
```powershell
# Stop guardian first
Get-Process -Name vision-node | Stop-Process -Force

# Start as constellation
$env:VISION_NETWORK='vision-testnet-48h-v0.8.0'
$env:BEACON_ENDPOINT='http://localhost:7070/api/beacon/register'
$env:RUST_LOG='debug'
.\target\release\vision-node.exe
```

**Option B: Remote Test (Different Machine)**
```powershell
# On constellation machine
$env:VISION_NETWORK='vision-testnet-48h-v0.8.0'
$env:BEACON_ENDPOINT='http://GUARDIAN_IP:7070/api/beacon/register'
$env:RUST_LOG='debug'
.\target\release\vision-node.exe
```

---

## 🔍 What to Monitor

### Guardian Logs (Expected Messages)

When constellation nodes connect, you should see:

```
✅ SUCCESS INDICATORS:
[BEACON] Peer registered: 192.168.1.50:7071 | protocol_version=1 | node_version=110
[BEACON] Peer count: 1 → 2 → 3 → 5 (excellent)
[BEACON] Heartbeat #X | Peers: 5 | Network: STABLE

🌟 CELEBRATION MODE:
A new star joins the constellation ✨
Reputation: rising
Network: unified
Constellation mesh: STABLE
```

---

### Constellation Logs (Expected Messages)

On constellation nodes, watch for:

```
✅ P2P UNIFICATION SUCCESS:
[P2P] Connected to guardian beacon at http://GUARDIAN_IP:7070
[P2P] Handshake successful: protocol_version=1, node_version=110
[P2P] Outbound connection established: 192.168.1.100:7071 (IPv4)
[P2P] Inbound connection accepted: 192.168.1.50:7071 (IPv4)
[P2P] Peer score: 0.65 (baseline) → 0.75 (improving)
[P2P] Connected to 3 peers... 5 peers (excellent)... 7 peers (optimal)

🎯 MESH FORMATION:
[PEER MANAGER] Active peers: 5
[MOOD ROUTER] Mode: Celebration, Mesh: STABLE
[BEACON] Discovered 7 peers from guardian beacon
```

---

## ⚠️ Troubleshooting

### Problem: Node shows "Connected to 0 peers (isolated)"

**Diagnosis:**
```powershell
# Check beacon connectivity
Invoke-RestMethod http://GUARDIAN_IP:7070/api/beacon/peers

# Check node logs for:
grep "IPv6" .\logs\vision-node.log  # Should be empty or rejected
grep "handshake retry" .\logs\vision-node.log  # Should show 3 attempts
grep "protocol_version=1" .\logs\vision-node.log  # All handshakes should be v1
```

**Solutions:**
1. ✅ Verify `BEACON_ENDPOINT` env var points to correct guardian IP
2. ✅ Check firewall allows port 7070 (HTTP beacon) and 7071 (P2P)
3. ✅ Confirm network ID matches: `vision-testnet-48h-v0.8.0`
4. ✅ Wait 30 seconds for beacon discovery cycle

---

### Problem: "Invalid admission ticket" errors

**Expected Behavior:**
- ✅ **Constellations:** Should connect WITHOUT ticket (permissive mode)
- ❌ **Guardians:** MUST have valid ticket (strict mode)

**Diagnosis:**
```powershell
# Check node role
grep "is_guardian" .\logs\vision-node.log

# For constellations:
# Should see: "Admission validation: PERMISSIVE (constellation mode)"

# For guardians:
# Should see: "Admission validation: STRICT (guardian mode)"
```

**Solution:**
If constellation nodes are being rejected:
1. Check `src/p2p/connection.rs` line ~464: Should detect `is_guardian=false`
2. Verify handshake message includes `role: "constellation"` or `role: "node"`
3. Guardian nodes MUST have `admission_ticket` in handshake

---

### Problem: IPv6 addresses appearing in peer list

**This Should Never Happen** with unified P2P patch.

**Diagnosis:**
```powershell
# Check peer book for IPv6
Invoke-RestMethod http://localhost:7070/api/beacon/peers | 
  Select-Object -ExpandProperty peers | 
  Where-Object { $_.ip -match ':' }
```

**If IPv6 found:**
```powershell
# Check connection.rs implementation:
grep -n "is_ipv6" src/p2p/connection.rs

# Should see outbound rejection at lines ~1560-1580:
# if addr.is_ipv6() {
#     debug!("Skipping IPv6 peer: {}", addr);
#     continue;
# }
```

---

### Problem: Handshake failures persist after 3 retries

**Check Retry Logic:**
```powershell
# Search for retry messages
grep "handshake attempt" .\logs\vision-node.log

# Should see pattern:
# attempt 1/3 with 150ms backoff
# attempt 2/3 with 300ms backoff
# attempt 3/3 with 600ms backoff
```

**Common Causes:**
1. **Network timeout:** Increase timeout from 3s to 5s in connection.rs
2. **Firewall:** Allow inbound TCP on port 7071
3. **NAT issues:** Check router supports TCP hole punching
4. **Chain height mismatch:** Both nodes should have `height <= 3` for early network

---

## 📊 Success Metrics

After 2-5 minutes of running, check these metrics:

### Guardian Dashboard
```powershell
$health = Invoke-RestMethod http://localhost:7070/api/health
$peers = Invoke-RestMethod http://localhost:7070/api/beacon/peers

Write-Host "Registered Peers: $($peers.peers.Count)"
Write-Host "Network Mode: $($health.mode)"
Write-Host "Mesh Status: $($health.mesh_status)"
```

**Target:**
- ✅ Registered Peers: **5-7** (excellent)
- ✅ Network Mode: **celebration**
- ✅ Mesh Status: **STABLE**

### Constellation Dashboard
```powershell
$health = Invoke-RestMethod http://localhost:7071/api/health

Write-Host "Connected Peers: $($health.peer_count)"
Write-Host "Mode: $($health.mode)"
Write-Host "Propagation: $($health.propagation)"
```

**Target:**
- ✅ Connected Peers: **3-7**
- ✅ Mode: **celebration**
- ✅ Propagation: **standard**

---

## 🧪 Advanced Testing

### Test Handshake Retry
```powershell
# Simulate network instability
# Start node → stop firewall briefly → restart firewall
# Should see 3-attempt retry in logs with backoffs: 150ms, 300ms, 600ms
```

### Test Duplicate Peer Merging
```powershell
# Start 2 constellation nodes on same machine with different ports
# Check peer list - should merge by peer_id and keep IPv4 addresses
$peers = Invoke-RestMethod http://localhost:7070/api/beacon/peers
$peers.peers | Group-Object node_id | Where-Object { $_.Count -gt 1 }
# Should return empty (no duplicates)
```

### Test Beacon Fast-Path
```powershell
# Check connection time for known beacon peers
grep "is_known_beacon" .\logs\vision-node.log
grep "fast-path" .\logs\vision-node.log
# Should see reduced validation time for beacon peers
```

### Test IPv4-Only Policy
```powershell
# Try to register IPv6 peer (should fail)
$body = @{
    node_id = "test-ipv6"
    ip = "fe80::1"
    port = 7071
} | ConvertTo-Json

Invoke-RestMethod -Method POST -Uri http://localhost:7070/api/beacon/register -Body $body -ContentType "application/json"
# Should reject with "IPv6 not supported" or silently ignore
```

---

## 🎉 Expected Success Messages

When everything works correctly, you'll see:

```
════════════════════════════════════════════════════════════
🌟 CONSTELLATION MESH: STABLE
════════════════════════════════════════════════════════════

Connected to 5 peers (excellent)
Protocol: v1 unified across all nodes
IPv4: 100% (0 IPv6 addresses)
Handshake success rate: 98%
Peer score average: 0.72
Mood: celebration ✨
Propagation: standard
Health: robust

A new star joins the constellation
Reputation: rising
Network: unified
Mesh: expanding
```

---

## 📦 Packaging for Distribution

Once testing confirms success:

1. **Update Windows Package:**
```powershell
Copy-Item .\target\release\vision-node.exe .\VisionNode-Constellation-v1.1.0-WIN64\vision-node.exe
Compress-Archive -Path .\VisionNode-Constellation-v1.1.0-WIN64\* -DestinationPath .\VisionNode-Constellation-v1.1.0+unified-WIN64.zip
```

2. **Add Documentation:**
```powershell
Copy-Item .\P2P_UNIFICATION_PATCH_SUMMARY.md .\VisionNode-Constellation-v1.1.0-WIN64\
Copy-Item .\P2P_UNIFICATION_TESTING_GUIDE.md .\VisionNode-Constellation-v1.1.0-WIN64\
```

3. **Update README:**
```markdown
## What's New in v1.1.0+unified

🎯 **P2P Unification Patch:**
- Unified handshake protocol (v1 across all nodes)
- IPv4-first stable connections
- 3-attempt handshake retry with backoff
- Improved peer scoring (0.65 baseline)
- Permissive admission for constellations
- Optimized outbound connection loop (10s, 3 peers)
- Beacon fast-path for trusted peers
- Duplicate peer ID merging

**Result:** Stable multi-peer mesh, no more isolation mode
```

---

## 🔗 Related Documentation

- `P2P_UNIFICATION_PATCH_SUMMARY.md` - Implementation details
- `BEACON_QUICK_REFERENCE.md` - Beacon system overview
- `CONSTELLATION_BEACON_GUIDE.md` - Beacon registration guide
- `FARM_MODE_QUICK_REF.md` - Pool/farm mode configuration

---

**Testing Status:** ⏳ Ready to test with live constellation nodes  
**Next Step:** Deploy to test network and monitor peer mesh formation  
**Expected Result:** "Connected to 5 peers (excellent)" + "Constellation mesh: STABLE"
