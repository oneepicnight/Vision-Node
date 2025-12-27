# Mining Performance Tuning - Implementation Complete

## Overview
Added comprehensive mining performance tuning system to Vision Node v2.7.0, allowing users to optimize mining behavior based on their hardware configuration (Laptop, Balanced, Beast modes).

## Frontend Changes (panel.html)

### New UI Section - Performance Tuning
Added performance tuning controls to the Miner Configuration section:

1. **Mining Profile** (dropdown)
   - Laptop: Uses 50% of available CPU cores (gentle mining)
   - Balanced: Uses 75% of available CPU cores (optimal performance/efficiency)
   - Beast: Uses 100% of available CPU cores (maximum hashrate)

2. **Mining Threads** (numeric input, 0 = auto)
   - Allows manual thread count override
   - Set to 0 to use automatic detection based on profile
   - Takes precedence over profile settings

3. **SIMD Batch Size** (numeric input, 1-1024, default 4)
   - Controls nonce processing batch size
   - Higher values = more SIMD-friendly workloads
   - Tunable for CPU architecture optimization

### JavaScript Functions
- `initMinerControls()` - Loads current config from backend
- `saveMinerConfig()` - Saves performance settings to backend
- Modified Apply button - Calls `saveMinerConfig()` before updating threads

## Backend Changes

### 1. API Routes (`src/routes/miner.rs`)

**Extended MinerConfigResponse struct:**
```rust
pub struct MinerConfigResponse {
    pub threads: usize,
    pub enabled: bool,
    pub max_threads: usize,
    pub mining_profile: Option<String>,      // "laptop", "balanced", "beast"
    pub mining_threads: Option<usize>,       // 0 or null = auto
    pub simd_batch_size: Option<u64>,        // 1-1024, default 4
}
```

**Updated MinerConfigUpdate struct:**
```rust
pub struct MinerConfigUpdate {
    pub threads: Option<usize>,
    pub mining_profile: Option<String>,
    pub mining_threads: Option<usize>,
    pub simd_batch_size: Option<u64>,
}
```

**GET /api/miner/config:**
- Returns current thread count + performance tuning fields
- Loads mining_profile, mining_threads, simd_batch_size from `miner.json`

**POST /api/miner/config:**
- Accepts all performance tuning fields
- Updates config in memory and persists to `miner.json`
- Returns updated configuration

### 2. Configuration (`src/config/miner.rs`)

Performance fields already existed in MinerConfig:
- `mining_profile: Option<String>` - Default: "balanced"
- `mining_threads: Option<usize>` - Default: None (auto)
- `simd_batch_size: Option<u64>` - Default: 4

Config is stored in `miner.json` at node root directory.

### 3. Miner Manager (`src/miner/manager.rs`)

**Updated `start()` method:**
Applies mining profile logic when starting mining:

1. **Explicit Override** (highest priority):
   - If `mining_threads` is set to non-zero value, use that

2. **Profile Percentage** (if no override):
   - Laptop → 50% of available cores
   - Balanced → 75% of available cores (default)
   - Beast → 100% of available cores

3. **Fallback**:
   - If config load fails, use requested thread count

## Usage

### From Panel UI:
1. Open `http://localhost:8080/panel.html`
2. Navigate to Miner Configuration section
3. Select Mining Profile (Laptop/Balanced/Beast)
4. Optionally override thread count (0 = auto based on profile)
5. Optionally adjust SIMD batch size for your CPU
6. Click "Apply Configuration"
7. Start mining - profile settings will be applied

### From API:

**Get current config:**
```bash
curl http://localhost:8080/api/miner/config
```

**Set profile to Beast mode:**
```bash
curl -X POST http://localhost:8080/api/miner/config \
  -H "Content-Type: application/json" \
  -d '{"mining_profile":"beast"}'
```

**Override threads:**
```bash
curl -X POST http://localhost:8080/api/miner/config \
  -H "Content-Type: application/json" \
  -d '{"mining_profile":"balanced","mining_threads":8}'
```

**Tune SIMD batch size:**
```bash
curl -X POST http://localhost:8080/api/miner/config \
  -H "Content-Type: application/json" \
  -d '{"simd_batch_size":16}'
```

## Configuration Priority

When mining starts, thread count is determined by:

1. **mining_threads** (if set and > 0) → Use explicit value
2. **mining_profile** → Calculate percentage of cores
3. **Fallback** → Use default/requested threads

Example for 16-core CPU:
- Laptop mode: 8 threads (50%)
- Balanced mode: 12 threads (75%)
- Beast mode: 16 threads (100%)
- mining_threads=10: 10 threads (override)

## Testing

1. **Build:**
   ```powershell
   cargo build --release
   ```

2. **Start node:**
   ```powershell
   .\target\release\vision-node.exe
   ```

3. **Test GET endpoint:**
   ```bash
   curl http://localhost:8080/api/miner/config
   ```
   Expected: JSON with mining_profile, mining_threads, simd_batch_size fields

4. **Test POST endpoint:**
   ```bash
   curl -X POST http://localhost:8080/api/miner/config \
     -H "Content-Type: application/json" \
     -d '{"mining_profile":"beast","mining_threads":0,"simd_batch_size":8}'
   ```
   Expected: Updated config returned, miner.json file updated

5. **Test mining start:**
   - Set profile to "laptop" (50% cores)
   - Start mining
   - Verify thread count is 50% of available cores
   - Check logs for actual threads used

## Files Modified

1. `public/panel.html` - Added performance tuning UI section
2. `src/routes/miner.rs` - Extended API with performance fields
3. `src/miner/manager.rs` - Applied profile logic in `start()` method
4. `src/config/miner.rs` - (No changes, fields already existed)

## Version

Implemented in Vision Node v2.7.0 (Chain ID: VISION-CONSTELLATION-V2.7-TESTNET1)

## Package

Updated package: `VisionNode-Constellation-v2.7.0-WIN64.zip` (15.39 MB)
