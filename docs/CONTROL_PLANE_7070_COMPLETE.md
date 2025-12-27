# Control Plane (7070) Complete Conversion - v2.7.0+

## Architecture Overview

**NEW RULE**: Control plane (HTTP 7070) is the nervous system. Data plane (P2P 7072) is optional muscle.

### Port Separation

```
┌─────────────────────────────────────────────────────────────┐
│                    Vision Node v2.7.0+                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  🧠 CONTROL PLANE (7070)      💪 DATA PLANE (7072)         │
│  ════════════════════════      ═══════════════════         │
│                                                             │
│  ALWAYS REQUIRED:              OPTIONAL (best-effort):     │
│  - Identity/hello (signed)     - Block streaming           │
│  - Seed peer exchange          - TX gossip                 │
│  - Cluster membership          - Bulk transfer             │
│  - Health snapshots            - Legacy P2P                │
│  - Tip height/hash             │                           │
│  - Exchange ready signal       7072 can be DEAD            │
│  - Peer discovery              and network still works     │
│  - Telemetry                   │                           │
│                                                             │
│  Survival = HTTP only          Mining = HTTP               │
│  Cluster = HTTP only           Sync = HTTP                 │
│  Health = HTTP only            P2P = nice-to-have          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Complete

### 1. ✅ Control Plane Client (`src/control_plane.rs`)

**Centralized HTTP 7070 client** - all systems use this instead of scattered reqwest calls:

```rust
pub struct ControlPlaneClient {
    // Timeouts: 3s (fast fail)
    // Retries: Exponential backoff
    // Thread-safe: Yes (global CONTROL_PLANE instance)
}

impl ControlPlaneClient {
    // Core protocol methods:
    async fn hello_signed(&self, http_url: &str) -> HelloResponse
    async fn fetch_seed_peers(&self, http_url: &str) -> Vec<PublicPeerInfo>
    async fn fetch_status(&self, http_url: &str) -> StatusResponse
    async fn post_heartbeat(&self, http_url: &str, payload: HeartbeatPayload)
    async fn probe_with_retry(&self, http_url: &str, max_retries: u32) -> StatusResponse
}
```

**Usage everywhere**:
```rust
use crate::control_plane::CONTROL_PLANE;

let status = CONTROL_PLANE.fetch_status("http://1.2.3.4:7070").await?;
let peers = CONTROL_PLANE.fetch_seed_peers("http://1.2.3.4:7070").await?;
```

### 2. ✅ Backbone State (Global Truth)

**Single source of truth** for cluster health:

```rust
pub struct BackboneState {
    pub connected: bool,                    // Are we connected to control plane?
    pub best_anchor: Option<String>,        // Best anchor URL
    pub latency_ms: Option<u64>,            // HTTP latency
    pub last_ok: Option<SystemTime>,        // Last successful probe
    pub observed_tip_height: u64,           // Network tip from anchors
    pub observed_tip_hash: Option<String>,  // Network tip hash
    pub peerbook_count: usize,              // Peers discovered
    pub cluster_size_estimate: usize,       // Network size estimate
    pub exchange_ready: bool,               // Exchange endpoints ready
    pub last_error: Option<String>,         // Last error message
}

// Global instance
pub static BACKBONE_STATE: Lazy<RwLock<BackboneState>>;

