# Vision Node - Mining System Quick Reference

**Version:** v3.0.0 MINING TESTNET  
**Build:** December 15, 2025, 5:36 PM  

---

## Quick Start

### Start Node
```bash
vision-node.exe
```

### Check Mining Status
```bash
curl http://localhost:8080/api/mining/status
```

### Monitor Logs
Look for these messages:
- ðŸ† PRIMARY winner - You're selected to produce
- ðŸŽ« Backup #N - You're a backup
- âœ¨ Successfully produced - Block created
- ðŸ“¡ Block broadcast - Sent to peers

---

## Key Concepts

### Winner Selection
All nodes compute the same winner using:
```
seed = H(prev_hash || next_height || epoch_salt)
winner_index = seed % pool_size
winner = eligible_pool[winner_index]
```

### Eligibility Requirements
- âœ… Synced within 2 blocks of network tip
- âœ… At least 1 connected peer
- âœ… In "AtTip" state for 3+ seconds (hysteresis)

### Timing
- **Primary winner:** 50-400ms jitter
- **Backup #1:** 1200ms + jitter
- **Backup #2:** 2400ms + jitter
- ... up to Backup #6 at ~7200ms

### Pool Membership
- All Connected peers (PeerState::Connected)
- Sorted alphabetically by EBID
- Your node included automatically

---

## API Response Example

```json
{
  "next_height": 12345,
  "current_winner": "a1b2c3d4...",
  "backups": ["f6e5d4...", "c3b2a1...", ...],
  "my_node_id": "1234567890...",
  "my_slot": 0,
  "eligible": true,
  "sync_state": "AtTip",
  "local_height": 12344,
  "network_height": 12344,
  "lag_blocks": 0,
  "eligible_pool_size": 8,
  "ready_peers": 7,
  "eligible_streak": 5
}
```

**Key Fields:**
- `my_slot: 0` â†’ You're the primary winner
- `my_slot: 1-6` â†’ You're a backup
- `my_slot: null` â†’ You're not selected this round
- `eligible: true` â†’ You can produce blocks
- `lag_blocks: 0` â†’ You're synced

---

## Troubleshooting

### Not Eligible?
Check `eligible_streak` - needs to be â‰¥ 3  
Check `lag_blocks` - needs to be â‰¤ 2  
Check `ready_peers` - needs to be â‰¥ 1  

### Never Winning?
This is normal! With 8 nodes:
- Primary chance: 1/8 = 12.5%
- Any slot: 7/8 = 87.5%
- Expected wait: ~8 blocks to be primary

### Block Not Broadcasting?
Check P2P connections: `ready_peers` should be > 0  
Check port 7072 is accessible  
Verify seed peers in `.env` are reachable  

---

## Testing Commands

```bash
# Check if node is running
curl http://localhost:8080/api/status

# Check mining status
curl http://localhost:8080/api/mining/status | jq

# Check miner address
curl http://localhost:8080/api/miner/wallet

# Get block height
curl http://localhost:8080/height

# Get peer count
curl http://localhost:8080/api/status | jq .peers
```

---

## Network Constants

```
MINING_TICK_MS: 250          # Producer loop frequency
CLAIM_TIMEOUT_MS: 1200        # Time per slot
BACKUP_SLOTS: 6               # Number of backups
WINNER_JITTER_MAX_MS: 400     # Max jitter
WINNER_JITTER_MIN_MS: 50      # Min jitter
MAX_DESYNC_BLOCKS: 2          # Sync threshold
MIN_READY_PEERS: 1            # Minimum peers
ELIGIBLE_STREAK_REQUIRED: 3   # Hysteresis ticks
```

---

## Log Patterns

### Success
```
[MINING] ðŸ† I am PRIMARY winner for height 12345 (waiting 123ms)
[MINING] ðŸ“¦ Proposing block for height 12345
[MINING] âœ… Block 12345 created: 42 txs, hash: 0x...
[MINING] ðŸ“¡ Block 12345 broadcast: 7 success, 0 failed
[MINING] âœ¨ Successfully produced block 12345
```

### Backup Takeover
```
[MINING] ðŸŽ« I am backup #1 for height 12345 (waiting 1323ms)
[MINING] ðŸ“¦ Proposing block for height 12345
[MINING] âœ¨ Successfully produced block 12345
```

### Someone Else Won
```
[MINING] ðŸ‘€ Height 12345 - winner: "a1b2c3d4..."
[MINING] â­ï¸  Height 12346 already produced by someone else
```

### Not Eligible
```
Network snapshot: height=12344, lag=3, peers=7, state=Behind, streak=0
```

---

## Files

**Binary:** `vision-node.exe` (27.45 MB)  
**Config:** `.env`  
**Seed Peers:** `seed_peers.json`  
**Docs:** `MINING_SYSTEM_COMPLETE.md` (full details)  

---

**Status: READY** âœ…  
**Network: MINING TESTNET v3.0.0** ðŸŽ°

