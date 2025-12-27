# P2P Tuning Hints - Distributed Miner Intelligence

## Overview

The P2P Tuning Hints system enables Vision Network miners to share proven configurations via peer-to-peer gossip, creating a **distributed learning network** where elite configs propagate organically.

**Philosophy**: "No blind trust. Local validation required."

## Key Principles

### 1. Hybrid Intelligence Model

Unlike pure telemetry (centralized) or blind trust (dangerous), Vision uses a hybrid approach:

- **Centralized Telemetry** (Optional): Anonymous aggregate statistics for network-wide trends
- **P2P Hints** (Default): Peer-to-peer config sharing with local validation
- **Local Learning** (Always): Every miner builds its own performance history

This creates a three-tier intelligence hierarchy:
1. Personal experience (highest trust)
2. Validated peer hints (medium trust, proven locally)
3. Network telemetry (lowest trust, informational only)

### 2. Privacy-Safe Broadcasting

Hints broadcast only:
- **CPU Bucket**: Normalized family/cores (e.g., "Intel 8C/16T")
- **Config**: Threads, batch size, NUMA node
- **Performance**: Gain ratio, confidence, sample count
- **NO personal data**: No addresses, node IDs, exact CPU models

Example broadcast:
```json
{
  "cpu_bucket": { "family": "Intel", "cores": 8, "threads": 16 },
  "pow_algo": "vision-pow-v1",
  "threads": 12,
  "batch_size": 256,
  "gain_ratio": 0.15,
  "sample_count": 20,
  "confidence": 1.0,
  "timestamp": 1704067200
}
```

### 3. Survival of the Fittest

Bad hints die locally, good hints propagate naturally:

- Each miner validates hints independently
- Adoption requires ≥3% hashrate improvement (configurable)
- Failed hints are rejected and never re-tested
- Successful hints become candidates for further broadcast
- Natural selection: network converges on elite configs organically

### 4. Reputation-Aware Prioritization

Not all peers are equal:

- Hints from high-reputation peers tested first
- Low-reputation peers (<30 score) ignored by default
- Trial priority: `gain_ratio × confidence × freshness × reputation_bonus`
- Reputation is a **priority signal**, not absolute trust (still validate!)

## System Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    IntelligentTuner                          │
│  (Master coordinator - decides when to test hints)           │
└──────────────────┬──────────────────────────────────────────┘
                   │
                   ├─► HintManager (Validation & Trial Scheduling)
                   ├─► TelemetryClient (Network-wide trends)
                   ├─► ThermalMonitor (Safety constraints)
                   ├─► PowerMonitor (Battery/AC detection)
                   └─► NumaCoordinator (Thread placement)
```

### Data Flow

#### Receiving Hints

```
1. Peer broadcasts MinerTuningHint via P2P gossip
   │
2. HintManager.receive_hint()
   ├─► Sanity checks (threads: 1-128, batch: 1-10000)
   ├─► Freshness check (< 7 days old)
   ├─► CPU bucket similarity (Intel/AMD match, ±2 cores)
   ├─► Reputation filter (≥30.0 score)
   └─► Deduplication (not already verified/rejected)
   │
3. Queued in pending_hints (max 50)
   │
4. Prioritized by: gain_ratio × confidence × freshness × rep
   │
5. HintManager.get_next_trial() (rate-limited: 10/hour)
   │
6. ActiveMiner tests config for 60-second window
   │
7. HintManager.complete_trial(measured_hashrate, baseline)
   ├─► If gain ≥ 3%: Mark VERIFIED, adopt permanently
   └─► If gain < 3%: Mark REJECTED, never retry
```

#### Broadcasting Hints

```
1. Every 30 minutes, IntelligentTuner.should_broadcast_hints()
   │
2. Scan local PerfStore for elite configs:
   ├─► Sample count ≥ 5
   ├─► Gain ratio ≥ 5%
   └─► Confidence ≥ 0.25
   │
3. Create MinerTuningHint for each elite config
   │
4. Broadcast via P2P gossip to all connected peers
   │