// Read from anywhere:
let state = control_plane::get_backbone_state();
if state.connected && state.observed_tip_height > local_height {
    // Sync needed
}
```

**All systems read from BackboneState**:
- `/api/status` API
- Miner eligibility checks
- Mining panel UI
- Sync health snapshot
- Peer healing loop
- Exchange gating

### 3. ✅ Backbone Probe Loop

**Always-on background task** that maintains BackboneState:

```rust
control_plane::start_backbone_probe_loop();
// Runs every 5 seconds:
// 1. Probes all anchors from VISION_ANCHOR_SEEDS
// 2. Picks best anchor (lowest latency + highest tip)
// 3. Updates BACKBONE_STATE with:
//    - connected = true/false
//    - best_anchor URL
//    - observed_tip_height
//    - latency_ms
//    - cluster_size_estimate
// 4. Logs success/failure
```

### 4. ✅ Peer Healing Loop

**HTTP-based peer discovery** (no P2P required):

```rust
control_plane::start_peer_healing_loop();
// Runs every 30 seconds:
// 1. Checks if backbone connected
// 2. Fetches seed peers from best anchor via HTTP
// 3. Updates peer store with HTTP-discovered peers
// 4. Updates peerbook_count in BackboneState
// 5. Never touches P2P port 7072
```

**Flow**:
```
Anchor → GET /api/p2p/seed_peers → JSON response
↓
[
  {"address": "1.2.3.4:7072", "http_address": "1.2.3.4:7070", "is_anchor": true},
  {"address": "5.6.7.8:7072", "http_address": "5.6.7.8:7070", "is_anchor": false},
  ...
]
↓
Insert into peer store (health=50, http_discovered=true)
↓
Peer book populated - P2P can now connect if desired
```

### 5. ✅ Mining Gate Removal

**NO MORE BLOCKING** - mining just pauses/resumes based on eligibility:

**OLD (v2.6 - BAD)**:
```rust
pub fn start(&self, threads: usize) {
    // CHECK IF READY
    if !can_mine {
        return; // REJECT THE START COMMAND
    }
    // Actually start mining
}
```

**NEW (v2.7 - GOOD)**:
```rust
pub fn start(&self, threads: usize) {
    // NO GATE - just log and set enabled_by_user flag
    info!("Mining enabled - will start when conditions allow");
    // Miner loop checks eligibility every tick and pauses/resumes
}
```

**Miner loop pseudo-code**:
```rust
loop {
    if !enabled_by_user {
        state = StoppedByUser;
        sleep; continue;
    }
    
    let backbone = get_backbone_state();
    if !backbone.connected || local_height + 2 < backbone.observed_tip_height {
        state = PausedUnsynced { lag };
        sleep; continue; // Pause, don't freeze
    }
    
    state = Running;
    // Mine real blocks
}
```

### 6. ✅ Sync Health from Backbone

**Network tip comes from HTTP 7070, not P2P gossip**:

**File**: `src/auto_sync.rs`
```rust
// PRIORITY ORDER for network tip:
let backbone = control_plane::get_backbone_state();

let network_tip = if backbone.connected && backbone.observed_tip_height > 0 {
    backbone.observed_tip_height  // 🌐 PRIMARY: Control plane
} else if anchor_http_response > 0 {
    anchor_http_response          // 📡 FALLBACK: Legacy anchor HTTP
} else {
    p2p_peer_gossip              // 🔧 LAST RESORT: P2P gossip
};
```

**Impact**:
- Mining eligibility uses HTTP tip (not P2P)
- Sync detection uses HTTP tip
- Lag calculation uses HTTP tip
- UI shows HTTP tip

### 7. ✅ Status API Integration

**File**: `src/api/website_api.rs`

**Added fields to `StatusResponse`**:
```rust
pub struct StatusResponse {
    // Existing fields...
    pub http_backbone: HttpBackboneStatus,
    pub exchange_ok: bool,
}

pub struct HttpBackboneStatus {
    pub connected: bool,
    pub anchor: Option<String>,       // "http://1.2.3.4:7070"
    pub latency_ms: Option<u64>,
    pub tip_height: Option<u64>,
    pub tip_hash: Option<String>,
    pub last_ok_unix: Option<u64>,
    pub last_error: Option<String>,
}
```

**Populated from BackboneState**:
```rust
let backbone = control_plane::get_backbone_state();
let http_backbone = HttpBackboneStatus {
    connected: backbone.connected,
    anchor: backbone.best_anchor,
    latency_ms: backbone.latency_ms,
    tip_height: Some(backbone.observed_tip_height),
    tip_hash: backbone.observed_tip_hash,
    last_ok_unix: backbone.last_ok.map(|t| unix_timestamp(t)),
    last_error: backbone.last_error,
};
```

### 8. ✅ Miner Panel UI

**New "Backbone (7070)" card** in `public/panel.html`:

```html
<div class="constellation-card">
    <div class="constellation-header">
        <span class="constellation-star">🌐</span>
        <h3>Backbone (7070)</h3>
        <p id="backbone-subtitle">Connecting to anchor truth...</p>
    </div>
    
    <div class="constellation-metrics">
        <div><span class="metric-label">Status</span><span id="backbone-status">⏳ Probing</span></div>
        <div><span class="metric-label">Anchor</span><span id="backbone-anchor">—</span></div>
        <div><span class="metric-label">Latency</span><span id="backbone-latency">—</span></div>
        <div><span class="metric-label">Tip Height</span><span id="backbone-tip">—</span></div>
    </div>
    
    <p class="mine-hint">
        <span id="exchange-status">🏦 Exchange: Checking...</span>
    </p>
