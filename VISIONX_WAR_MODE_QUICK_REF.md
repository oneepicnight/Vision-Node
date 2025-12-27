# VisionX War Mode Quick Reference

## ⚠️ CRITICAL: Consensus-Only Mining (Fork-Safety)

### Consensus Parameters (HARDCODED - ALL NODES MUST MATCH)

These parameters are **hardcoded in the source code** and used by **ALL nodes** (validators AND miners).  
Miners MUST use consensus params for block digest computation or blocks will be rejected.

**Current Consensus Params:**
```
dataset_mb: 256
scratch_mb: 32
mix_iters: 65536
reads_per_iter: 4
write_every: 4
epoch_blocks: 32
```

**Location:** `src/main.rs` → `VISIONX_CONSENSUS_PARAMS`

### Production Mining (Default)

**NO CONFIGURATION NEEDED!** Miners automatically use consensus params.

Your blocks will:
- ✅ Compute digest with consensus params
- ✅ Be validated by all network nodes
- ✅ Be accepted into the blockchain

### Experimental Dev Mode (Research Only)

For isolated testing ONLY, set:

```bash
# Enable experimental parameter tuning (ISOLATED NETWORKS ONLY!)
VISIONX_DEV_MODE=true

# WARNING: The following params are ignored unless VISIONX_DEV_MODE=true
VISIONX_MINER_DATASET_MB=128        # Custom dataset (blocks will be rejected by mainnet!)
VISIONX_MINER_SCRATCH_MB=16         # Custom scratchpad
VISIONX_MINER_MIX_ITERS=32768       # Custom iterations
VISIONX_MINER_READS_PER_ITER=2      # Custom reads
VISIONX_MINER_WRITE_EVERY=2         # Custom write frequency
```

**⚠️ WARNING:** Dev mode blocks will be REJECTED by mainnet! Use only for:
```
⚠️  WARNING: Miner params differ from consensus params!
    Blocks you mine may be REJECTED by network if params don't match!
```

## Startup Message

When the node starts, you'll see:

```
⚔️  VISIONX WAR MODE: dataset=256MB scratch=32MB reads=4 writes=every 4 iter mix=65536
    Memory: 288 MB/hash | Ops: 262144 reads + 16384 writes/hash
```

## Network Consensus Profile (CURRENT MAINNET)

**Active Consensus Parameters:**
- Dataset: 256 MB
- Scratchpad: 32 MB  
- Mix iterations: 65,536
- Reads per iteration: 4
- Write every: 4 iterations
- Epoch: 32 blocks

**Memory:** 288 MB/hash  
**Operations:** 262,144 reads + 16,384 writes  

**This is the ONLY valid configuration for mainnet block validation.**

## Experimental Miner Profiles (TESTNET/ISOLATED ONLY)

⚠️ These profiles are for **local testing only**. Blocks mined with these params **will be rejected by mainnet**.

### Lite Mode (Testing)
```bash
VISIONX_MINER_DATASET_MB=64
VISIONX_MINER_SCRATCH_MB=8
VISIONX_MINER_MIX_ITERS=16384
VISIONX_MINER_READS_PER_ITER=3
VISIONX_MINER_WRITE_EVERY=16
```
- Memory: 72 MB/hash
- Operations: 49K reads + 1K writes
- **Use case:** Development/debugging only

## Key Features

### 1. Dataset Caching ✅
- Epoch datasets are cached globally
- Validators don't rebuild 256MB for every block
- Cache holds up to 3 epochs automatically

### 2. Multi-Dependent Reads ✅
- 4-chain dependent memory reads per iteration
- GPU parallelism killer: each read depends on previous value
- Forces sequential memory access pattern

### 3. Deterministic Write-Back ✅
- Frequent random writes to scratchpad (every 4 iterations)
- Write location depends on current mix state
- Prevents GPU memory caching optimizations

### 4. Single-Track Validation ✅
- Local mined blocks and P2P blocks use identical validation
- VisionX PoW replaces legacy "leading zeros" system
- No more height divergence issues

### 5. Anti-DoS Guards ✅
- Parameter limits enforced during verification
- Max dataset: 512 MB
- Max scratchpad: 128 MB
- Max iterations: 1M
- Invalid parameters = block rejection

## Verification

All blocks are validated using:
1. VisionX hash computed from block header
2. Digest compared against difficulty target
3. Digest must match block's pow_hash field
4. Uses cached dataset for speed

## Performance Impact

### For Miners:
- War mode: ~1-5 H/s per CPU core (depends on hardware)
- Memory-bound, benefits from fast RAM
- L3 cache size matters significantly

### For Validators:
- First block in epoch: ~100-200ms (builds dataset)
- Subsequent blocks: ~5-10ms (uses cached dataset)
- Negligible impact on sync speed

## Monitoring

Check your war mode settings at runtime:
```bash
grep "VISIONX WAR MODE" logs/vision-node-*.log
```

## Troubleshooting

**Problem**: "VisionX verify: dataset_mb exceeds limit"
- **Solution**: Check VISIONX_DATASET_MB environment variable

**Problem**: Slow mining
- **Solution**: War mode is working as intended! Try balanced/lite mode for testing

**Problem**: High memory usage
- **Solution**: Reduce VISIONX_DATASET_MB and VISIONX_SCRATCH_MB

**Problem**: Blocks rejected with "pow_hash mismatch"
- **Solution**: Ensure miner and validator use same VisionX parameters

## Implementation Files

- `src/pow/visionx.rs` - Core VisionX algorithm
- `src/pow/mod.rs` - PoW target comparison fixes
- `src/main.rs` - Integration and validation
- `src/consensus_pow/` - Block submission pipeline
- `src/miner/` - Mining engine

---

**War mode: ARMED ⚔️**