5. Peers independently validate (see "Receiving Hints")
```

## Configuration

### miner.json

```json
{
  "p2p_hints_enabled": true,              // Enable P2P hints system
  "hint_trial_threshold": 0.03,           // 3% gain required to adopt
  "hint_max_pending": 50,                 // Max queued hints
  "hint_min_peer_reputation": 30.0,       // Min reputation to accept from
  "hint_broadcast_interval_mins": 30      // How often to share elite configs
}
```

### Defaults

| Setting | Default | Purpose |
|---------|---------|---------|
| `p2p_hints_enabled` | `true` | Enable/disable hints system |
| `hint_trial_threshold` | `0.03` | 3% minimum gain to adopt |
| `hint_max_pending` | `50` | Max hints in queue |
| `hint_min_peer_reputation` | `30.0` | Reputation filter |
| `hint_broadcast_interval_mins` | `30` | Broadcast schedule |
| `evaluation_window_secs` | `60` | Trial duration |
| `max_trials_per_hour` | `10` | Rate limit |

## Safety Mechanisms

### 1. Sanity Checks

All hints validated before testing:

- **Threads**: 1-128 (beyond 128 is suspicious)
- **Batch size**: 1-10000 (beyond 10k is spam)
- **Gain ratio**: 0.0-10.0 (beyond 10x is unrealistic)
- **Timestamp**: Must be fresh (< 7 days)

### 2. Rate Limiting

Prevents resource exhaustion:

- **Max trials per hour**: 10 (configurable)
- **Max pending hints**: 50 (drop oldest if full)
- **Broadcast interval**: 30 minutes minimum

### 3. CPU Bucket Filtering

Only consider hints for similar hardware:

- **Vendor match**: Intel/AMD/ARM must match
- **Core tolerance**: ±2 physical cores
- **Thread tolerance**: ±4 logical threads

Example:
- Intel 8C/16T accepts hints from: Intel 6-10C / 12-20T
- AMD 16C/32T rejects hints from: Intel (different vendor)

### 4. Local Validation Requirement

**Never blindly adopt peer suggestions.**

Every hint tested locally for 60 seconds:
1. Measure baseline hashrate with current config
2. Switch to hint config
3. Measure hashrate over evaluation window
4. Calculate gain: `(new - baseline) / baseline`
5. Adopt only if gain ≥ threshold (default 3%)

### 5. Reputation Filtering

Ignore hints from low-quality peers:

- Minimum reputation: 30.0 (0-100 scale)
- Reputation affects **priority**, not absolute trust
- High-reputation hints tested first, but still validated

## Monitoring

### HintStats

Query `HintManager.stats()` for real-time status:

```rust
pub struct HintStats {
    pub pending_count: usize,      // Hints queued for testing
    pub testing: bool,             // Currently testing a hint
    pub verified_count: usize,     // Hints adopted successfully
    pub rejected_count: usize,     // Hints rejected (gain < threshold)
    pub recent_trials: usize,      // Trials in last hour
}
```

### Logs

**Hint Reception**:
```
INFO: Queued hint for validation: algo=vision-pow-v1, threads=12, batch=256, 
      gain=8.0%, confidence=1.00, peer_rep=85.3
```

**Trial Start**:
```
INFO: Starting P2P hint trial: threads=12, batch=256, expected_gain=8.0%
```

**Trial Success**:
```
INFO: ✓ Hint VERIFIED: algo=vision-pow-v1, threads=12, batch=256, 
      measured_gain=9.2%, expected=8.0%
```

**Trial Failure**:
```
INFO: ✗ Hint REJECTED: algo=vision-pow-v1, threads=12, batch=256, 
      measured_gain=1.5%, expected=8.0% - Measured gain 1.5% below threshold 3.0%
```

## API Integration

### For Mining Manager

```rust
// Check if hint system wants to test a config
if let Some((threads, batch, reason)) = tuner.consider_peer_hints() {
    log::info!("Testing peer hint: {} threads, {} batch - {}", threads, batch, reason);
    // Switch to hint config for trial
}

// After trial completes
let measured = calculate_hashrate();
let baseline = get_baseline_hashrate();
tuner.hint_manager().lock().unwrap().complete_trial(measured, baseline);

