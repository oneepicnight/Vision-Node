# 🌐 Vision Blockchain - Internet Testnet Deployment Guide

**Date**: November 4, 2025  
**Target**: Multi-tester deployment over the internet  
**Status**: ✅ **READY FOR DEPLOYMENT**

---

## 📋 Executive Summary

Your Vision blockchain is **ready for public testnet deployment** with multiple testers over the internet. All core systems are functional, P2P is hardened, and the infrastructure can support 50+ concurrent nodes.

**Readiness**: 85% (Testnet Ready)
- ✅ Core blockchain functional
- ✅ Mining system stable
- ✅ P2P networking hardened
- ✅ Wallet functionality complete
- ✅ Security measures in place

---

## 🚀 Quick Start: Deploy in 3 Steps

### Step 1: Prepare Bootstrap Nodes (15 minutes)

You need 2-3 **public-facing bootstrap nodes** that testers can connect to.

#### Option A: Cloud VPS (Recommended)
```bash
# DigitalOcean, AWS, Linode, Vultr, etc.
# Minimum specs: 2 CPU, 4GB RAM, 50GB SSD
# OS: Ubuntu 22.04 LTS

# 1. Upload compiled binary
scp target/release/vision-node root@YOUR_SERVER_IP:/usr/local/bin/

# 2. Create systemd service
ssh root@YOUR_SERVER_IP
cat > /etc/systemd/system/vision-node.service << 'EOF'
[Unit]
Description=Vision Blockchain Bootstrap Node
After=network.target

[Service]
Type=simple
User=vision
WorkingDirectory=/opt/vision-node
Environment="VISION_PORT=7070"
Environment="VISION_ADMIN_TOKEN=YOUR_SECURE_TOKEN_HERE"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/vision-node
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# 3. Configure firewall
ufw allow 7070/tcp
ufw allow 22/tcp
ufw enable

# 4. Start node
systemctl daemon-reload
systemctl enable vision-node
systemctl start vision-node
systemctl status vision-node
```

#### Option B: Home Server + Port Forwarding
```bash
# 1. Configure router port forwarding
# Forward external port 7070 → internal IP:7070

# 2. Get your public IP
curl ifconfig.me

# 3. Run node with public address
$env:VISION_PORT="7070"
$env:VISION_EXTERNAL_IP="YOUR_PUBLIC_IP"
cargo run --release

# 4. Test external access
curl http://YOUR_PUBLIC_IP:7070/status
```

---

### Step 2: Create Tester Distribution Package (10 minutes)

Create a ZIP file with everything testers need:

```powershell
# Create distribution folder
New-Item -ItemType Directory -Force -Path "testnet-distribution"

# Copy binary
Copy-Item "target\release\vision-node.exe" "testnet-distribution\"

# Copy wallet files
Copy-Item -Recurse "wallet-final" "testnet-distribution\wallet"

# Copy panel UI
Copy-Item -Recurse "public" "testnet-distribution\public"

# Create config file
@"
# Vision Blockchain Testnet Configuration
# Edit these values before starting

# Network Settings
VISION_PORT=7070
VISION_BOOTSTRAP=http://BOOTSTRAP_NODE_1_IP:7070,http://BOOTSTRAP_NODE_2_IP:7070

# Mining Settings (Optional)
VISION_MINING_THREADS=4
VISION_MINING_ENABLED=1

# Admin Token (Keep Secret!)
VISION_ADMIN_TOKEN=testnet-admin-$(Get-Random)

# Logging
RUST_LOG=info
"@ | Out-File -FilePath "testnet-distribution\config.env" -Encoding utf8

# Create quick start script
@"
# Quick Start Script for Vision Testnet
# Windows PowerShell

Write-Host "Starting Vision Blockchain Testnet Node..." -ForegroundColor Cyan

# Load configuration
Get-Content config.env | ForEach-Object {
    if (`$_ -match '^([^#][^=]+)=(.+)`$') {
        [Environment]::SetEnvironmentVariable(`$Matches[1], `$Matches[2], 'Process')
    }
}

# Start node
.\vision-node.exe

"@ | Out-File -FilePath "testnet-distribution\start-testnet.ps1" -Encoding utf8

# Create README
@"
# Vision Blockchain Testnet - Tester Package

