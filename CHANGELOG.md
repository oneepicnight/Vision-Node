# Changelog

All notable changes to the VisionX blockchain node are documented in this file.

---

## v1.0.3 (2026-01-13) - Critical Sync Fix

### 🚨 CRITICAL FIX - Cumulative Work Calculation

**Problem:** Blocks in `side_blocks` had cumulative work incorrectly defaulting to 0, breaking fork chain synchronization and preventing reorgs.

**Impact:**
- Nodes stuck at old heights unable to sync (e.g., h339 while network at h1079)
- Fork chains showed incorrect work values (2, 12, 14 instead of 553,654,260+)
- Reorgs failed even when fork had more total work
- Network mining halted due to excessive height spread (723 blocks)

**Solution:**
- Added recursive `calculate_cumulative_work()` function in `src/chain/accept.rs`
- Properly traverses `side_blocks` to calculate cumulative work for fork chains
- Uses memoization via `cumulative_work` map for performance
- Returns correct cumulative values for blocks in both canonical and fork chains

**Files Changed:**
- `src/chain/accept.rs` (lines 11-28: new function, line ~290: updated call)

**Verification:**
- Tested on production network at height 1079
- Successfully synced 9 blocks (h1061-1069) with proper reorg
- Cumulative work values now correctly show 553,655,720+ (not 2, 12, 14)
- Reorgs trigger automatically when fork work exceeds canonical chain

**Log Evidence:**
```
BEFORE v1.0.3:
  [INSERT_RESULT] Block inserted into side_blocks
    inserted_work=2           ❌ (should be 553,654,260+)
    became_canonical=false    ❌ (should reorg!)

AFTER v1.0.3:
  [INSERT_RESULT] ✅ Block became CANONICAL (via reorg)
    new_tip_height=1069
    new_tip_work=553655720    ✅ (CORRECT!)
    became_canonical=true     ✅ (reorg triggered!)
```

### ✅ Side-Blocks Processing Improvements

**Added:**
- `process_side_blocks_for_tip()` function to re-evaluate side-blocks after reorgs
- Automatic chaining of blocks that can extend the new canonical tip
- Iterative processing to handle cascading chain extensions

**Impact:**
- Blocks no longer stuck in `side_blocks` after parent becomes canonical
- Orphan resolution triggers side-blocks re-check
- Faster convergence to network tip during sync

**Logging:**
- `[SIDE-BLOCKS]` prefix for processing messages
- `processed_count` and `remaining_side_blocks` displayed
- Clear indication when blocks are chained

### 🔄 Reorg Support Enhancements

**Improved:**
- Fork chain detection and evaluation
- Work comparison between canonical and fork chains
- State rollback and replay during reorgs
- Balance reconciliation after reorg (rewards applied/reverted correctly)

**Logging:**
- `[INSERT_RESULT]` shows when blocks become canonical via reorg
- `[REORG]` messages display height changes and block counts
- `new_tip_work` visible in logs for verification
- `blocks_removed` and `blocks_applied` counts shown

### 📊 Orphan Pool Processing

**Added:**
- Automatic orphan pool processing after reorgs
- Recursive acceptance of orphans whose parents became available
- Proper tracking of processed orphans vs. remaining count

**Logging:**
- `[ORPHAN-POOL]` prefix for orphan processing messages
- `processed_count` and `remaining_orphans` displayed
- Clear indication when orphans are accepted

### 🔍 Enhanced Block Validation Logging

**Added comprehensive logging for:**
- **Local Mining:**
  - `[PAYOUT]` Miner rewarded (32 LAND)
  - `[CANON]` Block became canonical
  - `[ORPHAN]` Block orphaned
  - `[REJECT]` Local block rejected

- **P2P Network:**
  - `[P2P]` Received block from peer
  - `[ACCEPT]` Block accepted - added to chain
  - `[REJECT]` Block rejected from peer

- **Mining Safety:**
  - `[JOB-CHECK]` Target verified
  - `[MINER-ERROR]` Target mismatch detected

- **Proof-Grade Evidence:**
  - `chain_id=mainnet` (network verification)
  - `pow_fp=bb113fec` (algorithm fingerprint)
  - `parent_hash` (chain continuity)
  - `miner` address (reward recipient)

### 🛡️ Security Enhancements

**Quarantine System:**
- Inbound connection quarantine check added
- Quarantined peers now rejected on BOTH inbound and outbound
- Previously only blocked outbound dials
- Protects against repeat offenders sending invalid blocks

**Files Changed:**
- `src/p2p/connection.rs` (quarantine validation on accept)

### 🔧 Mining Readiness Improvements

**Mining Gate Logic:**
- Enhanced peer height divergence checking
- Spread calculation for compatible peers
- Auto-pause when spread > 10 blocks
- Auto-resume when network converges