</div>
```

**JavaScript updates every 2 seconds**:
```javascript
function updateHttpBackboneStatus(data) {
    const httpBackbone = data.http_backbone || {};
    
    if (httpBackbone.connected) {
        // ✅ Connected - show green
        statusElem.textContent = '✅ Connected';
        anchorElem.textContent = extractIP(httpBackbone.anchor);
        latencyElem.textContent = `${httpBackbone.latency_ms}ms`;
        tipElem.textContent = httpBackbone.tip_height.toLocaleString();
    } else {
        // ⚠️ Disconnected - show warning
        statusElem.textContent = '⚠️ No response';
        subtitleElem.textContent = 'Retrying connection...';
    }
    
    // Exchange status
    if (data.exchange_ok) {
        exchangeElem.textContent = '🏦 Exchange: ✅ Ready';
    } else {
        exchangeElem.textContent = '🏦 Exchange: ⏸ Disabled';
    }
}
```

## What Changed

### Files Modified

1. **`src/control_plane.rs`** - NEW FILE
   - ControlPlaneClient implementation
   - BackboneState global state
   - Backbone probe loop
   - Peer healing loop

2. **`src/main.rs`**
   - Added `mod control_plane;`
   - Replaced `p2p::anchor_http::start_anchor_http_probe()` with:
     - `control_plane::start_backbone_probe_loop()`
     - `control_plane::start_peer_healing_loop()`

3. **`src/miner/manager.rs`**
   - Removed blocking gate from `start()` method
   - Removed blocking gate from `start_with_config()` method
   - Now just logs and sets enabled flag
   - Miner loop handles pause/resume internally

4. **`src/auto_sync.rs`**
   - Added backbone state as primary network tip source
   - Priority: Backbone → Anchor HTTP → P2P gossip
   - Logs which source is used

5. **`src/api/website_api.rs`**
   - Added `HttpBackboneStatus` struct
   - Added `http_backbone` field to `StatusResponse`
   - Added `exchange_ok` field
   - Populated from `control_plane::get_backbone_state()`

6. **`public/panel.html`**
   - Added "Backbone (7070)" card
   - Added `updateHttpBackboneStatus()` JavaScript function
   - Shows connection status, anchor, latency, tip height
   - Shows exchange ready status

### Files NOT Modified (P2P Remains)

- `src/p2p/connection.rs` - P2P handshake still works for block streaming
- `src/p2p/peer_manager.rs` - P2P connections still maintained
- `src/p2p/bootstrap.rs` - P2P bootstrap still runs (after HTTP hydration)
- All P2P infrastructure intact - just optional now

## Testing the Conversion

### 1. Start Node

```bash
# Windows
START-PUBLIC-NODE.bat