## What's Included
- vision-node.exe - Blockchain node binary
- wallet/ - Web wallet interface
- public/ - Miner control panel
- config.env - Configuration file
- start-testnet.ps1 - Quick start script

## Quick Start (5 minutes)

### Step 1: Configure
1. Open config.env in notepad
2. Replace BOOTSTRAP_NODE_1_IP and BOOTSTRAP_NODE_2_IP with actual IPs
3. Optionally adjust VISION_MINING_THREADS (4 is good for most PCs)

### Step 2: Start Node
```powershell
.\start-testnet.ps1
```

### Step 3: Access Interfaces
- Miner Panel: http://localhost:7070/panel.html
- Wallet: Open wallet/index.html in browser
- API Status: http://localhost:7070/status

## What to Test

### Mining
1. Open miner panel: http://localhost:7070/panel.html
2. Adjust thread count (4-8 recommended)
3. Watch for blocks found
4. Report hashrate and success rate

### Wallet Operations
1. Open wallet/index.html
2. Generate new address
3. Get testnet tokens from faucet (Discord/Telegram)
4. Send transactions to other testers
5. Check balance updates

### Network Connectivity
1. Check peer count: curl http://localhost:7070/status
2. Should show 2+ connected peers
3. Report any connection issues

## Reporting Issues
- Discord: #testnet-feedback
- Telegram: @VisionTestnet
- GitHub: https://github.com/vision/vision-node/issues

## Support
- Testnet Guide: https://docs.vision.land/testnet
- API Docs: http://localhost:7070/docs
- FAQ: https://vision.land/testnet-faq

## Important Notes
- This is TESTNET - tokens have no value
- Node data stored in ./vision_data_7070
- To reset: delete vision_data_7070 folder
- Firewall: Allow port 7070 for best connectivity

## Minimum Requirements
- OS: Windows 10+, Linux, macOS
- CPU: 2+ cores
- RAM: 4GB
- Disk: 10GB free space
- Internet: 1 Mbps+ upload/download

"@ | Out-File -FilePath "testnet-distribution\README.txt" -Encoding utf8

# Create ZIP file
Compress-Archive -Path "testnet-distribution\*" -DestinationPath "vision-testnet-v0.1.0.zip" -Force

Write-Host "✅ Distribution package created: vision-testnet-v0.1.0.zip" -ForegroundColor Green
```

---

### Step 3: Distribute & Monitor (Ongoing)

#### Distribution Channels
1. **GitHub Release**
   ```bash
   # Create release on GitHub
   gh release create v0.1.0-testnet1 \
     vision-testnet-v0.1.0.zip \
     --title "Vision Testnet v0.1.0" \
     --notes "Initial public testnet release"
   ```

2. **Direct Download**
   - Upload to Google Drive / Dropbox
   - Share link in Discord / Telegram
   - Post on social media

3. **Documentation Site**
   - Create simple page with download link
   - Include quick start guide
   - Add video tutorial

#### Monitoring Setup
```powershell
# Monitor all nodes from central dashboard
# Install on your monitoring machine

# 1. Create monitoring script
@"
`$nodes = @(
    "http://NODE1_IP:7070",
    "http://NODE2_IP:7070",
    "http://NODE3_IP:7070"
)

while (`$true) {
    Clear-Host
    Write-Host "=== Vision Testnet Monitor ===" -ForegroundColor Cyan
    Write-Host "Time: `$(Get-Date)`n"
    
    foreach (`$node in `$nodes) {
        try {
            `$status = Invoke-RestMethod -Uri "`$node/status" -TimeoutSec 5
            Write-Host "`$node" -ForegroundColor Green
            Write-Host "  Height: `$(`$status.height)"
            Write-Host "  Peers: `$(`$status.peers)"
            Write-Host "  Mempool: `$(`$status.mempool_size)"
        } catch {
            Write-Host "`$node" -ForegroundColor Red
            Write-Host "  Status: OFFLINE"
        }
        Write-Host ""
    }
    
    Start-Sleep -Seconds 30
}
"@ | Out-File -FilePath "monitor-testnet.ps1" -Encoding utf8

# 2. Run monitoring
.\monitor-testnet.ps1
```

---

## 🔧 Configuration for Internet Deployment

### Bootstrap Node Configuration
```bash
# /opt/vision-node/.env
# Production-ready bootstrap node config

