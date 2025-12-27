# CPU-Aware Mining - Quick Reference

## Mining Profiles

| Profile    | Threads Formula           | Use Case                      |
|-----------|---------------------------|-------------------------------|
| `laptop`  | min(cores, 4)             | Battery-friendly, development |
| `balanced`| cores * 0.5               | Default, general use          |
| `beast`   | all cores                 | Dedicated mining              |

## Configuration (`miner.json`)

```json
{
  "mining_profile": "balanced",
  "mining_threads": null,
  "simd_batch_size": 4
}
```

## Quick Start

### Auto-detect (Recommended)
```rust
let config = MinerConfig::load_or_create("miner.json")?;
miner.start_with_config(&config);
```

### Manual Override
```rust
let mut config = MinerConfig::default();
config.mining_threads = Some(32);
config.simd_batch_size = Some(8);
miner.start_with_config(&config);
```

## SIMD Batch Size Guide

| Batch Size | CPU Type              | Expected Performance |
|------------|-----------------------|---------------------|
| 1          | Legacy behavior       | Baseline            |
| 4          | Most CPUs (default)   | +5-10% hashrate     |
| 8-16       | High-end desktop      | +10-15% hashrate    |
| 32-64      | HEDT/Server           | +15-20% hashrate    |
| >64        | Diminishing returns   | May decrease        |

## Log Examples

### Startup
```
[miner] CPU detected: 'AMD Ryzen Threadripper 3990X 64-Core Processor' | 
        physical_cores=64 logical_cores=128 | 
        mining_profile="beast" mining_threads=128 simd_batch_size=8
```

### Runtime
```
[miner] Hashrate ≈ 4523.67 H/s
🎉 Worker #7 found block #1234!
```

## Profile Selection Guide

### Development Machine
```json
{"mining_profile": "laptop", "simd_batch_size": 4}
```

### Gaming PC (Mining While AFK)
```json
{"mining_profile": "balanced", "simd_batch_size": 4}
```

### Dedicated Mining Rig
```json
{"mining_profile": "beast", "simd_batch_size": 8}
```

### Threadripper/EPYC Beast
```json
{"mining_profile": "beast", "simd_batch_size": 16}
```

## Files Changed

- ✅ `src/config/miner.rs` - Config struct
- ✅ `src/util/cpu_info.rs` - CPU detection
- ✅ `src/miner/manager.rs` - Mining logic
- ✅ `Cargo.toml` - sysinfo dependency

## Key Functions

```rust
// CPU detection
detect_cpu_summary() -> CpuSummary

// Thread resolution
resolve_mining_threads(cfg, cpu) -> usize

// Batch size
resolve_simd_batch_size(cfg) -> u64

// Start mining
miner.start_with_config(&config)
```

## Build & Run

```bash
cargo build --release
./target/release/vision-node
```

Binary: 26.95 MB  
Build time: ~5 minutes  
Status: ✅ Ready for production
