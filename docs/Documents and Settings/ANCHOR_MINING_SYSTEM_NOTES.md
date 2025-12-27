# Anchor/Miner Role System - Implementation Complete âœ…

## ðŸŽ¯ Philosophy Implemented

**Mining = Mining Ticket**
- Must be close to tip + see same chain
- No more "must be publicly reachable" for regular miners
- Anchors = backbone / truth keepers
- Everyone else = outbound-only mining miners

## ðŸ“¦ What Was Added

### 1. **Role Flags & Constants** (`src/vision_constants.rs`)
```rust
// Environment flags
pub static VISION_ANCHOR_NODE: Lazy<bool>;      // Backbone node (strict rules)
pub static VISION_OUTBOUND_ONLY: Lazy<bool>;   // Home miner (relaxed rules)

// Mining window
pub const MAX_MINING_LAG_BLOCKS: i64 = 2;      // Max blocks behind to participate
```

### 2. **Enhanced SyncHealthSnapshot** (`src/auto_sync.rs`)
```rust
pub struct SyncHealthSnapshot {
    // ... existing fields ...
    pub public_reachable: Option<bool>,  // For anchor eligibility
}

impl SyncHealthSnapshot {
    pub fn height_lag(&self) -> i64 {
        // Positive = behind, negative = ahead
    }
    
    pub fn peer_count(&self) -> usize {
        self.connected_peers
    }
}
```

### 3. **Mining Eligibility Module** (`src/mining_eligibility.rs`)
Centralized reward eligibility logic:

```rust
pub fn is_reward_eligible(snapshot: &SyncHealthSnapshot) -> bool {
    // Anchors: Must be reachable + 3+ peers + in sync
    // Miners: Just need 1+ peer + in sync (outbound-only is fine!)
}

pub fn mining_status_message(snapshot: &SyncHealthSnapshot) -> String {
    // Human-friendly status: "âœ… Mining ready" or "â³ disabled: reason"
}

pub struct MiningEligibilityInfo {
    pub eligible: bool,
    pub role: String,
    pub lag_blocks: i64,
    pub peer_count: usize,
    pub public_reachable: Option<bool>,
    pub reason: String,
}
```

### 4. **Tokenomics Integration** (`src/main.rs::apply_tokenomics()`)
Replaced old reward gating with new mining system:

```rust
let sync_health = auto_sync::SyncHealthSnapshot::current();
let eligible = mining_eligibility::is_reward_eligible(&sync_health);

if !eligible {
    miner_emission = 0;  // No mining ticket = no rewards
}
```

### 5. **Anchor Seed Support** (`src/p2p/seed_peers.rs`)
```rust
// Parse anchors from env
VISION_ANCHOR_SEEDS=ip1:port1,ip2:port2,ip3:port3

pub fn parse_anchor_seeds_from_env() -> Vec<String>;
pub fn get_seeds_with_anchors(&self) -> Vec<String>;  // Anchors first, then genesis
```

### 6. **VisionPeer Extended** (`src/p2p/peer_store.rs`)
```rust
pub struct VisionPeer {
    // ... existing fields ...
    pub is_anchor: bool,  // Marks backbone nodes for routing priority
}
```

### 7. **Routing Helpers** (`src/p2p/routing_helpers.rs`)
Smart peer selection for broadcast:

```rust
pub fn choose_broadcast_peers(peers: &[VisionPeer], max: usize) -> Vec<VisionPeer> {
    // Always chooses anchors first, then fills with regular peers
    // Ensures outbound-only nodes get truth from backbone
}
```

### 8. **Mining Status API** (`src/routes/miner.rs`)
```http
GET /mining/status

Response:
{
  "eligible": true,
  "role": "miner",
  "lag_blocks": 0,
  "peer_count": 5,
  "public_reachable": false,
  "reason": "âœ… Mining ready: Full eligibility"
}
```

### 9. **.env Templates**
- `.env.anchor-template` - For public backbone nodes (guardians, exchanges)
- `.env.miner-template` - For home miners behind CGNAT/firewalls

## ðŸŽ« Eligibility Rules

### Anchor Nodes (VISION_ANCHOR_NODE=true)
```
âœ… Must be publicly reachable (has ADVERTISED_P2P_ADDRESS)
âœ… Must have 3+ peer connections
âœ… Must be within 2 blocks of network tip
âœ… Must see same chain ID
âŒ Too far ahead/behind = NO REWARDS
```