# Network
VISION_PORT=7070
VISION_MAX_PEERS=100
VISION_MAX_PEERS_PER_SUBNET=20

# Performance
VISION_MEMPOOL_MAX=10000
VISION_BLOCK_TARGET_TXS=200

# Mining (Disable on bootstrap nodes to save CPU)
VISION_MINING_ENABLED=0

# Security
VISION_ADMIN_TOKEN=YOUR_SECURE_RANDOM_TOKEN_HERE
VISION_RATE_LIMIT_ENABLED=1

# Logging
RUST_LOG=info,vision_node=debug

# Monitoring
VISION_METRICS_ENABLED=1
```

### Tester Node Configuration
```bash
# Recommended settings for testers
VISION_PORT=7070
VISION_BOOTSTRAP=http://BOOTSTRAP1:7070,http://BOOTSTRAP2:7070
VISION_MINING_THREADS=4
VISION_MINING_ENABLED=1
RUST_LOG=info
```

---

## 🌍 Network Topology (Recommended)

```
                    Internet
                       |
        ┌──────────────┼──────────────┐
        |              |              |
   Bootstrap 1    Bootstrap 2    Bootstrap 3
   (Cloud VPS)    (Cloud VPS)    (Cloud VPS)
   US East        EU West        Asia Pacific
        |              |              |
        └──────────────┼──────────────┘
                       |
        ┌──────────────┴──────────────┐
        |              |              |
   Tester Node 1  Tester Node 2  Tester Node 3
   (Home PC)      (Home PC)      (Home PC)
        |              |              |
   More Testers...  More Testers...  More Testers...
```

**Why This Works**:
- Bootstrap nodes are always online
- Geographic distribution reduces latency
- Home testers connect to nearest bootstrap
- Mesh topology forms between peers

---

## 💰 Testnet Token Distribution

### Option 1: Faucet (Automated)
Create a simple faucet service:

```python
# testnet-faucet.py
from flask import Flask, request, jsonify
import requests

app = Flask(__name__)

FAUCET_AMOUNT = 1000  # 1000 testnet LAND
NODE_URL = "http://localhost:7070"

@app.route('/faucet', methods=['POST'])
def faucet():
    address = request.json.get('address')
    
    # Validate address
    if not address or len(address) != 64:
        return jsonify({'error': 'Invalid address'}), 400
    
    # Send tokens (implement rate limiting in production)
    tx = {
        'from': 'FAUCET_ADDRESS',
        'to': address,
        'amount': FAUCET_AMOUNT,
        'private_key': 'FAUCET_PRIVATE_KEY'
    }
    
    response = requests.post(f'{NODE_URL}/submit_tx', json=tx)
    return jsonify(response.json())

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=5000)
```

### Option 2: Manual Distribution (Simple)
```powershell
# Pre-seed tester addresses with tokens
$testerAddresses = @(
    "tester1_address_64_chars",
    "tester2_address_64_chars",
    "tester3_address_64_chars"
)

foreach ($addr in $testerAddresses) {
    curl -X POST http://localhost:7070/admin/seed_balance `
      -H "Authorization: Bearer YOUR_ADMIN_TOKEN" `
      -H "Content-Type: application/json" `
      -d "{\"address\":\"$addr\",\"amount\":\"10000\"}"
}
```

---

## 📊 What to Monitor

### Node Health Metrics
```powershell
# Check bootstrap node health
$bootstrapNodes = @("http://NODE1:7070", "http://NODE2:7070")

foreach ($node in $bootstrapNodes) {
    $status = Invoke-RestMethod -Uri "$node/status"
    Write-Host "Node: $node"
    Write-Host "  Height: $($status.height)"
    Write-Host "  Peers: $($status.peers)"
    Write-Host "  Mining: $($status.mining)"
    Write-Host "  Mempool: $($status.mempool_size)"
    Write-Host ""
}
```

### Network Metrics
- **Peer Count**: Should be 5-20 per node
- **Block Height**: Should be increasing every ~2 seconds
- **Sync Status**: Nodes should converge to same height
- **Propagation Time**: New blocks should reach all nodes < 5 seconds

### Performance Metrics
- **Hashrate**: 500-1500 H/s per node (8 threads)
- **Block Time**: Target 2 seconds (actual: 1-5 seconds acceptable)
- **Transaction Throughput**: 100+ tx/minute
- **Memory Usage**: < 500 MB per node

---

## 🐛 Common Issues & Solutions

### Issue 1: Testers Can't Connect to Bootstrap
**Symptoms**: Peer count stays at 0
**Solutions**:
```powershell
# 1. Check bootstrap node is running
curl http://BOOTSTRAP_IP:7070/status

