# Vision Node Packaging Checklist

## Pre-Release Build
- [ ] Run `cargo build --release`
- [ ] Test binary: `.\target\release\vision-node.exe`
- [ ] Verify version number in output
- [ ] Check no compiler errors or critical warnings

## Package Structure Setup
Create package directory: `VisionNode-Constellation-v{VERSION}-TESTERS/`

## Required Files & Folders

### Core Binary
- [ ] `vision-node.exe` (from `target\release\`)

### Configuration Files
- [ ] `Cargo.toml` (project metadata)
- [ ] `Cargo.lock` (dependency lock)
- [ ] `.env` (tester configuration - bootstrap enabled)
- [ ] `seed_peers.json` (genesis seed IP)

### Directory Structure
- [ ] `config/` folder (p2p.json, network configs)
- [ ] `wallet/` folder (web wallet HTML/CSS/JS)
- [ ] `public/` folder (panel UI, status page)

### Launch Scripts
- [ ] `START-PUBLIC-NODE.bat` (tester node launcher)
- [ ] `START-GENESIS-NODE.bat` (genesis node launcher - if packaging genesis)

### Documentation
- [ ] `README.md` (setup instructions, troubleshooting)
- [ ] Version-specific docs (e.g., `v3.0.0-ED25519-NODE-APPROVAL.md`)

## Configuration Verification

### Tester `.env` Must Include:
- [ ] `VISION_PURE_SWARM_MODE=true` (enables bootstrap)
- [ ] `VISION_MIN_PEERS_FOR_MINING=1` (requires network sync)
- [ ] `VISION_BEACON_BOOTSTRAP=true` (connects to seed)
- [ ] `VISION_MIN_HEALTHY_CONNECTIONS=1`
- [ ] Documentation about testnet features

### `seed_peers.json` Must Contain:
- [ ] Genesis node IP and port (e.g., `"35.151.236.81:7072"`)
- [ ] Proper JSON format: `{"peers": ["IP:PORT"]}`

## Package Creation

### Final Steps
1. [ ] Copy all files to package directory
2. [ ] Verify all folders present: `config/`, `wallet/`, `public/`
3. [ ] Verify all core files: `Cargo.toml`, `Cargo.lock`, `.env`, `seed_peers.json`
4. [ ] Confirm NO local DB folders shipped: no `vision_data*` directories inside the package
5. [ ] Test launcher batch file (opens without errors)
6. [ ] Create ZIP: `VisionNode-Constellation-v{VERSION}-TESTERS.zip`
7. [ ] Verify ZIP size (should be ~19-20 MB with all files)
8. [ ] Extract ZIP and test run on clean system (if possible)

## Distribution

### Before Sending to Testers
- [ ] Genesis node is running and producing blocks
- [ ] Port 7072 is open for incoming connections
- [ ] Genesis node IP is reachable from external network
- [ ] Test that a second node can bootstrap to genesis

### Tester Instructions Should Include
- [ ] System requirements (Windows 64-bit, etc.)
- [ ] Port requirements (7072, 7070, 7777)
- [ ] How to extract ZIP
- [ ] How to run batch file
- [ ] Where to view node status (http://localhost:7777)
- [ ] Troubleshooting common issues
- [ ] Testnet duration (e.g., 11 days, 475,200 blocks)
- [ ] How to create/import wallet

## Genesis-Specific Package (If Needed)

### Additional Items for Genesis Package
- [ ] `START-GENESIS-NODE.bat` with environment variables:
  - `VISION_PURE_SWARM_MODE=false`
  - `VISION_MIN_PEERS_FOR_MINING=0`
  - `VISION_BEACON_BOOTSTRAP=false`
- [ ] Genesis-specific README explaining first-node setup
- [ ] Note: Genesis does NOT need wallet for block production

## Version Control
- [ ] Tag release in git: `git tag v{VERSION}`
- [ ] Document packaging date and time
- [ ] Save packaging notes (any special configurations)
- [ ] Archive package for future reference

## Post-Distribution
- [ ] Monitor genesis node stability
- [ ] Track tester connections
- [ ] Respond to tester questions/issues
- [ ] Document any bugs or improvements needed

---

## Current Version Package Contents (v3.0.0)
```
VisionNode-Constellation-v3.0.0-TESTERS/
├── vision-node.exe           (28.5 MB - main binary)
├── Cargo.toml                (project metadata)
├── Cargo.lock                (dependency versions)
├── .env                      (tester config)
├── seed_peers.json           (genesis: 35.151.236.81:7072)
├── README.md                 (instructions)
├── START-PUBLIC-NODE.bat     (launcher)
├── config/                   (p2p.json, network configs)
├── wallet/                   (web wallet UI)
└── public/                   (panel UI, status page)
```

**Total Size**: ~19.47 MB compressed

**Genesis Node**: 35.151.236.81:7072 (must be running)
**Testnet Duration**: 11 days (475,200 blocks at 2 sec/block)
**Sunset Height**: 475,210 (from launch height 10)