// Check if it's time to broadcast
if tuner.should_broadcast_hints() {
    let hints = tuner.get_broadcast_hints();
    for hint in hints {
        p2p_handle.broadcast(P2PMessage::MinerTuningHint { hint });
    }
}
```

### For P2P Handler

```rust
// When receiving MinerTuningHint message
match message {
    P2PMessage::MinerTuningHint { hint } => {
        let peer_reputation = reputation_tracker.get_score(&peer_id);
        let accepted = tuner.hint_manager()
            .lock()
            .unwrap()
            .receive_hint(hint, peer_reputation);
        
        if accepted {
            log::debug!("Accepted hint from peer {}", peer_id);
        }
    }
    // ...
}
```

## Performance Impact

### Overhead

- **Network**: ~200 bytes per hint broadcast (every 30 minutes)
- **CPU**: <0.1% (hint validation is trivial)
- **Memory**: ~10 KB per hint (max 50 pending)
- **Trial cost**: 60 seconds per hint (10/hour max = 10 minutes/hour)

### Benefits

- **New nodes**: Benefit from collective wisdom immediately
- **Network**: Elite configs go viral in <1 hour
- **Individual**: Discover optimal configs faster than solo tuning
- **Resilience**: No central server dependency

## Security Considerations

### Spam Prevention

- Rate limits per peer (max 1 hint/minute)
- Total pending cap (50 hints)
- Reputation-based filtering
- Sanity checks on all fields

### Denial of Service

Cannot exhaust resources:
- Max 10 trials per hour (10 minutes total trial time)
- Automatic rejection of insane configs
- CPU bucket filtering (only test relevant hints)

### Privacy

Broadcasts contain:
- ✅ Generic CPU bucket (Intel 8C, AMD 16C, etc.)
- ✅ Config parameters (public data)
- ✅ Performance metrics (relative gains)
- ❌ Node identity (no addresses/IDs)
- ❌ Exact hardware specs
- ❌ Mining history

## Future Enhancements

### Confidence Decay

Hints lose priority over time as network evolves:
- Fresh (0-1 day): 1.0x priority
- Recent (1-3 days): 0.8x priority
- Old (3-7 days): 0.5x priority
- Stale (>7 days): Rejected

### Multi-Algo Support

Track hints per PoW algorithm:
- `vision-pow-v1`: Current algorithm
- `vision-pow-v2`: Future fork
- Separate hint pools prevent cross-contamination

### NUMA Hints

Share NUMA topology strategies:
- Single-node placement
- Cross-socket binding
- NUMA-local memory allocation

### Circuit Breaker

If hint trial success rate < 20%, disable hints temporarily:
- Prevents wasting resources on low-quality hints
- Re-enable after 1 hour cooldown
- Reputation system should naturally filter bad hints

## Troubleshooting

### No hints received

**Check**:
1. `p2p_hints_enabled = true` in miner.json
2. Connected peers > 0
3. Peer reputation > 30.0 (check with `vn-cli p2p peers`)
4. CPU bucket similar to network majority

### Hints not adopting

**Check**:
1. Trial threshold (default 3% gain required)
2. Evaluation window long enough (60s default)
3. Baseline hashrate stable (no thermal/power throttling)
4. Hints actually beneficial for your hardware

### Too many trials

**Reduce rate**:
```json
{
  "hint_trial_threshold": 0.05,    // Require 5% gain (higher bar)
  "hint_min_peer_reputation": 50.0 // Only trust high-rep peers
}
```

### Broadcast too frequent

**Adjust interval**:
```json
{
  "hint_broadcast_interval_mins": 60  // Broadcast every hour instead
}
```

## Example Scenario

### Network Bootstrap

**Hour 0**:
- 100 miners online, all using default config
- Everyone mining at baseline hashrate

**Hour 1**:
- Miner A discovers: 12 threads, 256 batch = +8% hashrate
- Miner A broadcasts hint after 5 samples
- 50 peers receive hint, queue for trial

**Hour 2**:
- 40/50 peers verify +7-9% gain, adopt config
- 10/50 peers reject (different hardware)
- Adopters now broadcast same hint to their peers

**Hour 3**:
- Elite config (12T/256B) reaches 80% of network
- Network average hashrate +8%
- New miners joining inherit elite config immediately

**Hour 4**:
- Miner B discovers: 14 threads, 512 batch = +12% on AMD
- AMD miners adopt new elite, Intel miners keep 12T/256B
- Network naturally segments by hardware family

**Result**: Self-organizing network optimization without central coordination.

## Comparison with Alternatives

| Approach | Trust Model | Latency | Privacy | Resilience |
|----------|-------------|---------|---------|------------|
| **Pure Local** | Full trust | Days | Perfect | Isolated |
| **Centralized Telemetry** | Server trust | Hours | Aggregate | Single point |
| **Blind P2P** | Peer trust | Minutes | Exposed | Vulnerable |
| **Vision Hybrid** | Local validation | <1 hour | Protected | Distributed |

Vision's approach combines the best of all worlds:
- **Fast** like blind P2P (gossip propagation)
- **Safe** like pure local (validate everything)
- **Private** like aggregate telemetry (CPU buckets only)
- **Resilient** like decentralized networks (no server)

---

**Status**: Production-ready (Phase 4 - Adversarial Resilience)
**Version**: 1.0.0
**Last Updated**: 2025-01-08