# Linux
./start-public-node.sh
```

### 2. Check Logs

Look for these messages:

```
[BACKBONE] 🌐 Starting 7070 control plane probe loop
[HEALING] 🔄 Starting HTTP-based peer healing loop
[BACKBONE] ✅ Connected to http://1.2.3.4:7070 (128ms) - tip=12345 peers=8
[HEALING] 📥 Fetched 64 peers from anchor
[SYNC_HEALTH] ✅ Using backbone tip height: 12345
[MINER] ⛏️ Mining enabled - will start when conditions allow
```

### 3. Check Miner Panel

Visit: http://localhost:7070

**Look for**:
- 🌐 **Backbone (7070)** card showing:
  - Status: ✅ Connected
  - Anchor: 1.2.3.4
  - Latency: 128ms
  - Tip Height: 12,345
- 🏦 **Exchange**: ✅ Ready (if full feature enabled)

### 4. Test Mining

Click "Start Mining" in panel:
- **Should NOT freeze**
- **Should NOT reject** with "Mining blocked: not synced"
- **Should log**: "Mining enabled - will start when conditions allow"
- If behind tip: Miner pauses cleanly until synced

### 5. Test Without P2P

Block port 7072 (firewall/CGNAT):
- Node should still form cluster (via HTTP 7070)
- Peers discovered via HTTP seed exchange
- Network tip known from anchors
- Mining eligibility determined from HTTP
- Only block streaming affected (will use HTTP fallback)

## Benefits

### 1. No More Freezes

**OLD**: Mining start command waits for network → **FREEZES UI**
```rust
wait_for_network_ready(min_peers=2, timeout=None); // BLOCKS FOREVER
```

**NEW**: Mining start returns immediately → **NEVER FREEZES**
```rust
info!("Mining enabled");
// Miner loop checks and pauses internally
```

### 2. Works Behind CGNAT

**OLD**: P2P required for peer discovery → **FAILS BEHIND CGNAT**

**NEW**: HTTP peer discovery → **WORKS EVERYWHERE**
```
HTTP GET http://anchor:7070/api/p2p/seed_peers
→ Receive peer list
→ Populate peer book
→ Cluster formed without P2P
```

### 3. Reliable Network Tip

**OLD**: P2P gossip → **UNRELIABLE** (peers may lie, network partitions)

**NEW**: HTTP anchor truth → **RELIABLE** (anchors are canonical)
```rust
let tip = backbone_state.observed_tip_height; // From trusted anchors
```

### 4. Exchange Gating

**OLD**: No way to signal "ready for exchange integration"

**NEW**: Clear signal in API
```rust
exchange_ready = backbone.connected && lag <= 1
```

### 5. Unified Health Model

**OLD**: Different systems had different views of network health

**NEW**: Single BackboneState struct
```rust
// Everywhere reads same truth:
let state = control_plane::get_backbone_state();
```

## Environment Variables

### Required

```bash
# Anchor nodes for control plane (comma-separated IP:PORT)
VISION_ANCHOR_SEEDS=16.163.123.221:7072,other.anchor:7072
```

### Optional

```bash
# Enable strict P2P validation (default: relaxed)
VISION_P2P_STRICT=1

# Minimum peers for mining (default: 1 with backbone, 2 without)
VISION_MIN_PEERS_FOR_MINING=1
```

## Migration Path

### Phase 1: Control Plane Added (v2.7.0) ✅
- Control plane client created
- Backbone state implemented
- Probe/healing loops running
- Mining gate removed
- UI updated

### Phase 2: P2P Optional (v2.8.0) - Future
- HTTP block streaming
- HTTP transaction relay
- P2P purely optional
- Can run with 7072 disabled

### Phase 3: Full HTTP (v3.0.0) - Future
- All communication via 7070
- 7072 removed entirely
- Pure HTTP/REST architecture

## Architecture Philosophy

```
OLD (P2P-centric):
  Survival = P2P
  If P2P fails → Network dead

NEW (HTTP-centric):
  Survival = HTTP
  If HTTP fails → Network dead
  If P2P fails → Slight performance hit

FUTURE (HTTP-only):
  Survival = HTTP
  P2P removed
  Simpler, more reliable
```

## Summary

✅ **Control plane (7070)** = Nervous system (identity, health, discovery)  
✅ **Data plane (7072)** = Optional muscle (block streaming, gossip)  
✅ **Mining never freezes** - just pauses cleanly when behind  
✅ **Works behind CGNAT** - HTTP peer discovery  
✅ **Reliable network tip** - from trusted anchors  
✅ **Unified health model** - single BackboneState  
✅ **Exchange ready signal** - clear API indicator  
✅ **Miner panel shows 7070 status** - visible proof of connection  

**The punchline**: Nodes can form a cluster, discover peers, determine sync state, and mine blocks **even if port 7072 is completely dead**. HTTP 7070 is the new foundation.