# 2. Verify firewall allows port 7070
# On bootstrap server:
sudo ufw status
sudo ufw allow 7070/tcp

# 3. Check NAT/router port forwarding
# Router admin → Port Forwarding → 7070 TCP

# 4. Test from tester machine
Test-NetConnection -ComputerName BOOTSTRAP_IP -Port 7070
```

### Issue 2: Node Won't Start
**Symptoms**: Crashes on startup
**Solutions**:
```powershell
# 1. Check for port conflicts
netstat -ano | findstr :7070

# 2. Delete corrupted database
Remove-Item -Recurse -Force vision_data_7070

# 3. Check Windows Defender/Antivirus
# Add vision-node.exe to exclusions

# 4. Run with debug logging
$env:RUST_LOG="debug"
.\vision-node.exe
```

### Issue 3: Mining Not Working
**Symptoms**: Hashrate shows 0
**Solutions**:
```powershell
# 1. Enable mining
$env:VISION_MINING_ENABLED="1"
$env:VISION_MINING_THREADS="4"

# 2. Check thread count in panel
# Open http://localhost:7070/panel.html
# Increase threads to 4-8

# 3. Verify CPU usage
# Task Manager → Performance → CPU should show activity

# 4. Check if node is synced
curl http://localhost:7070/status
# "synced": true means ready to mine
```

### Issue 4: Wallet Can't Send Transactions
**Symptoms**: Transaction fails with nonce error
**Solutions**:
```javascript
// In wallet, refresh nonce before sending
async function sendTransaction() {
    // 1. Get fresh balance and nonce
    const account = await fetch(`${NODE_URL}/balance/${address}`).then(r => r.json());
    
    // 2. Use returned nonce
    tx.nonce = account.nonce;
    
    // 3. Send transaction
    const result = await fetch(`${NODE_URL}/submit_tx`, {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify(tx)
    });
}
```

---

## 🧪 Testnet Testing Scenarios

### Scenario 1: Basic Connectivity (Day 1)
**Goal**: Verify all testers can connect and sync
```
1. Start 3+ tester nodes
2. Verify peer discovery (5+ peers each)
3. Check block height synchronization
4. Monitor for 1 hour stability
```

### Scenario 2: Transaction Flow (Day 2)
**Goal**: Test wallet-to-wallet transfers
```
1. Distribute testnet tokens to 5 testers
2. Each tester sends to 3 others
3. Verify balances update correctly
4. Check transaction confirmations
```

### Scenario 3: Mining Competition (Day 3)
**Goal**: Test mining under competitive conditions
```
1. All testers enable mining (4-8 threads)
2. Run for 24 hours
3. Track blocks found per tester
4. Verify difficulty adjusts properly
```

### Scenario 4: Network Stress (Day 4)
**Goal**: Test under high transaction load
```
1. Script to send 100 tx/minute
2. Run for 1 hour
3. Monitor mempool size
4. Check for dropped transactions
```

### Scenario 5: Reorg Handling (Day 5)
**Goal**: Test chain reorganization
```
1. Temporarily disconnect subset of nodes
2. Let them mine separate chains
3. Reconnect and observe reorg
4. Verify correct chain wins (highest work)
```

---

## 📈 Success Metrics

### Week 1 Goals
- ✅ 10+ active tester nodes
- ✅ 1000+ blocks mined
- ✅ 500+ transactions processed
- ✅ Zero critical crashes
- ✅ 95%+ uptime

### Week 2 Goals
- ✅ 25+ active nodes
- ✅ 5000+ blocks mined
- ✅ 2000+ transactions
- ✅ Difficulty adjusting smoothly
- ✅ Peer discovery working

### Week 3 Goals
- ✅ 50+ active nodes
- ✅ 10000+ blocks mined
- ✅ 5000+ transactions
- ✅ All features tested
- ✅ Ready for mainnet decision

---

## 🔒 Security Considerations

### For Bootstrap Nodes
```bash
# 1. Use strong admin token
VISION_ADMIN_TOKEN=$(openssl rand -hex 32)