**Logging:**
- `[MINING-GATE]` messages show divergence status
- `min_h`, `max_h`, and `spread` clearly displayed
- Eligibility criteria visible in logs

### 🌐 Auto-Sync Enhancements

**Improvements:**
- Better sync-gate thresholds (max spread: 100 blocks)
- Peer height querying from PEER_MANAGER
- Sync status logging every 10 seconds
- Pull-based sync via P2P connections

**Logging:**
- `[AUTO-SYNC-TICK]` periodic sync checks
- `[AUTO-SYNC]` successful block pulls
- `[SYNC-GATE]` divergence warnings
- Block count and duration for each sync

### 📦 P2P Protocol

**Maintained Compatibility:**
- Protocol version: 2
- Handshake version: 103 (v1.0.3)
- Chain ID: `1cc7066e2af70fb8` (mainnet)
- P2P port: 7072 (fixed, no longer defaults to 7070)

### 🐛 Bug Fixes

**Fixed:**
- Cumulative work calculation for fork chains (CRITICAL)
- Side-blocks stuck after parent becomes canonical
- Orphans not re-evaluated after reorgs
- Reorgs failing when fork has more work
- Nodes unable to sync to network height
- Mining blocked by height spread > 10

### 📈 Performance

**Optimizations:**
- Memoization of cumulative work calculations
- Iterative side-blocks processing (no recursion overflow)
- Efficient orphan pool traversal
- Reduced redundant block validations

### 🔄 Database

**No Breaking Changes:**
- Database format unchanged (no migration required)
- Backward compatible with v1.0.2 data
- Safe upgrade path (backup recommended)

### 📝 Documentation

**Added:**
- `CRITICAL_UPGRADE_v1.0.3.md` - Comprehensive upgrade guide
- `BUILD.md` - Linux build instructions
- `DISTRIBUTION_CHECKLIST_v1.0.3.md` - Deployment plan
- `CHANGELOG.md` - Complete version history

### 🧪 Testing & Verification

**Production Testing:**
- Deployed on live network at height 1079
- Successfully synced through reorg (h1061-1069)
- Verified cumulative work values correct
- Confirmed reorgs trigger properly
- No consensus failures observed

**Network Impact:**
- Fixed nodes able to sync to tip in minutes
- Reorgs working as intended
- Side-blocks processing correctly
- Mining can resume once spread < 10

### ⚠️ Upgrade Priority

**🚨 URGENT:** v1.0.3 is a **critical upgrade** that fixes blockchain sync functionality.

**Deploy immediately to restore network mining capability.**

---

## v1.0.2 (2026-01-11) - Hotfix

### Critical Fixes
- **Fixed genesis hash compatibility**: Reverted to v1.0.0 genesis hash (`d6469ec95f56b56be4921ef40b9795902c96f2ad26582ef8db8fac46f4a7aa13`) - miner field is backward compatible with `#[serde(default)]`, no network reset required
- **Fixed wallet asset loading**: Corrected wallet deployment structure to preserve `assets/` subdirectory (fixes JavaScript module MIME type errors)

### Enhanced Diagnostics
- **Startup genesis logging**: Added `[GENESIS]` and `[CHAIN]` logs showing compiled vs database genesis hashes for troubleshooting
- **Enhanced handshake rejection logs**: Show local_compiled, local_active, and remote genesis with actionable debugging hints
- **Ghost node diagnostic script**: Added `diagnose-ghost-node.ps1` to identify wrong/old vision-node.exe processes

### Infrastructure
- Confirmed P2P binding to `0.0.0.0:7072` by default (accepts connections from all network interfaces)
- Distribution packages updated with corrected genesis and diagnostic tools

## v1.0.1 (2026-01-10)

### Major Features
- **Miner Identity Propagation**: Block rewards now go to actual miner addresses with full P2P propagation
- **Consensus Quorum Validation**: Fixed critical security vulnerability in sync/mining/exchange gates to validate chain compatibility
- **Operational Visibility**: Added periodic quorum logging (30s) and API endpoint fields for debugging
- **Mining Button Persistence**: Fixed "Make Fans Go Brrrrr" button state across tab switches

### Security Fixes
- **CRITICAL**: All gates (sync/mining/exchange) now check `compatible_peers` instead of raw TCP connection count
- Risk eliminated: Nodes can no longer sync/mine/trade with peers on different forks/chains

### Platform Support
- **Linux GNU Compatibility**: Added probestack shim for Linux GNU linker
- Enhanced `install.sh` with mandatory dependency checks

## v1.0.0
- Initial public release of Vision Node under Apache 2.0.
- Added security hardening: admin token warning when unset, HTTP defaults to localhost with explicit opt-in for remote binding.
- Redacted baked-in key files and documented secure defaults.
- Added OSS release artifacts: LICENSE, SECURITY.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, FORKING.md, README.md, NOTICE alignment.
- Added release checklist and security audit report stubs.