### Regular Miners (VISION_OUTBOUND_ONLY=true)
```
âœ… Only need 1+ peer connection (to any anchor)
âœ… Must be within 2 blocks of network tip
âœ… Must see same chain ID
ðŸŽ‰ NO PUBLIC REACHABILITY REQUIRED!
âŒ Too far ahead/behind = NO REWARDS
```

## ðŸ›°ï¸ How Anchors Work

1. **Set anchor flag**: `VISION_ANCHOR_NODE=true`
2. **Port forward**: P2P port (7072) must be publicly accessible
3. **Connect to other anchors**: Via `VISION_ANCHOR_SEEDS`
4. **Accept inbound**: Home miners connect to you
5. **Strict eligibility**: Must maintain 3+ peers and public reachability

## ðŸ  How Home Miners Work

1. **Set outbound flag**: `VISION_OUTBOUND_ONLY=true`
2. **Connect to anchors**: Via `VISION_ANCHOR_SEEDS=anchor1:7072,anchor2:7072`
3. **No port forwarding**: Works behind CGNAT, double-NAT, ISP firewalls
4. **Relaxed eligibility**: Just need 1+ anchor connection + stay synced
5. **Full mining ticket**: Same reward chance as anchors when eligible!

## ðŸ”Œ Connection Flow

```
Home Miner (Outbound-Only)
    â†“ (outbound TCP)
Anchor Nodes (Public Mesh)
    â†‘â†“ (full mesh P2P)
Anchor Nodes (Public Mesh)
    â†“ (outbound TCP)
Home Miner (Outbound-Only)
```

Home miners get truth from anchors. Anchors maintain consensus mesh.

## ðŸ“Š Status Checking

### For Operators
```bash
# Check mining eligibility
curl http://localhost:7070/mining/status

# Expected responses:
âœ… "eligible": true, "reason": "âœ… Mining ready: Full eligibility"
â³ "eligible": false, "reason": "â³ Mining disabled: Too far behind (5 blocks, max=2)"
ðŸ›°ï¸ "eligible": false, "reason": "ðŸ›°ï¸ Anchor mining disabled: Need 3+ peers (have 1)"
```

### In Logs
```
[mining] âœ… Mining rewards ENABLED: âœ… Mining ready: Full eligibility
  height=12345, lag=0 blocks, peers=5

[mining] ðŸŽ« Mining rewards DISABLED: â³ Mining disabled: Too far behind (5 blocks, max=2)
  height=12340, lag=5 blocks, peers=3, role=miner
```

## ðŸš€ Deployment Guide

### For Anchors
1. Copy `.env.anchor-template` to `.env`
2. Set `VISION_WALLET_ADDRESS=your_address`
3. Port forward 7072 through router
4. Add other anchors to `VISION_ANCHOR_SEEDS`
5. Start node: `./vision-node` or `./START-PUBLIC-NODE.bat`
6. Verify: `curl localhost:7070/mining/status` shows `"eligible": true`

### For Home Miners
1. Copy `.env.miner-template` to `.env`
2. Set `VISION_WALLET_ADDRESS=your_address`
3. Add anchor IPs to `VISION_ANCHOR_SEEDS=anchor1:7072,anchor2:7072`
4. Start node: `./vision-node` or `./START-PUBLIC-NODE.bat`
5. Verify: `curl localhost:7070/mining/status` shows `"eligible": true`
6. **No port forwarding needed!** ðŸŽ‰

## ðŸŽ¨ Key Benefits

### For the Network
- **Decentralized rewards**: Home miners can participate despite ISP chaos
- **Backbone stability**: Anchors maintain high-quality consensus mesh
- **Clear roles**: Truth keepers (anchors) vs mining players (miners)
- **Resilient**: Network works even if most nodes are outbound-only

### For Home Miners
- **No port forwarding**: Works behind any firewall/CGNAT
- **Fair mining**: Same reward chance as anchors when synced
- **Simple setup**: Just point to 1-2 anchors and mine
- **Your ISP can't stop you**: Outbound connections always work