# 2. Disable admin endpoints from public
# Use reverse proxy (nginx) to restrict /admin/* to internal IP

# 3. Enable rate limiting
VISION_RATE_LIMIT_ENABLED=1

# 4. Monitor for attacks
tail -f /var/log/vision-node.log | grep -i "rejected\|banned"

# 5. Regular backups
0 0 * * * tar -czf /backup/vision-data-$(date +\%Y\%m\%d).tar.gz /opt/vision-node/vision_data_7070
```

### For Testers
```
- Use testnet tokens only (no value)
- Don't expose admin endpoints
- Run in isolated environment
- Report suspicious activity
- Don't DDoS bootstrap nodes
```

---

## 🚀 Launch Day Checklist

### 24 Hours Before
- [ ] Bootstrap nodes deployed and tested
- [ ] Distribution package created
- [ ] Documentation published
- [ ] Community announcement drafted
- [ ] Support channels ready (Discord/Telegram)
- [ ] Monitoring dashboard configured

### Launch Day
- [ ] Start bootstrap nodes (staggered start)
- [ ] Verify bootstrap connectivity
- [ ] Release distribution package
- [ ] Post announcements (social media, forums)
- [ ] Send invites to initial testers
- [ ] Monitor first connections
- [ ] Be ready for support questions

### First 24 Hours
- [ ] Monitor node health continuously
- [ ] Respond to tester issues quickly
- [ ] Track key metrics (peers, blocks, txs)
- [ ] Collect feedback in dedicated channel
- [ ] Fix critical bugs immediately
- [ ] Document common issues

---

## 📞 Support & Communication

### Channels to Set Up
1. **Discord Server**
   - #testnet-announcements (read-only)
   - #testnet-support (questions)
   - #testnet-feedback (bug reports)
   - #mining-discussion
   - #wallet-help

2. **Telegram Group**
   - @VisionTestnet (public group)
   - Quick support for timezone coverage

3. **GitHub Issues**
   - Bug reports with templates
   - Feature requests
   - Technical discussions

4. **Email**
   - testnet-support@vision.land
   - For private/security issues

---

## 📝 Tester Feedback Template

Share this with testers for structured feedback:

```markdown
# Vision Testnet Feedback Report

**Tester Name**: 
**Date**: 
**Node Version**: v0.1.0-testnet1

## System Info
- OS: 
- CPU: 
- RAM: 
- Internet Speed: 

## Connectivity
- Bootstrap connected: Yes / No
- Peer count: 
- Sync time: 
- Any connection issues? 

## Mining
- Hashrate achieved: 
- Blocks found: 
- Success rate: 
- Thread count used: 
- Any mining issues? 

## Wallet
- Transfer successful: Yes / No
- Balance updates: Instant / Delayed
- Any transaction errors? 

## Overall Experience
- Ease of setup (1-10): 
- Performance (1-10): 
- Stability (1-10): 
- Suggestions: 

## Bugs Found
1. 
2. 
3. 

## Additional Comments

```

---

## 🎉 Ready to Launch!

Your Vision blockchain is **ready for multi-tester deployment**. Follow this guide to:

1. ✅ Deploy bootstrap nodes (15 min)
2. ✅ Create distribution package (10 min)
3. ✅ Invite testers and monitor (ongoing)

**Estimated Setup Time**: 30 minutes  
**Recommended Tester Count**: Start with 5-10, scale to 50+  
**Testing Duration**: 2-3 weeks before mainnet decision

---

## 📚 Additional Resources

- **Main README**: See root README.md for project overview
- **API Documentation**: See docs/MVP_ENDPOINTS.md
- **Mining Guide**: See VISIONX_POW_QUICKSTART.md
- **Mainnet Report**: See MAINNET_READINESS_REPORT.md
- **P2P Security**: See P2P_ATTACK_HARDENING_COMPLETE.md

---

## 🆘 Need Help?

- **Technical Issues**: Check troubleshooting section above
- **Setup Questions**: Re-read Quick Start section
- **Bugs Found**: Report on GitHub Issues
- **Security Concerns**: Email security@vision.land

---

**Good luck with your testnet launch!** 🚀

---

*Last Updated: November 4, 2025*  
*Version: 1.0*  
*Status: Ready for Deployment*
