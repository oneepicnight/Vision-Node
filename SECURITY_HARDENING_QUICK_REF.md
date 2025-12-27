# Security Hardening Quick Reference

**Version**: v1.0.0 mainnet  
**Date**: December 26, 2025

---

## TL;DR

✅ **Windows seed** → `%APPDATA%\Vision\external_master_seed.bin` (with ACL)  
✅ **Custody language** → "stored locally" (not "node controls")  
✅ **Pure swarm** → 7070 probe disabled unless explicit anchors  
✅ **HTLC docs** → Emphatic SHA256 warnings everywhere

---

## Seed Storage

### Windows
```
%APPDATA%\Vision\external_master_seed.bin
```

Example: `C:\Users\alice\AppData\Roaming\Vision\external_master_seed.bin`

### Unix
```
./data/external_master_seed.bin
```

### Security

- **Windows**: ACL tightened to owner-only (best-effort via `icacls`)
- **Unix**: Permissions set to `0600` (owner read/write only)

---

## Pure Swarm Mode

### Default (No anchors)
```bash
PURE_SWARM=1 ./vision-node
```
**Result**: 7070 probe **disabled** ✅

### With Explicit Anchors
```bash
PURE_SWARM=1 VISION_ANCHOR_SEEDS=1.2.3.4,5.6.7.8 ./vision-node
```
**Result**: 7070 probe **enabled** (user requested)

### Traditional Mode
```bash
./vision-node
```
**Result**: 7070 probe **enabled** (HTTP fallback available)

---

## Custody Language

### Before
> "Node controls keys (custody)"

### After
> "Keys stored locally on the user node (non-custodial)"

### Files Updated
- [src/market/deposits.rs](src/market/deposits.rs)
- All documentation

---

## HTLC SHA256 Warnings

### Module Header
```rust
//! ⚠️  CRITICAL SECURITY NOTICE ⚠️
//! ❌ NEVER USE BLAKE3 FOR HASH LOCKS ❌
```

### All Functions
```rust
/// ⚠️  SHA256 ONLY - DO NOT replace with BLAKE3!
pub fn htlc_hash_lock(preimage: &[u8]) -> [u8; 32] { ... }
```

### CI Guard
```powershell
powershell -ExecutionPolicy Bypass -File check-htlc-hashlock-safety.ps1
```

---

## Migration (Windows Only, Optional)

```powershell
# Stop node
Stop-Process -Name vision-node

# Move seed to new location
mkdir $env:APPDATA\Vision
Move-Item .\data\external_master_seed.bin $env:APPDATA\Vision\

# Restart
.\vision-node.exe
```

---

## Verification Commands

### Check seed location
```powershell
# Windows
ls $env:APPDATA\Vision\external_master_seed.bin

# Unix
ls ./data/external_master_seed.bin
```

### Test pure swarm guard
```bash
PURE_SWARM=1 ./vision-node | grep BACKBONE
# Should see: "✅ Pure swarm mode - 7070 probe disabled"
```

### Run HTLC safety check
```powershell
powershell -ExecutionPolicy Bypass -File check-htlc-hashlock-safety.ps1
# Should see: "PASS: HTLC HASH LOCK SAFETY CHECK PASSED"
```

### Verify ACL (Windows)
```powershell
icacls $env:APPDATA\Vision\external_master_seed.bin
# Should show: USERNAME:(F)
```

---

## Build

```bash
cargo build --release
```

**Result**: ✅ 38.9 MB binary in 16m 13s

---

## Files Changed

| File | Change |
|------|--------|
| [src/market/real_addresses.rs](src/market/real_addresses.rs) | Windows %APPDATA% path + ACL |
| [src/market/deposits.rs](src/market/deposits.rs) | Custody language update |
| [src/swap/watch_only.rs](src/swap/watch_only.rs) | Use new seed path |
| [src/control_plane.rs](src/control_plane.rs) | Pure swarm guard |
| [src/swap/hashlock.rs](src/swap/hashlock.rs) | SHA256 emphasis |

---

## Logs to Watch For

### Seed Creation
```
📁 Created seed directory: %APPDATA%\Vision
🔐 Generated NEW external master seed: ...
🔒 Windows ACL tightened (owner-only access)
⚠️  BACKUP THIS FILE or funds will be LOST!
```

### Pure Swarm Mode
```
[BACKBONE] ✅ Pure swarm mode - 7070 probe disabled
```

### Explicit Anchors
```
[BACKBONE] 🌐 Pure swarm mode with explicit anchors - enabling probe
```

---

## Help

**Full Documentation**: [SECURITY_HARDENING_COMPLETE.md](SECURITY_HARDENING_COMPLETE.md)

**Key References**:
- Seed storage: [src/market/real_addresses.rs](src/market/real_addresses.rs#L17-L37)
- Backbone probe: [src/control_plane.rs](src/control_plane.rs#L246-L268)
- HTLC warnings: [src/swap/hashlock.rs](src/swap/hashlock.rs#L1-L10)
