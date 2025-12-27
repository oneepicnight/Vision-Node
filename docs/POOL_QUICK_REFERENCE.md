# Vision Mining Pool - Quick Reference

## 🎯 Overview

The Vision Mining Pool system allows nodes to:
- **Solo Mine**: Keep 100% of block rewards (minus protocol fee)
- **Host a Pool**: Coordinate multiple miners, charge pool fee
- **Join a Pool**: Mine as worker, earn proportional rewards

## 📡 API Endpoints

### Pool Management
```
POST /pool/start         - Start hosting a pool
POST /pool/stop          - Stop hosting
POST /pool/configure     - Update pool settings
GET  /pool/mode          - Get current mining mode
POST /pool/mode          - Set mining mode (Solo/HostPool/JoinPool)
```

### Worker Operations
```
POST /pool/register      - Register as worker
GET  /pool/job           - Fetch mining job
POST /pool/share         - Submit share/block
GET  /pool/stats         - Pool statistics
```

## 💰 Fee Structure

Every block reward is split:
1. **Protocol Fee**: 0.4 LAND → Protocol vault (automatic)
2. **Foundation Fee**: 1.0% → Vision Foundation (ecosystem dev)
3. **Pool Fee**: 0-10% (default 1.5%) → Pool host
4. **Workers**: Remaining amount split proportionally by shares

### Example: 32 LAND Block Reward

```
Total Miner Reward: 31.6 LAND (after 0.4 protocol fee)
Foundation (1%):    0.316 LAND
Pool Fee (1.5%):    0.474 LAND
Workers (97.5%):    30.81 LAND

If Worker A has 70% of shares: 21.57 LAND
If Worker B has 30% of shares:  9.24 LAND
```

## 🔧 Configuration

### Host a Pool

```javascript
// panel.html or via API
POST /pool/start
{
  "pool_name": "My Pool",
  "pool_fee": 1.5,        // percentage
  "pool_port": 7072       // or 8082
}
```

### Join a Pool

```javascript
POST /pool/register
{
  "worker_id": "worker-123",
  "wallet_address": "vland1abc...",
  "version": "1.0.0"
}

// Then repeatedly:
GET /pool/job?worker_id=worker-123
POST /pool/share { ... }
```

## 🎮 UI Panel Controls

### Mode Selector (panel.html)
- **⛏️ Solo**: Traditional solo mining
- **🏊 Host Pool**: Become pool coordinator
- **🤝 Join Pool**: Mine for existing pool

### Host Configuration
- Pool Name: Display name visible to world
- Pool Port: 7072 or 8082 (standard ports)
- Pool Fee: 0-10%, default 1.5%
- Auto-generated URL for worker connections

### Join Configuration  
- Pool URL: http://host:port
- Worker Name: Display name
- Worker Wallet: Address for payouts

## 📊 Monitoring

### Pool Stats Response
```json
{
  "worker_count": 5,
  "total_shares": 1000,
  "total_hashrate": 50000000,
  "blocks_found": 3,
  "workers": [
    {
      "worker_id": "w1",
      "wallet_address": "vland1...",
      "total_shares": 600,
      "reported_hashrate": 30000000,
      "estimated_payout": "19.2 LAND"
    }
  ]
}
```

## 🧪 Testing Guide

### Local Multi-Node Setup

**Terminal 1 - Pool Host:**
```powershell
.\START-VISION-NODE.bat
# Open http://localhost:7070/panel.html
# Select "Host Pool" mode
# Configure and start pool
```

**Terminal 2 - Worker 1:**
```powershell
$env:VISION_DATA_DIR="vision_data_worker1"
$env:VISION_PORT="7071"
cargo run --release
# Open http://localhost:7071/panel.html
# Select "Join Pool" mode
# Enter http://localhost:7070 as pool URL
```

**Terminal 3 - Worker 2:**
```powershell
$env:VISION_DATA_DIR="vision_data_worker2"
$env:VISION_PORT="7072"
cargo run --release
# Join pool like worker 1
```

### Verify Pool Operation

1. Check pool stats: `GET http://localhost:7070/pool/stats`
2. Watch for shares in host logs
3. Lower difficulty to trigger block: 
   ```rust
   // In src/main.rs, temporarily:
   let difficulty = 1; // Easy mode for testing
   ```
4. Verify payouts in worker balances

## 🐛 Common Issues

### "This node is not hosting a pool"
- Ensure mode is set to HostPool via `/pool/start` or UI
- Check that POOL_STATE is initialized

### "Worker not registered"
- Workers must call `/pool/register` before submitting shares
- Check worker_id consistency across requests

### "Stale job ID"
- Job IDs change when new blocks arrive
- Workers should fetch fresh jobs periodically (every 30-60s)

### Payouts Not Appearing
- Currently uses direct balance updates (see TODO)
- Check logs for payout calculation errors
- Verify worker wallet addresses are correct

## 🔐 Security Notes

### Pool Hosts Should:
- Use firewall rules to limit /pool/* endpoints to trusted IPs
- Monitor for invalid share spam (tracked per worker)
- Set reasonable worker_timeout_secs (default 300)
- Prune stale workers periodically

### Workers Should:
- Only join pools from trusted operators
- Verify foundation_fee_bps = 100 (1%) in registration response
- Monitor estimated_payout to detect host cheating
- Keep worker software updated

## 📈 Performance Tips

### For Pool Hosts:
- Use SSD for fast DB operations
- Allocate 1-2 CPU cores for pool coordination
- Monitor worker count and total_hashrate
- Prune inactive workers every 5 minutes

### For Workers:
- Use all available CPU cores for mining
- Submit shares as soon as found (don't batch)
- Report accurate hashrate for better estimates
- Reconnect on connection loss

## 🚀 Future Enhancements

### Planned (see docs/POOL_SYSTEM_TODO.md):
- [ ] Worker mining loop for JoinPool mode
- [ ] Transaction-based payouts (instead of direct DB)
- [ ] Stratum protocol support
- [ ] WebSocket job streaming
- [ ] Pool reputation system
- [ ] Multi-pool failover for workers

### Ideas for Community:
- Public pool registry/discovery
- Pool performance dashboards
- Worker ranking/leaderboards
- Automated difficulty adjustment per worker
- Smart share validation (prevent share duplication)

## 📚 Code References

- **Core Logic**: `src/pool/`
  - `mod.rs` - Module exports and MiningMode enum
  - `state.rs` - PoolState and PoolConfig
  - `worker.rs` - WorkerInfo tracking
  - `protocol.rs` - Request/response types
  - `payouts.rs` - Reward distribution logic
  - `routes.rs` - HTTP handlers

- **UI**: `public/panel.html` (lines 920-1200)
- **Routes**: `src/main.rs` (lines 5884-5892)
- **Tests**: `src/pool/payouts.rs` (test_payout_calculation)

## 💡 Best Practices

### Pool Operators:
1. Set reasonable pool fees (1-3%)
2. Clearly communicate fee structure
3. Maintain >99% uptime
4. Provide worker statistics dashboard
5. Document payout schedule

### Workers:
1. Test pool with small hashrate first
2. Monitor estimated vs actual payouts
3. Keep logs for dispute resolution
4. Use unique worker_id per machine
5. Respect pool's share submission rate limits

---

**Questions?** See `docs/POOL_SYSTEM_TODO.md` for architecture details or ask in Vision community channels.