### For Anchors
- **Strict eligibility**: Ensures backbone quality
- **Priority routing**: Broadcasts prefer anchors for truth propagation
- **Protected status**: Home miners depend on you staying healthy
- **Clear requirements**: 3+ peers, public reachable, in-sync

## ðŸ“ Configuration Examples

### Anchor .env (Guardian/Exchange)
```bash
VISION_PORT=7070
VISION_P2P_PORT=7072
VISION_PUBLIC_PORT=7072

VISION_ANCHOR_NODE=true
VISION_OUTBOUND_ONLY=false

VISION_ANCHOR_SEEDS=other.anchor.1:7072,other.anchor.2:7072
VISION_WALLET_ADDRESS=0xYourAnchorWalletHere
```

### Home Miner .env (Behind CGNAT)
```bash
VISION_PORT=7070
VISION_P2P_PORT=7072
VISION_PUBLIC_PORT=7072

VISION_ANCHOR_NODE=false
VISION_OUTBOUND_ONLY=true

VISION_ANCHOR_SEEDS=community.anchor.1:7072,community.anchor.2:7072
VISION_WALLET_ADDRESS=0xYourMinerWalletHere
```

## ðŸ§ª Testing Eligibility

### Test Anchor Eligibility
```bash
# Should require: public_reachable=true, peer_count>=3, lag<=2
curl http://localhost:7070/mining/status | jq .

# Expected:
{
  "eligible": true,
  "role": "anchor",
  "lag_blocks": 0,
  "peer_count": 5,
  "public_reachable": true,
  "reason": "âœ… Anchor mining ready: Full eligibility"
}
```

### Test Miner Eligibility
```bash
# Should require: peer_count>=1, lag<=2 (no public_reachable needed!)
curl http://localhost:7070/mining/status | jq .

# Expected:
{
  "eligible": true,
  "role": "miner",
  "lag_blocks": 1,
  "peer_count": 2,
  "public_reachable": false,
  "reason": "âœ… Mining ready: Full eligibility"
}
```

## ðŸŽ¯ Success Criteria

âœ… Anchor nodes require strict health (public + 3+ peers)
âœ… Home miners only need 1+ peer (outbound-only works!)
âœ… Both roles must stay within 2 blocks of tip
âœ… Mining eligibility visible via /mining/status API
âœ… Logs clearly show eligibility state changes
âœ… .env templates for both roles
âœ… Anchor seeds prioritized in routing
âœ… Tokenomics gates rewards based on role

## ðŸ”§ Troubleshooting

### Anchor Not Eligible
```
Problem: "ðŸ›°ï¸ Anchor mining disabled: Not publicly reachable"
Solution: Check port forwarding, verify VISION_PUBLIC_PORT is accessible externally
```

### Miner Not Eligible
```
Problem: "ðŸŽ« Mining disabled: No peer connections"
Solution: Check VISION_ANCHOR_SEEDS, verify anchors are online and reachable
```

### Behind Network Tip
```
Problem: "â³ Mining disabled: Too far behind (5 blocks, max=2)"
Solution: Wait for sync, check anchor connection speed, verify internet stability
```

## ðŸ“š Files Modified/Created

**Created:**
- `src/mining_eligibility.rs` - Core eligibility logic
- `src/p2p/routing_helpers.rs` - Anchor-priority broadcast
- `.env.anchor-template` - Anchor node config
- `.env.miner-template` - Home miner config

**Modified:**
- `src/vision_constants.rs` - Added role flags
- `src/auto_sync.rs` - Added height_lag(), public_reachable
- `src/main.rs` - Integrated mining eligibility in tokenomics
- `src/routes/miner.rs` - Added /mining/status endpoint
- `src/p2p/peer_store.rs` - Added is_anchor field
- `src/p2p/seed_peers.rs` - Added anchor seed parsing
- `src/p2p/mod.rs` - Registered routing_helpers module

## ðŸŽ‰ Result

**Vision is now a proper mining-based PoW with role separation:**
- Blocks = just the show
- Coin distribution = decentralized via mining tickets
- Truth = maintained by public anchor backbone
- Home miners = full participants without ISP dependency

"As long as your node says it's within 2 blocks of the network tip and connected to at least 1 anchor, you're in the mining. Your ISP can't stop you from participating."

âœ… **SHIPPED AND READY TO TEST** ðŸš€

